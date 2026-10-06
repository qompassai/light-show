//! Repository structure integration tests.
//!
//! Validation: workspace membership and shared package metadata, publication
//! inputs (fastlane metadata, Android packaging, store docs), game asset
//! directories with parseable JSON, and basic repo hygiene.
//!
//! Every validator takes a root path and returns a list of distinct
//! `Failure`s instead of panicking, so the adversarial tests can point the
//! same code at a temp dir with pieces removed and assert each absence is
//! reported on its own.

use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Manifests and docs are small text; refuse anything bigger than this.
const TEXT_BYTES_MAX: u64 = 1024 * 1024;
/// Upper bound on entries read from one directory listing.
const DIR_ENTRY_COUNT_MAX: usize = 4096;

const EXPECTED_MEMBERS: &[&str] = &["crates/osp_sim", "game", "tests"];
/// Keys every member manifest must inherit from `[workspace.package]`.
const INHERITED_KEYS: &[&str] = &["version", "edition", "license"];
/// Editions defined by the Rust reference as of edition 2024.
const KNOWN_EDITIONS: &[&str] = &["2015", "2018", "2021", "2024"];

const FASTLANE_LOCALE_DIR: &str = "fastlane/metadata/android";
const FASTLANE_REQUIRED_LOCALE: &str = "en-US";
const FASTLANE_LOCALE_FILES: &[&str] =
    &["title.txt", "short_description.txt", "full_description.txt"];

const ANDROID_FILES: &[&str] = &[
    "android/app/src/main/AndroidManifest.xml",
    "android/app/build.gradle.kts",
    "android/build.gradle.kts",
    "android/settings.gradle.kts",
    "android/gradle.properties",
    "android/gradle/wrapper/gradle-wrapper.properties",
    "android/gradle/wrapper/gradle-wrapper.jar",
    "android/gradlew",
    "android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml",
    "android/app/src/main/res/mipmap-anydpi-v26/ic_launcher_round.xml",
];
const ANDROID_ICON_DENSITIES: &[&str] = &["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"];

const REQUIRED_DOCS: &[&str] = &["docs/BUILD.md", "docs/FDROID.md", "docs/GAME_DESIGN.md"];
const ASSET_DIRS: &[&str] = &["dialogue", "fonts", "levels", "music", "sfx", "sprites"];
const JSON_ASSET_DIRS: &[&str] = &["levels", "dialogue"];
const HYGIENE_FILES: &[&str] = &["README.md", "LICENSE"];
const TODO_FILE_CANDIDATES: &[&str] = &["TODO.md", "docs/TODO.md", "game/assets/TODO.md"];

fn repo_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

// ---------------------------------------------------------------------------
// Failure model
// ---------------------------------------------------------------------------

/// One structural defect. Paths are relative to the validated root.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Failure {
    MissingDir(PathBuf),
    MissingFile(PathBuf),
    EmptyFile(PathBuf),
    Unreadable {
        path: PathBuf,
        reason: String,
    },
    InvalidJson {
        path: PathBuf,
        reason: String,
    },
    /// A directory that must hold at least one `what` holds none.
    NothingMatching {
        dir: PathBuf,
        what: &'static str,
    },
    BadChangelogName(PathBuf),
    WorkspaceMembers {
        found: Vec<String>,
    },
    WorkspacePackageKey {
        key: &'static str,
        problem: String,
    },
    NotInherited {
        manifest: PathBuf,
        key: &'static str,
    },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDir(path) => write!(f, "missing dir {}", path.display()),
            Self::MissingFile(path) => write!(f, "missing file {}", path.display()),
            Self::EmptyFile(path) => write!(f, "empty file {}", path.display()),
            Self::Unreadable { path, reason } => {
                write!(f, "unreadable {}: {reason}", path.display())
            }
            Self::InvalidJson { path, reason } => {
                write!(f, "invalid JSON {}: {reason}", path.display())
            }
            Self::NothingMatching { dir, what } => write!(f, "no {what} in {}", dir.display()),
            Self::BadChangelogName(path) => {
                write!(
                    f,
                    "changelog not named <versionCode>.txt: {}",
                    path.display()
                )
            }
            Self::WorkspaceMembers { found } => write!(f, "workspace members {found:?}"),
            Self::WorkspacePackageKey { key, problem } => {
                write!(f, "[workspace.package] {key}: {problem}")
            }
            Self::NotInherited { manifest, key } => {
                write!(f, "{} lacks `{key}.workspace = true`", manifest.display())
            }
        }
    }
}

