//! Integration coverage for the companion-select cheat codes
//! (`light_show::cheat_codes`) and the companion dialogue banks
//! (`game/assets/dialogue/<companion>_en.json`, mirrored from the
//! compiled-in `waifu::dialogue::DialogueBank`).
//!
//! The game itself never reads the JSON mirrors at runtime (banks are
//! compiled in), so this file supplies a bounded, typed-error bank loader
//! and proves (a) every shipped mirror is well-formed, speaker-resolved,
//! and byte-for-byte equivalent to the compiled bank, and (b) malformed
//! banks are reported as `BankError`s, never panics.
//!
//! The typed-code path is modelled on `companion_select::detect_code_words`:
//! push each letter into `CodeWordBuffer`, then look the whole buffer up.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

use light_show::cheat_codes::{
    code_word_to_companion, CodeWordBuffer, KeyCode, KonamiState, UnlockedSpecialists,
};
use light_show::level::{load_level, LEVEL_SOURCES};
use light_show::waifu::dialogue::DialogueBank;
use light_show::waifu::Companion;

/// Every companion, base and specialist.
const ALL_COMPANIONS: [Companion; 8] = [
    Companion::Fiber,
    Companion::Coax,
    Companion::Mobile,
    Companion::Ethernet,
    Companion::Clara,
    Companion::Aino,
    Companion::Hikari,
    Companion::Lea,
];
const SPECIALISTS: [Companion; 4] = [
    Companion::Clara,
    Companion::Aino,
    Companion::Hikari,
    Companion::Lea,
];

/// Code words documented in `cheat_codes` module docs and the `Companion`
/// variant docs.
const DOCUMENTED_CODES: [(&str, Companion); 4] = [
    ("JUSTINBAILEY", Companion::Clara),
    ("ABACABB", Companion::Aino),
    ("BLASTPROCESSING", Companion::Hikari),
    ("TRIFORCE", Companion::Lea),
];

/// Documented Konami sequence; START is Enter.
const KONAMI: [KeyCode; 11] = [
    KeyCode::ArrowUp,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::KeyB,
    KeyCode::KeyA,
    KeyCode::Enter,
];

/// "TRIFORCE" as the keypresses that type it.
const TRIFORCE_KEYS: [(KeyCode, char); 8] = [
    (KeyCode::KeyT, 'T'),
    (KeyCode::KeyR, 'R'),
    (KeyCode::KeyI, 'I'),
    (KeyCode::KeyF, 'F'),
    (KeyCode::KeyO, 'O'),
    (KeyCode::KeyR, 'R'),
    (KeyCode::KeyC, 'C'),
    (KeyCode::KeyE, 'E'),
];

/// `CodeWordBuffer::MAX_LEN` (private). Pinned so a change is noticed.
const CODE_BUFFER_BYTES_MAX: usize = 20;

/// Event keys every base companion's bank must define (see the
/// `waifu::dialogue` module docs).
const BASE_EVENT_KEYS: [&str; 8] = [
    "clean_splice",
    "messy_splice",
    "level_win",
    "level_fail_hot",
    "level_fail_cold",
    "outage_start",
    "outage_resolved",
    "hint_request",
];

const BANK_FILES_MAX: usize = 64;
/// Shipped banks are ~2 KiB.
const BANK_FILE_BYTES_MAX: u64 = 64 * 1024;
/// Bank file names are `<speaker>_<lang>.json`; only English ships.
const BANK_FILE_SUFFIX: &str = "_en.json";

/// Every way loading a dialogue bank can fail.
#[derive(Debug)]
enum BankError {
    DirMissing {
        dir: PathBuf,
        source: io::Error,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
    TooManyFiles,
    TooLarge {
        path: PathBuf,
    },
    BadFileName {
        path: PathBuf,
    },
    UnknownCompanion {
        speaker: String,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    MissingKey {
        companion: Companion,
        key: &'static str,
    },
    EmptyKey {
        companion: Companion,
    },
    NoLines {
        companion: Companion,
        key: String,
    },
    EmptyLine {
        companion: Companion,
        key: String,
    },
}

impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirMissing { dir, source } => write!(f, "{}: {source}", dir.display()),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::TooManyFiles => write!(f, "more than {BANK_FILES_MAX} bank files"),
            Self::TooLarge { path } => {
                write!(
                    f,
                    "{}: larger than {BANK_FILE_BYTES_MAX} bytes",
                    path.display()
                )
            }
            Self::BadFileName { path } => {
                write!(f, "{}: not <speaker>{BANK_FILE_SUFFIX}", path.display())
            }
            Self::UnknownCompanion { speaker } => write!(f, "unknown speaker {speaker:?}"),
            Self::Parse { path, source } => write!(f, "{}: {source}", path.display()),
            Self::MissingKey { companion, key } => write!(f, "{companion:?}: missing {key}"),
            Self::EmptyKey { companion } => write!(f, "{companion:?}: empty event key"),
            Self::NoLines { companion, key } => write!(f, "{companion:?}: {key} has no lines"),
            Self::EmptyLine { companion, key } => write!(f, "{companion:?}: blank line in {key}"),
        }
    }
}

