# Light Show — Animation Fluidity Audit

**Date:** 2026-10-02
**Scope:** All animation systems in `game/src/` (Rust + Bevy 0.14)
**Method:** Static code review of every animation timer, state transition,
sprite system, UI interaction, and particle/FX system.

---

## Summary

The game has **zero easing and zero transition animations**. Every visual
change in the codebase is an instant swap: mood changes pop, state
transitions cut, banners appear at full opacity, outlines jump 0→3px,
and cameras are despawned/respawned in a single frame. The sprite
frame-stepping itself is fine (fixed timers, correct loop behavior), but
nothing *between* states is animated.

There is also **no SFX system** — only background music tiers. UI actions
(button presses, pill picks, splice placements) are silent.

Bevy 0.14 ships `bevy::math::curve::EaseFunction` (CubicInOut, etc.),
so all recommendations below can use the engine's built-in easings with
no new dependencies.

---

## Finding 1 — Companion mood changes pop with no transition
**Severity:** High (most visible character animation in the game)

Every mood assignment in the codebase is an instant `sprite.mood = X`
plus `sprite.frame = 0`:

| Location | Trigger |
|---|---|
| `game/src/waifu/mod.rs:61-66` (`react_to_splice_events`) | Splice placed (Blush/Pout) |
| `game/src/states/outage.rs:69-73` (`alarm_companion`) | Outage fires (Alarmed) |
| `game/src/states/playing.rs:240-244` (`check_win_condition`) | Level won (Celebrate) |
| `game/src/states/playing.rs:141-144` (`setup_level`) | Level start (Idle reset) |
| `game/src/states/outage.rs:182-184` (`check_outage_resolution`) | Outage repaired (Wink) |
| `game/src/waifu/mod.rs:336-352` (`respawn_on_companion_change`) | Companion swap (full despawn/respawn) |

`sync_companion_atlas_index` (`waifu/mod.rs:358-364`) then applies the
new atlas cell on the next frame the component changes — one frame of
the old mood, then a hard cut.

**Recommendation:** Add a 0.25s mood crossfade. Keep two atlas indices
(old/new) and lerp the sprite's alpha from 1→0 on the old mood while
fading 0→1 on the new, using `EaseFunction::CubicInOut`. Alternatively,
a 0.15s scale "pop" (1.0 → 1.08 → 1.0, `EaseFunction::BackOut`) on mood
change reads as a reaction without needing dual-sprite blending.

---

## Finding 2 — State transitions cut with no fade/slide
**Severity:** High (every screen change in the game)

All six states use bare `OnEnter` spawn / `OnExit` despawn:

| Transition | Enter system | Exit system |
|---|---|---|
| → MainMenu | `menu.rs:setup_menu` | `menu.rs:teardown_menu` |
| → CompanionSelect | `companion_select.rs:setup_select` | `teardown_select` |
| → Playing | `playing.rs:setup_level` | (board teardown on Results/Menu enter) |
| → OutageActive | `outage.rs:spawn_outage_banner` + `alarm_companion` | `outage.rs:teardown_outage_banner` |
| → Results | `results.rs:show_results` | `results.rs:teardown_results` |
| → Credits | `credits.rs:setup_credits` | `credits.rs:teardown_credits` |

**Recommendation:** Add a fullscreen fade overlay resource
(`TransitionFade { alpha: f32, direction: In/Out }`). On any
`NextState::set`, run a 0.3s fade-out (`EaseFunction::CubicInOut`),
swap states at full black, then 0.3s fade-in. This is the single
highest-leverage change: it smooths every transition at once without
touching individual state code.

---

## Finding 3 — Camera is despawned and respawned instantly
**Severity:** High (visible hitch on every level start)

`game/src/states/playing.rs:setup_level` (lines ~118-131) despawns all
cameras and spawns a new `Camera2dBundle` at `(0, 200, 0)` in the same
frame — one frame the player sees the menu framing, the next frame the
board framing, with no blend.

**Recommendation:** Keep a single persistent camera. On
`OnEnter(Playing)`, lerp its transform from the menu position to
`(0, 200, 0)` over 0.5s with `EaseFunction::CubicInOut`. This also
removes the Android camera-framing workaround's visual pop (the comment
at `playing.rs:114-117` notes the dedicated camera exists for Android
framing — the lerp preserves that fix).

---

## Finding 4 — Outage banner slams in at full opacity
**Severity:** Medium (key dramatic moment lands flat)

`game/src/states/outage.rs:spawn_outage_banner` (lines 79-113) spawns
the full-width red banner at `top: 0px` with `0.85` alpha immediately.
The outage firing is the game's central dramatic beat, and it currently
has less ceremony than a toast notification.

**Recommendation:** Slide the banner down from `top: -80px` to `top: 0`
over 0.35s with `EaseFunction::BackOut` (slight overshoot sells the
"alarm" feel), and fade its alpha 0→0.85 over the same window. On
`teardown_outage_banner`, reverse: 0.25s slide-up + fade-out instead of
instant despawn.

---

## Finding 5 — Companion-select hover outline jumps 0→3px
**Severity:** Medium (primary menu interaction feels cheap)

`game/src/states/companion_select.rs:highlight_select_cards`
(lines 301-311) snaps `outline.width` between `Val::Px(0.0)` and
`Val::Px(3.0)` on `Changed<Interaction>`.

**Recommendation:** Lerp `outline.width` toward its target at ~12px/s
(or over 0.15s with `EaseFunction::CubicOut`) in an `Update` system
instead of snapping on `Changed<Interaction>`. Same for the accent
color — lerp the outline color alpha 0→1 on hover.

