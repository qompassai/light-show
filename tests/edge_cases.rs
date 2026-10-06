//! Edge case and stress tests for light-show.
//!
//! Covers common game pain points:
//! - Malformed/corrupted level data
//! - Invalid indices and boundary conditions
//! - Data validation edge cases
//! - Stress testing (repeated parsing, large payloads)
//! - Unicode and special character handling

use light_show::level::{LevelDef, LEVEL_SOURCES};
use std::collections::HashSet;

// ============================================================================
// Level Data Edge Cases
// ============================================================================

#[test]
fn level_with_minimal_required_fields_parses() {
    // Based on actual LevelDef structure
    let minimal = r#"{
        "id": "test_minimal",
        "title": "Minimal",
        "world": 0,
        "briefing": "Test briefing",
        "tx_dbm": -10.0,
        "wavelength": "Nm1490",
        "window_min_dbm": -14.0,
        "window_max_dbm": -8.0,
        "medium": "Fiber",
        "nodes": [],
        "fixed_edges": [],
        "available_components": [],
        "source_node": 0,
        "target_node": 0
    }"#;
    let result: Result<LevelDef, _> = serde_json::from_str(minimal);
    assert!(result.is_ok(), "minimal level should parse: {:?}", result.err());
}

#[test]
fn level_missing_required_medium_field_fails_loudly() {
    // medium is required (no silent default per code comments)
    let no_medium = r#"{
        "id": "test_no_medium",
        "title": "No Medium",
        "world": 0,
        "briefing": "Test",
        "tx_dbm": -10.0,
        "wavelength": "Nm1490",
        "window_min_dbm": -14.0,
        "window_max_dbm": -8.0,
        "nodes": [],
        "fixed_edges": [],
        "available_components": [],
        "source_node": 0,
        "target_node": 0
    }"#;
    let result: Result<LevelDef, _> = serde_json::from_str(no_medium);
    assert!(result.is_err(), "missing medium should fail loudly, not silently default");
}

#[test]
fn level_with_malformed_json_returns_error_not_panic() {
    let malformed = r#"{"id": "broken", "title": }"#;
    let result: Result<LevelDef, _> = serde_json::from_str(malformed);
    assert!(result.is_err(), "malformed JSON should error, not panic");
}

#[test]
fn level_with_empty_nodes_parses() {
    let empty = r#"{
        "id": "test_empty",
        "title": "Empty",
        "world": 0,
        "briefing": "Test",
        "tx_dbm": -10.0,
        "wavelength": "Nm1490",
        "window_min_dbm": -14.0,
        "window_max_dbm": -8.0,
        "medium": "Fiber",
        "nodes": [],
        "fixed_edges": [],
        "available_components": [],
        "source_node": 0,
        "target_node": 0
    }"#;
    let level: LevelDef = serde_json::from_str(empty).expect("should parse");
    assert_eq!(level.nodes.len(), 0);
}

// ============================================================================
// Registry Integrity
// ============================================================================

#[test]
fn all_80_levels_have_unique_ids() {
    let mut ids = HashSet::new();
    for (idx, src) in LEVEL_SOURCES.iter().enumerate() {
        let level: LevelDef = serde_json::from_str(src)
            .unwrap_or_else(|e| panic!("level {idx} failed to parse: {e}"));
        assert!(
            ids.insert(level.id.clone()),
            "duplicate level id '{}' at index {idx}",
            level.id
        );
    }
    assert_eq!(ids.len(), 80, "expected 80 unique level ids");
}

#[test]
fn all_levels_have_non_empty_titles() {
    for (idx, src) in LEVEL_SOURCES.iter().enumerate() {
        let level: LevelDef = serde_json::from_str(src)
            .unwrap_or_else(|e| panic!("level {idx} failed to parse: {e}"));
        assert!(
            !level.title.trim().is_empty(),
            "level {idx} ({}) has empty title",
            level.id
        );
    }
}

#[test]
fn all_levels_have_non_empty_briefings() {
    for (idx, src) in LEVEL_SOURCES.iter().enumerate() {
        let level: LevelDef = serde_json::from_str(src)
            .unwrap_or_else(|e| panic!("level {idx} failed to parse: {e}"));
        assert!(
            !level.briefing.trim().is_empty(),
            "level {idx} ({}) has empty briefing",
            level.id
        );
    }
}

#[test]
fn all_levels_have_valid_world_indices() {
    for (idx, src) in LEVEL_SOURCES.iter().enumerate() {
        let level: LevelDef = serde_json::from_str(src)
            .unwrap_or_else(|e| panic!("level {idx} failed to parse: {e}"));
        // Worlds 1-8 (8 companions; Léa's quiz uses world 8)
        assert!(
            (1..=8).contains(&level.world),
            "level {idx} ({}) has invalid world {}",
            level.id,
            level.world
        );
    }
}

#[test]
fn all_levels_have_sane_power_windows() {
    for (idx, src) in LEVEL_SOURCES.iter().enumerate() {
        let level: LevelDef = serde_json::from_str(src)
            .unwrap_or_else(|e| panic!("level {idx} failed to parse: {e}"));
        assert!(
            level.window_min_dbm < level.window_max_dbm,
            "level {idx} ({}) has inverted power window: min {} >= max {}",
            level.id,
            level.window_min_dbm,
            level.window_max_dbm
        );
        // Sanity: windows shouldn't be absurdly wide (>100dB) or narrow (<0.1dB)
        let width = level.window_max_dbm - level.window_min_dbm;
        assert!(
            width > 0.05 && width < 100.0,
            "level {idx} ({}) has suspicious window width: {width}dB",
            level.id
        );
    }
}

