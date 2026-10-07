# Lattice — Ethernet

**Role:** Companion for the Ethernet / copper LAN track (levels 31–40).
Accent: networking amber `#eab308`. Tagline: *"Ethernet / copper LAN."*
Picker hook: *"No decibels here. Just physics and paperwork."*

## Personality and voice

Dry, exact, and quietly delighted by tidy cable dressing. Her dialogue
bank (`game/assets/dialogue/lattice_en.json`) is structured-cabling
humor — pinouts, crosstalk, link lights. On a clean splice:

> "Ooh, a punch-down with zero crosstalk? Be still my collision domain,
> senpai."

Her first briefing lays down *"Lattice's first law: copper has a
distance limit, not just a speed limit."*

## Appearance

From her canonical portrait (`art/companions/lattice_portrait.jpg`):
blue hair in patch-cable braids with RJ45-clip hair pins, amber eyes,
and a white-and-blue grid-patterned jacket printed with LAN topology,
a holo display reading CONNECTION STABLE at her fingertips.
`docs/ART_STYLE.md` fixes the silhouette: RJ45-clip hair pin,
grid-patterned jacket, patch-cable braids.

## Track

Ten Ethernet levels (`game/assets/levels/ethernet1`–`ethernet10`): the
only track with **no dB budget at all**. Winning means satisfying every
structured-cabling constraint — 100 m segment limits, switch placement,
PoE power budgets, bandwidth — instead of landing a level in a window.

## Assets

- Portrait: `game/assets/art/companions/lattice_portrait.jpg` and
  `assets/art/companions/lattice_portrait.jpg` (1024×1536). No separate
  mature portrait — her mature art lives in the sprite sheet.
- Sprite sheets: `game/assets/sprites/lattice/lattice_sheet.png`
  (256×384), `lattice_sheet_fullbody.png` (384×1152), and the active
  `lattice_sheet_fullbody_mature.png` (1152×3456), each with its
  `.aseprite` source and folder READMEs. All six moods × 4 frames.
- Picker: `game/assets/sprites/picker/lattice_select_{0..5}.png` +
  strip + source.
- Expression portraits: `assets/art/expressions/` — alarmed, blush,
  celebrate, pout, wink.
- Idle/profile: `assets/art/companions/lattice_animated.gif`,
  `_animated_base.gif`, `_animated_mature.gif`, `_animated_mature.png`,
  `_profile_64.png`.
- Dialogue: `game/assets/dialogue/lattice_en.json` (full bank).
- Music: `game/assets/music/themes/lattice.ogg`,
  `game/assets/music/ambience/lattice_hum.ogg`. Footage under
  `game/assets/footage/e1l*/`.
