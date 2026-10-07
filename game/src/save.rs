//! Save file support: progression, unlocks, and settings persist across
//! sessions.
//!
//! 80 levels is too many to replay from scratch every launch. This module
//! owns the on-disk format, the platform-specific path, and the Bevy
//! wiring that keeps the in-memory game state and the file in sync.
//!
//! Design notes:
//! * Format is JSON (debuggable, forward-compatible). `version` gates
//!   migrations; unknown versions load as defaults, never panic.
//! * Every load path is total: missing file, corrupt JSON, wrong types,
//!   and unknown enum variants all degrade to defaults with a log line.
//!   A save file is user-editable by definition — it is untrusted input.
//! * A save that cannot be loaded (unparseable, invalid UTF-8, wrong
//!   version) is moved aside to `save.json.corrupt` before defaults are
//!   used, so the next successful write never silently destroys the
//!   evidence.
//! * Writes are atomic (temp file + rename) so a crash mid-write never
//!   leaves a half-written save.
//! * `LIGHTSHOW_SAVE_DIR` overrides the path (testing, custom launchers).

use crate::cheat_codes::UnlockedSpecialists;
use crate::waifu::{Companion, Cores};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Current save format version. Bump when `SaveData` changes shape and
/// add a migration in `load`.
pub const SAVE_VERSION: u32 = 1;

/// Save file name inside the platform data directory.
pub const SAVE_FILENAME: &str = "save.json";

/// Android package name (matches `[package.metadata.android]`).
#[cfg(target_os = "android")]
const ANDROID_PACKAGE: &str = "ai.qompass.lightshow";

/// Volume settings. Defaults match the current in-code constants
/// (`AMBIENCE_VOLUME`, `SFX_VOLUME` in audio.rs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
        }
    }
}

/// Everything persisted across sessions.
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct SaveData {
    pub version: u32,
    /// Level IDs the player has completed (see `LevelDef::id`).
    pub completed_levels: Vec<String>,
    /// Specialist companions unlocked via cheat codes, by debug name
    /// (`"Clara"`). Stored as strings so an unknown future name degrades
    /// to "skip" instead of failing the whole load.
    pub unlocked_specialists: Vec<String>,
    /// Core balance (see `Cores`): dead parts traded in at the
    /// Warehouse. Renamed from `favor_points`; the serde alias keeps
    /// saves written before the rename loading, and `default` covers
    /// saves old enough to predate the field entirely.
    #[serde(default, alias = "favor_points")]
    pub cores: u32,
    pub settings: Settings,
    /// Warehouse permanents owned, by item id (see `warehouse::SHOP`).
    /// `serde(default)` keeps pre-Warehouse saves loading.
    #[serde(default)]
    pub owned_gear: Vec<String>,
    /// Warehouse tools whose quiz the player has passed, by tool id.
    #[serde(default)]
    pub warehouse_quiz_passed: Vec<String>,
    /// Warehouse consumables carried, by item id; duplicates are the count.
    #[serde(default)]
    pub consumables: Vec<String>,
    /// Warehouse backdrop shuffle bag: indices into
    /// `warehouse::BACKDROPS` not yet shown this cycle, in the order
    /// they were bagged (drawn from the end). Persisted so the
    /// no-repeat rotation survives restarts. `serde(default)` keeps
    /// pre-rotation saves loading; an empty bag refills on entry.
    /// Contents are untrusted input — the bag sanitizes on load.
    #[serde(default)]
    pub warehouse_backdrop_bag: Vec<u8>,
    /// Backdrop index on screen at the last Warehouse visit.
    #[serde(default)]
    pub warehouse_backdrop_current: u8,
    /// Companions whose first-level tutorial has been seen (marked
    /// on her level-1 first clear), by picker stem (`"seraphine"`).
    /// `serde(default)` keeps pre-tutorial saves loading — and
    /// `SAVE_VERSION` is deliberately NOT bumped for this field: a
    /// version mismatch loads defaults and orphans every existing
    /// save's completions (there is no migration machinery).
    #[serde(default)]
    pub tutorials_seen: Vec<String>,
    /// Astra badge bitmask per level id (see `crate::astra`:
    /// Diagnosis / Workmanship-or-Configuration / Verification).
    /// `serde(default)` keeps pre-Astra saves loading with no
    /// badges — deliberately NO SAVE_VERSION bump and no migration
    /// (coordinator ruling: a version bump would orphan every
    /// existing save, since `load()` returns defaults on mismatch).
    /// Follows the `owned_gear` precedent.
    #[serde(default)]
    pub level_badges: HashMap<String, u8>,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            completed_levels: Vec::new(),
            unlocked_specialists: Vec::new(),
            cores: 0,
            settings: Settings::default(),
            owned_gear: Vec::new(),
            warehouse_quiz_passed: Vec::new(),
            consumables: Vec::new(),
            warehouse_backdrop_bag: Vec::new(),
            warehouse_backdrop_current: 0,
            tutorials_seen: Vec::new(),
            level_badges: HashMap::new(),
        }
    }
}

