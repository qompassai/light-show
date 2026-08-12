# Art Direction — "64-bit Anime OSP"

Light Show uses a **64×64 pixel-art sprite standard** with an anime-inspired
character design language, layered over a clean "technical schematic" UI
(think OTDR trace meets subway map) for the puzzle board itself.

## Why 64×64

- Large enough to carry expressive anime faces (visible blush, distinct eye
  shapes, hair highlights) without going full high-res.
- Small enough to batch cheaply in Bevy sprite atlases on low/mid-range
  Android hardware — important since this targets `minSdk 26` phones, not
  just flagship devices.
- Matches the "retro-meets-anime" tone requested for the project: readable
  at a glance, charming rather than photorealistic.

## Palette

- **Board / OSP schematic:** cool blues and slate greys (`#0d0d1e`,
  `#1c2541`, `#5bc0eb`) — evokes OTDR trace screens and network diagrams.
- **Fiber light itself:** warm amber-to-white gradient (`#ffd166` →
  `#fffbe6`) so the "light traveling down the line" reads instantly against
  the cool board.
- **Séraphine:** magenta/cyan anime-hologram accent (`#ff6fae`, `#6fd6ff`)
  to visually mark her as a friendly overlay, not part of the technical
  schematic.
- **Outage/hazard state:** warm red/orange alarm accents (`#ff4d4d`,
  `#ffb84d`) reserved only for active faults, so players immediately
  recognize "something is wrong" without reading text.

## Séraphine Character Brief

- Anime AI-hologram companion who lives in the player's tablet/OTDR device.
- Silhouette: ponytail with a fiber-optic-cable motif braid, visor/glasses
  with a subtle light-pipe glow, utility-vest-over-hoodie look (nods to
  field-tech PPE without being literal safety gear).
- Sprite sheet layout: 6 mood rows × 4 frames each, 64×64 per frame
  (`assets/sprites/seraphine/seraphine_sheet.png`, 256×384 total):
  1. Idle (gentle bob/blink loop)
  2. Blush (reacts to compliments / clean splices)
  3. Wink (offers a hint)
  4. Pout (reacts to a messy splice)
  5. Celebrate (level win)
  6. Alarmed (outage event)
- Expression-forward, hands/gesture-forward — most "acting" should read from
  face + one raised hand, since 64px doesn't support fine full-body posing.
- Outfit and color story stay stable across all promotional art, store
  listing screenshots, and in-game sprites for brand consistency.

## Board & Component Iconography

Every OSP component gets a small (32×32, stamped at 2× onto the 64px grid)
glyph so the puzzle stays readable without constant label-reading:

| Component | Icon motif |
|---|---|
| OLT | Server-rack silhouette with light rays |
| ONT | House silhouette with light rays entering |
| Fusion splice | Two fiber ends meeting inside a splicer clamp |
| Mechanical splice | Two fiber ends inside a gel-filled sleeve |
| UPC connector | Flat-face circular connector tip |
| APC connector | Angled-face circular connector tip (green-keyed, matching real APC color convention) |
| Splitter | Single line fanning into N lines inside a rounded box |
| Macrobend hazard | Kinked line with a small warning glyph |
| Water intrusion hazard | Droplet glyph over a splice enclosure |

## Placeholder Asset Generation (this repo)

Until final hand-drawn/AI-art passes are approved, `tools/gen_placeholder_art.py`
generates palette-correct 64×64 placeholder sprites (solid silhouettes +
mood-tinted overlays) so the game is playable and screenshots are possible
before final art lands. Replace these before a production store release —
see `docs/CREDITS.md` for the licensing checklist on whatever final art
pipeline is used (in-house illustration, commissioned artist, or AI-assisted
with a human finishing pass).
