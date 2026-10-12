# Inventory — everything animated in light-show

Method: `rg` over `game/src` for the drive patterns (`Time<`, `delta_secs`,
`Timer`, `times_finished_this_tick`, atlas index stepping, `sin(`/`cos(` of a
clock, `EaseFunction` sampling, shader `time` uniforms), plus a full read of
`anim.rs`, `fx.rs`, `shaders.rs`, the four WGSL shaders, and the animation
systems in `board.rs`, `waifu/`, and the state screens. Cross-checked against
the project's own `docs/ANIMATION_AUDIT.md` (its sprite-timing table agrees
with every timing found here).

**Count: 19 time-animated systems**, in four drive classes, plus 2 authored
cycles that ship unwired, and a short list of things that look animated but
are state- or input-driven. Every verdict uses one vocabulary:

- **pure f(t)** — the motion math is already a pure function of a time value;
  only the clock feeding it accumulates wall-clock dt.
- **f(t)-expressible** — today it is integrated/accumulated per frame, but a
  closed form with the game's real constants reproduces it exactly.
- **frame-indexed** — authored as discrete frames; playback is a step
  function of elapsed time. Deterministic, but the art itself is quantised.
- **not f(t)** — driven by input or state, not by time; nothing to port.

<details>
<summary>Frame-stepped sprite playback (Timer-driven, frame-indexed)</summary>