fn report(failures: &[Failure]) -> String {
    failures
        .iter()
        .map(|failure| format!("  - {failure}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Filesystem primitives
// ---------------------------------------------------------------------------

fn unreadable(relative: &Path, source: &io::Error) -> Failure {
    Failure::Unreadable {
        path: relative.to_path_buf(),
        reason: source.to_string(),
    }
}

fn require_dir(root: &Path, relative: &Path, failures: &mut Vec<Failure>) -> bool {
    match fs::metadata(root.join(relative)) {
        Ok(metadata) if metadata.is_dir() => true,
        Ok(_) => {
            failures.push(Failure::MissingDir(relative.to_path_buf()));
            false
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            failures.push(Failure::MissingDir(relative.to_path_buf()));
            false
        }
        Err(source) => {
            failures.push(unreadable(relative, &source));
            false
        }
    }
}

/// Reads `root/relative` as UTF-8 text, bounded by `TEXT_BYTES_MAX`.
/// Missing, non-regular, empty, oversized, and unreadable files each map to
/// their own `Failure`.
fn read_text(root: &Path, relative: &Path) -> Result<String, Failure> {
    let path = root.join(relative);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(Failure::MissingFile(relative.to_path_buf()));
        }
        Err(source) => return Err(unreadable(relative, &source)),
    };
    if !metadata.is_file() {
        let reason = "not a regular file".to_owned();
        return Err(Failure::Unreadable {
            path: relative.to_path_buf(),
            reason,
        });
    }
    if metadata.len() == 0 {
        return Err(Failure::EmptyFile(relative.to_path_buf()));
    }
    if metadata.len() > TEXT_BYTES_MAX {
        let reason = format!("{} bytes > {TEXT_BYTES_MAX}", metadata.len());
        return Err(Failure::Unreadable {
            path: relative.to_path_buf(),
            reason,
        });
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .and_then(|file| file.take(TEXT_BYTES_MAX).read_to_end(&mut bytes))
        .map_err(|source| unreadable(relative, &source))?;
    String::from_utf8(bytes).map_err(|source| Failure::Unreadable {
        path: relative.to_path_buf(),
        reason: source.to_string(),
    })
}

/// Like `read_text` but for binary files: present, regular, non-empty, and
/// openable. Content is not interpreted.
fn require_nonempty_file(root: &Path, relative: &Path, failures: &mut Vec<Failure>) {
    let path = root.join(relative);
    let checked = match fs::metadata(&path) {
        Ok(metadata) if !metadata.is_file() => Err(Failure::Unreadable {
            path: relative.to_path_buf(),
            reason: "not a regular file".to_owned(),
        }),
        Ok(metadata) if metadata.len() == 0 => Err(Failure::EmptyFile(relative.to_path_buf())),
        Ok(_) => fs::File::open(&path)
            .map(drop)
            .map_err(|source| unreadable(relative, &source)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            Err(Failure::MissingFile(relative.to_path_buf()))
        }
        Err(source) => Err(unreadable(relative, &source)),
    };
    if let Err(failure) = checked {
        failures.push(failure);
    }
}

/// Sorted entry names of `root/relative`, bounded by `DIR_ENTRY_COUNT_MAX`.
/// Non-UTF-8 names are reported as unreadable rather than skipped.
fn list_dir(root: &Path, relative: &Path) -> Result<Vec<String>, Failure> {
    let entries =
        fs::read_dir(root.join(relative)).map_err(|source| unreadable(relative, &source))?;
    let mut names = Vec::new();
    for entry in entries {
        if names.len() == DIR_ENTRY_COUNT_MAX {
            let reason = format!("more than {DIR_ENTRY_COUNT_MAX} entries");
            return Err(Failure::Unreadable {
                path: relative.to_path_buf(),
                reason,
            });
        }
        let entry = entry.map_err(|source| unreadable(relative, &source))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|raw| Failure::Unreadable {
                path: relative.join(&raw),
                reason: "non-UTF-8 file name".to_owned(),
            })?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}

// ---------------------------------------------------------------------------
// Cargo.toml (minimal, line-oriented: the tests crate has no TOML parser)
// ---------------------------------------------------------------------------

/// Lines belonging to `[section]`, comments and blanks removed.
fn section_lines<'a>(toml: &'a str, section: &str) -> Vec<&'a str> {
    let header = format!("[{section}]");
    let mut inside = false;
    let mut lines = Vec::new();
    for line in toml.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == header;
            continue;
        }
        if inside && !line.is_empty() && !line.starts_with('#') {
            lines.push(line);
        }
    }
    lines
}

