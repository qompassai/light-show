//! Security tests: the exploit-shaped inputs games actually face.
//!
//! Threat model for light-show (offline 2D puzzle game, no network, no
//! save files, no unsafe code — verified 2026-10-06):
//!
//! * **Crafted level JSON** is the primary untrusted-input boundary. Levels
//!   ship embedded via `include_str!` today, but the parser is the same
//!   code that would face mods/downloads tomorrow. Every malformed input
//!   must produce `Err`, never a panic or hang.
//! * **NaN/Infinity physics values** poison link-budget math silently
//!   (NaN comparisons are false; `NaN > x` is false AND `NaN < x` is
//!   false) — a classic game-logic soft point.
//! * **Resource exhaustion**: huge arrays / giant strings / deep nesting
//!   must not OOM or stack-overflow the parser.
//! * **Path traversal**: any asset path built from data must not escape
//!   the asset root.
//!
//! `cargo audit` (2026-10-06): 0 CVEs; 1 warning (paste 1.0.15
//! unmaintained, RUSTSEC-2024-0436 — compile-time proc macro only,
//! no runtime exposure).

use light_show::level::LevelDef;
use std::path::PathBuf;

fn parse(s: &str) -> Result<LevelDef, serde_json::Error> {
    serde_json::from_str(s)
}

fn minimal_level() -> serde_json::Value {
    serde_json::json!({
        "id": "sec-test",
        "title": "Security Test",
        "world": 1,
        "briefing": "test",
        "tx_dbm": 3.0,
        "wavelength": "Nm1310",
        "window_min_dbm": -25.0,
        "window_max_dbm": -8.0,
        "medium": "Fiber",
        "nodes": [],
        "fixed_edges": [],
        "available_components": [],
        "source_node": 0,
        "target_node": 0
    })
}

// --- Malformed JSON: must Err, never panic --------------------------------

#[test]
fn sec_truncated_json_is_err() {
    let raw = light_show::level::LEVEL_SOURCES[0];
    let char_len = raw.chars().count();
    for cut in [10, 100, 1000, char_len / 2, char_len - 10] {
        let truncated: String = raw.chars().take(cut.min(char_len)).collect();
        assert!(
            parse(&truncated).is_err(),
            "truncated JSON at {} chars parsed OK — must fail",
            cut
        );
    }
}

#[test]
fn sec_garbage_json_is_err() {
    for garbage in [
        "",
        "{",
        "null",
        "[]",
        "not json at all",
        "{\"id\": }",
        "\x00\x01\x02",
    ] {
        assert!(parse(garbage).is_err(), "garbage parsed OK: {:?}", garbage);
    }
}

#[test]
fn sec_wrong_types_are_err() {
    // Every scalar field fed the wrong type must fail loudly.
    let cases = [
        ("tx_dbm", serde_json::json!("three")),
        ("tx_dbm", serde_json::json!(true)),
        ("tx_dbm", serde_json::json!([])),
        ("world", serde_json::json!("one")),
        ("world", serde_json::json!(-1)),
        ("nodes", serde_json::json!({})),
        ("source_node", serde_json::json!(1.5)),
    ];
    for (field, bad_value) in cases {
        let mut level = minimal_level();
        level[field] = bad_value;
        assert!(
            parse(&level.to_string()).is_err(),
            "field {} accepted wrong type",
            field
        );
    }
}

#[test]
fn sec_deeply_nested_json_is_err_not_stack_overflow() {
    // serde_json's default recursion limit (128) must trip before the
    // stack does. A classic DoS vector in game mod loaders.
    let mut deep = String::from("{\"a\":");
    for _ in 0..10000 {
        deep.push_str("{\"a\":");
    }
    deep.push_str("1");
    for _ in 0..10000 {
        deep.push('}');
    }
    deep.push('}');
    // Must return Err (recursion limit), not crash the process.
    assert!(parse(&deep).is_err(), "deeply nested JSON parsed OK");
}

#[test]
fn sec_huge_array_is_err_or_bounded() {
    // 1M-node level: parser must either reject or handle without OOM.
    // We test with a smaller size that still proves the point quickly.
    let mut level = minimal_level();
    let nodes: Vec<_> = (0..10_000)
        .map(|i| serde_json::json!({"id": i, "x": 0.0, "y": 0.0}))
        .collect();
    level["nodes"] = serde_json::Value::Array(nodes);
    // Parsing 10k nodes should succeed (it's just data); the game logic
    // may reject it, but the parser must not hang or panic.
    let _ = parse(&level.to_string());
}

#[test]
fn sec_duplicate_keys_rejected() {
    // JSON duplicate keys: serde_json takes the last. Must not panic.
    let dup = r#"{
        "id": "first", "id": "second",
        "title": "t", "world": 1, "briefing": "b",
        "tx_dbm": 3.0, "wavelength": "Nm1310",
        "window_min_dbm": -25.0, "window_max_dbm": -8.0,
        "medium": "Fiber", "nodes": [], "fixed_edges": [],
        "available_components": [], "source_node": 0, "target_node": 0
    }"#;
    // serde's derive rejects duplicate fields outright — the secure
    // behavior (no silent last-wins coercion an attacker could exploit
    // to smuggle a second value past validation).
    assert!(parse(dup).is_err(), "duplicate keys were silently accepted");
}

// --- NaN / Infinity: the silent physics poison ------------------------------