// ============================================================================
// Index Boundaries
// ============================================================================

#[test]
fn level_registry_has_exactly_80_entries() {
    assert_eq!(LEVEL_SOURCES.len(), 80, "expected 80 levels");
}

#[test]
fn first_and_last_indices_are_valid() {
    let _: LevelDef = serde_json::from_str(LEVEL_SOURCES[0]).expect("index 0");
    let _: LevelDef = serde_json::from_str(LEVEL_SOURCES[79]).expect("index 79");
}

#[test]
#[should_panic]
fn index_80_out_of_bounds_panics() {
    let _ = LEVEL_SOURCES[80];
}

#[test]
fn track_boundaries_cover_all_80() {
    // 8 tracks × 10 levels = 80
    for track_start in [0, 10, 20, 30, 40, 50, 60, 70] {
        for offset in 0..10 {
            let idx = track_start + offset;
            let level: LevelDef = serde_json::from_str(LEVEL_SOURCES[idx])
                .unwrap_or_else(|e| panic!("track {track_start}, idx {idx}: {e}"));
            assert!(!level.id.is_empty());
        }
    }
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn stress_parse_all_levels_ten_times() {
    for round in 0..10 {
        for (idx, src) in LEVEL_SOURCES.iter().enumerate() {
            let _: LevelDef = serde_json::from_str(src)
                .unwrap_or_else(|e| panic!("round {round}, level {idx}: {e}"));
        }
    }
}

#[test]
fn stress_id_uniqueness_repeated() {
    for _ in 0..5 {
        let mut ids = HashSet::new();
        for src in LEVEL_SOURCES.iter() {
            let level: LevelDef = serde_json::from_str(src).unwrap();
            assert!(ids.insert(level.id.clone()), "dup: {}", level.id);
        }
        assert_eq!(ids.len(), 80);
    }
}

// ============================================================================
// Unicode & Special Characters
// ============================================================================

#[test]
fn level_with_unicode_title_parses() {
    let json = r#"{
        "id": "test_unicode",
        "title": "Séraphine — 日本語テスト 🎮",
        "world": 0,
        "briefing": "Test",
        "tx_dbm": -10.0,
        "wavelength": "Nm1490",
        "window_min_dbm": -14.0,
        "window_max_dbm": -8.0,
        "medium": "Fiber",
        "nodes": [],
        "fixed_edges": [],
        "available_components": [],
        "source_node": 0,
        "target_node": 0
    }"#;
    let level: LevelDef = serde_json::from_str(json).expect("unicode parses");
    assert!(level.title.contains("Séraphine"));
}

#[test]
fn level_with_very_long_briefing_parses() {
    let long = "B".repeat(50000);
    let json = format!(
        r#"{{"id": "t", "title": "T", "world": 0, "briefing": "{long}", "tx_dbm": -10.0, "wavelength": "Nm1490", "window_min_dbm": -14.0, "window_max_dbm": -8.0, "medium": "Fiber", "nodes": [], "fixed_edges": [], "available_components": [], "source_node": 0, "target_node": 0}}"#
    );
    let level: LevelDef = serde_json::from_str(&json).expect("long briefing parses");
    assert_eq!(level.briefing.len(), 50000);
}

#[test]
fn level_ids_with_common_slug_chars_parse() {
    for id in ["test-1", "test_2", "test.3"] {
        let json = format!(
            r#"{{"id": "{id}", "title": "T", "world": 0, "briefing": "B", "tx_dbm": -10.0, "wavelength": "Nm1490", "window_min_dbm": -14.0, "window_max_dbm": -8.0, "medium": "Fiber", "nodes": [], "fixed_edges": [], "available_components": [], "source_node": 0, "target_node": 0}}"#
        );
        assert!(serde_json::from_str::<LevelDef>(&json).is_ok(), "id {id}");
    }
}

// ============================================================================
// Numeric Edge Cases
// ============================================================================

#[test]
fn level_with_extreme_power_values_parses() {
    // Very high/low dBm values should parse (validation is separate)
    let json = r#"{
        "id": "t", "title": "T", "world": 0, "briefing": "B",
        "tx_dbm": -100.0,
        "wavelength": "Nm1490",
        "window_min_dbm": -120.0,
        "window_max_dbm": 50.0,
        "medium": "Fiber",
        "nodes": [], "fixed_edges": [], "available_components": [],
        "source_node": 0, "target_node": 0
    }"#;
    let level: LevelDef = serde_json::from_str(json).expect("extreme values parse");
    assert_eq!(level.tx_dbm, -100.0);
}

#[test]
fn level_with_zero_world_parses() {
    let json = r#"{
        "id": "t", "title": "T", "world": 0, "briefing": "B",
        "tx_dbm": -10.0, "wavelength": "Nm1490",
        "window_min_dbm": -14.0, "window_max_dbm": -8.0,
        "medium": "Fiber",
        "nodes": [], "fixed_edges": [], "available_components": [],
        "source_node": 0, "target_node": 0
    }"#;
    let level: LevelDef = serde_json::from_str(json).unwrap();
    assert_eq!(level.world, 0);
}