impl SaveData {
    /// Record a level completion. Returns true if this is new.
    pub fn complete_level(&mut self, level_id: &str) -> bool {
        if self.completed_levels.iter().any(|id| id == level_id) {
            return false;
        }
        self.completed_levels.push(level_id.to_owned());
        true
    }

    /// True if the level was completed in a previous session.
    pub fn is_completed(&self, level_id: &str) -> bool {
        self.completed_levels.iter().any(|id| id == level_id)
    }

    /// The Astra badge bitmask recorded for a level (0 = none).
    pub fn badges_for(&self, level_id: &str) -> u8 {
        self.level_badges.get(level_id).copied().unwrap_or(0) & BADGE_MASK
    }

    /// Merge newly earned badge bits into a level's record (badges
    /// are never revoked). Returns true if the record changed.
    pub fn record_badges(&mut self, level_id: &str, earned: u8) -> bool {
        let earned = earned & BADGE_MASK;
        if earned == 0 || level_id.is_empty() {
            return false;
        }
        let entry = self.level_badges.entry(level_id.to_owned()).or_insert(0);
        let merged = *entry | earned;
        if merged == *entry {
            return false;
        }
        *entry = merged;
        true
    }

    /// Record a specialist unlock. Returns true if this is new.
    pub fn unlock_specialist(&mut self, companion: Companion) -> bool {
        let name = format!("{:?}", companion);
        if self.unlocked_specialists.iter().any(|n| n == &name) {
            return false;
        }
        self.unlocked_specialists.push(name);
        true
    }

    /// Record a companion's tutorial as seen (by picker stem).
    /// Returns true if this is new.
    pub fn mark_tutorial_seen(&mut self, companion_stem: &str) -> bool {
        if self.tutorials_seen.iter().any(|s| s == companion_stem) {
            return false;
        }
        self.tutorials_seen.push(companion_stem.to_owned());
        true
    }

    /// True if the companion's tutorial was marked seen.
    pub fn is_tutorial_seen(&self, companion_stem: &str) -> bool {
        self.tutorials_seen.iter().any(|s| s == companion_stem)
    }

    /// Specialists in this save, as a set (unknown names dropped).
    pub fn unlocked_set(&self) -> HashSet<Companion> {
        self.unlocked_specialists
            .iter()
            .filter_map(|name| match name.as_str() {
                "Clara" => Some(Companion::Clara),
                "Aino" => Some(Companion::Aino),
                "Hikari" => Some(Companion::Hikari),
                "Lea" => Some(Companion::Lea),
                _ => None,
            })
            .collect()
    }
}

/// Resolve where the save file lives, or `None` when no writable
/// location is available (game keeps in-memory progress and logs once).
pub fn save_path() -> Option<PathBuf> {
    // Explicit override wins (tests, custom launchers, debugging).
    if let Ok(dir) = std::env::var("LIGHTSHOW_SAVE_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return Some(PathBuf::from(dir).join(SAVE_FILENAME));
        }
    }
    // Desktop platforms: OS data dir (`dirs` handles Linux/Windows/macOS).
    if let Some(dir) = dirs::data_dir() {
        return Some(dir.join("light-show").join(SAVE_FILENAME));
    }
    // Android: internal storage. `dirs` returns None on Android, so use
    // the package's files dir directly (always writable, private).
    #[cfg(target_os = "android")]
    {
        return Some(
            PathBuf::from(format!("/data/data/{}/files", ANDROID_PACKAGE)).join(SAVE_FILENAME),
        );
    }
    #[allow(unreachable_code)]
    None
}