/// Quoted strings of `members = [ ... ]` in `[workspace]`, which may span
/// lines. `None` when the key is absent or its array is unterminated.
fn workspace_members(toml: &str) -> Option<Vec<String>> {
    let lines = section_lines(toml, "workspace");
    let start = lines.iter().position(|line| line.starts_with("members"))?;
    let mut array = String::new();
    for line in &lines[start..] {
        array.push_str(line);
        if line.contains(']') {
            let (_, body) = array.split_once('[')?;
            let (body, _) = body.split_once(']')?;
            let members = body.split(',').map(|item| item.trim().trim_matches('"'));
            return Some(
                members
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect(),
            );
        }
    }
    None
}

/// Value of a `key = "value"` line in `[section]`; arrays and tables return `None`.
fn string_key<'a>(toml: &'a str, section: &str, key: &str) -> Option<&'a str> {
    section_lines(toml, section).into_iter().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        let value = value.trim();
        let quoted = value.len() >= 2 && value.starts_with('"') && value.ends_with('"');
        (name.trim() == key && quoted).then(|| &value[1..value.len() - 1])
    })
}

fn is_semver_core(version: &str) -> bool {
    let core = version.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

/// Why a `[workspace.package]` value is invalid, or `None` if it is valid.
fn package_value_problem(key: &str, value: &str) -> Option<&'static str> {
    match key {
        "version" if !is_semver_core(value) => Some("not MAJOR.MINOR.PATCH"),
        "edition" if !KNOWN_EDITIONS.contains(&value) => Some("unknown edition"),
        "license" if value.trim().is_empty() => Some("blank"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Validators
// ---------------------------------------------------------------------------

/// Root manifest lists exactly `EXPECTED_MEMBERS`, `[workspace.package]`
/// carries a valid version/edition/license, and every member inherits them.
fn check_workspace(root: &Path) -> Vec<Failure> {
    let mut failures = Vec::new();
    let toml = match read_text(root, Path::new("Cargo.toml")) {
        Ok(toml) => toml,
        Err(failure) => return vec![failure],
    };
    let mut found = workspace_members(&toml).unwrap_or_default();
    found.sort();
    if found != EXPECTED_MEMBERS {
        failures.push(Failure::WorkspaceMembers { found });
    }
    for key in INHERITED_KEYS {
        let problem = match string_key(&toml, "workspace.package", key) {
            Some(value) => {
                package_value_problem(key, value).map(|reason| format!("{reason}: {value:?}"))
            }
            None => Some("missing".to_owned()),
        };
        if let Some(problem) = problem {
            failures.push(Failure::WorkspacePackageKey { key, problem });
        }
    }
    for member in EXPECTED_MEMBERS {
        let manifest = Path::new(member).join("Cargo.toml");
        let text = match read_text(root, &manifest) {
            Ok(text) => text,
            Err(failure) => {
                failures.push(failure);
                continue;
            }
        };
        for key in INHERITED_KEYS {
            let inherited = format!("{key}.workspace = true");
            if !section_lines(&text, "package").contains(&inherited.as_str()) {
                failures.push(Failure::NotInherited {
                    manifest: manifest.clone(),
                    key,
                });
            }
        }
    }
    failures
}

/// Fastlane store metadata: `en-US` exists; every locale has title, short and
/// full descriptions and at least one `<versionCode>.txt` changelog; `en-US`
/// carries the icon and at least one phone screenshot.
fn check_fastlane(root: &Path) -> Vec<Failure> {
    let mut failures = Vec::new();
    let base = Path::new(FASTLANE_LOCALE_DIR);
    if !require_dir(root, base, &mut failures) {
        return failures;
    }
    let locales = match list_dir(root, base) {
        Ok(names) => names,
        Err(failure) => return vec![failure],
    };
    if !locales
        .iter()
        .any(|locale| locale == FASTLANE_REQUIRED_LOCALE)
    {
        failures.push(Failure::MissingDir(base.join(FASTLANE_REQUIRED_LOCALE)));
    }
    for locale in &locales {
        let locale_dir = base.join(locale);
        for file in FASTLANE_LOCALE_FILES {
            if let Err(failure) = read_text(root, &locale_dir.join(file)) {
                failures.push(failure);
            }
        }
        check_changelogs(root, &locale_dir.join("changelogs"), &mut failures);
    }
    if locales
        .iter()
        .any(|locale| locale == FASTLANE_REQUIRED_LOCALE)
    {
        let images = base.join(FASTLANE_REQUIRED_LOCALE).join("images");
        require_nonempty_file(root, &images.join("icon.png"), &mut failures);
        let screenshots = images.join("phoneScreenshots");
        if require_dir(root, &screenshots, &mut failures) {
            match list_dir(root, &screenshots) {
                Ok(names) if names.iter().any(|name| name.ends_with(".png")) => {}
                Ok(_) => failures.push(Failure::NothingMatching {
                    dir: screenshots,
                    what: "*.png",
                }),
                Err(failure) => failures.push(failure),
            }
        }
    }
    failures
}

fn check_changelogs(root: &Path, dir: &Path, failures: &mut Vec<Failure>) {
    if !require_dir(root, dir, failures) {
        return;
    }
    let names = match list_dir(root, dir) {
        Ok(names) => names,
        Err(failure) => {
            failures.push(failure);
            return;
        }
    };
    if names.is_empty() {
        failures.push(Failure::NothingMatching {
            dir: dir.to_path_buf(),
            what: "changelog",
        });
    }
    for name in names {
        let path = dir.join(&name);
        let stem = name.strip_suffix(".txt").unwrap_or("");
        if stem.is_empty() || !stem.bytes().all(|b| b.is_ascii_digit()) {
            failures.push(Failure::BadChangelogName(path));
        } else if let Err(failure) = read_text(root, &path) {
            failures.push(failure);
        }
    }
}

/// Android packaging inputs: manifest, Gradle build + wrapper, launcher icons
/// at every density, and the store docs.
fn check_android_and_docs(root: &Path) -> Vec<Failure> {
    let mut failures = Vec::new();
    for file in ANDROID_FILES {
        require_nonempty_file(root, Path::new(file), &mut failures);
    }
    let res = Path::new("android/app/src/main/res");
    for density in ANDROID_ICON_DENSITIES {
        for icon in ["ic_launcher.png", "ic_launcher_round.png"] {
            let path = res.join(format!("mipmap-{density}")).join(icon);
            require_nonempty_file(root, &path, &mut failures);
        }
    }
    for doc in REQUIRED_DOCS {
        if let Err(failure) = read_text(root, Path::new(doc)) {
            failures.push(failure);
        }
    }
    failures
}

/// `game/assets/<dir>` exists for every `ASSET_DIRS` entry; each
/// `JSON_ASSET_DIRS` dir holds at least one `*.json` and all of them parse.
fn check_asset_dirs(root: &Path) -> Vec<Failure> {
    let mut failures = Vec::new();
    let assets = Path::new("game/assets");
    for dir in ASSET_DIRS {
        require_dir(root, &assets.join(dir), &mut failures);
    }
    for dir in JSON_ASSET_DIRS {
        let dir = assets.join(dir);
        if !root.join(&dir).is_dir() {
            continue; // Already reported as MissingDir above.
        }
        let names = match list_dir(root, &dir) {
            Ok(names) => names,
            Err(failure) => {
                failures.push(failure);
                continue;
            }
        };
        let json_names: Vec<&String> = names
            .iter()
            .filter(|name| name.ends_with(".json"))
            .collect();
        if json_names.is_empty() {
            failures.push(Failure::NothingMatching {
                dir: dir.clone(),
                what: "*.json",
            });
        }
        for name in json_names {
            let path = dir.join(name);
            match read_text(root, &path) {
                Ok(text) => {
                    if let Err(source) = serde_json::from_str::<serde_json::Value>(&text) {
                        failures.push(Failure::InvalidJson {
                            path,
                            reason: source.to_string(),
                        });
                    }
                }
                Err(failure) => failures.push(failure),
            }
        }
    }
    failures
}

fn check_hygiene(root: &Path) -> Vec<Failure> {
    HYGIENE_FILES
        .iter()
        .filter_map(|file| read_text(root, Path::new(file)).err())
        .collect()
}

/// Every validator, concatenated in a fixed order.
fn validate_repo(root: &Path) -> Vec<Failure> {
    let mut failures = check_workspace(root);
    failures.extend(check_fastlane(root));
    failures.extend(check_android_and_docs(root));
    failures.extend(check_asset_dirs(root));
    failures.extend(check_hygiene(root));
    failures
}

/// Report-only: TODO.md files present, and `TODO` lines inside required
/// store-facing docs. Never a failure; printed for the human reviewer.
fn todo_notices(root: &Path) -> Vec<String> {
    let mut notices = Vec::new();
    for candidate in TODO_FILE_CANDIDATES {
        if root.join(candidate).is_file() {
            notices.push(format!("TODO file present: {candidate}"));
        }
    }
    let fastlane = format!("{FASTLANE_LOCALE_DIR}/{FASTLANE_REQUIRED_LOCALE}");
    let fastlane_files = FASTLANE_LOCALE_FILES
        .iter()
        .map(|file| format!("{fastlane}/{file}"));
    let docs = REQUIRED_DOCS
        .iter()
        .chain(HYGIENE_FILES)
        .map(|doc| (*doc).to_owned());
    for doc in docs.chain(fastlane_files) {
        let Ok(text) = read_text(root, Path::new(&doc)) else {
            continue; // Absence is a Failure elsewhere, not a notice.
        };
        for (index, line) in text
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains("TODO"))
        {
            notices.push(format!("{doc}:{}: {}", index + 1, line.trim()));
        }
    }
    notices
}

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

/// Scratch directory under the OS temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        static SEQUENCE: AtomicU32 = AtomicU32::new(0);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let name = format!(
            "light-show-structural-{label}-{}-{nanos}-{sequence}",
            std::process::id()
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).expect("temp dir must be creatable for adversarial fixtures");
        Self(path)
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent dir must be creatable");
        }
        fs::write(&path, contents).expect("fixture file must be writable");
    }

    fn remove(&self, relative: &str) {
        let path = self.0.join(relative);
        let removed = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        removed.expect("fixture path must exist before removal");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best-effort cleanup: a leftover dir in /tmp must not fail a test.
        let _ = fs::remove_dir_all(&self.0);
    }
}

