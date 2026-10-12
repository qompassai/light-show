# Deterministic-motion test pass (Pax, 2026-10-12)

Matt asked for the deterministic-motion technique from the Claude Code
animation research (scene state as a pure function of `t`, seek-rendered
frame by frame) to be tested against everything animated in light-show.
The full work product is the new top-level `deterministic-motion/`
directory (inventory, harness, measured results, rendered ports); this
note records the durable findings.

## Findings

- The game's procedural motion math is already pure f(t): all three
  animated shaders (pulse flow, glow, atmosphere drift) and the wipe are
  pure functions of a time uniform. The only wall-clock coupling is that
  clocks are accumulated (`settings.time += dt` in `tick_shader_time`,
  `elapsed_secs += delta_secs` in the transition/pop/entrance/banner
  systems) rather than seekable. No motion function needs new math to
  become deterministic/seekable — only the clock source.
- At 240 Hz sampling, no continuous curve in the game spikes beyond its
  own easing profile (sinusoids at the theoretical spikiness π/2 ≈ 1.57;
  eased one-shots 2.9–3.9; pulse-flow brightness field CV 0.003).
- Win-ring and success-burst scale/alpha are continuous formulas
  re-sampled at frame steps (0.12 s / 0.10 s) — measured delta
  spikiness 73× and 36× at 240 Hz, purely from quantisation. Sampling
  the same formulas from elapsed time is a zero-art-change fix;
  recorded as a recommendation in deterministic-motion/README.md, not
  implemented (analysis-only pass).
- Companion loop seams are uniform across all 24 mood rows (seam ratio
  1.30–1.38). One standout: picker `select-linka` seam ratio 1.88.
  README GIFs are the chunkiest shipped motion (linka base GIF:
  0.4 s/frame, seam 1.73; lattice mature GIF: irregular 0.06–0.46 s
  frame durations baked into the file).
- Storm streak spawn ramp is per-rendered-frame (2/frame), so outage
  sky-fill time varies with refresh rate; per-streak motion is linear.
- Asset-coverage fact: `Companion::sprite_path()` names 8 sheets, but
  only 4 companions' directories ship in `game/assets/sprites/`
  (seraphine/ondine/linka/lattice mature fullbody). The
  clara/aino/hikari/lea `*_sheet_fullbody.png` targets are absent from
  the repo. Recorded as found; not addressed by this pass.

## Validation

- `cargo test --workspace` (keyless) in the worktree after adding the
  directory: **806 passed / 0 failed** — identical to the pre-change
  baseline; the new directory does not disturb the build.
- Four procedural ports rendered at 240 fps (H.264, ffprobe-verified
  frame counts/durations) with re-render SHA-256 determinism checks:
  12/12 sample frames MATCH.
- 38 sprite strips measured at native timing; 18 motion curves
  evaluated at 240 Hz. Data: `deterministic-motion/results/`.

Affected paths: `deterministic-motion/**` (new), this inbox note.
No game code or assets were modified.