/// What went wrong saving. Loading never returns this — it degrades to
/// defaults instead (see `load`).
#[derive(Debug)]
pub enum SaveError {
    NoPath,
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::NoPath => write!(f, "no writable save location on this platform"),
            SaveError::Io(e) => write!(f, "save IO error: {}", e),
            SaveError::Json(e) => write!(f, "save serialization error: {}", e),
        }
    }
}

/// Sidecar path next to the save file: `save.json` + `"tmp"` is the
/// atomic-write temp file, + `"corrupt"` is where an unloadable save is
/// preserved (see `load`). Same directory, so renames stay on one
/// filesystem and remain atomic.
fn sidecar_path(path: &Path, extra_extension: &str) -> PathBuf {
    path.with_extension(format!("json.{extra_extension}"))
}

/// Write `data` atomically (temp file + rename). Creates parent dirs.
pub fn save(data: &SaveData) -> Result<(), SaveError> {
    let path = save_path().ok_or(SaveError::NoPath)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(SaveError::Io)?;
    }
    let text = serde_json::to_string_pretty(data).map_err(SaveError::Json)?;
    // Temp file in the same directory so rename is atomic on all platforms.
    let tmp = sidecar_path(&path, "tmp");
    std::fs::write(&tmp, text).map_err(SaveError::Io)?;
    std::fs::rename(&tmp, &path).map_err(SaveError::Io)?;
    Ok(())
}

/// Delete the save file and its sidecars (atomic-write temp, preserved
/// corrupt copy). Backs the menu's New Game reset: after this returns
/// `Ok`, `load` yields defaults. Missing files are success — erasing a
/// game that was never saved is not an error. A sidecar that cannot be
/// removed does not stop the remaining removals; the first failure is
/// reported after all three have been attempted.
pub fn erase() -> Result<(), SaveError> {
    let path = save_path().ok_or(SaveError::NoPath)?;
    let candidates = [
        path.clone(),
        sidecar_path(&path, "tmp"),
        sidecar_path(&path, "corrupt"),
    ];
    let mut first_error: Option<std::io::Error> = None;
    for candidate in candidates {
        match std::fs::remove_file(&candidate) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    match first_error {
        None => Ok(()),
        Some(e) => Err(SaveError::Io(e)),
    }
}

/// Move a save that failed to load aside to `save.json.corrupt` so it
/// survives for inspection instead of being overwritten by the next
/// successful write. Best-effort: failure is logged, never propagated —
/// starting the game matters more than the forensics.
fn preserve_unloadable_save(path: &Path) {
    let preserved = sidecar_path(path, "corrupt");
    // `rename` refuses to replace an existing destination on Windows;
    // drop a previously preserved copy first so the newest corruption
    // is always the one kept.
    let _ = std::fs::remove_file(&preserved);
    match std::fs::rename(path, &preserved) {
        Ok(()) => warn!("preserved unloadable save as {}", preserved.display()),
        Err(e) => warn!(
            "could not preserve unloadable save {} ({})",
            path.display(),
            e
        ),
    }
}

/// Load the save file. Total: any failure (no path, missing file,
/// corrupt JSON, wrong version) yields defaults with a log line.
/// Never panics — a save file is untrusted input.
pub fn load() -> SaveData {
    let path = match save_path() {
        Some(p) => p,
        None => {
            warn!("no writable save location; progress will not persist");
            return SaveData::default();
        }
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // First launch — not an error.
            return SaveData::default();
        }
        Err(e) => {
            warn!(
                "cannot read save file {} ({}); starting fresh",
                path.display(),
                e
            );
            if e.kind() == std::io::ErrorKind::InvalidData {
                // Not valid UTF-8: unparseable by definition. Preserve
                // it like any other corrupt save instead of leaving it
                // in place to be overwritten by the next write. Other
                // read errors (permissions, a directory in the way) may
                // be transient, so those files stay where they are.
                preserve_unloadable_save(&path);
            }
            return SaveData::default();
        }
    };
    let mut data: SaveData = match serde_json::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            warn!(
                "corrupt save file {} ({}); starting fresh",
                path.display(),
                e
            );
            preserve_unloadable_save(&path);
            return SaveData::default();
        }
    };
    if data.version != SAVE_VERSION {
        warn!(
            "save version {} != supported {}; starting fresh",
            data.version, SAVE_VERSION
        );
        // Preserve the mismatch too: a save written by a newer build
        // must survive being opened once by an older one.
        preserve_unloadable_save(&path);
        return SaveData::default();
    }
    validate(&mut data);
    data
}