const FIXTURE_ROOT_TOML: &str = "[workspace]\nresolver = \"2\"\nmembers = [\n    \
    \"crates/osp_sim\",\n    \"game\",\n    \"tests\",\n]\n\n[workspace.package]\n\
    version = \"0.1.0\"\nedition = \"2021\"\nlicense = \"Apache-2.0\"\n";
const FIXTURE_MEMBER_TOML: &str = "[package]\nname = \"m\"\nversion.workspace = true\n\
    edition.workspace = true\nlicense.workspace = true\n";

/// A minimal tree that passes `validate_repo`; tests then break one piece.
fn complete_fixture(label: &str) -> TempDir {
    let temp = TempDir::new(label);
    temp.write("Cargo.toml", FIXTURE_ROOT_TOML);
    for member in EXPECTED_MEMBERS {
        temp.write(&format!("{member}/Cargo.toml"), FIXTURE_MEMBER_TOML);
    }
    let locale = format!("{FASTLANE_LOCALE_DIR}/{FASTLANE_REQUIRED_LOCALE}");
    for file in FASTLANE_LOCALE_FILES {
        temp.write(&format!("{locale}/{file}"), "text");
    }
    temp.write(&format!("{locale}/changelogs/1.txt"), "first release");
    temp.write(&format!("{locale}/images/icon.png"), "png");
    temp.write(&format!("{locale}/images/phoneScreenshots/01.png"), "png");
    for file in ANDROID_FILES {
        temp.write(file, "x");
    }
    for density in ANDROID_ICON_DENSITIES {
        for icon in ["ic_launcher.png", "ic_launcher_round.png"] {
            temp.write(
                &format!("android/app/src/main/res/mipmap-{density}/{icon}"),
                "png",
            );
        }
    }
    for file in REQUIRED_DOCS.iter().chain(HYGIENE_FILES) {
        temp.write(file, "content");
    }
    for dir in ASSET_DIRS {
        temp.write(&format!("game/assets/{dir}/placeholder.txt"), "x");
    }
    for dir in JSON_ASSET_DIRS {
        temp.write(&format!("game/assets/{dir}/sample.json"), "{\"ok\": true}");
    }
    temp
}

