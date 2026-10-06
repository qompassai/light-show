//! Cheat code unlock system for specialist companions.
//!
//! Two unlock paths at the companion select screen:
//! 1. Code words (typed): JUSTINBAILEY → Clara, ABACABB → Aino,
//!    BLASTPROCESSING → Hikari, TRIFORCE → Léa
//! 2. Konami Code (buttons): UP UP DOWN DOWN LEFT RIGHT LEFT RIGHT B A START
//!    → unlocks all four at once
//!
//! Codes are easter eggs from the SNES/Sega/Game Boy Color era.

use bevy::prelude::*;
use crate::waifu::Companion;
use std::collections::HashSet;

/// Re-exported so integration tests can drive `KonamiState::feed` without
/// taking a direct `bevy` dependency.
pub use bevy::prelude::KeyCode;

/// Which specialist companions are unlocked. Base four (Fiber/Coax/Mobile/
/// Ethernet) are always available; these four need codes or level completion.
#[derive(Resource, Default)]
pub struct UnlockedSpecialists {
    pub unlocked: HashSet<Companion>,
}

impl UnlockedSpecialists {
    /// Public API for the companion-select UI to grey out locked specialists.
    /// Currently exercised by unit tests; UI wiring is pending.
    #[allow(dead_code)]
    pub fn is_unlocked(&self, c: &Companion) -> bool {
        // Base four are always unlocked
        matches!(c, Companion::Fiber | Companion::Coax | Companion::Mobile | Companion::Ethernet)
            || self.unlocked.contains(c)
    }

    pub fn unlock(&mut self, c: Companion) -> bool {
        // Returns true if this was a new unlock
        self.unlocked.insert(c)
    }

    pub fn unlock_all(&mut self) {
        for c in [Companion::Clara, Companion::Aino, Companion::Hikari, Companion::Lea] {
            self.unlocked.insert(c);
        }
    }
}

/// Code words → companion. Case-insensitive.
pub fn code_word_to_companion(code: &str) -> Option<Companion> {
    match code.to_uppercase().as_str() {
        "JUSTINBAILEY" => Some(Companion::Clara),   // Metroid (NES) - full power-up
        "ABACABB" => Some(Companion::Aino),          // Mortal Kombat (Genesis) - blood code
        "BLASTPROCESSING" => Some(Companion::Hikari), // Sega Genesis marketing - speed/power
        "TRIFORCE" => Some(Companion::Lea),          // Zelda - wisdom, for the study coach
        _ => None,
    }
}

/// Konami Code sequence: UP UP DOWN DOWN LEFT RIGHT LEFT RIGHT B A START
#[derive(Resource, Default)]
pub struct KonamiState {
    /// Keys of `SEQUENCE` matched so far. Private so nothing outside `feed`
    /// can break the `progress < SEQUENCE.len()` invariant that indexing
    /// relies on.
    progress: usize,
}

// The failure table must cover the sequence one-to-one.
const _: () = assert!(KonamiState::FAILURE.len() == KonamiState::SEQUENCE.len());

