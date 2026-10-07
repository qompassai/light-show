//! Save-file integration tests: the public save API from outside the crate.
//!
//! Covers the player-facing contract: progress persists across sessions,
//! corrupt saves degrade gracefully, and the format stays valid.

use light_show::save::{self, SaveData};
use light_show::waifu::Companion;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_save_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("light-show-itest-{}-{}", tag, std::process::id()))
}

fn with_save_dir(tag: &str, f: impl FnOnce(&PathBuf)) {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = temp_save_dir(tag);
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
    f(&dir);
    std::env::remove_var("LIGHTSHOW_SAVE_DIR");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_unlocks_survive_reload() {
    with_save_dir("unlocks", |_| {
        let mut data = SaveData::default();
        assert!(data.unlock_specialist(Companion::Clara));
        assert!(data.unlock_specialist(Companion::Lea));
        save::save(&data).expect("save");
        let loaded = save::load();
        let set = loaded.unlocked_set();
        assert!(set.contains(&Companion::Clara));
        assert!(set.contains(&Companion::Lea));
        assert_eq!(set.len(), 2);
    });
}

#[test]
fn save_level_progress_survives_reload() {
    with_save_dir("progress", |_| {
        let mut data = SaveData::default();
        for id in ["c1l1", "c1l2", "m1l5"] {
            assert!(data.complete_level(id));
        }
        save::save(&data).expect("save");
        let loaded = save::load();
        assert!(loaded.is_completed("c1l1"));
        assert!(loaded.is_completed("m1l5"));
        assert!(!loaded.is_completed("c1l3"));
    });
}

#[test]
fn save_cores_and_settings_survive_reload() {
    with_save_dir("cores", |_| {
        let mut data = SaveData::default();
        data.cores = 42;
        data.settings.master_volume = 0.5;
        save::save(&data).expect("save");
        let loaded = save::load();
        assert_eq!(loaded.cores, 42);
        assert!((loaded.settings.master_volume - 0.5).abs() < f32::EPSILON);
    });
}

#[test]
fn save_is_valid_json_with_version() {
    with_save_dir("format", |dir| {
        let data = SaveData::default();
        save::save(&data).expect("save");
        let text = std::fs::read_to_string(dir.join(light_show::save::SAVE_FILENAME))
            .expect("read save file");
        let v: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(v["version"], light_show::save::SAVE_VERSION);
        assert!(v["completed_levels"].is_array());
        assert!(v["unlocked_specialists"].is_array());
    });
}

#[test]
fn save_empty_string_env_falls_through() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var("LIGHTSHOW_SAVE_DIR", "");
    // Empty override must not produce a bogus path; falls to platform default.
    let path = save::save_path();
    std::env::remove_var("LIGHTSHOW_SAVE_DIR");
    // On this machine dirs::data_dir() exists, so we get a real path —
    // the point is it didn't join "" into a relative "save.json".
    if let Some(p) = path {
        assert!(
            p.is_absolute(),
            "save path should be absolute, got {}",
            p.display()
        );
    }
}

#[test]
fn save_garbage_file_loads_defaults_safely() {
    with_save_dir("garbage", |dir| {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(light_show::save::SAVE_FILENAME),
            b"\x00\xff\xfe garbage",
        )
        .unwrap();
        let loaded = save::load();
        assert_eq!(loaded.completed_levels.len(), 0);
        assert_eq!(loaded.cores, 0);
    });
}

#[test]
fn save_unknown_specialist_names_ignored() {
    with_save_dir("unknown", |dir| {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(light_show::save::SAVE_FILENAME),
            r#"{"version":1,"completed_levels":[],"unlocked_specialists":["Clara","Nope"],"favor_points":0,"settings":{"master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0}}"#,
        )
        .unwrap();
        let loaded = save::load();
        let set = loaded.unlocked_set();
        assert!(set.contains(&Companion::Clara));
        assert_eq!(set.len(), 1, "unknown names must be dropped");
    });
}