fn assert_failures(temp: &TempDir, expected: &[Failure]) {
    let failures = validate_repo(&temp.0);
    assert_eq!(failures, expected, "got:\n{}", report(&failures));
}

// ---------------------------------------------------------------------------
// Validation (real repo)
// ---------------------------------------------------------------------------

#[test]
fn workspace_members_and_package_metadata_consistent() {
    let failures = check_workspace(&repo_root());
    assert!(failures.is_empty(), "workspace:\n{}", report(&failures));
}

#[test]
fn fastlane_metadata_complete() {
    let failures = check_fastlane(&repo_root());
    assert!(failures.is_empty(), "fastlane:\n{}", report(&failures));
}

#[test]
fn android_packaging_inputs_and_store_docs_present() {
    let failures = check_android_and_docs(&repo_root());
    assert!(failures.is_empty(), "android/docs:\n{}", report(&failures));
}

#[test]
fn asset_dirs_present_and_json_parses() {
    let failures = check_asset_dirs(&repo_root());
    assert!(failures.is_empty(), "assets:\n{}", report(&failures));
}

#[test]
fn readme_and_license_present() {
    let failures = check_hygiene(&repo_root());
    assert!(failures.is_empty(), "hygiene:\n{}", report(&failures));
}

#[test]
fn todo_markers_reported_not_failed() {
    for notice in todo_notices(&repo_root()) {
        println!("NOTICE {notice}");
    }
}