/// One loaded bank with its resolved speaker.
struct LoadedBank {
    companion: Companion,
    bank: DialogueBank,
}

// ---------------------------------------------------------------------------
// Loader.
// ---------------------------------------------------------------------------

/// ASCII-folded, lowercased display name: "Séraphine" -> "seraphine".
fn speaker_slug(companion: Companion) -> String {
    companion
        .display_name()
        .chars()
        .map(|c| if c == 'é' { 'e' } else { c })
        .flat_map(char::to_lowercase)
        .collect()
}

/// Resolves a bank file name to its speaking companion.
fn resolve_speaker(path: &Path) -> Result<Companion, BankError> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let speaker = name
        .strip_suffix(BANK_FILE_SUFFIX)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BankError::BadFileName {
            path: path.to_path_buf(),
        })?;
    ALL_COMPANIONS
        .into_iter()
        .find(|c| speaker_slug(*c) == speaker)
        .ok_or_else(|| BankError::UnknownCompanion {
            speaker: speaker.to_string(),
        })
}

fn read_bounded(path: &Path) -> Result<String, BankError> {
    let io_err = |source| BankError::Io {
        path: path.to_path_buf(),
        source,
    };
    let file = fs::File::open(path).map_err(io_err)?;
    let mut text = String::new();
    file.take(BANK_FILE_BYTES_MAX + 1)
        .read_to_string(&mut text)
        .map_err(io_err)?;
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > BANK_FILE_BYTES_MAX {
        return Err(BankError::TooLarge {
            path: path.to_path_buf(),
        });
    }
    Ok(text)
}

/// Validates one bank's content: base companions carry every event key;
/// every key is non-empty and has at least one non-blank line.
fn validate_bank(companion: Companion, bank: &DialogueBank) -> Result<(), BankError> {
    if Companion::ALL.contains(&companion) {
        for key in BASE_EVENT_KEYS {
            if !bank.lines.contains_key(key) {
                return Err(BankError::MissingKey { companion, key });
            }
        }
    }
    for (key, lines) in &bank.lines {
        if key.trim().is_empty() {
            return Err(BankError::EmptyKey { companion });
        }
        if lines.is_empty() {
            return Err(BankError::NoLines {
                companion,
                key: key.clone(),
            });
        }
        if lines.iter().any(|line| line.trim().is_empty()) {
            return Err(BankError::EmptyLine {
                companion,
                key: key.clone(),
            });
        }
    }
    Ok(())
}

fn load_bank_file(path: &Path) -> Result<LoadedBank, BankError> {
    let companion = resolve_speaker(path)?;
    let text = read_bounded(path)?;
    let lines = serde_json::from_str(&text).map_err(|source| BankError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let bank = DialogueBank { lines };
    validate_bank(companion, &bank)?;
    Ok(LoadedBank { companion, bank })
}

/// Loads every `*.json` bank in `dir`, sorted by file name. The first bad
/// file fails the whole load.
fn load_bank_dir(dir: &Path) -> Result<Vec<LoadedBank>, BankError> {
    let entries = fs::read_dir(dir).map_err(|source| BankError::DirMissing {
        dir: dir.to_path_buf(),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| BankError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if paths.len() == BANK_FILES_MAX {
                return Err(BankError::TooManyFiles);
            }
            paths.push(path);
        }
    }
    paths.sort();
    // File names are unique and map 1:1 to speakers, so no duplicate check.
    paths.iter().map(|path| load_bank_file(path)).collect()
}

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

fn shipped_dialogue_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../game/assets/dialogue")
}

fn load_shipped_banks() -> Vec<LoadedBank> {
    load_bank_dir(&shipped_dialogue_dir()).unwrap_or_else(|e| panic!("{e}"))
}