/// Clamp and sanitize loaded data in place.
fn validate(data: &mut SaveData) {
    // Volumes must be finite and in range.
    for v in [
        &mut data.settings.master_volume,
        &mut data.settings.music_volume,
        &mut data.settings.sfx_volume,
    ] {
        if !v.is_finite() {
            *v = 1.0;
        } else {
            *v = v.clamp(0.0, 1.0);
        }
    }
    // Level IDs: non-empty, bounded length, deduplicated.
    data.completed_levels
        .retain(|id| !id.is_empty() && id.len() <= 128);
    data.completed_levels.sort();
    data.completed_levels.dedup();
    // Unknown specialist names are dropped by `unlocked_set`; keep the
    // raw list tidy too.
    data.unlocked_specialists
        .retain(|n| !n.is_empty() && n.len() <= 32);
    data.unlocked_specialists.sort();
    data.unlocked_specialists.dedup();
    // Tutorial stems: same sanitizing as specialist names —
    // non-empty, bounded length, deduplicated (sorted for a stable
    // on-disk shape; the sequencer only does membership checks).
    data.tutorials_seen
        .retain(|s| !s.is_empty() && s.len() <= 32);
    data.tutorials_seen.sort();
    data.tutorials_seen.dedup();
    // Warehouse ids: same bounds. Gear and passed quizzes are sets;
    // consumables are a multiset, so they are only length-capped.
    for list in [&mut data.owned_gear, &mut data.warehouse_quiz_passed] {
        list.retain(|id| !id.is_empty() && id.len() <= 32);
        list.sort();
        list.dedup();
    }
    data.consumables
        .retain(|id| !id.is_empty() && id.len() <= 32);
    data.consumables.truncate(CONSUMABLES_MAX);
    // Backdrop rotation: indices must name pool entries, each at most
    // once (draw order is meaningful, so no sort); current in range.
    let backdrop_count = crate::warehouse::BACKDROPS.len() as u8;
    let mut seen = HashSet::new();
    data.warehouse_backdrop_bag
        .retain(|&idx| idx < backdrop_count && seen.insert(idx));
    if data.warehouse_backdrop_current >= backdrop_count {
        data.warehouse_backdrop_current = 0;
    }
    // Astra badges: keys are level ids (same bounds as completions),
    // values are bitmasks — unknown bits are masked off, empty
    // records dropped, and the map is size-capped. Untrusted input,
    // sanitized like everything else in this function.
    data.level_badges
        .retain(|id, mask| !id.is_empty() && id.len() <= 128 && (*mask & BADGE_MASK) != 0);
    for mask in data.level_badges.values_mut() {
        *mask &= BADGE_MASK;
    }
    if data.level_badges.len() > LEVEL_BADGES_MAX {
        let mut keys: Vec<String> = data.level_badges.keys().cloned().collect();
        keys.sort();
        keys.truncate(LEVEL_BADGES_MAX);
        data.level_badges.retain(|id, _| keys.contains(id));
    }
}

/// The Astra badge bits a save record may hold (see `crate::astra`).
pub const BADGE_MASK: u8 = 0b111;

/// Upper bound on badge records: comfortably above the level count.
const LEVEL_BADGES_MAX: usize = 512;

/// Upper bound on carried consumables across all kinds: every shop
/// consumable at its stack cap. Bounds a hand-edited save.
const CONSUMABLES_MAX: usize =
    crate::warehouse::SHOP.len() * crate::warehouse::CONSUMABLE_STACK_MAX;

// ---------------------------------------------------------------------------
// Bevy wiring
// ---------------------------------------------------------------------------

/// Request a save write. Systems emit this after mutating `SaveData`,
/// `UnlockedSpecialists`, or `Cores`.
#[derive(Message)]
pub struct SaveRequest;

/// Loads the save at startup, seeds runtime resources from it, and
/// persists changes back to disk.
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        let data = load();
        // Seed the runtime unlock state so persisted codes apply.
        // `UnlockedSpecialists` is init_resource'd by CompanionSelectPlugin;
        // startup systems run after all plugins build, so it exists here.
        let seed = PendingUnlockSeed(data.unlocked_set());
        app.insert_resource(data);
        app.insert_resource(seed);
        app.add_message::<SaveRequest>();
        app.add_systems(Startup, seed_unlocks_from_save);
        app.add_systems(
            Update,
            (sync_progression_to_save, write_save_on_request).chain(),
        );
    }
}

