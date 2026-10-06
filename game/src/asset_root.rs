//! Resolves the one directory the desktop build loads assets from.
//!
//! Bevy's default root is `<exe dir>/assets` unless `BEVY_ASSET_ROOT` or
//! `CARGO_MANIFEST_DIR` is set. That holds for `cargo run`, but an installed
//! binary (e.g. `~/.local/bin/light-show`) has no `assets/` beside it: every
//! font, sprite, and track then fails to load and the game opens to an empty
//! window with no panic. This module keeps Bevy's precedence, adds the XDG
//! data dir (`~/.local/share/light-show/assets`) as an install location, and
//! hands the winner to both `AssetPlugin` and the music decode guard so the
//! two can never read different files.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

const ASSET_DIR_NAME: &str = "assets";
const APP_DIR_NAME: &str = "light-show";

/// The process inputs the search reads, split out so tests can drive it.
#[derive(Debug, Default)]
pub struct AssetSearchEnv {
    pub bevy_asset_root: Option<OsString>,
    pub cargo_manifest_dir: Option<OsString>,
    pub exe_dir: Option<PathBuf>,
    pub xdg_data_home: Option<OsString>,
    pub home: Option<OsString>,
}

impl AssetSearchEnv {
    pub fn from_process() -> Self {
        Self {
            bevy_asset_root: std::env::var_os("BEVY_ASSET_ROOT"),
            cargo_manifest_dir: std::env::var_os("CARGO_MANIFEST_DIR"),
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
            xdg_data_home: std::env::var_os("XDG_DATA_HOME"),
            home: std::env::var_os("HOME"),
        }
    }
}

/// Candidate directories in priority order. An explicit override
/// (`BEVY_ASSET_ROOT`, then `CARGO_MANIFEST_DIR`, Bevy's own order) is the
/// only candidate when set: a wrong override must fail loudly, not fall
/// through to a different, possibly stale, asset tree.
pub fn candidates(env: &AssetSearchEnv) -> Vec<PathBuf> {
    let override_root = env
        .bevy_asset_root
        .as_ref()
        .or(env.cargo_manifest_dir.as_ref());
    if let Some(root) = override_root {
        return vec![PathBuf::from(root).join(ASSET_DIR_NAME)];
    }
    let mut found = Vec::with_capacity(2);
    if let Some(exe_dir) = &env.exe_dir {
        found.push(exe_dir.join(ASSET_DIR_NAME));
    }
    if let Some(data_home) = xdg_data_home(env) {
        found.push(data_home.join(APP_DIR_NAME).join(ASSET_DIR_NAME));
    }
    found
}

/// Per the XDG Base Directory spec: `$XDG_DATA_HOME` only if absolute,
/// else `$HOME/.local/share`. A relative value would resolve against the
/// launch cwd, so it is ignored rather than trusted.
fn xdg_data_home(env: &AssetSearchEnv) -> Option<PathBuf> {
    let from_xdg = env
        .xdg_data_home
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    if from_xdg.is_some() {
        return from_xdg;
    }
    let home = env
        .home
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())?;
    Some(home.join(".local").join("share"))
}

/// Returns the first candidate `probe` accepts, or every candidate tried.
/// `probe` maps a path to its canonical absolute form iff it is a directory.
/// The winner must be UTF-8 because `AssetPlugin::file_path` is a `String`.
pub fn resolve(
    env: &AssetSearchEnv,
    probe: impl Fn(&Path) -> Option<PathBuf>,
) -> Result<PathBuf, Vec<PathBuf>> {
    let tried = candidates(env);
    for candidate in &tried {
        let Some(dir) = probe(candidate) else {
            continue;
        };
        assert!(dir.is_absolute(), "probe must return absolute paths");
        if dir.to_str().is_some() {
            return Ok(dir);
        }
    }
    Err(tried)
}

