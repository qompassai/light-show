# UI Sprites — `game/assets/sprites/ui/`

Interface accents and background elements in the neon aesthetic.
All crafted in Aseprite 1.3 (`.aseprite` sources kept), RGBA.

Generation:
- `gen_ui.lua` — signal bars, panel corner, button glow, progress tick
- `gen_bg.lua` — seamless circuit tile
Run: `aseprite -b --script-param outdir=<dir> --script gen_ui.lua`

## Assets

| File | Size | Frames | Purpose |
|------|------|--------|---------|
| `signal_bars` | 48×32 | 5 | Signal strength 0–4 bars. Cyan active bars with white top edge; dim outline for inactive. For link-budget / signal-quality displays. |
| `panel_corner` | 24×24 | 1 | Neon L-corner accent (cyan with white core, gold joint dot). Rotate 90°/180°/270° in-game for the other three corners. |
| `button_glow` | 64×32 | 3 | Button states: 0=normal (dim outline), 1=hover (bright + outer glow), 2=pressed (gold tint fill). |
| `progress_tick` | 16×16 | 2 | Progress diamonds: 0=empty outline, 1=filled cyan. Tile horizontally for step indicators. |
| `circuit_tile` | 128×128 | 1 | Seamless tiling background: dark ink base with faint cyan circuit traces, via points, one gold accent. Drawn with wraparound so edges tile cleanly. |
| `board_bg` | 1200×1800 | 1 | (pre-existing) Full board background |
| `node_ring_*` | 80×80 | 1 | (pre-existing) Node selection rings |
| `pill_ring_*` | — | 1 | (pre-existing) Pill selection rings |

## Wiring status

These are **assets ready, not yet wired** — the game currently draws UI
accents procedurally (gizmos, Bevy UI styles). Recommended integration
points for the next pass:
- `signal_bars`: link-budget readout in the ledger panel.
- `circuit_tile`: `ImageNode` with `ImageScaleMode::Tiled` behind dialogue
  panels or as an alternate board background.
- `button_glow`: swap in for the results-screen buttons.
- `panel_corner`: dialogue panel corners (4 rotations).
- `progress_tick`: track progress in the companion-select screen.

## Conventions

- Numbered frames: `<name>_<i>.png`.
- `.aseprite` source kept for every asset.
- 9-slice or rotation preferred over baking variants: `panel_corner` is
  one corner, rotate for the others.