| # | Item | Where | Timing | Verdict |
|---|------|-------|--------|---------|
| 1 | Companion idle/mood sheet loop | `game/src/waifu/mod.rs:383` (timer), `:434` `animate_companion`; layout `game/src/waifu/sprite.rs:8` (288×576 cells, 4 frames × 6 mood rows) | 0.18 s/frame, 4-frame loop per mood row | frame-indexed. The timer counts *every* elapsed period (`times_finished_this_tick`), so the loop is phase-locked to elapsed time: frame = ⌊elapsed / 0.18⌋ mod 4. |
| 2 | Connect spark one-shot | `game/src/fx.rs:19` consts, `animate_connect_sparks` | 4 frames × 0.06 s = 0.24 s | frame-indexed one-shot |
| 3 | Success burst one-shot | `game/src/fx.rs:27`, `animate_success_bursts` | 6 frames × 0.10 s = 0.6 s; scale 0.6→1.4 and alpha 1→0.3 are linear in frame index | frame-indexed; the scale/alpha progressions are f(t)-expressible (they are closed-form in the index, only re-sampled at steps) |
| 4 | Win ring one-shot | `game/src/states/results.rs:84` consts, `:242` `animate_win_ring` | 6 frames × 0.12 s = 0.72 s; scale 0.5→1.2 by `CubicOut(index/6)`, alpha 1→0 | frame-indexed; progression f(t)-expressible as above |
| 5 | Companion-select silhouettes | `game/src/states/companion_select.rs:173` consts, `:510` `animate_select_cards` | 6 frames × 0.12 s, plays only while hovered/pressed, snaps to frame 0 on unhover | frame-indexed, input-gated |
| 6 | Fiber pulse cycle (asset) | `game/assets/sprites/fx/fiber_pulse_*.png` + `.aseprite` (100 ms/frame) | 4 frames × 0.10 s | frame-indexed — but **not wired**: `game/assets/sprites/fx/README.md` marks it "Reserved … not yet wired"; no Rust references it. The pulse *shader* (#8) replaced it. |
| 7 | Error flicker cycle (asset) | `game/assets/sprites/fx/error_flicker_*.png` + `.aseprite` (100 ms/frame) | 4 frames × 0.10 s | frame-indexed — also **not wired** (same README note). |

Runtime-sheet coverage fact: `Companion::sprite_path()` (`waifu/mod.rs:324`)
names 8 sheet files, but only the four original companions' directories ship
in `game/assets/sprites/` (seraphine, ondine, linka, lattice —
`*_sheet_fullbody_mature.png`, 1152×3456). The clara/aino/hikari/lea
`*_sheet_fullbody.png` targets are absent from the repo's assets. Recorded
as found; fixing asset coverage is outside this test's scope.

</details>

<details>
<summary>Shader-time procedural motion (pure f(t) math, accumulated clock)</summary>

All three materials receive time from one system, `tick_shader_time`
(`game/src/shaders.rs`), which does `settings.time += time.delta_secs()`
per frame. The shader functions themselves never accumulate anything.

| # | Item | Where | Motion | Verdict |
|---|------|-------|--------|---------|
| 8 | Fiber pulse flow | `game/assets/shaders/pulse.wgsl`; spawned per placed edge by `board.rs:2966` `sync_fiber_flows` with `PulseSettings::default()` (`shaders.rs`): speed 0.8 cycles/s, width 0.08, base_brightness 0.25, color (1.0, 0.85, 0.55) | `head = fract(time·0.8)`; brightness = 0.25 + exp(−d²/0.08²) with wrapped distance d; vertical sin falloff | **pure f(t)** — ported to canvas (`harness/scenes/pulse-flow.html`) |
| 9 | Glow breathing | `game/assets/shaders/glow.wgsl`; spawned on win by `states/playing.rs:486` (400 px quad, color (1.0, 0.9, 0.6)) | intensity factor `1 + 0.25·sin(3·time)` over a quadratic radial falloff | **pure f(t)** — ported (`harness/scenes/glow-breathing.html`) |
| 10 | Atmosphere backdrop | `game/assets/shaders/atmosphere.wgsl`; menu backdrop (`states/menu.rs:281`, default settings) | gradient drift `sin(uv.y·3 + time·0.25)·0.04`; vignette + scanlines are static functions of position | **pure f(t)** (the drift term; the rest is not motion) — evaluated analytically in `results/curves.json` |
| 11 | Transition wipe (spatial) | `game/assets/shaders/wipe.wgsl`; `WipeSettings::default()`: softness 0.12, direction (0.7, 0.7) | coverage = 1 − smoothstep(edge − 0.12, edge, proj(uv)), edge = progress·1.12 | pure f(progress); progress comes from #12 — ported together (`harness/scenes/wipe-transition.html`) |

</details>

<details>
<summary>Eased transitions (accumulated elapsed × easing curve — all f(t)-expressible)</summary>

Each of these keeps an `elapsed_secs += delta_secs` accumulator and samples
an easing curve at `elapsed / duration`. The closed form in absolute time is
identical; accumulation only adds frame-rate-dependent rounding.

| # | Item | Where | Curve & constants | Verdict |
|---|------|-------|-------------------|---------|
| 12 | State-transition fade envelope | `game/src/anim.rs:20` consts, `drive_transition_fade` | 0.3 s out + 0.3 s in, `CubicInOut`; drives the wipe progress of #11 | f(t)-expressible — ported with #11 |
| 13 | Mood pop (companion reaction) | `game/src/waifu/mod.rs:60`, `animate_mood_pop` | 0.15 s, scale 1.0 → 1.08 → 1.0, `BackOut` on both halves | f(t)-expressible — evaluated in `results/curves.json` |
| 14 | Results-screen entrances | `game/src/states/results.rs:111` delays, `animate_result_entrances` | banner pop `BackOut` 0.3 s (scale 0.8→1.0); text fades `CubicOut` 0.2 s; button slide `CubicOut` 0.25 s (20 px + fade); delays staggered 0/0.1/0.2/0.3/0.4 s | f(t)-expressible — evaluated in `results/curves.json` |
| 15 | Outage banner slide | `game/src/states/outage.rs:208` consts, `animate_banner_entrance` / `animate_banner_exit` | in: 0.35 s `BackOut`, top −80→0 px, alpha →0.85; out: 0.25 s `CubicInOut` | f(t)-expressible — evaluated in `results/curves.json` |

Easing formulas are Bevy 0.19.1's own (`bevy_math::curve::easing`, verified
against the vendored crate source): CubicOut `1−(1−t)³`, CubicInOut
`4t³ / 1−(−2t+2)³/2`, SineInOut `−(cos πt −1)/2`, BackOut
`1 + 2.70158(t−1)³ + 1.70158(t−1)²`.

