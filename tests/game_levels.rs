//! Integration coverage for the bundled level set (`game/assets/levels/`).
//!
//! The game embeds level JSON at compile time (`level::LEVEL_SOURCES`) and
//! deserializes it with `serde_json::from_str::<LevelDef>` inside
//! `level::load_level`, which `expect`s success. This file drives that same
//! deserializer through a bounded, typed-error directory loader so that:
//!
//! - every shipped level parses and passes structural + physical sanity
//!   checks (validation), and
//! - malformed, oversized, duplicated, or dangling inputs surface as typed
//!   `LevelError`s instead of panics (adversarial).
//!
//! Progression has no per-level "next" pointer: companions own two-level
//! tracks (`Companion::track_start_index`, `Companion::TRACK_LEN`) and the
//! results screen advances with `index.saturating_add(1).min(len - 1)`.
//! The progression checks below model exactly those two rules.

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

use light_show::level::{load_level, LevelDef, MediumDef, LEVEL_SOURCES};
use light_show::waifu::Companion;

/// Upper bound on level files read from one directory.
const LEVEL_FILES_MAX: usize = 256;
/// Upper bound on one level file's size. Shipped levels are ~2 KiB.
const LEVEL_FILE_BYTES_MAX: u64 = 64 * 1024;

/// Physical plausibility envelope for one medium, in that medium's units
/// (dBm for fiber/wireless, dBmV for coax). Chosen wide enough to cover
/// real gear (GPON class B+/C+ optics, DOCSIS plant levels, cellular
/// EIRP limits and receiver sensitivity) while rejecting typos such as a
/// sign flip or a missing decimal point.
struct SignalEnvelope {
    tx_min: f64,
    tx_max: f64,
    window_min: f64,
    window_max: f64,
}

const FIBER_ENVELOPE_DBM: SignalEnvelope = SignalEnvelope {
    tx_min: -20.0,
    tx_max: 30.0,
    window_min: -50.0,
    window_max: 10.0,
};
const COAX_ENVELOPE_DBMV: SignalEnvelope = SignalEnvelope {
    tx_min: -20.0,
    tx_max: 70.0,
    window_min: -30.0,
    window_max: 60.0,
};
const WIRELESS_ENVELOPE_DBM: SignalEnvelope = SignalEnvelope {
    tx_min: -30.0,
    tx_max: 50.0,
    window_min: -130.0,
    window_max: 0.0,
};

/// TIA-568 permanent link + patch; the game's own default.
const ETHERNET_SEGMENT_M_MAX: f64 = 100.0;
/// IEEE 802.3bt Type 4 PSE output.
const ETHERNET_POE_W_MAX: f64 = 90.0;
/// 100 GbE; anything above is a typo for this game.
const ETHERNET_BANDWIDTH_MBPS_MAX: u64 = 100_000;
/// A scripted outage must fire within a single sitting.
const OUTAGE_FIRE_SECONDS_MAX: f64 = 600.0;