/// Unlocks to seed into the runtime resource on startup. Removed
/// after seeding so it doesn't linger.
#[derive(Resource)]
struct PendingUnlockSeed(HashSet<Companion>);

/// Copy persisted unlocks and the core balance into the runtime
/// resources on startup. Cores must be seeded before the first
/// `sync_progression_to_save` run, which otherwise sees the default 0 as
/// a change and overwrites the saved balance with it.
fn seed_unlocks_from_save(
    mut commands: Commands,
    seed: Res<PendingUnlockSeed>,
    mut specialists: ResMut<UnlockedSpecialists>,
    save: Res<SaveData>,
    mut cores: ResMut<Cores>,
) {
    cores.0 = save.cores;
    let before = specialists.unlocked.len();
    specialists.unlocked.extend(seed.0.iter().copied());
    let added = specialists.unlocked.len() - before;
    if added > 0 {
        info!("restored {} specialist unlock(s) from save", added);
    }
    commands.remove_resource::<PendingUnlockSeed>();
}

/// Keep `SaveData` in sync with runtime progression. Change-detected so
/// the disk write only happens when something actually changed.
fn sync_progression_to_save(
    unlocked: Res<UnlockedSpecialists>,
    cores: Res<Cores>,
    mut save: ResMut<SaveData>,
    mut writer: MessageWriter<SaveRequest>,
) {
    if !unlocked.is_changed() && !cores.is_changed() {
        return;
    }
    let mut dirty = false;
    for c in unlocked.unlocked.iter() {
        // Base four are always available; only specialists persist.
        if matches!(
            c,
            Companion::Clara | Companion::Aino | Companion::Hikari | Companion::Lea
        ) {
            dirty |= save.unlock_specialist(*c);
        }
    }
    if save.cores != cores.0 {
        save.cores = cores.0;
        dirty = true;
    }
    if dirty {
        writer.write(SaveRequest);
    }
}

