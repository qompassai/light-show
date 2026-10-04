# Current Work — light-show

Rust + Bevy fiber-optic puzzle game. Goal: ship to Google Play, F-Droid, Windows.

## Active (2026-10-04)

- **Bevy 0.19.1 migration**: Rendering fixed (added `bevy_sprite_render`,
  `bevy_ui_render` features — Bevy 0.19 split render from logic features).
  Audio decode guard added (rodio pre-decode check skips undecodable files
  instead of panicking in worker thread). Tree is dirty; preserve unrelated
  paths when staging.
- **Companion art**: Base idle GIFs stabilized and pushed (d2ec02a).
  Mature idle GIFs stabilized and pushed (4131321). Four companions:
  Séraphine (Fiber), Ondine (Coax), Linka (Wireless), Lattice (Wired).
- **Pending**: Gameplay capture re-shoots (title/level1/outage) against
  repaired build; README media refresh; aroused portraits for
  Séraphine/Ondine/Linka (Lattice has hers); expression trigger integration.

## Standing rules

- Scoped push authorization: qompassai/light-show (gated). Stage surgically,
  `git diff --cached --stat` before commit, verify remote byte-identical.
- 144×288 base GIFs, 200ms/frame, transparent, no blink frames.
- Verify worker claims on the machine before accepting completion.
