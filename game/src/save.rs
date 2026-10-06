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
//! * Writes are atomic (temp file + rename) so a crash mid-write never
//!   leaves a half-written save.
//! * `LIGHTSHOW_SAVE_DIR` overrides the path (testing, custom launchers).

use crate::cheat_codes::UnlockedSpecialists;
use crate::waifu::{Companion, FavorPoints};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

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
    /// Hint-economy balance (see `FavorPoints`).
    pub favor_points: u32,
    pub settings: Settings,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            completed_levels: Vec::new(),
            unlocked_specialists: Vec::new(),
            favor_points: 0,
            settings: Settings::default(),
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

    /// Record a specialist unlock. Returns true if this is new.
    pub fn unlock_specialist(&mut self, companion: Companion) -> bool {
        let name = format!("{:?}", companion);
        if self.unlocked_specialists.iter().any(|n| n == &name) {
            return false;
        }
        self.unlocked_specialists.push(name);
        true
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

/// Write `data` atomically (temp file + rename). Creates parent dirs.
pub fn save(data: &SaveData) -> Result<(), SaveError> {
    let path = save_path().ok_or(SaveError::NoPath)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(SaveError::Io)?;
    }
    let text = serde_json::to_string_pretty(data).map_err(SaveError::Json)?;
    // Temp file in the same directory so rename is atomic on all platforms.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(SaveError::Io)?;
    std::fs::rename(&tmp, &path).map_err(SaveError::Io)?;
    Ok(())
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
            warn!("cannot read save file {} ({}); starting fresh", path.display(), e);
            return SaveData::default();
        }
    };
    let mut data: SaveData = match serde_json::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            warn!("corrupt save file {} ({}); starting fresh", path.display(), e);
            return SaveData::default();
        }
    };
    if data.version != SAVE_VERSION {
        warn!(
            "save version {} != supported {}; starting fresh",
            data.version, SAVE_VERSION
        );
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
    data.completed_levels.retain(|id| !id.is_empty() && id.len() <= 128);
    data.completed_levels.sort();
    data.completed_levels.dedup();
    // Unknown specialist names are dropped by `unlocked_set`; keep the
    // raw list tidy too.
    data.unlocked_specialists.retain(|n| !n.is_empty() && n.len() <= 32);
    data.unlocked_specialists.sort();
    data.unlocked_specialists.dedup();
}

// ---------------------------------------------------------------------------
// Bevy wiring
// ---------------------------------------------------------------------------

/// Request a save write. Systems emit this after mutating `SaveData`,
/// `UnlockedSpecialists`, or `FavorPoints`.
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

/// Copy persisted unlocks into the runtime resource on startup.
fn seed_unlocks_from_save(
    mut commands: Commands,
    seed: Res<PendingUnlockSeed>,
    mut specialists: ResMut<UnlockedSpecialists>,
) {
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
    favor: Res<FavorPoints>,
    mut save: ResMut<SaveData>,
    mut writer: MessageWriter<SaveRequest>,
) {
    if !unlocked.is_changed() && !favor.is_changed() {
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
    if save.favor_points != favor.0 {
        save.favor_points = favor.0;
        dirty = true;
    }
    if dirty {
        writer.write(SaveRequest);
    }
}

/// Perform the actual disk write. Errors are logged, never panic —
/// a failed save must not crash the game.
fn write_save_on_request(
    mut reader: MessageReader<SaveRequest>,
    data: Res<SaveData>,
) {
    if reader.read().count() == 0 {
        return;
    }
    match save(&data) {
        Ok(()) => info!("progress saved"),
        Err(e) => warn!("failed to save progress: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Tests that mutate `LIGHTSHOW_SAVE_DIR` must not run concurrently:
    /// env vars are process-global and Rust runs tests in threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        s.favor_points = 30;
        save(&s).expect("save should succeed");
        let loaded = load();
        assert!(loaded.is_completed("c1l1"));
        assert!(loaded.unlocked_set().contains(&Companion::Aino));
        assert_eq!(loaded.favor_points, 30);
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
}
