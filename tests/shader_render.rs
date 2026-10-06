//! Rendering pipeline tests: shaders, sprites, fonts, and every asset path
//! the renderer touches.
//!
//! light-show ships **no custom shaders** — all rendering goes through
//! Bevy's built-in 2D pipelines (sprites, text via parley, gizmos, UI).
//! These tests verify that contract and that every image/font path the
//! game loads resolves on a **case-sensitive** filesystem (Android APKs
//! are case-sensitive; a `Foo.PNG` vs `foo.png` mismatch that works on
//! Windows/macOS breaks on device).

use light_show::waifu::Companion;

/// Mirrors the path constants in `game/src/fonts.rs` (that module is
/// private; these tests verify the files those constants point at).
use std::path::{Path, PathBuf};

const FONTS: [&str; 4] = [
    "fonts/MonaspaceNeon-Regular.otf",
    "fonts/MonaspaceNeon-Bold.otf",
    "fonts/Inter-Regular.ttf",
    "fonts/Inter-Medium.ttf",
];

fn repo_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn asset(rel: &str) -> PathBuf {
    repo_root().join("game").join("assets").join(rel)
}

/// Case-sensitive existence: `Path::exists` follows the OS, so on a
/// case-insensitive dev machine we additionally verify the exact casing
/// by comparing against the parent directory listing.
fn exists_case_sensitive(path: &Path) -> bool {
    let parent = match path.parent() {
        Some(p) => p,
        None => return false,
    };
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };
    let entries = match std::fs::read_dir(parent) {
        Ok(e) => e,
        Err(_) => return false,
    };
    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .any(|entry| entry == name)
}

fn assert_asset(rel: &str) {
    let p = asset(rel);
    assert!(
        p.exists(),
        "asset missing: {} (resolved to {})",
        rel,
        p.display()
    );
    assert!(
        exists_case_sensitive(&p),
        "asset path casing mismatch (breaks on Android): {}",
        rel
    );
}

// --- Shader contract: no custom shaders ---------------------------------

#[test]
fn custom_shaders_are_expected_and_portable() {
    // The canary fired 2026-10-06: Matt directed all four WGSL benefit
    // cases be implemented (glow, pulse, atmosphere, wipe). This test now
    // asserts the shaders exist AND stay within the portable subset
    // (Vulkan/Metal/D3D12/GLES3-safe constructs only).
    let mut wgsl = Vec::new();
    collect(&repo_root().join("game"), "wgsl", &mut wgsl);
    let names: Vec<String> = wgsl
        .iter()
        .filter_map(|p| p.file_name()?.to_str().map(String::from))
        .collect();
    for expected in ["glow.wgsl", "pulse.wgsl", "atmosphere.wgsl", "wipe.wgsl"] {
        assert!(
            names.iter().any(|n| n == expected),
            "expected shader {} is missing; found: {:?}",
            expected,
            names
        );
    }
    // Portability: no non-portable constructs. GLES3 (Android) is the
    // most restrictive target; these would break or misbehave there.
    let banned = [
        // Dynamic indexing of matrices/arrays is restricted on GLES3.
        "dynamic",
        // Compute shaders need explicit backend testing; we use none.
        "@compute",
        // 64-bit floats are not portable to mobile GPUs.
        "f64",
        // textureSampleLevel with explicit LOD can behave differently;
        // our shaders don't sample textures at all.
        "textureSampleLevel",
    ];
    for path in &wgsl {
        let src = std::fs::read_to_string(path).unwrap_or_default();
        // Strip Bevy #import lines (preprocessor, not WGSL).
        let body: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("#import"))
            .collect::<Vec<_>>()
            .join("\n");
        for b in &banned {
            assert!(
                !body.contains(b),
                "{:?} contains non-portable construct {:?}",
                path,
                b
            );
        }
        // Every shader must declare its bind group explicitly.
        assert!(
            body.contains("@group("),
            "{:?} has no explicit bind group",
            path
        );
    }
}

fn collect(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            if p.file_name().map(|n| n != "target").unwrap_or(true) {
                collect(&p, ext, out);
            }
        } else if p.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(p);
        }
    }
}

// --- Companion portraits and sprite sheets -------------------------------

/// Companions actually selectable in-game (`Companion::ALL` drives the
/// companion-select screen). Clara/Aino/Hikari/Lea have enum variants and
/// levels but are not yet selectable; their art gaps are tracked in
/// `unshipped_companions_art_gaps_are_known` below.
const SHIPPED_COMPANIONS: [Companion; 4] = Companion::ALL;

#[test]
fn all_portrait_paths_resolve() {
    for c in SHIPPED_COMPANIONS {
        assert_asset(c.portrait_path());
    }
}