#[test]
fn complete_fixture_passes_every_validator() {
    // Positive control: proves the adversarial failures below come from the
    // removed piece, not from an incomplete fixture.
    let temp = complete_fixture("complete");
    assert_failures(&temp, &[]);
}

// ---------------------------------------------------------------------------
// Adversarial (temp trees)
// ---------------------------------------------------------------------------

#[test]
fn each_missing_required_file_reported_distinctly() {
    let mut required: Vec<String> = ANDROID_FILES
        .iter()
        .map(|file| (*file).to_owned())
        .collect();
    required.extend(
        REQUIRED_DOCS
            .iter()
            .chain(HYGIENE_FILES)
            .map(|file| (*file).to_owned()),
    );
    let locale = format!("{FASTLANE_LOCALE_DIR}/{FASTLANE_REQUIRED_LOCALE}");
    required.extend(
        FASTLANE_LOCALE_FILES
            .iter()
            .map(|file| format!("{locale}/{file}")),
    );
    required.push(format!("{locale}/images/icon.png"));
    required.push("android/app/src/main/res/mipmap-xxhdpi/ic_launcher_round.png".to_owned());
    for (index, relative) in required.iter().enumerate() {
        let temp = complete_fixture(&format!("missing-{index}"));
        temp.remove(relative);
        assert_failures(&temp, &[Failure::MissingFile(PathBuf::from(relative))]);
    }
}