impl KonamiState {
    /// KMP prefix function of `SEQUENCE`: `FAILURE[i]` is the length of the
    /// longest proper prefix of `SEQUENCE[..=i]` that is also its suffix.
    /// Only the leading `Up, Up` overlaps itself, so every other entry is 0.
    /// Verified against an independent recomputation in the unit tests.
    const FAILURE: [usize; 11] = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    const SEQUENCE: &'static [KeyCode] = &[
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
        // START = Enter
        KeyCode::Enter,
    ];

    /// True when this keypress advances the Konami sequence and therefore
    /// belongs to it (used to keep Konami keys out of the code-word buffer).
    pub fn consumes(&self, key: KeyCode) -> bool {
        self.progress < Self::SEQUENCE.len() && key == Self::SEQUENCE[self.progress]
    }

    /// Feed a keypress. Returns true when the full sequence completes, after
    /// which the matcher re-arms at progress 0.
    ///
    /// On a mismatch it falls back through `FAILURE` (KMP) instead of
    /// restarting, so an extra leading Up still completes. Bounded: each
    /// fallback strictly shrinks the match, so at most `SEQUENCE.len()`
    /// iterations per key.
    pub fn feed(&mut self, key: KeyCode) -> bool {
        debug_assert!(self.progress < Self::SEQUENCE.len());
        let mut matched = self.progress;
        while matched > 0 && key != Self::SEQUENCE[matched] {
            matched = Self::FAILURE[matched - 1];
        }
        if key == Self::SEQUENCE[matched] {
            matched += 1;
        }
        if matched == Self::SEQUENCE.len() {
            self.progress = 0;
            return true;
        }
        self.progress = matched;
        false
    }

    /// Keys of the sequence matched so far.
    /// Invariant: `0 <= progress < SEQUENCE.len()` (11).
    pub fn progress(&self) -> usize {
        self.progress
    }
}

/// Buffer for typed code words at the select screen.
#[derive(Resource, Default)]
pub struct CodeWordBuffer {
    pub buffer: String,
}

impl CodeWordBuffer {
    /// Max code word length (BLASTPROCESSING = 15)
    const MAX_LEN: usize = 20;

    pub fn push(&mut self, c: char) {
        if self.buffer.len() < Self::MAX_LEN {
            self.buffer.push(c.to_ascii_uppercase());
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_words_map_correctly() {
        assert_eq!(code_word_to_companion("justinbailey"), Some(Companion::Clara));
        assert_eq!(code_word_to_companion("ABACABB"), Some(Companion::Aino));
        assert_eq!(code_word_to_companion("blastprocessing"), Some(Companion::Hikari));
        assert_eq!(code_word_to_companion("triforce"), Some(Companion::Lea));
        assert_eq!(code_word_to_companion("nonsense"), None);
    }

    #[test]
    fn konami_completes() {
        let mut k = KonamiState::default();
        let seq = [
            KeyCode::ArrowUp, KeyCode::ArrowUp,
            KeyCode::ArrowDown, KeyCode::ArrowDown,
            KeyCode::ArrowLeft, KeyCode::ArrowRight,
            KeyCode::ArrowLeft, KeyCode::ArrowRight,
            KeyCode::KeyB, KeyCode::KeyA,
            KeyCode::Enter,
        ];
        for (i, key) in seq.iter().enumerate() {
            let done = k.feed(*key);
            assert_eq!(done, i == seq.len() - 1);
        }
    }

    #[test]
    fn konami_resets_on_wrong_key() {
        let mut k = KonamiState::default();
        k.feed(KeyCode::ArrowUp);
        k.feed(KeyCode::ArrowUp);
        k.feed(KeyCode::KeyX); // wrong
        assert_eq!(k.progress(), 0);
    }

    #[test]
    fn konami_failure_table_matches_a_naive_recomputation() {
        // Definition, not KMP: for each i, the longest k < i + 1 with
        // seq[..k] == seq[i + 1 - k..=i], compared key by key.
        let seq = KonamiState::SEQUENCE;
        for i in 0..seq.len() {
            let mut longest = 0;
            for k in 1..=i {
                if (0..k).all(|j| seq[j] == seq[i + 1 - k + j]) {
                    longest = k;
                }
            }
            assert_eq!(KonamiState::FAILURE[i], longest, "FAILURE[{i}]");
        }
    }

    #[test]
    fn unlock_idempotent() {
        let mut u = UnlockedSpecialists::default();
        assert!(u.unlock(Companion::Clara));
        assert!(!u.unlock(Companion::Clara)); // already unlocked
        assert!(u.is_unlocked(&Companion::Clara));
        assert!(u.is_unlocked(&Companion::Fiber)); // base always unlocked
        assert!(!u.is_unlocked(&Companion::Aino));
    }
}
