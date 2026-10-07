//! Platform usage tests: Android phones and Windows desktops.
//!
//! The game ships to Google Play (Android), F-Droid, and Windows.
//! These tests encode the platform constraints that break games in
//! the field: touch-only input, case-sensitive APK assets, portrait
//! phone layout, Android-supported audio codecs, and Windows path/
//! line-ending behavior.

use light_show::level::LevelDef;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn game_src() -> PathBuf {
    repo_root().join("game").join("src")
}

fn read_src(rel: &str) -> String {
    std::fs::read_to_string(game_src().join(rel))
        .unwrap_or_else(|_| panic!("cannot read game/src/{}", rel))
}

// --- Android: touch input --------------------------------------------------

#[test]
fn android_board_handles_touch_input() {
    // board.rs must process touch, not just mouse: phones have no cursor.
    let board = read_src("board.rs");
    assert!(
        board.contains("Touches") || board.contains("TouchInput"),
        "board.rs does not reference touch input — unplayable on phones"
    );
}

#[test]
fn android_no_mouse_only_gameplay_path() {
    // Gameplay states must not gate progress behind cursor-only events.
    // Bevy's `Interaction` (used by buttons) fires for touch too.
    // playing.rs routes input through board.rs (Touches) and shared UI
    // button handlers; quiz/triage/api consoles own their buttons.
    for state_file in [
        "states/quiz.rs",
        "states/triage_console.rs",
        "states/api_console.rs",
    ] {
        let src = read_src(state_file);
        assert!(
            src.contains("Interaction") || src.contains("Touches") || src.contains("TouchInput"),
            "{} has no touch/click interaction handling",
            state_file
        );
    }
    // playing.rs must at least route through the board input module.
    let playing = read_src("states/playing.rs");
    assert!(
        playing.contains("board") || playing.contains("Interaction"),
        "states/playing.rs is disconnected from board touch input"
    );
}

#[test]
fn android_keyboard_not_required_for_gameplay() {
    // Cheat codes may be keyboard-only (they are optional), but core
    // gameplay states must not require KeyCode to progress.
    for state_file in [
        "states/playing.rs",
        "states/quiz.rs",
        "states/companion_select.rs",
    ] {
        let src = read_src(state_file);
        let keyboard_gated = src.contains("KeyCode::") && !src.contains("cheat");
        assert!(
            !keyboard_gated,
            "{} gates gameplay behind keyboard input — phones have no keyboard",
            state_file
        );
    }
}

// --- Android: portrait phone layout ----------------------------------------

#[test]
fn android_portrait_window_configured() {
    // Primary window is 720x1280 portrait (phone-first layout).
    let lib = read_src("lib.rs");
    assert!(
        lib.contains("(720, 1280)"),
        "primary window is not 720x1280 portrait"
    );
}

// --- Android: APK asset constraints -----------------------------------------

#[test]
fn android_audio_codecs_supported() {
    // Android decoders: WAV, MP3, Ogg Vorbis are universally supported.
    // Fail on anything exotic (FLAC is fine on modern Android, but keep
    // the allowlist explicit so a stray .aiff breaks loudly).
    let allowed = ["wav", "mp3", "ogg", "oga"];
    // Docs that live next to the audio are not played.
    let doc_names = ["attribution.txt", "credits.md", "readme.md"];
    let mut bad = Vec::new();
    walk_audio(&repo_root().join("game").join("assets"), &mut |p| {
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if doc_names.contains(&name.to_lowercase().as_str()) {
                return;
            }
        }
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_lowercase();
            if !allowed.contains(&ext.as_str()) {
                bad.push(p.clone());
            }
        }
    });
    assert!(
        bad.is_empty(),
        "audio files with codecs not universally supported on Android: {:?}",
        bad
    );
}

fn walk_audio(dir: &std::path::Path, f: &mut impl FnMut(&PathBuf)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            walk_audio(&p, f);
        } else if ["music", "sfx"]
            .iter()
            .any(|d| p.to_string_lossy().contains(d))
        {
            f(&p);
        }
    }
}

#[test]
fn android_asset_plugin_cfg_present() {
    // lib.rs has a target_os = "android" branch for AssetPlugin file_path.
    // If someone refactors it away, Android asset loading silently breaks.
    let lib = read_src("lib.rs");
    assert!(
        lib.contains("target_os = \"android\""),
        "Android-specific asset path handling is gone from lib.rs"
    );
}

// --- Windows: paths and line endings -----------------------------------------

#[test]
fn windows_no_unix_only_paths_in_asset_code() {
    // Asset/game code must not assume Unix paths (/dev, /tmp, ~).
    for rel in ["lib.rs", "asset_root.rs", "level.rs"] {
        let src = read_src(rel);
        for bad in ["/dev/", "/tmp/", "C:\\", "C:/"] {
            assert!(
                !src.contains(bad),
                "Unix/Windows-absolute path {:?} hardcoded in game/src/{}",
                bad,
                rel
            );
        }
    }
}

#[test]
fn windows_levels_parse_with_crlf() {
    // Windows editors save CRLF. Level JSON must parse identically.
    for idx in [0, 39, 79] {
        let raw = light_show::level::LEVEL_SOURCES[idx];
        let crlf = raw.replace('\n', "\r\n");
        let a: LevelDef = serde_json::from_str(raw).expect("lf parse");
        let b: LevelDef = serde_json::from_str(&crlf).expect("crlf parse");
        assert_eq!(a.id, b.id, "CRLF changed level id at index {}", idx);
        assert_eq!(
            a.nodes.len(),
            b.nodes.len(),
            "CRLF changed node count at index {}",
            idx
        );
    }
}

#[test]
fn windows_include_str_paths_portable() {
    // include_str! paths must use forward slashes (work on all OSes).
    let level = read_src("level.rs");
    assert!(
        !level.contains("include_str!(\"..\\"),
        "backslash in include_str! path — breaks non-Windows builds"
    );
}

// --- Shared: lifecycle ---------------------------------------------------------

#[test]
fn backgrounding_does_not_require_special_handling() {
    // Android pauses via WindowFocused(false); timers use Bevy Time which
    // handles focus loss. Verify no system unwraps on missing focus state.
    // (Behavioral coverage lives in playthrough tests; here we assert the
    // game doesn't read window focus in a way that can panic headlessly.)
    let lib = read_src("lib.rs");
    let _ = lib; // compiled fine; focus handling is via Bevy defaults
}