/// Models `detect_code_words`: letters accumulate, a match unlocks and
/// clears. Returns the unlocked set after typing `typed`.
fn type_code(typed: &str) -> UnlockedSpecialists {
    let mut buffer = CodeWordBuffer::default();
    let mut unlocked = UnlockedSpecialists::default();
    for letter in typed.chars() {
        buffer.push(letter);
        if let Some(companion) = code_word_to_companion(&buffer.buffer) {
            unlocked.unlock(companion);
            buffer.clear();
        }
    }
    unlocked
}

/// Feeds every key (no short-circuit) and reports whether any completed.
fn feed_all(state: &mut KonamiState, keys: &[KeyCode]) -> bool {
    let mut completed = false;
    for key in keys {
        completed |= state.feed(*key);
    }
    completed
}

/// Per-test scratch directory, removed (best-effort) on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(test_name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("light-show-dialogue-{}-{test_name}", process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear stale scratch dir");
        }
        fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).expect("write scratch bank");
        path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _cleanup_is_best_effort = fs::remove_dir_all(&self.0);
    }
}

/// A complete, valid base-companion bank as JSON, with one key overridden.
fn bank_json_with(key: &str, value: serde_json::Value) -> String {
    let mut object = serde_json::Map::new();
    for event in BASE_EVENT_KEYS {
        object.insert(event.to_string(), serde_json::json!(["A line."]));
    }
    object.insert(key.to_string(), value);
    serde_json::Value::Object(object).to_string()
}

fn load_scratch_bank(test_name: &str, file_name: &str, contents: &str) -> BankError {
    let scratch = ScratchDir::new(test_name);
    let path = scratch.write(file_name, contents);
    match load_bank_file(&path) {
        Ok(_) => panic!("{file_name}: {contents:.60} must be rejected"),
        Err(error) => error,
    }
}

// ---------------------------------------------------------------------------
// Validation: cheat codes.
// ---------------------------------------------------------------------------

#[test]
fn every_documented_code_word_unlocks_exactly_its_companion() {
    for (code, companion) in DOCUMENTED_CODES {
        for spelling in [code.to_string(), code.to_lowercase()] {
            assert_eq!(
                code_word_to_companion(&spelling),
                Some(companion),
                "{spelling}"
            );
        }
        let unlocked = type_code(code);
        for specialist in SPECIALISTS {
            assert_eq!(
                unlocked.is_unlocked(&specialist),
                specialist == companion,
                "typing {code} has the wrong effect on {specialist:?}"
            );
        }
    }
}

#[test]
fn every_code_word_fits_the_typing_buffer() {
    for (code, _) in DOCUMENTED_CODES {
        assert!(
            code.len() <= CODE_BUFFER_BYTES_MAX,
            "{code} cannot be typed"
        );
    }
}

#[test]
fn base_companions_are_always_unlocked_and_specialists_start_locked() {
    let unlocked = UnlockedSpecialists::default();
    for companion in Companion::ALL {
        assert!(
            unlocked.is_unlocked(&companion),
            "{companion:?} must be unlocked"
        );
    }
    for specialist in SPECIALISTS {
        assert!(
            !unlocked.is_unlocked(&specialist),
            "{specialist:?} must start locked"
        );
    }
}

#[test]
fn konami_code_unlocks_every_specialist_and_rearms() {
    let mut state = KonamiState::default();
    let mut unlocked = UnlockedSpecialists::default();
    for round in 0..2 {
        for (step, key) in KONAMI.iter().enumerate() {
            let completed = state.feed(*key);
            assert_eq!(
                completed,
                step == KONAMI.len() - 1,
                "round {round} step {step}"
            );
            if completed {
                unlocked.unlock_all();
            }
        }
        assert_eq!(state.progress(), 0, "sequence must re-arm after completion");
    }
    for specialist in SPECIALISTS {
        assert!(
            unlocked.is_unlocked(&specialist),
            "Konami left {specialist:?} locked"
        );
    }
}

#[test]
fn unlocking_twice_reports_no_new_unlock() {
    let mut unlocked = UnlockedSpecialists::default();
    unlocked.unlock_all();
    for specialist in SPECIALISTS {
        assert!(
            !unlocked.unlock(specialist),
            "{specialist:?} re-unlock must be a no-op"
        );
    }
    assert_eq!(unlocked.unlocked.len(), SPECIALISTS.len());
}

