# Light-Show Testing

**Last updated:** 2026-10-06
**Status:** 458 tests passing, 0 failing

## Test Suite Overview

| Suite | Tests | Coverage |
|-------|-------|----------|
| `light-show` lib (unit + playthrough) | 173 | Game logic, state machines, playthrough |
| `physics_link_budget` | 18 | Optical power calculations |
| `physics_adversarial` | 12 | Adversarial physics inputs |
| `game_levels` | 25 | Level registry, validation |
| `game_audio` | 22 | Audio file validation |
| `game_quiz` | 6 | Quiz mechanics |
| `game_cheats_dialogue` | 25 | Cheat codes, dialogue |
| `assets_images` | 19 | Image asset validation |
| `structural` | 19 | Repo structure |
| `edge_cases` | 20 | Edge cases, stress tests |
| **Total** | **458** | |

## Edge Case Coverage (new)

The `edge_cases` test suite (20 tests) covers:

### Data Validation
- Minimal level JSON parses correctly
- Missing required `medium` field fails loudly (no silent defaults)
- Malformed JSON returns error, doesn't panic
- Empty nodes arrays handled

### Registry Integrity
- All 80 levels have unique IDs
- All levels have non-empty titles and briefings
- All levels have valid world indices (1-8)
- Power windows are sane (min < max, reasonable width)

### Boundary Conditions
- Index 0 and 79 valid, index 80 panics (not UB)
- All 8 tracks × 10 levels contiguous

### Stress Tests
- Parse all 80 levels 10× repeatedly
- ID uniqueness verified 5×

### Unicode & Special Characters
- Unicode titles (Séraphine, 日本語, emoji) parse
- 50KB briefings handled
- Common slug characters in IDs

### Numeric Edge Cases
- Extreme dBm values (-100 to +50) parse
- Zero world index handled

## Recent Fixes (2026-10-06)

### Playthrough Harness B0001 (fixed)
**Problem:** 14 playthrough tests failed with Bevy B0001 ECS scheduling conflicts after the 80-level plugin expansion. The Quiz, TriageConsole, and ApiConsole plugins added systems with conflicting `&mut Text` and `Interaction` queries.

**Fix:**
- Base `playthrough_app()` no longer adds UI plugins; initializes their resources manually
- New `playthrough_app_with_api()` variant for Clara tests needing API buttons
- Chained quiz systems to run sequentially
- Made button marker types public with disjoint `Without` filters

**Result:** 15/15 playthrough tests pass.

### World Index Range (found by edge case tests)
**Problem:** Test assumed worlds 0-7, but Léa's quiz levels use world 8.

**Fix:** Updated test to allow worlds 1-8. This was a test bug, not a game bug — the game correctly uses world 8 for Léa.

### JPEG Portrait Loading (fixed 2026-10-05)
**Problem:** Game failed to load companion portraits (`.jpg` files) — bevy `jpeg` feature was not enabled.

**Fix:** Added `"jpeg"` to bevy features in `game/Cargo.toml` (both desktop and Android blocks).

## Running Tests

```bash
# Full workspace
cargo test --workspace

# Specific suite
cargo test --test edge_cases
cargo test -p light-show --lib playthrough::

# With output
cargo test -- --nocapture
```

## Test Conventions

- **Unit tests:** In `game/src/` alongside code, `#[cfg(test)]`
- **Integration tests:** In `tests/`, flat layout per Matt's preference
- **Playthrough tests:** In `game/src/playthrough.rs`, use headless Bevy app
- **Physics tests:** Adversarial + validation split (50/50 per Matt's rule)

## CI Integration

Tests run via `cargo test --workspace`. The SCIP index (`index.scip`) is generated separately via `rust-analyzer scip .`.
