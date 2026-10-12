# Deterministic motion — tested against everything light-show animates

This directory is a test bench, not game code. It asks one question: if
light-show's animation were driven the way the "Claude Code animation"
videos are made — scene state as a **pure function of time `t`**, rendered
by *seeking* to each frame instead of playing in real time — what would
that buy, and what would it cost?

Method, in one paragraph: everything animated in the game was inventoried
([inventory.md](inventory.md) — 19 time-animated systems, each with its
drive mechanism and a determinism verdict). The sprite strips were
measured numerically at their **native authored timing** (no interpolated
frames — inventing frames would fake the result). The procedural motions
were re-expressed as self-contained `renderFrame(t)` scenes using the
game's **real constants** and rendered at **240 fps** on primo with the
seek renderer adapted from the reference spike, then hash-verified for
determinism. The motions that live inside Bevy systems were instead
evaluated analytically — their actual curves sampled at 240 Hz — which is
the same measurement with no canvas in the way.

**Headline verdict:** the game's motion math is already deterministic in
shape. Every shader is a pure function of its time uniform; every eased
transition is a closed-form curve; the only wall-clock coupling is that
the clocks are *accumulated* (`+= dt`) rather than *seekable*. At 240 Hz,
no continuous curve in the game shows a judder spike beyond its own
easing profile. The quantisation that does exist lives exactly where
you'd expect: in frame-stepped sprite playback (by authoring) and in two
places where continuous progressions are re-sampled at frame steps
(win ring, success burst) — both fixable with zero art changes, as
recommendations below.

## The rendered ports (all 240 fps)

| Port | Source motion | Duration | Frames | Render wall time | MP4 size |
|------|---------------|----------|--------|------------------|----------|
| [pulse-flow](results/pulse-flow.mp4) | `pulse.wgsl` fiber flow, speed 0.8, width 0.08, base 0.25 | 5.0 s | 1200 | 7.1 s | 127,108 B |
| [signal-pulse](results/signal-pulse.mp4) | `board.rs` SignalPulse, SineInOut @ 260 u/s, 0.45 s spawn stagger | 2.0 s | 480 | 3.2 s | 48,002 B |
| [glow-breathing](results/glow-breathing.mp4) | `glow.wgsl` win glow, 1 + 0.25·sin(3t), 400 px quad | 4.2 s | 1008 | 8.3 s | 125,778 B |
| [wipe-transition](results/wipe-transition.mp4) | `anim.rs` fade envelope (CubicInOut 0.3 s + 0.3 s) through `wipe.wgsl` | 1.0 s | 240 | ~2–4 s | 38,876 B |

Every MP4 is H.264, 640×360, `r_frame_rate = 240/1`, frame count exact
(ffprobe-verified). No scene needed the 120 fps fallback: at 640×360 the
240 fps rate cost seconds, not minutes (effective 110–170 rendered
frames/s including PNG capture). Contact sheets (12 tiles each) sit next
to the MP4s as `results/<name>-contact.png` and were read frame by frame
for this report — visible, correctly ordered progression, no blank
frames.

<details>
<summary>How the measurements work</summary>

- **Sprite metric** (`harness/metrics_sprites.py` → `results/metrics.json`):
  for each strip, per-frame *pixel-diff energy* = mean absolute RGBA
  difference between consecutive frames (0..1), at native timing only.
  Reported: median and max intra-cycle delta, and for looping strips the
  **seam delta** (last frame → first) and **seam ratio** (seam ÷ median).
  A seam ratio near 1.0 means the loop wraps as smoothly as it plays;
  much larger means the wrap is the cycle's biggest visible step.
- **Curve metric** (`harness/metrics_curves.py` → `results/curves.json`):
  each continuous or step motion function from the source, sampled at
  240 Hz. Reported per curve: max/mean per-frame delta, delta CV
  (0 = perfectly uniform motion), and *spikiness* (max ÷ mean delta).
  Reading: a pure sinusoid scores spikiness π/2 ≈ 1.57 by construction;
  an easing curve scores ~3–4 from its own shape. A judder problem would
  appear as a spike far above the curve's family value. Step functions
  score in the tens — all their motion is concentrated at authored step
  instants, which is the quantisation signature, not a defect per se.
