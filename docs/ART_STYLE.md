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
- **Companion accent colors** (one per access technology, so each reads as
  a distinct "overlay," not part of the technical schematic):
  - **Séraphine (Fiber):** magenta/cyan (`#ff6fae`, `#6fd6ff`)
  - **Ondine (Coax):** copper/signal-teal (`#b87333`, `#2ec4b6`)
  - **Linka (Mobile):** violet/electric-blue (`#7c3aed`, `#38bdf8`)
  - **Lattice (Ethernet):** networking-blue/amber (`#2563eb`, `#eab308`)
- **Outage/hazard state:** warm red/orange alarm accents (`#ff4d4d`,
  `#ffb84d`) reserved only for active faults, so players immediately
  recognize "something is wrong" without reading text.

## Companion Roster

Four anime AI-hologram companions live in the player's tablet/OTDR device —
one per access technology the game teaches. All four share the same sprite
sheet contract (6 mood rows × 4 frames each, 64×64 per frame, 256×384 total)
and the same expression-forward, hands/gesture-forward acting style — most
"acting" should read from face + one raised hand, since 64px doesn't support
fine full-body posing. Mood rows are identical across companions so the
gameplay code (`game/src/waifu/sprite.rs`) can treat them uniformly:

1. Idle (gentle bob/blink loop)
2. Blush (reacts to compliments / clean splices)
3. Wink (offers a hint)
4. Pout (reacts to a messy splice)
5. Celebrate (level win)
6. Alarmed (outage event)

| Companion | Tech | Silhouette | Sprite sheet |
|---|---|---|---|
| **Séraphine** | Fiber | Ponytail with a fiber-optic-cable motif braid, visor/glasses with a subtle light-pipe glow, utility-vest-over-hoodie (nods to field-tech PPE without being literal safety gear) | `game/assets/sprites/seraphine/seraphine_sheet.png` |
| **Ondine** | Coax | Coiled coax-cable ponytail, retro CATV-style headset with a round numbered channel dial, F-connector necklace | `game/assets/sprites/ondine/ondine_sheet.png` |
| **Linka** | Mobile | Antenna-fin ponytail, signal-bar hair clips, holographic visor with a signal-bar HUD overlay | `game/assets/sprites/linka/linka_sheet.png` |
| **Lattice** | Ethernet | RJ45-clip hair pin, grid-patterned jacket, patch-cable braids | `game/assets/sprites/lattice/lattice_sheet.png` |

Each companion's outfit and color story stay stable across all promotional
art, store listing screenshots, and in-game sprites for brand consistency.
Source portraits and animated README profile GIFs live in
`assets/art/companions/`; see [`docs/CREDITS.md`](CREDITS.md) for
AI-generation tooling and license attribution.

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

## Asset Generation Pipeline (this repo)

- `tools/gen_placeholder_art.py` generates palette-correct 64×64 placeholder
  sprites (solid silhouettes + mood-tinted overlays) for OSP component icons
  and any companion not yet illustrated — useful for quick iteration on new
  levels/components without waiting on art.
- `tools/gen_companion_art.py` builds each companion's real in-engine sprite
  sheet and README animated profile GIF from a single AI-generated anime
  portrait per character (`assets/art/companions/<name>_portrait.jpg`): it
  crops the face/shoulders, downsamples to a 64×64 pixel-art base frame, then
  derives all 6 mood rows via tint/brightness treatment + a small per-frame
  vertical bob (the same animation trick `gen_placeholder_art.py` uses, just
  applied to real art instead of primitive shapes).
- `tools/gen_app_icon.py` builds the Android launcher icon set (legacy
  mipmap densities + adaptive icon foreground/background) from the
  four-companion group portrait at `assets/art/companions/app_icon_group.jpg`.

See `docs/CREDITS.md` for the AI-generation tooling and licensing checklist
covering all companion art.
