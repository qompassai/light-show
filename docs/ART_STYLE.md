# Art Direction — "Neon-Circuit Anime OSP"

Light Show's visual voice is a collision of two worlds: a painted cyberpunk
title card (night city, neon signage) and crisp hand-pixeled game art drawn
in Aseprite to live inside that card's color story. The puzzle board is a
"technical schematic" (think OTDR trace meets subway map) dressed in neon;
the companions are full-body anime sprites with expression-forward acting.

## The Keeper Title Card

The game's visual authority is a single AI-generated, art-directed title
painting: **LIGHT SHOW** in angular cyan-to-gold letterforms inside a
circuit-bracket frame with hex nodes, the four companions at the corners with
signal streams visibly originating at their devices and terminating at the
frame's corner nodes, a full moon over a sprawling night city. Every UI
palette decision flows from this painting — if a color doesn't appear in the
keeper, it doesn't appear in the game.

In-game title treatment:
- The full painting sits behind the main menu as a backdrop.
- The logo is re-drawn as **hand-pixeled Aseprite lettering** in the painting's
  cyan-to-gold gradient with the circuit-bracket frame and hex-node corners.
- Menu and picker text uses the **neon-circuit treatment**: a bright core
  glyph with a two-ring luminous halo — eight glow copies at 1 px (~55%
  alpha) plus eight at 2 px (~25% alpha). The alpha falloff is what sells
  "glow" instead of "outline" (see `game/src/ui/neon.rs`). The glyphs stay
  monospace Monaspace Neon by design — the halo softens them; it doesn't replace
  them.

## Palette

Sampled from the keeper artwork:

- **Neon cyan** `#6ff2ff` — primary glow, fiber strands, interactive elements.
- **Neon gold** `#ffd166` — accents, endpoints, "SERVICE RESTORED".
- **Night-city dark** `#0a0e1a` — board ground, text ink, nameplates.
- **Dim slate-cyan** — secondary text.
- **Outage/hazard:** warm red/orange alarm accents (`#ff4d4d`, `#ffb84d`),
  reserved only for active faults so "something is wrong" reads instantly.
- **Companion accents** (one per discipline, so each reads as a distinct
  identity against the schematic):
  - **Séraphine (Fiber):** `#ff6fae` (magenta-pink)
  - **Ondine (Coax):** `#2ec4b6` (signal-teal)
  - **Linka (Wireless):** `#38bdf8` (electric blue)
  - **Lattice (Ethernet):** `#eab308` (networking amber)

## Companion Sprites

All four companions share one sprite contract: **6 mood rows × 4 frames**
(24 cells; cell index = mood row × 4 + frame). Two art tiers follow it:

- **Mature set (active in game): 288×576 per frame**, 1152×3456 sheets
  (`<name>_sheet_fullbody_mature.png`, displayed at scale 1.0). The
  2026-10 redesign: new 21-year-old models drawn as full illustrations
  per expression, converted through the shared-palette pixel pipeline
  (96 k-means colors + blush rose per character). Frames within a row
  are breathing-bob offsets of the locked expression art.
- **Base set (archived): 96×192 per frame**, 384×1152 sheets
  (`<name>_sheet_fullbody.png`, formerly displayed at 3.0×). Kept in
  the tree as the standard set for the Play flavor.

The mood rows are:

1. Idle (gentle bob/blink loop)
2. Blush (reacts to compliments / clean splices)
3. Wink (offers a hint)
4. Pout (reacts to a messy splice)
5. Celebrate (level win)
6. Alarmed (outage event)

Mood rows are identical across companions so `game/src/waifu/sprite.rs`
atlas-indexes them uniformly. Most "acting" reads from face + one raised
hand — the expression-forward, gesture-forward house style.

| Companion | Tech | Silhouette |
|---|---|---|
| **Séraphine** | Fiber | Ponytail with a fiber-optic-cable motif braid, visor/glasses with a subtle light-pipe glow, utility-vest-over-hoodie (nods to field-tech PPE without being literal safety gear) |
| **Ondine** | Coax | Coiled coax-cable ponytail, retro CATV-style headset with a round numbered channel dial, F-connector necklace |
| **Linka** | Wireless | Antenna-fin ponytail, signal-bar hair clips, holographic visor with a signal-bar HUD overlay |
| **Lattice** | Ethernet | RJ45-clip hair pin, grid-patterned jacket, patch-cable braids |