</details>

<details>
<summary>Integrated travellers and clock oscillators (f(t)-expressible)</summary>

| # | Item | Where | Motion | Verdict |
|---|------|-------|--------|---------|
| 16 | Signal pulse travel | `game/src/board.rs:1332` consts, `:1431` `move_signal_pulses` | `progress += 260·dt / edge_len`; position = lerp(from, to, `SineInOut(progress)`); spawns every 0.45 s, max 24 alive | f(t)-expressible (constant speed ⇒ progress is linear in t) — ported (`harness/scenes/signal-pulse.html`) |
| 17 | Storm streak rain (outage) | `game/src/board.rs:1452` consts, storm systems | constant 520 u/s fall, wrap y 1200 → −900; max 40 streaks, lanes by golden-ratio stride; spawn ramp 2 per rendered frame | per-streak position is linear f(t); the *spawn ramp* is frame-count-driven, so a full-sky fill takes a frame-rate-dependent wall time (40 streaks = 20 frames) |
| 18 | Dialogue talk bounce | `game/src/waifu/dialogue_ui.rs:47` consts, `:439` | portrait scale `1 + 0.035·sin(talk_time·18)` while the typewriter runs; `talk_time` accumulates dt | pure f(t) of an accumulated clock — evaluated in `results/curves.json` |
| 19 | Advance-indicator pulse | `game/src/waifu/dialogue_ui.rs:450` | alpha `0.55 + 0.45·sin(talk_time·6)` when a line completes | pure f(t) of an accumulated clock — evaluated in `results/curves.json` |
| 20 | Typewriter reveal | `game/src/waifu/dialogue_ui.rs:31`, `:112` `Typewriter::tick` | 30 chars/s, accumulated dt, floored reveal count | step f(t) — quantised by design |
| 21 | Companion face stack (blink / talk flap / squash) | `game/src/waifu/pill.rs:38` consts, `:314` `FaceAnim` | blink: 2.8 s period, eyes closed for the first 0.12 s; flap: mouth toggles every 0.12 s while talking; `scale_y` steps 0.90 (blink) / 0.97 (mouth open) / 1.0 | step f(t) — the source is explicit: "Deterministic: no RNG, so tests pin the exact cycle" |

</details>

<details>
<summary>Not time-animated (checked, excluded from ports)</summary>

- **Neon title text** (`game/src/ui/neon.rs`): 17 static layered copies make
  the glow; hover changes alphas discretely (`states/menu.rs:490`).
  State-driven, no clock.
- **UI variant sets** (`sprites/ui/signal_bars_0..4`, `button_glow_0..2`,
  `progress_tick_0..1`): frame index = a state value (signal level, button
  state, step done). Not referenced from `game/src` at all today — only the
  generator `scripts/polish_gen_ui.lua` and `sprites/ui/README.md` name them.
- **Camera**: no motion system exists. The camera is despawned/respawned on
  state changes (`docs/ANIMATION_AUDIT.md` Finding 3) and `BoardFraming` is
  a static fit computed per level/resize. Nothing to port; see the README's
  recommendations.
- **Drag preview line** (Finding 11): follows the pointer — input-driven,
  not time-based.
- **README/store GIFs** (`assets/art/*/companions/*_animated_*.gif`, 5
  files): shipped media, not runtime animation. They *are* measured in
  `results/metrics.json` (kind `readme-gif`) because they are the game's
  public-facing motion. The `docs/*.gif` files are gameplay *recordings*
  (not authored animation) and are excluded; the trailer preview GIF is
  excluded with the paused trailer project.
- **Gameplay timing** (outage hazard timers, `LevelClock`, the 1.8 s
  `ResultsGate` completion hold in `anim.rs`): clocks that gate logic, with
  no visual motion function of their own. The banner they trigger is #15.

</details>