#[test]
fn each_missing_dir_reported_distinctly() {
    for dir in ["music", "sfx", "sprites", "fonts"] {
        let temp = complete_fixture(&format!("nodir-{dir}"));
        temp.remove(&format!("game/assets/{dir}"));
        let expected = Failure::MissingDir(Path::new("game/assets").join(dir));
        assert_failures(&temp, &[expected]);
    }
    let temp = complete_fixture("nodir-levels");
    temp.remove("game/assets/levels");
    assert_failures(
        &temp,
        &[Failure::MissingDir(PathBuf::from("game/assets/levels"))],
    );
    let temp = complete_fixture("nodir-fastlane");
    temp.remove("fastlane");
    assert_failures(
        &temp,
        &[Failure::MissingDir(PathBuf::from(FASTLANE_LOCALE_DIR))],
    );
    let temp = complete_fixture("nodir-screens");
    let screenshots =
        format!("{FASTLANE_LOCALE_DIR}/{FASTLANE_REQUIRED_LOCALE}/images/phoneScreenshots");
    temp.remove(&screenshots);
    assert_failures(&temp, &[Failure::MissingDir(PathBuf::from(screenshots))]);
}

#[test]
fn whole_android_dir_missing_reports_every_input() {
    let temp = complete_fixture("noandroid");
    temp.remove("android");
    let failures = validate_repo(&temp.0);
    let expected_count = ANDROID_FILES.len() + ANDROID_ICON_DENSITIES.len() * 2;
    assert_eq!(
        failures.len(),
        expected_count,
        "got:\n{}",
        report(&failures)
    );
    assert!(failures
        .iter()
        .all(|failure| matches!(failure, Failure::MissingFile(_))));
}

#[test]
fn empty_and_invalid_content_reported() {
    let temp = complete_fixture("content");
    temp.write("README.md", "");
    temp.write("game/assets/levels/broken.json", "{\"nodes\": [1, 2,");
    temp.write("game/assets/dialogue/sample.json", "not json at all");
    let failures = validate_repo(&temp.0);
    assert_eq!(failures.len(), 3, "got:\n{}", report(&failures));
    assert!(failures.contains(&Failure::EmptyFile(PathBuf::from("README.md"))));
    let invalid: Vec<&PathBuf> = failures
        .iter()
        .filter_map(|failure| match failure {
            Failure::InvalidJson { path, .. } => Some(path),
            _ => None,
        })
        .collect();
    assert_eq!(
        invalid,
        [
            Path::new("game/assets/levels/broken.json"),
            Path::new("game/assets/dialogue/sample.json")
        ]
    );
}

#[test]
fn json_dir_without_json_reported() {
    let temp = complete_fixture("nojson");
    temp.remove("game/assets/dialogue/sample.json");
    let dir = PathBuf::from("game/assets/dialogue");
    assert_failures(
        &temp,
        &[Failure::NothingMatching {
            dir,
            what: "*.json",
        }],
    );
}

#[test]
fn bad_changelog_and_missing_screenshots_reported() {
    let temp = complete_fixture("changelog");
    let locale = format!("{FASTLANE_LOCALE_DIR}/{FASTLANE_REQUIRED_LOCALE}");
    temp.write(
        &format!("{locale}/changelogs/v1.0.txt"),
        "named by semver, not versionCode",
    );
    temp.remove(&format!("{locale}/images/phoneScreenshots/01.png"));
    let screenshots = PathBuf::from(format!("{locale}/images/phoneScreenshots"));
    assert_failures(
        &temp,
        &[
            Failure::BadChangelogName(PathBuf::from(format!("{locale}/changelogs/v1.0.txt"))),
            Failure::NothingMatching {
                dir: screenshots,
                what: "*.png",
            },
        ],
    );
}