Each companion's outfit and color story stay stable across all promotional
art, store listing screenshots, and in-game sprites for brand consistency.
Source portraits and animated README profile GIFs live in
`assets/art/<name>/companions/` (one folder per companion); see [`docs/CREDITS.md`](CREDITS.md) for
AI-generation tooling and license attribution.

## Companion Picker Presentation

The picker cards use **dark silhouettes** derived from the companions' own
idle frames (the girls are never redrawn for the picker — the silhouette is
the same character, unlit), presented over the dimmed keeper artwork:

- 6-frame Aseprite animation strips (`game/assets/sprites/picker/<name>_select_{0..5}.png`,
  160×256 per frame, 0.12 s per frame), displayed at 96×144.
- The animation plays only while its card is hovered or pressed; at rest
  every card sits on frame 0.
- Each companion gets a **discipline-specific highlight animation** in her
  accent color:
  - **Séraphine:** light pulses racing along fiber strands.
  - **Ondine:** expanding RF wavefronts.
  - **Linka:** radiating broadcast arcs.
  - **Lattice:** packet blips traveling angular copper paths.
- A glowing accent outline marks highlight/selection.
- Card text (name, discipline, hook line) uses the neon-circuit treatment.

## Board & Component Iconography

Every component gets a hand-pixeled **Aseprite icon** (source `.aseprite`
kept next to the exported `.png`), replacing the old text-only pills:

| Component | Icon |
|---|---|
| Fusion splice | Two fiber ends meeting inside a splicer clamp |
| Mechanical splice | Two fiber ends inside a gel-filled sleeve |
| UPC connector | Flat-face circular connector tip |
| APC connector | Angled-face circular connector tip (green-keyed, matching real APC color convention) |
| Splitter | Single line fanning into N lines inside a rounded box |
| Macrobend hazard | Kinked line with a small warning glyph |
| Amplifier | Coax line amplifier housing |
| Tap | Directional-coupler tap |
| Coax span | Coaxial cable segment |
| Wireless hop | Radiating antenna link |
| Repeater | Regenerative repeater node |
| Ethernet run | Twisted-pair copper run |
| Switch | Network switch |

Board furniture, all Aseprite-authored:
- **Node rings** in four flavors: gold (endpoints), cyan (standard),
  tap, site.
- **Pill rings**: normal and selected states for component placement slots.
- **Board backdrop** (`sprites/ui/board_bg.png`): subtle dark circuit/fiber
  texture — present but never competing with the puzzle.
- **Nameplates**: dark semi-transparent plates behind node labels so long
  labels ("Splice Enclosure 14+00") read cleanly where they cross rings or
  pills. Labels stagger above/below the node to avoid label-on-label
  collision.
- **Effects**: `pulse_dot` (signal-flow pulses traveling live routes),
  `storm_streak` (outage weather), `win_ring` (expanding neon ring behind
  "SERVICE RESTORED" on the results screen).

## Asset Pipeline (this repo)

- Aseprite is the source of truth for game art: every sprite ships as a
  `.aseprite` source plus its exported `.png`, kept side by side in
  `game/assets/sprites/`.
- `scripts/build_fullbody_aseprite.lua` builds companion full-body sheets;
  `scripts/verify_roundtrip.lua` gates pixel-identical round-trips.
- `tools/gen_placeholder_art.py` generates palette-correct placeholder
  sprites for quick iteration on new levels/components without waiting on
  art.
- `tools/gen_companion_art.py` builds each companion's real in-engine sprite
  sheet and README animated profile GIF from a single AI-generated anime
  portrait per character (`assets/art/<name>/companions/<name>_portrait.jpg`).
- `tools/gen_app_icon.py` builds the Android launcher icon set (legacy
  mipmap densities + adaptive icon foreground/background) from the
  four-companion group portrait at `assets/art/companions/app_icon_group.jpg`.

See `docs/CREDITS.md` for the AI-generation tooling and licensing checklist
covering all companion art.

## Honest Rendering Notes

- The UI type system pairs Monaspace Neon (display: headings, labels, scores)
  with Inter (body: briefings, dialogs, buttons). The neon-circuit glow
  softens the display face; it does not make it a smooth font.
- Verification screenshots are taken under Xvfb with software rendering.
  That validates layout, art integration, and the full playthrough path; it
  is not physical-GPU or real-device acceptance.