- **Determinism check**: after each render pass, three sample frames
  (0, 37.5%, 83% through the scene) are re-rendered from scratch and
  SHA-256-compared against the first pass. A mismatch fails the render.

</details>

<details>
<summary>Deterministic ports — detail and hash evidence</summary>

**pulse-flow** — `harness/scenes/pulse-flow.html`. The WGSL fragment math
re-implemented per-pixel: `head = fract(0.8t)`, wrapped gaussian
`exp(−d²/0.08²)`, base brightness 0.25, color (1.0, 0.85, 0.55) from
`board.rs`. The contact sheet shows the head travelling and wrapping
(the wrapped distance makes it appear at both ends simultaneously at the
wrap instant — the shader's actual behaviour). Analytic check: the
brightness profile's per-frame delta at 240 Hz has CV 0.003 — a
constant-velocity translation, the smoothest motion measured anywhere in
this test.
Hashes: f0000 `91132f741129a153…`, f0450 `fd901aec702abc16…`, f0996
`f008577f73b4eb9f…` — all MATCH.

**signal-pulse** — `harness/scenes/signal-pulse.html`. Position is the
closed form `lerp(from, to, SineInOut(260·(t−t0)/400))` on a
representative 400 px edge, two pulses staggered by the real 0.45 s
spawn interval. The contact sheet shows the SineInOut signature plainly:
small steps near the nodes, wide spacing mid-edge, closing up again on
arrival. Sampled at 240 Hz the peak per-frame step is 1.70 px (peak
velocity 408 px/s = (π/2)·260, exactly the SineInOut construction).
Hashes: f0000 `6e7c233ea12f7c59…`, f0180 `1566bacc644b0bfa…`, f0398
`d08bc2825ac07533…` — all MATCH.

**glow-breathing** — `harness/scenes/glow-breathing.html`. The win glow
exactly as `playing.rs` spawns it: 400 px quad, color (1.0, 0.9, 0.6),
quadratic falloff, factor `1 + 0.25·sin(3t)`. Two full breaths across
4.2 s on the contact sheet; the factor's per-frame delta peaks at
0.003125 (spikiness 1.57, pure-sine signature).
Hashes: f0000 `ea14e8dc005e0759…`, f0378 `9919a8325a7bcb57…`, f0836
`8147db6d0a1915ad…` — all MATCH.

**wipe-transition** — `harness/scenes/wipe-transition.html`. The
`anim.rs` envelope (0.3 s CubicInOut out, swap at full cover, 0.3 s in;
placed at t = 0.15 s in a 1 s scene so both states show) driving
`wipe.wgsl` verbatim: direction (0.7, 0.7), softness 0.12, projection
normalised by the L1 extent. Frame 60 (t = 0.25 s) inspected full-size:
the soft black wedge enters from the top-left corner — the corner the
projection math says it must. The fade envelope sampled at 240 Hz:
max per-frame Δalpha 0.0405, spikiness 2.92 (CubicInOut's own profile).
Hashes: f0000 `39779425952825c9…`, f0090 `6a3e3c22610c97db…`, f0199
`571e5059f5d6d9c9…` — all MATCH.

Where a motion is Bevy-internal (mood pop, banner slides, results
entrances, talk bounce, typewriter, face stack, storm fall), no canvas
port was made: the curve evaluation in `results/curves.json` measures
the *actual* function deterministically, which is stronger evidence than
a redraw. That route choice is per-item in the verdicts section.

</details>

<details>
<summary>Sprite-strip measurements — what the numbers say</summary>

Full data: `results/metrics.json` (38 strips). The patterns that matter:

- **Companion loops are uniformly built.** All 24 mood rows (4 runtime
  companions × 6 moods, 0.18 s/frame) have seam ratios in a tight
  1.30–1.38 band: the wrap step is consistently ~⅓ larger than the
  median step, and in most rows the seam *is* the cycle's largest step.
  That's a property of the 4-frame authoring (frame 4 must return to
  frame 1's pose neighbourhood), evenly spread — no single row is an
  outlier, so there's no one bad strip to fix.
- **Picker silhouettes wrap cleanly** (seam 1.04–1.11) except
  **select-linka at 1.88** — Linka's hover loop has the repo's largest
  relative wrap step among runtime strips. Its absolute deltas are
  small (median 0.015), so this is a modest effect, but it is the one
  measurable seam standout.
- **The unwired fiber_pulse cycle is perfectly uniform** — every delta
  exactly 0.03346, seam ratio 1.00. (It was superseded by the pulse
  shader, which is smoother still: continuous, not 4-step.)
- **One-shots have the biggest absolute deltas**, as designed: win ring
  median 0.150 / max 0.216 (an expanding ring *should* change a lot per
  frame at 0.12 s steps), connect spark median 0.075 at a snappy
  0.06 s/frame.
- **README GIFs are the chunkiest motion the project ships.** Linka's
  base GIF runs at 0.4 s/frame (2.5 fps) with seam ratio 1.73; Lattice's
  mature GIF has *irregular* frame durations baked into the file
  (0.06–0.46 s) and the largest median delta measured (0.195).
  Séraphine's mature GIF is the inverse case: seam ratio 0.28 — its loop
  closes gently while an intra-cycle step (max 0.162) does the jumping.

| Strip | Kind | Frames | s/frame | Median Δ | Max Δ | Seam Δ | Seam ratio |
|-------|------|--------|---------|----------|-------|--------|------------|
| `companion-seraphine-idle` | runtime-sheet-row | 4 | 0.18 | 0.02904 | 0.03819 | 0.03819 | 1.31 |
| `companion-seraphine-blush` | runtime-sheet-row | 4 | 0.18 | 0.02645 | 0.03489 | 0.03489 | 1.32 |
| `companion-seraphine-wink` | runtime-sheet-row | 4 | 0.18 | 0.02954 | 0.03918 | 0.03918 | 1.33 |
| `companion-seraphine-pout` | runtime-sheet-row | 4 | 0.18 | 0.02577 | 0.03409 | 0.03409 | 1.32 |
| `companion-seraphine-celebrate` | runtime-sheet-row | 4 | 0.18 | 0.02984 | 0.04097 | 0.04097 | 1.37 |
| `companion-seraphine-alarmed` | runtime-sheet-row | 4 | 0.18 | 0.03099 | 0.04110 | 0.04110 | 1.33 |
| `companion-ondine-idle` | runtime-sheet-row | 4 | 0.18 | 0.04093 | 0.05306 | 0.05306 | 1.30 |
| `companion-ondine-blush` | runtime-sheet-row | 4 | 0.18 | 0.03886 | 0.05087 | 0.05087 | 1.31 |
| `companion-ondine-wink` | runtime-sheet-row | 4 | 0.18 | 0.04231 | 0.05626 | 0.05626 | 1.33 |
| `companion-ondine-pout` | runtime-sheet-row | 4 | 0.18 | 0.03841 | 0.05051 | 0.05051 | 1.31 |
| `companion-ondine-celebrate` | runtime-sheet-row | 4 | 0.18 | 0.04658 | 0.06212 | 0.06212 | 1.33 |
| `companion-ondine-alarmed` | runtime-sheet-row | 4 | 0.18 | 0.04680 | 0.06302 | 0.06302 | 1.35 |
| `companion-linka-idle` | runtime-sheet-row | 4 | 0.18 | 0.04738 | 0.06528 | 0.06528 | 1.38 |
| `companion-linka-blush` | runtime-sheet-row | 4 | 0.18 | 0.02868 | 0.03782 | 0.03782 | 1.32 |
| `companion-linka-wink` | runtime-sheet-row | 4 | 0.18 | 0.04576 | 0.06216 | 0.06216 | 1.36 |
| `companion-linka-pout` | runtime-sheet-row | 4 | 0.18 | 0.03052 | 0.04132 | 0.04132 | 1.35 |
| `companion-linka-celebrate` | runtime-sheet-row | 4 | 0.18 | 0.02715 | 0.03710 | 0.03710 | 1.37 |
| `companion-linka-alarmed` | runtime-sheet-row | 4 | 0.18 | 0.03869 | 0.05309 | 0.05309 | 1.37 |
| `companion-lattice-idle` | runtime-sheet-row | 4 | 0.18 | 0.03289 | 0.04295 | 0.04295 | 1.31 |
| `companion-lattice-blush` | runtime-sheet-row | 4 | 0.18 | 0.03088 | 0.04026 | 0.04026 | 1.30 |
| `companion-lattice-wink` | runtime-sheet-row | 4 | 0.18 | 0.03172 | 0.04219 | 0.04219 | 1.33 |
| `companion-lattice-pout` | runtime-sheet-row | 4 | 0.18 | 0.02980 | 0.03960 | 0.03960 | 1.33 |
| `companion-lattice-celebrate` | runtime-sheet-row | 4 | 0.18 | 0.03026 | 0.03970 | 0.03970 | 1.31 |
| `companion-lattice-alarmed` | runtime-sheet-row | 4 | 0.18 | 0.03198 | 0.04204 | 0.04204 | 1.31 |
| `fx-connect_spark` | fx-sequence | 4 | 0.06 | 0.07512 | 0.10907 | — | — |
| `fx-success_burst` | fx-sequence | 6 | 0.1 | 0.05863 | 0.09520 | — | — |
| `fx-win_ring` | fx-sequence | 6 | 0.12 | 0.14983 | 0.21595 | — | — |
| `fx-fiber_pulse-unwired` | fx-sequence-unwired | 4 | 0.1 | 0.03346 | 0.03346 | 0.03346 | 1.00 |
| `fx-error_flicker-unwired` | fx-sequence-unwired | 4 | 0.1 | 0.04719 | 0.06037 | 0.03701 | 0.78 |
| `select-seraphine` | picker-select | 6 | 0.12 | 0.02367 | 0.03943 | 0.02463 | 1.04 |
| `select-ondine` | picker-select | 6 | 0.12 | 0.05893 | 0.07745 | 0.06569 | 1.11 |
| `select-linka` | picker-select | 6 | 0.12 | 0.01498 | 0.02369 | 0.02819 | 1.88 |
| `select-lattice` | picker-select | 6 | 0.12 | 0.00476 | 0.00637 | 0.00509 | 1.07 |
| `gif-lattice_animated_mature` | readme-gif | 16 | varies | 0.19463 | 0.28801 | 0.22658 | 1.16 |
| `gif-linka_animated_base` | readme-gif | 6 | 0.4 | 0.10728 | 0.12899 | 0.18562 | 1.73 |
| `gif-ondine_animated_mature` | readme-gif | 12 | 0.2 | 0.02909 | 0.03955 | 0.02523 | 0.87 |
| `gif-seraphine_animated_base` | readme-gif | 4 | 0.18 | 0.06323 | 0.06381 | 0.06185 | 0.98 |
| `gif-seraphine_animated_mature` | readme-gif | 12 | 0.2 | 0.07851 | 0.16217 | 0.02159 | 0.28 |

</details>

<details>
<summary>Curve evaluations — the game's motion math at 240 Hz</summary>

Full data: `results/curves.json` (18 curves). Units are each curve's own
(alpha, scale, px, chars — see `source` in the JSON and inventory.md).

| Curve | Kind | Duration (s) | Max Δ/frame | Mean Δ/frame | CV | Spikiness |
|-------|------|--------------|-------------|--------------|-----|-----------|
| `fade-envelope-out` | continuous | 0.3 | 0.040520 | 0.013889 | 0.894 | 2.92 |
| `mood-pop-scale` | continuous | 0.15 | 0.019352 | 0.005324 | 1.108 | 3.63 |
| `outage-banner-slide-in` | continuous | 0.35 | 4.405462 | 1.142831 | 1.108 | 3.85 |
| `outage-banner-slide-out` | continuous | 0.25 | 3.868148 | 1.333333 | 0.894 | 2.90 |
| `results-banner-pop` | continuous | 0.3 | 0.012814 | 0.003333 | 1.108 | 3.84 |
| `results-fade-in` | continuous | 0.2 | 0.061207 | 0.020833 | 0.894 | 2.94 |
| `results-slide-up` | continuous | 0.25 | 0.983426 | 0.333333 | 0.894 | 2.95 |
| `signal-pulse-position` | continuous | 1.5385 | 1.701690 | 1.084010 | 0.483 | 1.57 |
| `glow-pulse-factor` | continuous | 2.0944 | 0.003125 | 0.001990 | 0.483 | 1.57 |
| `talk-bounce-scale` | continuous | 1.0 | 0.002624 | 0.001640 | 0.495 | 1.60 |
| `advance-indicator-pulse` | continuous | 1.0472 | 0.011250 | 0.007156 | 0.484 | 1.57 |
| `atmosphere-drift` | continuous | 4.0 | 0.000042 | 0.000035 | 0.165 | 1.19 |
| `pulse-flow-brightness` | continuous-field | 1.25 | 0.006722 | 0.006666 | 0.003 | 1.01 |
| `win-ring-scale` | step | 0.72 | 0.294907 | 0.004028 | 7.112 | 73.22 |
| `success-burst-scale` | step | 0.6 | 0.133333 | 0.003704 | 5.916 | 36.00 |
| `companion-frame-index` | step | 0.72 | 1.000000 | 0.017341 | 7.528 | 57.67 |
| `typewriter-chars` | step | 1.0 | 1.000000 | 0.120833 | 2.697 | 8.28 |
| `storm-streak-fall` | continuous-linear | 4.0385 | 2.166667 | 2.166667 | 0.0 | 1.00 |

Notes: every sinusoid lands on spikiness 1.57 = π/2, the theoretical
value — a useful self-check on the harness. The BackOut curves
(mood pop, banner-in, results pop) peak at 3.6–3.9 because BackOut's
velocity peaks early by design; the banner slide-in's peak step is
4.41 px/frame at 240 Hz, i.e. the overshoot sweep is fast but smooth —
no secondary spikes anywhere. The step rows are the honest contrast:
win-ring scale holds still for 28 frames at 240 Hz, then jumps 0.295 in
one frame — that *is* what 0.12 s frame-stepping means, measured rather
than asserted.

</details>

<details>
<summary>Per-animation verdicts (condensed; full citations in inventory.md)</summary>

- **Already pure f(t), only the clock accumulates:** fiber pulse flow,
  glow breathing, atmosphere drift, wipe coverage. Porting these was a
  clock swap, not a rewrite — the strongest evidence that the game's
  procedural layer is deterministic-motion-ready today.
- **f(t)-expressible with the game's real constants:** transition fade,
  mood pop, results entrances, outage banner, signal-pulse travel,
  talk bounce, advance pulse. Each is an accumulated integral of a
  closed form; the ports/evaluations use the closed forms.
- **Frame-indexed by authoring (measure, don't "fix"):** companion
  sheets, connect spark, picker silhouettes, face stack, typewriter.
  Their stepping is the art/direction, and their timing is already
  phase-locked (`times_finished_this_tick`), so playback rate does not
  drift with frame rate.
- **Continuous curves quantised by frame-stepping (the one real
  finding):** win-ring and success-burst scale/alpha. The formulas are
  continuous in the source; only the *sampling* is stepped.
- **Frame-rate-dependent by construction:** the storm streak spawn ramp
  (2 per rendered frame ⇒ sky fills in 20 frames at 60 fps, 10 at
  120 fps). Per-streak motion is a constant-velocity f(t); the ramp is
  the exception.
- **Not time-animated:** neon title (state-swapped alphas), UI variant
  sets (state-indexed, currently unwired in Rust), camera (no motion
  system — despawn/respawn per Finding 3), drag preview (input-driven).

</details>

<details>
<summary>Recommended adoption path (recommendations only — no game code was changed)</summary>

1. **One seekable clock for motion.** Replace the per-system
   `elapsed += delta_secs` accumulators (shader time in
   `tick_shader_time`, fade/mood-pop/entrance/banner elapsed, dialogue
   `talk_time`) with values read from a single elapsed source that
   tests/tools can set. The motion functions don't change at all; the
   payoff is that any visual state becomes reproducible at an exact t —
   which is what makes frame-exact regression screenshots and renders
   like these ports possible from the game itself.
2. **Sample progressions from elapsed, not frame index.** Win-ring and
   success-burst scale/alpha are already closed-form in the frame index;
   evaluating the same formulas at `elapsed / frame_secs` removes the
   measured 36–73× delta concentration with zero art or timing changes.
3. **Keep sprite playback exactly as-is.** The phase-locked timer
   pattern is the deterministic way to step authored frames, and the
   seam measurements show uniformly-authored loops. If a future art
   pass wants softer companion loops, the lever is a 5th/6th authored
   frame or crossfade — an art decision, not a clock decision.
4. **Fixed-step the storm spawn ramp** (spawn by elapsed time, e.g.
   120 streaks/s until full) so outage onset looks identical at any
   refresh rate.
5. **Reuse this harness for any future motion work.** Any scene written
   as `renderFrame(t)` renders at arbitrary frame rates with
   hash-verified determinism (240 fps used here); `harness/` is
   self-contained (`npm install`, then the commands below).

</details>

<details>
<summary>Reproduce it</summary>

```bash
# from this directory, on a machine with node 22 + chromium + ffmpeg
cd harness && npm install
node render.mjs scenes/pulse-flow.html      ../results/pulse-flow.mp4      ../results/pulse-flow-contact.png
node render.mjs scenes/signal-pulse.html    ../results/signal-pulse.mp4    ../results/signal-pulse-contact.png
node render.mjs scenes/glow-breathing.html  ../results/glow-breathing.mp4  ../results/glow-breathing-contact.png
node render.mjs scenes/wipe-transition.html ../results/wipe-transition.mp4 ../results/wipe-transition-contact.png

# metrics (python3 + Pillow + numpy), from the repo root
python3 deterministic-motion/harness/metrics_sprites.py . deterministic-motion/results/metrics.json
python3 deterministic-motion/harness/metrics_curves.py deterministic-motion/results/curves.json
```

Rendered frames land in `harness/.frames/` (scratch — delete before
committing; it is not part of the results). The render machine for the
committed artifacts was primo (Node v22.23.3, system Chromium,
FFmpeg n9.0.2).

</details>

<details>
<summary>Scope guards and provenance</summary>

- Analysis and harness only: no game code, assets, or existing files
  were modified; everything new lives in this directory (plus the
  project's standard `.ai/memory/inbox/` note).
- Work was done in a fresh worktree off `origin/main` (`de1751b`);
  `~/workspace/repos/light-show` and the `pax/api36` /
  `pax/splice-tray` branches were not touched.
- The trailer project is paused and untouched: no trailer assets, no
  music, no game-art generation. The `docs/media` trailer preview GIF
  was deliberately excluded from measurement.
- Sprite strips were measured as shipped at native timing. No strip was
  interpolated, re-timed, or "smoothed"; the only re-timing anywhere in
  this directory is the *procedural* ports, whose functions are
  continuous in t by construction — and that distinction is labelled
  wherever it appears.

</details>