Related: `animate_select_cards` (lines 275-297) advances the silhouette
at 0.12s/frame while hovered but leaves the card frozen mid-cycle on
unhover. Reset `anim.index = 0` and swap back to frame 0 when
`Interaction` returns to `None`.

---

## Finding 6 — Results screen elements all appear in one frame
**Severity:** Medium

`game/src/states/results.rs:show_results` (lines 117-230) spawns the
win ring, banner text, world title, ledger, dialogue line, and buttons
simultaneously. No stagger.

**Recommendation:** Stagger entrance over 0.6s: banner scales 0.8→1.0
(`BackOut`, 0.3s), then ledger/dialogue fade in (0.2s each, staggered
0.1s), then buttons slide up 20px + fade (0.25s). The win ring one-shot
(`animate_win_ring`, lines 70-93) should also ease its scale 0.5→1.2
with alpha 1→0 across its 0.72s lifetime instead of playing at fixed
size.

---

## Finding 7 — Pill-ring pick feedback is a silent texture swap
**Severity:** Medium (core puzzle interaction has no juice)

`game/src/board.rs:update_pill_rings` (lines 1046-1061) swaps the ring
texture handle instantly when `PlacedChoices` changes. No scale pop, no
glow pulse, no sound.

**Recommendation:** On pick change, spawn a 0.2s scale pop on the ring
(1.0 → 1.25 → 1.0, `EaseFunction::BackOut`) plus a brief alpha pulse
on the glow. This is the highest-frequency player interaction in the
game — it deserves the most polish per occurrence.

---

## Finding 8 — Signal pulses move at constant linear speed
**Severity:** Low (acceptable, but could feel more organic)

`game/src/board.rs:move_signal_pulses` (lines 1194-1217) advances
`pulse.progress` linearly and lerps position directly.

**Recommendation:** Apply `EaseFunction::SineInOut` to
`pulse.progress` before the lerp, so pulses accelerate out of the
source and decelerate into the target. Keep `PULSE_SPEED` as-is; only
the easing curve changes. Alternatively, add a slight scale pulse
(1.0 → 1.15 → 1.0) at the midpoint for a "packet" feel.

---

## Finding 9 — No SFX system (music only)
**Severity:** Medium (game feel gap)

`game/src/audio.rs` exposes only music-tier selectors
(`menu_track`, `playing_track`, `outage_track`, `results_track`).
There are no sound effects for: button presses, pill picks, splice
placements, drag release, outage alarm, countdown ticks, win/lose
stingers, or companion reactions.

**Recommendation:** Add a lightweight `Sfx` resource with a
`play(commands, SfxKind)` helper backed by `AudioBundle`. Minimum viable
set: `Click` (buttons), `Pick` (pill select), `Place` (splice placed),
`Alarm` (outage fires), `Tick` (countdown last 5s), `Win`/`Lose`
(results). Chiptune-style square-wave blips match the existing
Megaman-esque soundtrack direction.

---

## Finding 10 — Dialogue text appears instantly (no typewriter)
**Severity:** Low

Companion dialogue lines (results screen, level-enter flavor) spawn as
complete `TextBundle`s. No character-by-character reveal.

**Recommendation:** Add a `TypewriterText { full: String, shown: usize,
timer: Timer }` component; reveal ~40 chars/sec. Skip to full on click/
tap. This is standard for visual-novel-style companion dialogue and
makes the companion feel more present.

---

## Finding 11 — Drag preview line follows pointer with no smoothing
**Severity:** Low

`game/src/board.rs:draw_dashed_line` / `neon_line_2d` (lines 976-996)
draw the drag preview directly at the pointer position each frame.

**Recommendation:** Lerp the preview endpoint toward the pointer at
~20/s for a slight trailing feel. Minor, but it makes the drag feel
less rigid on touchscreens.

---

## Sprite frame timing reference

All frame-stepped animations use fixed timers with no easing (correct
for sprite-sheet playback, listed here for completeness):

| System | Location | Timing |
|---|---|---|
| Companion idle/mood loop | `waifu/mod.rs:354-360` | 0.18s/frame, 4 frames, hard step |
| Win ring one-shot | `results.rs:70-93` | 0.12s/frame, 6 frames, then despawn |
| Companion-select silhouette | `companion_select.rs:275-297` | 0.12s/frame, 6 frames, hover-only |
| Signal pulse spawn | `board.rs:1119-1126` | 0.45s stagger between spawns |
| Storm streaks | `board.rs:1220-1283` | 520 u/s constant fall, wrap |

These are fine as-is; the gaps are all in *transitions between* states,
not in the frame playback itself.

---

## Recommended implementation order

1. **Fullscreen fade overlay** (Finding 2) — one system, fixes all transitions
2. **Mood crossfade/pop** (Finding 1) — most visible character animation
3. **Persistent camera lerp** (Finding 3) — removes the level-start hitch
4. **Outage banner slide-in** (Finding 4) — the dramatic beat needs ceremony
5. **Pill-ring pick pop** (Finding 7) — highest-frequency interaction
6. **SFX system** (Finding 9) — game feel gap
7. **Results stagger** (Finding 6) — polish
8. **Hover outline lerp** (Finding 5) — menu polish
9. **Pulse easing** (Finding 8) — subtle
10. **Typewriter dialogue** (Finding 10) — flavor
11. **Drag smoothing** (Finding 11) — subtle
