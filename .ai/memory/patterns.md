# Patterns — light-show

## GIF specs

- Base idle: 144×288, 200ms/frame, transparent background, 12 frames
  (Ondine: 11). No deliberate blink/closed-eye frames.
- Measure with alpha-weighted centroid per frame (`realign_frames.py`
  in `~/workspace/light-show/base-idle/`).
- `cx_span`/`cy_span` > 4px indicates systematic defect (seam/drift),
  not breathing. But verify visually — accessories (cables, braids)
  skew centroid metrics.

## PerfectPixel techniques (applied selectively)

- Reference: github.com/gykim80/perfectpixel-studio
  (`internal/sprite/extract.go`).
- Alpha-weighted centroid for horizontal anchoring; common baseline for
  vertical. Integer shifts only (no resampling) unless scale mismatch
  is confirmed (Linka base needed 0.93 Lanczos on row 2).
- Do NOT apply blindly: test per-character, keep genuine secondary motion.

## Build & push

- All cargo work on primo. `cargo test -p light-show` is the gate.
- Push auth: scoped to qompassai/light-show. Stage only intended paths.
  Verify with `git ls-remote origin main` + `git ls-tree` blob hashes.
- Never push with unrelated dirty files (Bevy migration tree is large).

## Primo access

- SSH: `~/workspace/bin/primo-ssh` (wrapper, NOT plain `ssh primo`).
- Repo: `/home/phaedrus/workspace/repos/light-show`.