/// Real filesystem probe for [`resolve`].
pub fn probe_dir(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok().filter(|dir| dir.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(value: &str) -> Option<OsString> {
        Some(OsString::from(value))
    }

    /// Accepts exactly the listed paths, returning them unchanged.
    fn only(existing: &'static [&'static str]) -> impl Fn(&Path) -> Option<PathBuf> {
        move |path| {
            existing
                .iter()
                .any(|e| Path::new(e) == path)
                .then(|| path.to_path_buf())
        }
    }

    fn installed_env() -> AssetSearchEnv {
        AssetSearchEnv {
            exe_dir: Some(PathBuf::from("/home/u/.local/bin")),
            home: os("/home/u"),
            ..Default::default()
        }
    }

    #[test]
    fn installed_binary_finds_xdg_data_assets() {
        let found = resolve(
            &installed_env(),
            only(&["/home/u/.local/share/light-show/assets"]),
        );
        assert_eq!(
            found.unwrap(),
            PathBuf::from("/home/u/.local/share/light-show/assets")
        );
    }

    #[test]
    fn assets_beside_the_exe_win_over_xdg() {
        let probe = only(&[
            "/home/u/.local/bin/assets",
            "/home/u/.local/share/light-show/assets",
        ]);
        let found = resolve(&installed_env(), probe);
        assert_eq!(found.unwrap(), PathBuf::from("/home/u/.local/bin/assets"));
    }

    #[test]
    fn xdg_data_home_replaces_home_default() {
        let env = AssetSearchEnv {
            xdg_data_home: os("/data"),
            ..installed_env()
        };
        let found = resolve(&env, only(&["/data/light-show/assets"]));
        assert_eq!(found.unwrap(), PathBuf::from("/data/light-show/assets"));
    }

    #[test]
    fn cargo_run_uses_manifest_dir() {
        let env = AssetSearchEnv {
            cargo_manifest_dir: os("/repo/game"),
            ..installed_env()
        };
        let found = resolve(&env, only(&["/repo/game/assets"]));
        assert_eq!(found.unwrap(), PathBuf::from("/repo/game/assets"));
    }

    #[test]
    fn missing_everywhere_reports_every_path_tried() {
        let tried = resolve(&installed_env(), only(&[])).unwrap_err();
        let expected = [
            "/home/u/.local/bin/assets",
            "/home/u/.local/share/light-show/assets",
        ];
        assert_eq!(tried, expected.map(PathBuf::from).to_vec());
    }

    #[test]
    fn broken_override_fails_instead_of_falling_through() {
        let env = AssetSearchEnv {
            bevy_asset_root: os("/nope"),
            ..installed_env()
        };
        let tried = resolve(&env, only(&["/home/u/.local/share/light-show/assets"])).unwrap_err();
        assert_eq!(tried, vec![PathBuf::from("/nope/assets")]);
    }

    #[test]
    fn bevy_asset_root_beats_cargo_manifest_dir() {
        let env = AssetSearchEnv {
            bevy_asset_root: os("/a"),
            cargo_manifest_dir: os("/b"),
            ..installed_env()
        };
        assert_eq!(candidates(&env), vec![PathBuf::from("/a/assets")]);
    }

    #[test]
    fn relative_xdg_and_home_are_ignored() {
        let env = AssetSearchEnv {
            exe_dir: None,
            xdg_data_home: os("rel/data"),
            home: os("rel/home"),
            ..Default::default()
        };
        assert!(candidates(&env).is_empty());
    }

    #[test]
    fn relative_xdg_falls_back_to_home() {
        let env = AssetSearchEnv {
            xdg_data_home: os("rel"),
            ..installed_env()
        };
        let found = resolve(&env, only(&["/home/u/.local/share/light-show/assets"]));
        assert!(found.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_directory_is_skipped() {
        use std::os::unix::ffi::OsStringExt;
        let bad = PathBuf::from(OsString::from_vec(b"/home/u/.local/bin/\xff".to_vec()));
        let probe = move |path: &Path| {
            if path == Path::new("/home/u/.local/bin/assets") {
                return Some(bad.clone());
            }
            (path == Path::new("/home/u/.local/share/light-show/assets"))
                .then(|| path.to_path_buf())
        };
        let found = resolve(&installed_env(), probe);
        assert_eq!(
            found.unwrap(),
            PathBuf::from("/home/u/.local/share/light-show/assets")
        );
    }
}