#[test]
fn all_sprite_sheet_paths_resolve() {
    for c in SHIPPED_COMPANIONS {
        assert_asset(c.sprite_path());
    }
}

#[test]
fn all_picker_frames_resolve() {
    // sprites/picker/{stem}_select_{i}.png, i in 0..SELECT_ANIM_FRAMES (6).
    for c in SHIPPED_COMPANIONS {
        for i in 0..6 {
            assert_asset(&format!("sprites/picker/{}_select_{}.png", c.picker_stem(), i));
        }
    }
}

// --- FX animation frames --------------------------------------------------

#[test]
fn fx_frame_ranges_resolve() {
    // Must match the *_FRAMES constants in game/src/fx.rs and results.rs.
    for i in 0..4 {
        assert_asset(&format!("sprites/fx/connect_spark_{}.png", i));
    }
    for i in 0..6 {
        assert_asset(&format!("sprites/fx/success_burst_{}.png", i));
    }
    for i in 0..6 {
        assert_asset(&format!("sprites/fx/win_ring_{}.png", i));
    }
}

// --- Fonts (parley text engine) -------------------------------------------

#[test]
fn all_font_paths_resolve() {
    for f in FONTS {
        assert_asset(f);
    }
}

// --- Static sprite references ---------------------------------------------

#[test]
fn static_sprite_paths_resolve() {
    for rel in [
        "sprites/fx/pulse_dot.png",
        "sprites/fx/storm_streak.png",
        "sprites/ui/board_bg.png",
        "sprites/ui/title_artwork.png",
        "sprites/ui/node_ring_gold.png",
        "sprites/ui/node_ring_tap.png",
        "sprites/ui/node_ring_site.png",
        "sprites/ui/node_ring_cyan.png",
        "sprites/ui/pill_ring_selected.png",
        "sprites/ui/pill_ring_normal.png",
    ] {
        assert_asset(rel);
    }
}

// --- Unshipped companions: known art gaps ------------------------------------

#[test]
fn unshipped_companions_art_gaps_are_known() {
    // Clara/Aino/Hikari/Lea are not in Companion::ALL (not selectable).
    // Their art is unfinished. This test pins the CURRENT gap set so a
    // missing file can never be mistaken for a regression, and so adding
    // the art is a visible event that should promote them into ALL.
    // (Léa's portrait is an acknowledged placeholder in waifu/mod.rs.)
    let gaps = [
        (Companion::Clara, "sprites/clara/clara_sheet_fullbody.png"),
        (Companion::Aino, "sprites/aino/aino_sheet_fullbody.png"),
        (Companion::Hikari, "sprites/hikari/hikari_sheet_fullbody.png"),
        (Companion::Lea, "sprites/lea/lea_sheet_fullbody.png"),
        (Companion::Lea, "art/companions/lea_portrait.jpg"),
    ];
    for (companion, rel) in gaps {
        assert!(
            !asset(rel).exists(),
            "{:?} art gap unexpectedly filled: {} — promote {:?} into Companion::ALL",
            companion,
            rel,
            companion
        );
    }
}

// --- Adversarial: path hygiene ---------------------------------------------

#[test]
fn asset_paths_use_forward_slashes_only() {
    // Bevy asset paths are `/`-separated; a backslash compiles on Windows
    // but breaks on Android/Linux asset resolution.
    for c in SHIPPED_COMPANIONS {
        for p in [c.portrait_path(), c.sprite_path()] {
            assert!(
                !p.contains('\\'),
                "backslash in asset path: {}",
                p
            );
        }
    }
    for f in FONTS {
        assert!(!f.contains('\\'), "backslash in font path: {}", f);
    }
}

#[test]
fn no_duplicate_asset_basenames_case_insensitive() {
    // Two files in the SAME directory differing only by case (`Foo.png`
    // vs `foo.png`) are indistinguishable on some filesystems and collide
    // in APK tooling. Same name in different directories is fine.
    walk_dirs(&repo_root().join("game").join("assets"), &mut |dir| {
        let mut seen = std::collections::HashMap::new();
        let entries = std::fs::read_dir(dir).unwrap();
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    let lower = name.to_lowercase();
                    if let Some(prev) = seen.insert(lower.clone(), p.clone()) {
                        panic!(
                            "case-insensitive basename collision in {} (breaks Android packaging):\n  {}\n  {}",
                            dir.display(),
                            prev.display(),
                            p.display()
                        );
                    }
                }
            }
        }
    });
}

fn walk_dirs(dir: &Path, f: &mut impl FnMut(&Path)) {
    f(dir);
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            walk_dirs(&p, f);
        }
    }
}

fn walk_files(dir: &Path, f: &mut impl FnMut(&PathBuf)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            walk_files(&p, f);
        } else {
            f(&p);
        }
    }
}
