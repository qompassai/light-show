# Inbox — integration tests: levels, cheats/dialogue, audio (2026-10-05)

Agent: claude-opus-5-5 (test-building worker). Uncommitted working tree.

## Durable discoveries (evidence-backed)

1. **Konami matcher is naive (bug).** `KonamiState::feed` resets to 0/1 on a
   mismatch with no KMP fallback, so `Up Up Up Down Down ... Enter` (one
   extra leading Up) never completes. Evidence:
   `cargo test -p light-show-tests --test game_cheats_dialogue -- --ignored`
   → `konami_tolerates_an_extra_leading_up` FAILS (kept `#[ignore]`).
   Path: `game/src/cheat_codes.rs`.
2. **`KonamiState.progress` is a pub field**; any value >= 11 makes `feed`
   index out of bounds (panic). Not tested (would require setting the
   field directly); hardening candidate.
3. **Code-word buffer saturates.** `CodeWordBuffer` caps at 20 bytes and
   `detect_code_words` only clears on match/Enter/Backspace, so any stray
   prefix letter blocks every code until cleared. Pinned by
   `stray_letters_before_a_code_block_it_until_cleared`.
4. **Specialist track indices dangle.** `Companion::{Clara,Aino,Hikari,Lea}
   .track_start_index()` = 8/10/12/14, all >= `LEVEL_SOURCES.len()` (8).
   `load_level` clamps to the last level, so no panic, but selecting one
   would silently start `e1l2`. Only base four are selectable today.
5. **`DialogueBank`'s serde shape ≠ the JSON mirrors.** The struct expects
   `{"lines": {...}}`; `assets/dialogue/*_en.json` are flat key→lines maps.
   The game never reads the mirrors (compiled-in banks), so harmless now;
   a future runtime JSON loader must wrap or change the shape. Mirrors are
   currently identical to the compiled banks (test
   `json_mirrors_match_the_compiled_in_banks_exactly`).
6. Level file `world4_level1_outage.json` has `"world": 1` (name says 4).
   Not wrong per se (it is the fiber track's 2nd level) but surprising.
7. `cargo clippy -p light-show-tests --all-targets -- -D warnings` fails on
   pre-existing game lint debt (`states/outage.rs:310`,
   `states/playing.rs:277` too_many_arguments; `states/results.rs:135`
   type_complexity). Use `--no-deps` to gate the tests crate alone.

## Visibility changes made for integration tests (no behavior change)

- `game/src/lib.rs`: `mod audio` → `pub mod audio`; `pub(crate) mod
  cheat_codes` → `pub mod cheat_codes`; `mod level` → `pub mod level`;
  `mod waifu` → `pub mod waifu`.
- `game/src/cheat_codes.rs`: added `pub use bevy::prelude::KeyCode;` so the
  tests crate (no direct bevy dep) can drive `KonamiState::feed`.

## Validation commands

```
cargo test -p light-show
cargo test -p light-show-tests --test game_levels --test game_cheats_dialogue --test game_audio
cargo clippy -p light-show-tests --all-targets --no-deps -- -D warnings
cargo fmt --check -p light-show-tests
```