#[test]
fn workspace_drift_reported_per_key() {
    let temp = complete_fixture("workspace");
    let drifted = FIXTURE_ROOT_TOML
        .replace("    \"tests\",\n", "    \"rogue\",\n")
        .replace("version = \"0.1.0\"", "version = \"v0.1\"")
        .replace("edition = \"2021\"", "edition = \"2027\"")
        .replace("license = \"Apache-2.0\"\n", "");
    temp.write("Cargo.toml", &drifted);
    temp.write(
        "game/Cargo.toml",
        "[package]\nname = \"game\"\nversion = \"9.9.9\"\n",
    );
    let found = vec![
        "crates/osp_sim".to_owned(),
        "game".to_owned(),
        "rogue".to_owned(),
    ];
    let manifest = PathBuf::from("game/Cargo.toml");
    assert_failures(
        &temp,
        &[
            Failure::WorkspaceMembers { found },
            Failure::WorkspacePackageKey {
                key: "version",
                problem: "not MAJOR.MINOR.PATCH: \"v0.1\"".to_owned(),
            },
            Failure::WorkspacePackageKey {
                key: "edition",
                problem: "unknown edition: \"2027\"".to_owned(),
            },
            Failure::WorkspacePackageKey {
                key: "license",
                problem: "missing".to_owned(),
            },
            Failure::NotInherited {
                manifest: manifest.clone(),
                key: "version",
            },
            Failure::NotInherited {
                manifest: manifest.clone(),
                key: "edition",
            },
            Failure::NotInherited {
                manifest,
                key: "license",
            },
        ],
    );
}

#[test]
fn unterminated_members_array_reported_not_panicked() {
    let temp = complete_fixture("unterminated");
    temp.write("Cargo.toml", "[workspace]\nmembers = [\n  \"game\",\n");
    let failures = check_workspace(&temp.0);
    assert!(
        failures.contains(&Failure::WorkspaceMembers { found: Vec::new() }),
        "{failures:?}"
    );
}

#[test]
fn directory_in_place_of_file_reported_unreadable() {
    let temp = complete_fixture("dirfile");
    temp.remove("LICENSE");
    fs::create_dir(temp.0.join("LICENSE")).expect("fixture dir must be creatable");
    let failures = validate_repo(&temp.0);
    assert!(
        matches!(failures.as_slice(), [Failure::Unreadable { path, .. }] if path == Path::new("LICENSE")),
        "got:\n{}",
        report(&failures)
    );
}

#[cfg(unix)]
#[test]
fn permission_denied_file_reported_unreadable() {
    use std::os::unix::fs::PermissionsExt;

    let temp = complete_fixture("perm");
    let path = temp.0.join("docs/FDROID.md");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("chmod fixture");
    if fs::File::open(&path).is_ok() {
        // Running as root (or with CAP_DAC_OVERRIDE): permissions are not
        // enforced, so this case cannot be exercised. Recorded, not faked.
        println!("SKIPPED permission_denied_file_reported_unreadable: DAC override active");
        return;
    }
    let failures = validate_repo(&temp.0);
    assert!(
        matches!(failures.as_slice(), [Failure::Unreadable { path, .. }] if path == Path::new("docs/FDROID.md")),
        "got:\n{}",
        report(&failures)
    );
}

#[test]
fn non_utf8_and_oversized_text_reported_unreadable() {
    let temp = complete_fixture("bytes");
    fs::write(temp.0.join("docs/BUILD.md"), [0xFF, 0xFE, 0x00, 0xC3]).expect("write fixture");
    fs::File::options()
        .write(true)
        .open(temp.0.join("docs/GAME_DESIGN.md"))
        .and_then(|file| file.set_len(TEXT_BYTES_MAX + 1))
        .expect("sparse fixture must be extendable");
    let failures = validate_repo(&temp.0);
    let paths: Vec<&Path> = failures
        .iter()
        .filter_map(|failure| match failure {
            Failure::Unreadable { path, .. } => Some(path.as_path()),
            _ => None,
        })
        .collect();
    assert_eq!(failures.len(), 2, "got:\n{}", report(&failures));
    assert_eq!(
        paths,
        [Path::new("docs/BUILD.md"), Path::new("docs/GAME_DESIGN.md")]
    );
}

#[test]
fn empty_root_reports_failures_without_panicking() {
    let temp = TempDir::new("empty-root");
    let failures = validate_repo(&temp.0);
    assert!(failures.contains(&Failure::MissingFile(PathBuf::from("Cargo.toml"))));
    assert!(failures.contains(&Failure::MissingDir(PathBuf::from(FASTLANE_LOCALE_DIR))));
    assert!(failures.contains(&Failure::MissingFile(PathBuf::from("LICENSE"))));
    let missing_root = temp.0.join("does-not-exist");
    assert_eq!(validate_repo(&missing_root).len(), failures.len());
}