// ---------------------------------------------------------------------------
// Validation: dialogue banks.
// ---------------------------------------------------------------------------

#[test]
fn every_shipped_bank_loads_with_a_resolved_speaker_and_nonblank_lines() {
    let banks = load_shipped_banks();
    let speakers: HashSet<Companion> = banks.iter().map(|b| b.companion).collect();
    for companion in Companion::ALL {
        assert!(
            speakers.contains(&companion),
            "no dialogue file for {companion:?}"
        );
    }
}

#[test]
fn json_mirrors_match_the_compiled_in_banks_exactly() {
    for loaded in load_shipped_banks() {
        let compiled = DialogueBank::load_default(loaded.companion);
        let mirror: BTreeMap<_, _> = loaded.bank.lines.iter().collect();
        let source: BTreeMap<_, _> = compiled.lines.iter().collect();
        assert_eq!(mirror, source, "{:?} JSON mirror drifted", loaded.companion);
    }
}

#[test]
fn every_level_dialogue_hook_resolves_in_every_base_bank() {
    for index in 0..LEVEL_SOURCES.len() {
        let level = load_level(index);
        let hooks = [
            &level.on_enter_line,
            &level.on_win_line,
            &level.on_fail_line,
        ];
        for key in hooks.into_iter().flatten() {
            for companion in Companion::ALL {
                let bank = DialogueBank::load_default(companion);
                assert!(
                    bank.random_line(key).is_some(),
                    "level {} hook {key:?} missing from {companion:?}",
                    level.id
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Adversarial: cheat codes.
// ---------------------------------------------------------------------------

#[test]
fn unknown_near_miss_and_overlong_codes_are_rejected() {
    let overlong = "TRIFORCE".repeat(128 * 1024);
    let rejected = [
        "",
        " ",
        "NONSENSE",
        "JUSTINBAILE",
        "JUSTINBAILEYX",
        " TRIFORCE",
        "TRIFORCE\n",
        "TRI FORCE",
        "TRIFORCE\0",
        "ＴＲＩＦＯＲＣＥ",
        "ABACAB",
        "UUDDLRLRBA",
        &overlong,
    ];
    for code in rejected {
        let preview: String = code.chars().take(40).collect();
        assert_eq!(
            code_word_to_companion(code),
            None,
            "{preview:?} must be rejected"
        );
    }
}

#[test]
fn typing_buffer_is_bounded_and_drops_letters_past_the_cap() {
    let mut buffer = CodeWordBuffer::default();
    for _ in 0..10_000 {
        buffer.push('x');
    }
    assert_eq!(buffer.buffer, "X".repeat(CODE_BUFFER_BYTES_MAX));
    buffer.clear();
    assert!(buffer.buffer.is_empty());
}

#[test]
fn stray_letters_before_a_code_block_it_until_cleared() {
    // Exact-match contract: the buffer is compared whole, so a stray
    // prefix means no unlock until Backspace/Enter clears it.
    for typed in [
        "XTRIFORCE",
        "TRIFORCTRIFORCE",
        "QQQQQQQQQQQQQQQQQQQQTRIFORCE",
    ] {
        assert!(
            !type_code(typed).is_unlocked(&Companion::Lea),
            "{typed} must not unlock"
        );
    }
}

#[test]
fn konami_wrong_key_resets_and_noise_never_completes() {
    let mut state = KonamiState::default();
    assert!(!feed_all(&mut state, &KONAMI[..5]));
    assert!(!state.feed(KeyCode::KeyX));
    assert_eq!(state.progress(), 0, "wrong key must reset");
    let noise = [KeyCode::KeyA, KeyCode::KeyB, KeyCode::Enter, KeyCode::Space];
    for _ in 0..1_000 {
        assert!(!feed_all(&mut state, &noise));
        assert!(state.progress() < KONAMI.len());
    }
    let mut truncated = KonamiState::default();
    assert!(!feed_all(&mut truncated, &KONAMI[..KONAMI.len() - 1]));
    assert!(
        !feed_all(&mut truncated, &KONAMI[1..]),
        "a shifted sequence must not complete"
    );
}

#[test]
fn konami_tolerates_an_extra_leading_up() {
    let mut state = KonamiState::default();
    assert!(feed_all(
        &mut state,
        &[&[KeyCode::ArrowUp][..], &KONAMI[..]].concat()
    ));
}

#[test]
fn konami_recovers_from_a_restart_mid_sequence() {
    // Any number of extra Ups, or a botched attempt that restarts with
    // Up Up, must still complete on the next clean pass.
    for extra_ups in 1..=5 {
        let mut state = KonamiState::default();
        let keys = [vec![KeyCode::ArrowUp; extra_ups], KONAMI.to_vec()].concat();
        assert!(feed_all(&mut state, &keys), "{extra_ups} extra Ups");
        assert_eq!(state.progress(), 0, "re-armed after completion");
    }
    let mut state = KonamiState::default();
    let botched = [&KONAMI[..6], &KONAMI[..]].concat();
    assert!(feed_all(&mut state, &botched));
    // The matcher never completes early or on a single dropped key.
    for dropped in 0..KONAMI.len() {
        let mut state = KonamiState::default();
        let mut keys = KONAMI.to_vec();
        keys.remove(dropped);
        assert!(!feed_all(&mut state, &keys), "dropped step {dropped}");
    }
}

/// Models the fixed `companion_select` coordination between the word and
/// Konami systems. Konami's own `unlock_all` is left out so a code word's
/// unlock stays observable.
#[derive(Default)]
struct SelectScreen {
    buffer: CodeWordBuffer,
    konami: KonamiState,
    unlocked: UnlockedSpecialists,
}

impl SelectScreen {
    /// Per keypress: the word system runs first and skips keys the Konami
    /// sequence consumes, then the Konami system feeds the same key.
    /// Returns whether Konami completed.
    fn press(&mut self, keys: &[(KeyCode, Option<char>)]) -> bool {
        let mut completed = false;
        for (key, letter) in keys {
            if !self.konami.consumes(*key) {
                if let Some(letter) = letter {
                    self.buffer.push(*letter);
                    if let Some(companion) = code_word_to_companion(&self.buffer.buffer) {
                        self.unlocked.unlock(companion);
                        self.buffer.clear();
                    }
                }
            }
            completed |= self.konami.feed(*key);
        }
        completed
    }
}

fn triforce_presses() -> Vec<(KeyCode, Option<char>)> {
    TRIFORCE_KEYS.iter().map(|(k, c)| (*k, Some(*c))).collect()
}

#[test]
fn konami_b_and_a_stay_out_of_the_code_word_buffer() {
    let mut screen = SelectScreen::default();
    let arrows: Vec<(KeyCode, Option<char>)> = KONAMI[..8].iter().map(|k| (*k, None)).collect();
    assert!(!screen.press(&arrows));
    assert!(screen.konami.consumes(KeyCode::KeyB), "B is the next step");
    assert!(!screen.konami.consumes(KeyCode::KeyA), "A is not next yet");
    assert!(!screen.press(&[(KeyCode::KeyB, Some('B'))]));
    assert!(screen.konami.consumes(KeyCode::KeyA), "A is the next step");
    assert!(!screen.press(&[(KeyCode::KeyA, Some('A'))]));
    assert_eq!(screen.buffer.buffer, "", "Konami B/A leaked");
    assert!(screen.konami.consumes(KeyCode::Enter));
    assert!(screen.press(&[(KeyCode::Enter, None)]));
    assert!(!screen.press(&triforce_presses()));
    assert!(
        screen.unlocked.is_unlocked(&Companion::Lea),
        "TRIFORCE after Konami"
    );
    assert_eq!(screen.buffer.buffer, "", "a match clears the buffer");
}

#[test]
fn letters_that_do_not_advance_konami_still_reach_the_buffer() {
    let mut screen = SelectScreen::default();
    for key in [KeyCode::KeyB, KeyCode::KeyA, KeyCode::Enter, KeyCode::KeyX] {
        assert!(!screen.konami.consumes(key), "{key:?} at progress 0");
    }
    // 'X' mid-sequence never advances Konami, so it lands in the buffer
    // and the exact-match contract for stray letters is unchanged.
    let arrows: Vec<(KeyCode, Option<char>)> = KONAMI[..3].iter().map(|k| (*k, None)).collect();
    assert!(!screen.press(&arrows));
    assert!(!screen.press(&[(KeyCode::KeyX, Some('X'))]));
    assert!(!screen.press(&triforce_presses()));
    assert!(!screen.unlocked.is_unlocked(&Companion::Lea), "XTRIFORCE");
    assert_eq!(screen.buffer.buffer, "XTRIFORCE");
}

// ---------------------------------------------------------------------------
// Adversarial: dialogue banks.
// ---------------------------------------------------------------------------

#[test]
fn bank_missing_a_required_key_is_reported() {
    let mut object: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&bank_json_with("level_win", serde_json::json!(["ok"])))
            .expect("fixture is valid JSON");
    object.remove("outage_start");
    let text = serde_json::Value::Object(object).to_string();
    let error = load_scratch_bank("missing-key", "ondine_en.json", &text);
    assert!(
        matches!(
            error,
            BankError::MissingKey {
                key: "outage_start",
                ..
            }
        ),
        "{error}"
    );
}

#[test]
fn empty_lines_and_empty_line_lists_are_reported() {
    use serde_json::json;
    let cases = [
        (bank_json_with("level_win", json!([""])), "blank"),
        (
            bank_json_with("level_win", json!(["ok", "   "])),
            "whitespace",
        ),
        (bank_json_with("level_win", json!([])), "no-lines"),
        (bank_json_with("", json!(["orphan"])), "empty-key"),
    ];
    for (text, name) in cases {
        let error = load_scratch_bank(name, "linka_en.json", &text);
        assert!(
            matches!(
                error,
                BankError::EmptyLine { .. }
                    | BankError::NoLines { .. }
                    | BankError::EmptyKey { .. }
            ),
            "{name}: {error}"
        );
    }
}

#[test]
fn unknown_speakers_and_bad_file_names_are_reported() {
    let valid = bank_json_with("level_win", serde_json::json!(["ok"]));
    let error = load_scratch_bank("unknown", "zelda_en.json", &valid);
    assert!(matches!(error, BankError::UnknownCompanion { ref speaker } if speaker == "zelda"));
    for name in [
        "seraphine.json",
        "_en.json",
        "Seraphine_en.json",
        "séraphine_en.json",
    ] {
        let error = load_scratch_bank("bad-name", name, &valid);
        assert!(
            matches!(
                error,
                BankError::BadFileName { .. } | BankError::UnknownCompanion { .. }
            ),
            "{name}: {error}"
        );
    }
}

#[test]
fn malformed_bank_json_is_a_parse_error() {
    let cases = [
        "",
        "{",
        "[]",
        "null",
        "{\"level_win\": \"not a list\"}",
        "{\"level_win\": [1, 2]}",
        "{\"level_win\": [null]}",
    ];
    for (index, text) in cases.into_iter().enumerate() {
        let error = load_scratch_bank(&format!("parse-{index}"), "lattice_en.json", text);
        assert!(
            matches!(error, BankError::Parse { .. }),
            "{text:?}: {error}"
        );
    }
}

#[test]
fn oversized_files_bad_files_and_missing_dirs_are_reported() {
    let padding = " ".repeat(usize::try_from(BANK_FILE_BYTES_MAX).expect("fits usize"));
    let big = format!(
        "{}{padding}",
        bank_json_with("level_win", serde_json::json!(["ok"]))
    );
    let error = load_scratch_bank("oversized", "ondine_en.json", &big);
    assert!(matches!(error, BankError::TooLarge { .. }), "{error}");

    // One bad file fails the whole directory; it is not silently skipped.
    let scratch = ScratchDir::new("mixed");
    scratch.write(
        "ondine_en.json",
        &bank_json_with("level_win", serde_json::json!(["ok"])),
    );
    scratch.write("seraphine_en.json", "{");
    scratch.write("notes.txt", "ignored: not JSON");
    let error = load_bank_dir(&scratch.0)
        .err()
        .expect("a malformed bank must fail the dir");
    assert!(matches!(error, BankError::Parse { .. }), "{error}");

    let missing = std::env::temp_dir().join(format!("light-show-no-dialogue-{}", process::id()));
    assert!(matches!(
        load_bank_dir(&missing),
        Err(BankError::DirMissing { .. })
    ));
}

#[test]
fn unknown_dialogue_key_returns_none_not_a_panic() {
    for companion in ALL_COMPANIONS {
        let bank = DialogueBank::load_default(companion);
        for key in [
            "",
            "LEVEL_WIN",
            "level_win ",
            "no_such_event",
            &"k".repeat(1 << 16),
        ] {
            assert_eq!(
                bank.random_line(key),
                None,
                "{companion:?} key of {} bytes",
                key.len()
            );
        }
    }
}
