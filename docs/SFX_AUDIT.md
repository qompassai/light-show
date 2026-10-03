# SFX Audit — light-show

**Date:** 2026-10-02
**Scope:** Sound effects coverage in the Bevy game (`game/src/`, `game/assets/`).
**Music is out of scope** — the 12 licensed tracks (menu/tutorial/early/mid/hard/
boss/victory/mystery) are fully wired through `MusicPlugin` and working.

## Headline finding

**There is no sound-effects system at all.** `game/src/audio.rs` contains only
`MusicPlugin` (per-state looping music tracks). Zero SFX files exist on disk,
zero SFX playback code exists, and no `game/assets/sfx/` directory exists.
Every interactive game event below is currently silent.

## Event catalog

| Event | Location | Has Sound | Sound File | Recommendation |
|---|---|---|---|---|
| Menu button click (Start) | `states/menu.rs` `handle_start_button` | No | — | **Add.** Short UI confirm blip. Highest priority — first thing the player touches. |
| Menu button click (Credits) | `states/menu.rs` `handle_credits_button` | No | — | **Add.** Same UI blip as Start. |
| Button hover tick | `states/menu.rs`, `states/companion_select.rs`, `states/results.rs` | No | — | **Add (low priority).** Subtle hover tick; make it very quiet or skip on mobile. |
| Companion card select | `states/companion_select.rs` `handle_card_select` | No | — | **Add.** Characterful "select" chime, distinct from generic button click. |
| Companion select back button | `states/companion_select.rs` | No | — | **Add.** Generic UI back blip. |
| Level start | `states/playing.rs` `setup_level` | No | — | **Add.** Short "level in" sweep when the board spawns. Music starts here too, so keep it subtle. |
| Component pill select | `board.rs` `handle_pointer_input` (`SelectPill`) | No | — | **Add.** Soft "pick up" click when choosing a component pill. |
| Cable drag start | `board.rs` `handle_pointer_input` (`StartDrag`) | No | — | **Add (low priority).** Faint grab sound; optional — the release matters more. |
| Successful splice / connect | `board.rs` `handle_pointer_input` (`ReleaseAction::Connect`) | No | — | **Add.** Satisfying "snap-in" splice sound. Core game feel — high priority. |
| Failed splice (out of window) | `board.rs` `rebuild_live_graph` → ledger shows OUT OF WINDOW | No | — | **Add.** Dull error buzz when a placed component leaves the budget out of window. Distinct from success. |
| Component swap (re-pick pill) | `board.rs` (`SelectPill` on occupied edge) | No | — | **Reuse** pill-select click; no new file needed. |
| Outage alarm fires | `states/outage.rs` `announce_outage` | No | — | **Add.** Alarm klaxon/siren sting. High priority — it's the game's boss-fight moment and currently only has music + banner. |
| Repair countdown ticks | `states/outage.rs` `tick_outage` | No | — | **Add.** Ticking clock, accelerating as time runs low. High priority for tension. |
| Countdown low-time warning | `states/outage.rs` (countdown < ~5s) | No | — | **Add.** Urgent warning beeps layered over ticks, or pitch-up the tick. |
| Service restored | `states/outage.rs` `check_outage_resolution` (win path) | No | — | **Add.** Triumphant "service restored" fanfare sting. Currently only the results-screen music plays. |
| Level win | `states/playing.rs` `check_win_condition` | No | — | **Add.** Win jingle sting on transition to Results. (The old `gen_chiptune_music.py` win/fail jingles were retired; results music covers the screen but not the moment.) |
| Level fail (outage timeout) | `states/outage.rs` `check_outage_resolution` (loss path) | No | — | **Add.** Fail sting — descending sad tones. Same reasoning as level win. |
| Result buttons (Retry / Continue / Menu / Select) | `states/results.rs` `handle_result_buttons` | No | — | **Add.** Reuse generic UI click blip. |
| Companion dialogue blips | `waifu/dialogue.rs` `random_line` | No | — | **Skip for now.** Dialogue appears as static text, not typewriter — blips would need a typewriter system first. Revisit if typewriter is added. |
| Companion mood reaction | `board.rs` `splice_reaction_for` / `waifu/mod.rs` | No | — | **Add (low priority).** Tiny per-mood chirp when the companion reacts to a splice. Nice juice, not essential. |
| Win ring animation | `states/results.rs` `animate_win_ring` | No | — | **Reuse** level-win sting; the animation is visual-only. |
| Signal pulse movement | `board.rs` `move_signal_pulses` | No | — | **Skip.** Continuous ambient hum would get annoying; the pulses are decorative. |
| Storm rain (visual) | `board.rs` `update_storm_rain` | No | — | **Add (low priority).** Soft rain noise during Storm Season levels would sell the outage fantasy. |

## Recommended SFX set (new files under `game/assets/sfx/`)

Chiptune-style to match the licensed music aesthetic (short, < 1s each
except where noted):

1. `ui_click.ogg` — generic button press
2. `ui_hover.ogg` — button hover tick (very quiet)
3. `ui_back.ogg` — back / cancel
4. `card_select.ogg` — companion card pick (slightly musical)
5. `level_start.ogg` — board spawn sweep
6. `pill_pickup.ogg` — component pill select
7. `splice_success.ogg` — splice snap-in (the money sound)
8. `splice_fail.ogg` — out-of-window error buzz
9. `outage_alarm.ogg` — klaxon sting (~2s)
10. `countdown_tick.ogg` — repair clock tick
11. `countdown_warning.ogg` — low-time urgent beeps
12. `service_restored.ogg` — triumphant fanfare sting (~2s)
13. `level_win.ogg` — win jingle sting
14. `level_fail.ogg` — fail jingle sting
15. `mood_chirp.ogg` — companion reaction blip (optional)
16. `rain_loop.ogg` — storm ambience loop (optional)

## Implementation notes

- Mirror the existing `MusicPlugin` pattern: a new `SfxPlugin` in
  `game/src/audio.rs` (or a new `game/src/sfx.rs`) with one-shot
  `AudioBundle` spawns (`PlaybackMode::Despawn`) fired from the event
  systems listed above via `EventWriter`/`EventReader` or direct
  `Commands` spawns.
- No volume/mute plumbing exists today — the SFX work should add a
  master volume resource (and respect the existing 25%-per-stream
  convention on primo test runs).
- The `every_referenced_track_exists_on_disk` test pattern in `audio.rs`
  should be extended to cover every referenced SFX path, so a missing
  file fails the build instead of surfacing as silence at playtest.
- Do NOT ship generated SFX without checking licensing if any are
  sourced from third parties; synthesized-from-scratch chiptune blips
  avoid this entirely.