/// Perform the actual disk write. Errors are logged, never panic —
/// a failed save must not crash the game.
fn write_save_on_request(mut reader: MessageReader<SaveRequest>, data: Res<SaveData>) {
    if reader.read().count() == 0 {
        return;
    }
    match save(&data) {
        Ok(()) => info!("progress saved"),
        Err(e) => warn!("failed to save progress: {}", e),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Tests that mutate `LIGHTSHOW_SAVE_DIR` must not run concurrently:
    /// env vars are process-global and Rust runs tests in threads.
    /// Shared with other modules' tests that write saves.
    pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn complete_level_dedups() {
        let mut s = SaveData::default();
        assert!(s.complete_level("c1l1"));
        assert!(!s.complete_level("c1l1"));
        assert!(s.is_completed("c1l1"));
        assert!(!s.is_completed("c1l2"));
    }

    #[test]
    fn unlock_specialist_dedups() {
        let mut s = SaveData::default();
        assert!(s.unlock_specialist(Companion::Clara));
        assert!(!s.unlock_specialist(Companion::Clara));
        assert!(s.unlocked_set().contains(&Companion::Clara));
    }

    #[test]
    fn unlocked_set_drops_unknown_names() {
        let mut s = SaveData::default();
        s.unlocked_specialists.push("Clara".to_owned());
        s.unlocked_specialists.push("FutureCompanion".to_owned());
        let set = s.unlocked_set();
        assert!(set.contains(&Companion::Clara));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn mark_tutorial_seen_dedups() {
        let mut s = SaveData::default();
        assert!(s.mark_tutorial_seen("seraphine"));
        assert!(!s.mark_tutorial_seen("seraphine"));
        assert!(s.is_tutorial_seen("seraphine"));
        assert!(!s.is_tutorial_seen("clara"));
    }

    #[test]
    fn validate_sanitizes_tutorials_seen() {
        // Adversarial: duplicate and garbage stems from a hand-edited
        // save are dropped/deduped, never trusted as-is.
        let mut s = SaveData {
            tutorials_seen: vec![
                "seraphine".into(),
                "seraphine".into(),
                "".into(),
                "x".repeat(33),
                "clara".into(),
            ],
            ..SaveData::default()
        };
        validate(&mut s);
        assert_eq!(
            s.tutorials_seen,
            vec!["clara".to_owned(), "seraphine".to_owned()]
        );
    }

    #[test]
    fn old_format_save_without_tutorials_seen_loads_with_completions_intact() {
        // A save written before the tutorial pass has no
        // `tutorials_seen` field at all: serde(default) must load it
        // with completions intact (no SAVE_VERSION bump — a bump
        // would orphan this save entirely).
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-pre-tutorial");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(SAVE_FILENAME),
            r#"{"version":1,"completed_levels":["w1l1"],"unlocked_specialists":[],"favor_points":7,"settings":{"master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0}}"#,
        )
        .unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert!(loaded.is_completed("w1l1"));
        assert_eq!(loaded.cores, 7);
        assert!(loaded.tutorials_seen.is_empty());
        assert!(!loaded.is_tutorial_seen("seraphine"));
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tutorials_seen_round_trips() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-tutorials-seen");
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let mut s = SaveData::default();
        assert!(s.mark_tutorial_seen("seraphine"));
        assert!(s.mark_tutorial_seen("lea"));
        save(&s).expect("save should succeed");
        let loaded = load();
        assert!(loaded.is_tutorial_seen("seraphine"));
        assert!(loaded.is_tutorial_seen("lea"));
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_clamps_volumes() {
        let mut s = SaveData::default();
        s.settings.master_volume = f32::NAN;
        s.settings.music_volume = 2.5;
        s.settings.sfx_volume = -1.0;
        validate(&mut s);
        assert_eq!(s.settings.master_volume, 1.0);
        assert_eq!(s.settings.music_volume, 1.0);
        assert_eq!(s.settings.sfx_volume, 0.0);
    }

    #[test]
    fn validate_dedups_levels() {
        let mut s = SaveData::default();
        s.completed_levels = vec!["b".into(), "a".into(), "b".into(), "".into()];
        validate(&mut s);
        assert_eq!(s.completed_levels, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn save_round_trip() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-test");
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let mut s = SaveData::default();
        s.complete_level("c1l1");
        s.unlock_specialist(Companion::Aino);
        s.cores = 30;
        save(&s).expect("save should succeed");
        let loaded = load();
        assert!(loaded.is_completed("c1l1"));
        assert!(loaded.unlocked_set().contains(&Companion::Aino));
        assert_eq!(loaded.cores, 30);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert_eq!(loaded.completed_levels.len(), 0);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SAVE_FILENAME), "{not json").unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert_eq!(loaded.completed_levels.len(), 0);
        assert_eq!(loaded.version, SAVE_VERSION);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_wrong_version_returns_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-version");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(SAVE_FILENAME),
            r#"{"version":999,"completed_levels":["c1l1"],"unlocked_specialists":[],"favor_points":0,"settings":{"master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0}}"#,
        )
        .unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert_eq!(loaded.completed_levels.len(), 0);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pre_astra_save_loads_with_empty_badges_and_completions_intact() {
        // Coordinator ruling: level_badges arrives via serde(default)
        // with NO version bump — a save written before Astra (same
        // version 1, no level_badges key) must load with its
        // completions untouched and an empty badge map.
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-pre-astra");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(SAVE_FILENAME),
            r#"{"version":1,"completed_levels":["c1l1","m1l2"],"unlocked_specialists":[],"favor_points":25,"settings":{"master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0}}"#,
        )
        .unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert!(loaded.is_completed("c1l1"));
        assert!(loaded.is_completed("m1l2"));
        assert!(loaded.level_badges.is_empty());
        assert_eq!(loaded.badges_for("c1l1"), 0);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn badges_merge_by_or_and_round_trip() {
        let mut s = SaveData::default();
        assert!(s.record_badges("c1l5", 0b001));
        assert!(!s.record_badges("c1l5", 0b001), "no change, no write");
        assert!(s.record_badges("c1l5", 0b100));
        assert_eq!(s.badges_for("c1l5"), 0b101);
        assert!(!s.record_badges("c1l5", 0), "zero earns nothing");
        assert!(!s.record_badges("", 0b111), "empty id rejected");
        // Unknown bits are masked at the boundary: nothing records.
        assert!(!s.record_badges("m1l2", 0b1111_1000));
        assert_eq!(s.badges_for("m1l2"), 0);
        // Persistence: badges survive a serialize/parse round trip.
        let text = serde_json::to_string(&s).unwrap();
        let back: SaveData = serde_json::from_str(&text).unwrap();
        assert_eq!(back.badges_for("c1l5"), 0b101);
    }

    #[test]
    fn validate_sanitizes_hostile_badge_records() {
        // Adversarial: a hand-edited save cannot smuggle unknown
        // bits, empty ids, or zero records past validate().
        let mut s = SaveData::default();
        s.level_badges.insert("c1l1".into(), 0b1111_1111);
        s.level_badges.insert("".into(), 0b001);
        s.level_badges.insert("ghost".into(), 0b1000_0000);
        validate(&mut s);
        assert_eq!(s.badges_for("c1l1"), 0b111);
        assert!(!s.level_badges.contains_key(""));
        assert!(!s.level_badges.contains_key("ghost"));
    }

    #[test]
    fn pre_warehouse_save_loads_with_empty_warehouse_fields() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-pre-warehouse");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(SAVE_FILENAME),
            r#"{"version":1,"completed_levels":["c1l1"],"unlocked_specialists":[],"favor_points":25,"settings":{"master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0}}"#,
        )
        .unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert!(loaded.is_completed("c1l1"));
        assert_eq!(loaded.cores, 25);
        assert!(loaded.owned_gear.is_empty());
        assert!(loaded.warehouse_quiz_passed.is_empty());
        assert!(loaded.consumables.is_empty());
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn warehouse_fields_round_trip() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-warehouse");
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let s = SaveData {
            owned_gear: vec!["headlamp".into(), "spare_battery".into()],
            warehouse_quiz_passed: vec!["scout_pro_3".into()],
            consumables: vec![
                "field_coffee".into(),
                "field_coffee".into(),
                "spare_remotes".into(),
            ],
            ..SaveData::default()
        };
        save(&s).expect("save should succeed");
        let loaded = load();
        assert_eq!(loaded.owned_gear, s.owned_gear);
        assert_eq!(loaded.warehouse_quiz_passed, s.warehouse_quiz_passed);
        // Duplicates are the count: they must survive the load.
        assert_eq!(loaded.consumables, s.consumables);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_bounds_hostile_warehouse_lists() {
        let mut s = SaveData {
            owned_gear: vec![
                "headlamp".into(),
                "headlamp".into(),
                "".into(),
                "x".repeat(33),
            ],
            warehouse_quiz_passed: vec!["scout_pro_3".into(), "scout_pro_3".into()],
            consumables: vec!["field_coffee".to_owned(); CONSUMABLES_MAX + 50],
            ..SaveData::default()
        };
        s.consumables.push(String::new());
        validate(&mut s);
        assert_eq!(s.owned_gear, vec!["headlamp".to_owned()]);
        assert_eq!(s.warehouse_quiz_passed, vec!["scout_pro_3".to_owned()]);
        assert_eq!(s.consumables.len(), CONSUMABLES_MAX);
    }

    #[test]
    fn wrong_typed_warehouse_field_degrades_to_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-warehouse-type");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(SAVE_FILENAME),
            r#"{"version":1,"completed_levels":["c1l1"],"unlocked_specialists":[],"favor_points":5,"settings":{"master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0},"owned_gear":"headlamp"}"#,
        )
        .unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        // Corrupt shape loads as a fresh save, never a panic.
        assert_eq!(loaded.completed_levels.len(), 0);
        assert!(loaded.owned_gear.is_empty());
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn startup_seeds_cores_so_sync_does_not_zero_the_save() {
        let mut app = App::new();
        app.insert_resource(SaveData {
            cores: 42,
            ..SaveData::default()
        });
        app.insert_resource(PendingUnlockSeed(HashSet::new()));
        app.init_resource::<UnlockedSpecialists>();
        app.init_resource::<Cores>();
        app.add_message::<SaveRequest>();
        app.add_systems(Startup, seed_unlocks_from_save);
        app.add_systems(Update, sync_progression_to_save);
        app.update();
        assert_eq!(app.world().resource::<Cores>().0, 42);
        assert_eq!(app.world().resource::<SaveData>().cores, 42);
    }

    /// Adversarial: an unparseable save is preserved byte-for-byte
    /// under the `.corrupt` suffix instead of being left at the live
    /// path, where the next successful write would silently destroy
    /// it. A second load still yields defaults — no crash-loop.
    #[test]
    fn corrupt_save_is_preserved_under_the_corrupt_suffix() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-preserve");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SAVE_FILENAME), "{not json").unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert_eq!(loaded.completed_levels.len(), 0);
        assert!(!dir.join(SAVE_FILENAME).exists());
        let preserved_path = sidecar_path(&dir.join(SAVE_FILENAME), "corrupt");
        let preserved = std::fs::read_to_string(&preserved_path).unwrap();
        assert_eq!(preserved, "{not json");
        let loaded_again = load();
        assert_eq!(loaded_again.completed_levels.len(), 0);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Adversarial: a second corruption replaces the first preserved
    /// copy — preservation never fails just because a `.corrupt` file
    /// already exists.
    #[test]
    fn repeated_corruption_preserves_the_newest_copy() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-preserve-twice");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        std::fs::write(dir.join(SAVE_FILENAME), "{first").unwrap();
        let _ = load();
        std::fs::write(dir.join(SAVE_FILENAME), "{second").unwrap();
        let _ = load();
        let preserved_path = sidecar_path(&dir.join(SAVE_FILENAME), "corrupt");
        let preserved = std::fs::read_to_string(&preserved_path).unwrap();
        assert_eq!(preserved, "{second");
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Adversarial: invalid UTF-8 is unparseable input too, and takes
    /// the same preserve-and-default path as broken JSON.
    #[test]
    fn non_utf8_save_is_preserved_and_loads_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-preserve-utf8");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SAVE_FILENAME), b"\xff\xfe\x00garbage").unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert_eq!(loaded.completed_levels.len(), 0);
        assert!(!dir.join(SAVE_FILENAME).exists());
        let preserved_path = sidecar_path(&dir.join(SAVE_FILENAME), "corrupt");
        assert_eq!(
            std::fs::read(&preserved_path).unwrap(),
            b"\xff\xfe\x00garbage"
        );
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Adversarial: a version-mismatched save (e.g. written by a newer
    /// build) is preserved rather than discarded, so downgrading once
    /// does not cost the player their progress.
    #[test]
    fn wrong_version_save_is_preserved() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-preserve-version");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let text = r#"{"version":999,"completed_levels":["c1l1"],"unlocked_specialists":[],"favor_points":0,"settings":{"master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0}}"#;
        std::fs::write(dir.join(SAVE_FILENAME), text).unwrap();
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let loaded = load();
        assert_eq!(loaded.completed_levels.len(), 0);
        assert!(!dir.join(SAVE_FILENAME).exists());
        let preserved_path = sidecar_path(&dir.join(SAVE_FILENAME), "corrupt");
        assert_eq!(std::fs::read_to_string(&preserved_path).unwrap(), text);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Adversarial: a crash mid-write leaves a half-written temp file
    /// next to the last good save. The good save still loads, and the
    /// next successful save consumes the temp name (rename), leaving
    /// no stale temp behind.
    #[test]
    fn stale_tmp_from_an_interrupted_write_is_ignored() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-stale-tmp");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let mut s = SaveData::default();
        s.complete_level("c1l1");
        save(&s).expect("save should succeed");
        let tmp = sidecar_path(&dir.join(SAVE_FILENAME), "tmp");
        std::fs::write(&tmp, "{\"version\":1,\"completed_lev").unwrap();
        let loaded = load();
        assert!(loaded.is_completed("c1l1"));
        save(&s).expect("save after a stale tmp should succeed");
        assert!(!tmp.exists());
        let loaded = load();
        assert!(loaded.is_completed("c1l1"));
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn erase_removes_the_save_and_its_sidecars() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-erase");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let mut s = SaveData::default();
        s.complete_level("c1l1");
        save(&s).expect("save should succeed");
        let corrupt = sidecar_path(&dir.join(SAVE_FILENAME), "corrupt");
        std::fs::write(&corrupt, "{old corruption").unwrap();
        erase().expect("erase should succeed");
        assert!(!dir.join(SAVE_FILENAME).exists());
        assert!(!corrupt.exists());
        let loaded = load();
        assert_eq!(loaded.completed_levels.len(), 0);
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Erasing when nothing was ever saved is success, not an error:
    /// the New Game reset must work on a fresh profile too.
    #[test]
    fn erase_without_any_files_is_ok() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-save-erase-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        erase().expect("erase of an absent save should succeed");
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