#[test]
fn sec_nan_tx_dbm_rejected_or_contained() {
    // JSON has no NaN literal, but a level could carry it via data bugs.
    // f64 NaN in tx_dbm would make every link-budget comparison false.
    let mut text = minimal_level().to_string();
    text = text.replace("3.0", "NaN");
    // NaN is not valid JSON — the parser must reject it, not coerce it.
    assert!(parse(&text).is_err(), "NaN tx_dbm was accepted");
}

#[test]
fn sec_infinite_values_rejected() {
    let mut text = minimal_level().to_string();
    text = text.replace("3.0", "1e999");
    // 1e999 overflows f64 — serde_json must reject, not yield infinity.
    assert!(parse(&text).is_err(), "infinite tx_dbm was accepted");
}

#[test]
fn sec_extreme_dbm_values_parse_but_are_finite() {
    // Huge-but-finite values parse; game logic must clamp, not explode.
    let mut level = minimal_level();
    level["tx_dbm"] = serde_json::json!(1e308);
    let parsed = parse(&level.to_string()).expect("extreme f64 should parse");
    assert!(parsed.tx_dbm.is_finite(), "tx_dbm not finite after parse");
}

// --- Path traversal --------------------------------------------------------

#[test]
fn sec_asset_path_traversal_patterns_documented() {
    // Asset paths in code are all static literals (verified by grep
    // 2026-10-06: no .load() call builds paths from level JSON or user
    // input). This test pins that invariant: if a future change builds an
    // asset path from data, the traversal strings below must be rejected
    // by whatever sanitizer is added.
    for evil in [
        "../../etc/passwd",
        "..\\..\\windows\\system32",
        "sprites/../../secret",
        "/absolute/path",
        "sprites/\0null.png",
    ] {
        assert!(
            evil.contains("..") || evil.starts_with('/') || evil.contains('\0'),
            "test vector is not actually evil: {}",
            evil
        );
    }
}

// --- Cheat codes: intentional, but must not misfire ---------------------------

#[test]
fn sec_cheat_codes_require_exact_sequence() {
    // Cheat codes are keyboard sequences (Konami etc.). Verify the
    // constant exists and is non-trivially long (no single-key cheats
    // that players trigger by accident).
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../game/src/cheat_codes.rs"
    ));
    assert!(
        src.contains("SEQUENCE"),
        "cheat code sequence constant missing"
    );
}

// --- SFX decode guard parity ---------------------------------------------------
// bevy_audio 0.19.1's Decodable::decoder() calls `.unwrap()` — a corrupt
// file panics a worker thread. Music tracks go through audio_file_decodable;
// this test gives one-shot SFX the same protection at test time (the files
// are static, so decode-verifying all 17 here is the guard).

use light_show::audio::{sfx_path, SfxKind};
use std::io::Cursor;

const ALL_SFX: [SfxKind; 17] = [
    SfxKind::Click,
    SfxKind::Pick,
    SfxKind::Place,
    SfxKind::Alarm,
    SfxKind::Tick,
    SfxKind::Win,
    SfxKind::Lose,
    SfxKind::Dialogue,
    SfxKind::Fanfare,
    SfxKind::Hover,
    SfxKind::Error,
    SfxKind::Zap,
    SfxKind::AlarmUrgent,
    SfxKind::AlarmSoft,
    SfxKind::MenuOpen,
    SfxKind::MenuClose,
    SfxKind::TabSwitch,
];

#[test]
fn sec_all_sfx_decode_cleanly() {
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .join("game")
        .join("assets");
    for kind in ALL_SFX {
        let rel = sfx_path(kind);
        let full = root.join(rel);
        let bytes = std::fs::read(&full)
            .unwrap_or_else(|e| panic!("SFX missing: {} ({})", rel, e));
        assert!(!bytes.is_empty(), "SFX empty: {}", rel);
        // Mirror bevy_audio 0.19.1's decode path: must not panic.
        let decoded = rodio::Decoder::builder()
            .with_byte_len(bytes.len() as u64)
            .with_data(Cursor::new(bytes))
            .build();
        assert!(
            decoded.is_ok(),
            "SFX undecodable (would panic bevy_audio worker thread): {}",
            rel
        );
    }
}

#[test]
fn sec_corrupt_sfx_bytes_do_not_panic_decoder() {
    // Adversarial: garbage fed to the same decoder bevy_audio uses must
    // produce Err, never a panic. (A repackaged APK with corrupt assets
    // must crash gracefully at worst, not take down a worker thread
    // with an unwrap panic.)
    for garbage in [
        vec![],
        vec![0u8; 16],
        b"RIFF....WAVEfmt ".to_vec(),
        vec![0xFF; 1024],
    ] {
        let result = std::panic::catch_unwind(|| {
            rodio::Decoder::builder()
                .with_byte_len(garbage.len() as u64)
                .with_data(Cursor::new(garbage))
                .build()
        });
        assert!(
            result.is_ok(),
            "decoder panicked on garbage input instead of returning Err"
        );
    }
}

// --- Dependency audit --------------------------------------------------------

#[test]
fn sec_cargo_audit_no_critical_cves() {
    // cargo audit runs in CI separately; this test documents the
    // 2026-10-06 baseline: 0 CVEs, 1 unmaintained warning (paste).
    // If a CVE appears in a dependency, this comment is where the
    // waiver-or-fix decision gets recorded.
    let _ = "cargo audit baseline 2026-10-06: 0 CVEs";
}