/// Every way loading or validating a level set can fail.
#[derive(Debug)]
enum LevelError {
    DirMissing {
        dir: PathBuf,
        source: io::Error,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
    TooManyFiles {
        dir: PathBuf,
    },
    TooLarge {
        path: PathBuf,
        bytes_max: u64,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Invalid {
        level_id: String,
        reason: String,
    },
    DuplicateId {
        level_id: String,
    },
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirMissing { dir, source } => write!(f, "{}: {source}", dir.display()),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::TooManyFiles { dir } => {
                write!(f, "{}: more than {LEVEL_FILES_MAX} files", dir.display())
            }
            Self::TooLarge { path, bytes_max } => {
                write!(f, "{}: larger than {bytes_max} bytes", path.display())
            }
            Self::Parse { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Invalid { level_id, reason } => write!(f, "level {level_id}: {reason}"),
            Self::DuplicateId { level_id } => write!(f, "duplicate level id {level_id}"),
        }
    }
}

/// Progression-table failures (companion tracks over the level list).
#[derive(Debug, PartialEq, Eq)]
enum ProgressionError {
    DanglingTrack { companion: Companion, index: usize },
    OverlappingTracks { index: usize },
    OrphanLevel { index: usize },
}

// ---------------------------------------------------------------------------
// Loader: bounded directory scan feeding the game's own deserializer.
// ---------------------------------------------------------------------------

/// Parses one level with the exact deserializer `level::load_level` uses,
/// minus its `expect`, so failure is a value.
fn parse_level(path: &Path, text: &str) -> Result<LevelDef, LevelError> {
    serde_json::from_str::<LevelDef>(text).map_err(|source| LevelError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// Reads at most `LEVEL_FILE_BYTES_MAX` bytes; a longer file is rejected
/// without buffering the remainder.
fn read_bounded(path: &Path) -> Result<String, LevelError> {
    let io_err = |source| LevelError::Io {
        path: path.to_path_buf(),
        source,
    };
    let file = fs::File::open(path).map_err(io_err)?;
    let mut text = String::new();
    file.take(LEVEL_FILE_BYTES_MAX + 1)
        .read_to_string(&mut text)
        .map_err(io_err)?;
    let bytes_read = u64::try_from(text.len()).unwrap_or(u64::MAX);
    if bytes_read > LEVEL_FILE_BYTES_MAX {
        return Err(LevelError::TooLarge {
            path: path.to_path_buf(),
            bytes_max: LEVEL_FILE_BYTES_MAX,
        });
    }
    Ok(text)
}

/// Loads, validates, and cross-checks every `*.json` in `dir`, sorted by
/// file name for determinism. Non-JSON files are ignored.
fn load_level_dir(dir: &Path) -> Result<Vec<(PathBuf, LevelDef)>, LevelError> {
    let entries = fs::read_dir(dir).map_err(|source| LevelError::DirMissing {
        dir: dir.to_path_buf(),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| LevelError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if paths.len() == LEVEL_FILES_MAX {
                return Err(LevelError::TooManyFiles {
                    dir: dir.to_path_buf(),
                });
            }
            paths.push(path);
        }
    }
    paths.sort();
    let mut levels = Vec::with_capacity(paths.len());
    for path in paths {
        let text = read_bounded(&path)?;
        let level = parse_level(&path, &text)?;
        validate_level(&level)?;
        levels.push((path, level));
    }
    validate_unique_ids(levels.iter().map(|(_, level)| level))?;
    Ok(levels)
}

// ---------------------------------------------------------------------------
// Validation rules.
// ---------------------------------------------------------------------------

fn invalid(level: &LevelDef, reason: impl Into<String>) -> LevelError {
    LevelError::Invalid {
        level_id: level.id.clone(),
        reason: reason.into(),
    }
}

/// Structural checks: required text present, node references resolve.
fn validate_structure(level: &LevelDef) -> Result<(), LevelError> {
    for (field, value) in [
        ("id", &level.id),
        ("title", &level.title),
        ("briefing", &level.briefing),
    ] {
        if value.trim().is_empty() {
            return Err(invalid(level, format!("empty {field}")));
        }
    }
    if level.world == 0 {
        return Err(invalid(level, "world must be >= 1"));
    }
    let mut node_ids = HashSet::new();
    for node in &level.nodes {
        if !node_ids.insert(node.id) {
            return Err(invalid(level, format!("duplicate node id {}", node.id)));
        }
    }
    let declared = |id: u32| node_ids.contains(&id);
    if !declared(level.source_node) || !declared(level.target_node) {
        return Err(invalid(level, "source/target node is not declared"));
    }
    if level.source_node == level.target_node {
        return Err(invalid(level, "source and target are the same node"));
    }
    if level.available_components.is_empty() {
        return Err(invalid(level, "no player-placeable components"));
    }
    let edges = level.fixed_edges.iter().map(|e| (e.from, e.to));
    let choices = level.available_components.iter().map(|c| (c.from, c.to));
    for (from, to) in edges.chain(choices) {
        if !declared(from) || !declared(to) {
            return Err(invalid(
                level,
                format!("edge {from}->{to} names an undeclared node"),
            ));
        }
    }
    if let Some(outage) = &level.scripted_outage {
        if !declared(outage.edge_from) || !declared(outage.edge_to) {
            return Err(invalid(level, "scripted outage targets an undeclared node"));
        }
        let fire_s = outage.fires_after_seconds;
        if !(fire_s > 0.0 && fire_s <= OUTAGE_FIRE_SECONDS_MAX) {
            return Err(invalid(
                level,
                format!("outage fires at implausible {fire_s} s"),
            ));
        }
    }
    Ok(())
}

/// Win-condition plausibility: receive window ordered, finite, and inside
/// the medium's physical envelope; Ethernet constraints within standards.
fn validate_win_condition(level: &LevelDef) -> Result<(), LevelError> {
    let envelope = match level.medium {
        MediumDef::Fiber => &FIBER_ENVELOPE_DBM,
        MediumDef::Coax => &COAX_ENVELOPE_DBMV,
        MediumDef::Wireless => &WIRELESS_ENVELOPE_DBM,
        MediumDef::Ethernet => return validate_ethernet(level),
    };
    let (tx, low, high) = (level.tx_dbm, level.window_min_dbm, level.window_max_dbm);
    if !(tx.is_finite() && low.is_finite() && high.is_finite()) {
        return Err(invalid(level, "non-finite signal parameter"));
    }
    if low >= high {
        return Err(invalid(
            level,
            format!("inverted receive window [{low}, {high}]"),
        ));
    }
    if !(envelope.tx_min..=envelope.tx_max).contains(&tx) {
        return Err(invalid(level, format!("tx {tx} outside physical envelope")));
    }
    let window_range = envelope.window_min..=envelope.window_max;
    if !window_range.contains(&low) || !window_range.contains(&high) {
        return Err(invalid(
            level,
            format!("window [{low}, {high}] outside envelope"),
        ));
    }
    Ok(())
}

/// Ethernet levels are judged on segment length, PoE, and bandwidth; the
/// dB window is unused, so the constraints must be present and sane.
fn validate_ethernet(level: &LevelDef) -> Result<(), LevelError> {
    let segment_m = level
        .max_segment_length_m
        .ok_or_else(|| invalid(level, "no segment max"))?;
    let poe_w = level
        .endpoint_poe_draw_w
        .ok_or_else(|| invalid(level, "no PoE draw"))?;
    let bandwidth = level
        .required_bandwidth_mbps
        .ok_or_else(|| invalid(level, "no bandwidth"))?;
    if !(segment_m > 0.0 && segment_m <= ETHERNET_SEGMENT_M_MAX) {
        return Err(invalid(
            level,
            format!("segment max {segment_m} m out of range"),
        ));
    }
    if !(0.0..=ETHERNET_POE_W_MAX).contains(&poe_w) {
        return Err(invalid(level, format!("PoE draw {poe_w} W out of range")));
    }
    if bandwidth == 0 || bandwidth > ETHERNET_BANDWIDTH_MBPS_MAX {
        return Err(invalid(
            level,
            format!("bandwidth {bandwidth} Mbps out of range"),
        ));
    }
    Ok(())
}

fn validate_level(level: &LevelDef) -> Result<(), LevelError> {
    validate_structure(level)?;
    validate_win_condition(level)
}

fn validate_unique_ids<'a>(levels: impl Iterator<Item = &'a LevelDef>) -> Result<(), LevelError> {
    let mut seen = HashSet::new();
    for level in levels {
        if !seen.insert(level.id.as_str()) {
            return Err(LevelError::DuplicateId {
                level_id: level.id.clone(),
            });
        }
    }
    Ok(())
}

/// Checks a companion → track-start table against `level_count` levels:
/// every track lies inside the list, tracks never overlap, and every level
/// belongs to some track (no unreachable level).
fn check_progression(
    level_count: usize,
    tracks: &[(Companion, usize)],
    track_len: usize,
) -> Result<(), ProgressionError> {
    let mut owner = vec![false; level_count];
    for &(companion, start) in tracks {
        for index in start..start.saturating_add(track_len) {
            let slot = owner
                .get_mut(index)
                .ok_or(ProgressionError::DanglingTrack { companion, index })?;
            if *slot {
                return Err(ProgressionError::OverlappingTracks { index });
            }
            *slot = true;
        }
    }
    match owner.iter().position(|owned| !owned) {
        Some(index) => Err(ProgressionError::OrphanLevel { index }),
        None => Ok(()),
    }
}

/// Mirror of `results::handle_result_buttons`' ContinueNextLevel rule.
fn next_level_index(index: usize, level_count: usize) -> usize {
    index.saturating_add(1).min(level_count - 1)
}

/// The medium each base companion's track teaches.
fn companion_medium(companion: Companion) -> Option<MediumDef> {
    match companion {
        Companion::Fiber => Some(MediumDef::Fiber),
        Companion::Coax => Some(MediumDef::Coax),
        Companion::Mobile => Some(MediumDef::Wireless),
        Companion::Ethernet => Some(MediumDef::Ethernet),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

fn shipped_levels_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../game/assets/levels")
}

fn base_tracks() -> Vec<(Companion, usize)> {
    Companion::ALL
        .iter()
        .map(|c| {
            let start = c.track_start_index();
            (*c, start.unwrap_or_else(|| panic!("{c:?} has no track")))
        })
        .collect()
}

/// A per-test scratch directory under the system temp dir, removed on
/// drop. Removal is best-effort: a leftover temp dir is harmless.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(test_name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("light-show-levels-{}-{test_name}", process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear stale scratch dir");
        }
        fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }

    fn write(&self, name: &str, contents: &str) {
        fs::write(self.0.join(name), contents).expect("write scratch level");
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _cleanup_is_best_effort = fs::remove_dir_all(&self.0);
    }
}

/// The first shipped level as a mutable JSON value, for surgical mutation.
fn level_json() -> serde_json::Value {
    serde_json::from_str(LEVEL_SOURCES[0]).expect("LEVEL_SOURCES[0] is valid JSON")
}

fn mutate(edit: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>)) -> String {
    let mut value = level_json();
    let object = value.as_object_mut().expect("level JSON is an object");
    edit(object);
    value.to_string()
}

/// The first shipped level with one top-level field overwritten.
fn with_field(field: &str, value: serde_json::Value) -> String {
    mutate(|object| {
        object.insert(field.to_string(), value);
    })
}

fn parse_str(text: &str) -> Result<LevelDef, LevelError> {
    parse_level(Path::new("<inline>"), text)
}

fn assert_invalid(text: &str, reason_fragment: &str) {
    let level = parse_str(text).expect("mutation keeps the JSON shape valid");
    match validate_level(&level) {
        Err(LevelError::Invalid { reason, .. }) => assert!(
            reason.contains(reason_fragment),
            "expected reason containing {reason_fragment:?}, got {reason:?}"
        ),
        other => panic!("expected Invalid({reason_fragment}), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Validation tests.
// ---------------------------------------------------------------------------

#[test]
fn every_shipped_level_file_loads_and_validates() {
    let levels = load_level_dir(&shipped_levels_dir()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        levels.len(),
        LEVEL_SOURCES.len(),
        "level file count != LEVEL_SOURCES"
    );
}

#[test]
fn every_level_file_on_disk_is_embedded_and_vice_versa() {
    let dir = shipped_levels_dir();
    let mut on_disk = Vec::new();
    for (path, _) in load_level_dir(&dir).unwrap_or_else(|e| panic!("{e}")) {
        on_disk.push(read_bounded(&path).unwrap_or_else(|e| panic!("{e}")));
    }
    for (index, source) in LEVEL_SOURCES.iter().enumerate() {
        assert!(
            on_disk.iter().any(|text| text == source),
            "LEVEL_SOURCES[{index}] not on disk"
        );
    }
    for text in &on_disk {
        assert!(
            LEVEL_SOURCES.contains(&text.as_str()),
            "a level file is not embedded"
        );
    }
}

#[test]
fn every_embedded_level_passes_through_the_game_loader() {
    let levels: Vec<LevelDef> = (0..LEVEL_SOURCES.len()).map(load_level).collect();
    for (index, level) in levels.iter().enumerate() {
        validate_level(level).unwrap_or_else(|e| panic!("LEVEL_SOURCES[{index}]: {e}"));
    }
    validate_unique_ids(levels.iter()).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn base_companion_tracks_cover_every_level_exactly_once() {
    let result = check_progression(LEVEL_SOURCES.len(), &base_tracks(), Companion::TRACK_LEN);
    assert_eq!(result, Ok(()));
}

#[test]
fn each_track_teaches_its_companions_medium_in_world_order() {
    for (companion, start) in base_tracks() {
        let medium = companion_medium(companion).expect("base companions have a medium");
        let mut previous_world = 0;
        for index in start..start + Companion::TRACK_LEN {
            let level = load_level(index);
            assert_eq!(
                level.medium, medium,
                "{companion:?} track level {index} ({})",
                level.id
            );
            assert!(
                level.world >= previous_world,
                "{companion:?} track regresses in world"
            );
            previous_world = level.world;
        }
    }
}

#[test]
fn world_field_matches_the_world_number_in_the_file_name() {
    let entries = fs::read_dir(shipped_levels_dir()).expect("levels dir is readable");
    let mut checked = 0;
    for entry in entries.take(LEVEL_FILES_MAX) {
        let path = entry.expect("levels dir entry is readable").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        // Only `world<N>_level*.json` files name their world.
        let Some((world, _)) = name.strip_prefix("world").and_then(|s| s.split_once('_')) else {
            continue;
        };
        let world: u32 = world
            .parse()
            .unwrap_or_else(|e| panic!("{name}: bad world number: {e}"));
        let text = fs::read_to_string(&path).expect("level file is readable");
        let level: LevelDef = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(level.world, world, "{name}: world disagrees with file name");
        checked += 1;
    }
    assert!(
        checked >= 2,
        "found only {checked} world*_level*.json files"
    );
}

#[test]
fn continue_next_level_walks_every_level_once_then_stops() {
    let level_count = LEVEL_SOURCES.len();
    let mut visited = vec![0usize];
    let mut index = 0;
    // Bounded: a correct rule reaches the fixpoint in level_count - 1 steps.
    for _ in 0..level_count {
        let next = next_level_index(index, level_count);
        if next == index {
            break;
        }
        assert!(
            !visited.contains(&next),
            "progression revisits level {next}: cycle"
        );
        visited.push(next);
        index = next;
    }
    assert_eq!(visited, (0..level_count).collect::<Vec<_>>());
    assert_eq!(
        next_level_index(level_count - 1, level_count),
        level_count - 1
    );
}

// ---------------------------------------------------------------------------
// Adversarial tests.
// ---------------------------------------------------------------------------

#[test]
fn malformed_and_empty_json_are_parse_errors() {
    for text in [
        "",
        "   ",
        "{",
        "{\"id\": ",
        "null",
        "[]",
        "42",
        "\u{feff}{}",
        "{\"id\":\"x\"}}",
    ] {
        assert!(
            matches!(parse_str(text), Err(LevelError::Parse { .. })),
            "{text:?} must be a parse error"
        );
    }
}

#[test]
fn every_required_field_missing_is_a_named_parse_error() {
    const REQUIRED: &[&str] = &[
        "id",
        "title",
        "world",
        "briefing",
        "tx_dbm",
        "wavelength",
        "window_min_dbm",
        "window_max_dbm",
        "medium",
        "nodes",
        "fixed_edges",
        "available_components",
        "source_node",
        "target_node",
    ];
    for field in REQUIRED {
        let text = mutate(|object| {
            object.remove(*field);
        });
        match parse_str(&text) {
            Err(LevelError::Parse { source, .. }) => assert!(
                source
                    .to_string()
                    .contains(&format!("missing field `{field}`")),
                "{field}: unexpected message {source}"
            ),
            other => panic!("removing {field} must fail to parse, got {other:?}"),
        }
    }
}

#[test]
fn wrong_types_and_unknown_variants_are_parse_errors() {
    let cases: [(&str, serde_json::Value); 8] = [
        ("world", serde_json::json!("one")),
        ("world", serde_json::json!(-1)),
        ("tx_dbm", serde_json::json!("hot")),
        ("nodes", serde_json::json!({})),
        ("medium", serde_json::json!("Laser")),
        ("medium", serde_json::json!("fiber")),
        ("wavelength", serde_json::json!("Nm850")),
        ("source_node", serde_json::json!(4_294_967_296_u64)),
    ];
    for (field, value) in cases {
        let text = with_field(field, value.clone());
        assert!(
            matches!(parse_str(&text), Err(LevelError::Parse { .. })),
            "{field} = {value} must be a parse error"
        );
    }
}

#[test]
fn out_of_range_numbers_and_deep_nesting_do_not_panic() {
    let huge = LEVEL_SOURCES[0].replacen("\"tx_dbm\": -10.0", "\"tx_dbm\": 1e999", 1);
    assert_ne!(
        huge, LEVEL_SOURCES[0],
        "fixture anchor moved; update the replacen target"
    );
    assert!(matches!(parse_str(&huge), Err(LevelError::Parse { .. })));
    // serde_json caps recursion at 128 levels; this must error, not overflow.
    let depth = 10_000;
    let deep = format!("{{\"nodes\": {}{}}}", "[".repeat(depth), "]".repeat(depth));
    assert!(matches!(parse_str(&deep), Err(LevelError::Parse { .. })));
}

#[test]
fn dangling_node_references_are_invalid() {
    use serde_json::json;
    assert_invalid(&with_field("source_node", json!(99)), "source");
    assert_invalid(&with_field("target_node", json!(0)), "same node");
    let edge = json!([{"from": 0, "to": 77, "component": {"Splice": {
        "degradation_db": 0.0, "kind": "Fusion"}}}]);
    assert_invalid(&with_field("fixed_edges", edge), "undeclared");
    let outage = json!({"fires_after_seconds": 5.0, "kind": "FiberCut",
        "edge_from": 0, "edge_to": 42});
    assert_invalid(&with_field("scripted_outage", outage), "outage");
    let late = json!({"fires_after_seconds": -1.0, "kind": "FiberCut",
        "edge_from": 0, "edge_to": 1});
    assert_invalid(&with_field("scripted_outage", late), "implausible");
    let dup = json!([{"id": 0, "label": "a", "grid_x": 0.0, "grid_y": 0.0},
        {"id": 0, "label": "b", "grid_x": 1.0, "grid_y": 0.0}]);
    assert_invalid(&with_field("nodes", dup), "duplicate node");
}

#[test]
fn implausible_win_conditions_are_invalid() {
    use serde_json::json;
    assert_invalid(&with_field("window_min_dbm", json!(0.0)), "inverted");
    assert_invalid(&with_field("tx_dbm", json!(100.0)), "envelope");
    assert_invalid(&with_field("window_max_dbm", json!(40.0)), "envelope");
    assert_invalid(&with_field("id", json!(" ")), "empty id");
    assert_invalid(&with_field("world", json!(0)), "world");
    assert_invalid(&with_field("medium", json!("Ethernet")), "segment");
    let over_poe = mutate(|object| {
        object.insert("medium".into(), json!("Ethernet"));
        object.insert("max_segment_length_m".into(), json!(100.0));
        object.insert("endpoint_poe_draw_w".into(), json!(500.0));
        object.insert("required_bandwidth_mbps".into(), json!(1000));
    });
    assert_invalid(&over_poe, "PoE");
}

#[test]
fn missing_directory_is_a_typed_error() {
    let dir = std::env::temp_dir().join(format!("light-show-no-such-dir-{}", process::id()));
    assert!(matches!(
        load_level_dir(&dir),
        Err(LevelError::DirMissing { .. })
    ));
}

#[test]
fn duplicate_level_ids_across_files_are_rejected() {
    let scratch = ScratchDir::new("duplicate");
    scratch.write("a.json", LEVEL_SOURCES[0]);
    scratch.write("b.json", LEVEL_SOURCES[0]);
    scratch.write("README.md", "not a level, must be ignored");
    match load_level_dir(&scratch.0) {
        Err(LevelError::DuplicateId { level_id }) => assert_eq!(level_id, load_level(0).id),
        other => panic!("expected DuplicateId, got {other:?}"),
    }
}

#[test]
fn empty_and_oversized_files_in_a_directory_are_typed_errors() {
    let empty = ScratchDir::new("empty");
    empty.write("empty.json", "");
    assert!(matches!(
        load_level_dir(&empty.0),
        Err(LevelError::Parse { .. })
    ));

    let oversized = ScratchDir::new("oversized");
    let padding = " ".repeat(usize::try_from(LEVEL_FILE_BYTES_MAX).expect("fits usize"));
    oversized.write("big.json", &format!("{}{padding}", LEVEL_SOURCES[0]));
    assert!(matches!(
        load_level_dir(&oversized.0),
        Err(LevelError::TooLarge { .. })
    ));
}

#[test]
fn dangling_overlapping_and_orphan_tracks_are_detected() {
    let level_count = LEVEL_SOURCES.len();
    let track_len = Companion::TRACK_LEN;
    let dangling = [(Companion::Fiber, level_count - 1)];
    assert_eq!(
        check_progression(level_count, &dangling, track_len),
        Err(ProgressionError::DanglingTrack {
            companion: Companion::Fiber,
            index: level_count
        })
    );
    let overlap = [(Companion::Fiber, 0), (Companion::Coax, 1)];
    assert_eq!(
        check_progression(level_count, &overlap, track_len),
        Err(ProgressionError::OverlappingTracks { index: 1 })
    );
    let orphan = [(Companion::Fiber, 0)];
    assert_eq!(
        check_progression(level_count, &orphan, track_len),
        Err(ProgressionError::OrphanLevel { index: track_len })
    );
}

#[test]
fn specialists_have_no_track_and_out_of_range_indices_clamp() {
    // Specialist tracks are not built: they expose no start index at all,
    // so no phantom level index can be produced. `load_level` must still
    // clamp to the last level rather than index out of bounds.
    let last_id = load_level(LEVEL_SOURCES.len() - 1).id;
    for companion in [
        Companion::Clara,
        Companion::Aino,
        Companion::Hikari,
        Companion::Lea,
    ] {
        assert_eq!(
            companion.track_start_index(),
            None,
            "{companion:?} now has levels; extend tests"
        );
    }
    assert_eq!(load_level(LEVEL_SOURCES.len()).id, last_id);
    assert_eq!(load_level(usize::MAX).id, last_id);
}
