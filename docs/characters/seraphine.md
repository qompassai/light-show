# Séraphine — Fiber

**Role:** Companion for the fiber-optic OSP splicing track (levels 1–10).
The original companion; the game's default. Accent: magenta-pink `#ff6fae`.
Tagline: *"Fiber-optic OSP splicing."* Picker hook: *"Every decibel is
earned. Spend them wisely."*

## Personality and voice

Playful, teasing, and fluent in decibels — she narrates splice work like a
romance and means it as instruction. Her dialogue bank
(`game/assets/dialogue/seraphine_en.json`) keys lines to game events;
on a clean splice:

> "Ara ara~ a 0.05 dB fusion splice? Be still my heart, senpai."

The teaching runs underneath the flirtation: her fail lines name the actual
fault (starved receiver, overdriven photodiode) and the actual fix.

## Appearance

From her canonical portrait (`art/companions/seraphine_portrait.jpg`):
long magenta-pink hair in a high side ponytail with a glowing fiber-motif
braid, pink eyes, clear light-pipe visor glasses, and a white utility vest
over a hoodie with pink accents and a LUMENET patch. `docs/ART_STYLE.md`
fixes the silhouette: ponytail with fiber-optic-cable motif braid,
visor/glasses with a subtle light-pipe glow, utility-vest-over-hoodie.

## Track

Ten fiber levels in `game/assets/levels/` (`world1_level1.json`,
`world4_level1_outage.json`, `fiber3`–`fiber10`): link budgets in dBm,
fusion vs. mechanical splices, splitter tiers, protected routing, and
timed outage reroutes. Her first briefing opens *"Welcome to Splice
School."*

## Assets

- Portrait: `game/assets/art/companions/seraphine_portrait.jpg` and
  `assets/art/companions/seraphine_portrait.jpg` (1024×1536). No separate
  mature portrait — her mature art lives in the sprite sheet.
- Sprite sheets: `game/assets/sprites/seraphine/seraphine_sheet.png`
  (256×384), `seraphine_sheet_fullbody.png` (384×1152), and the active
  `seraphine_sheet_fullbody_mature.png` (1152×3456), each with its
  `.aseprite` source and folder READMEs. All six moods × 4 frames.
- Picker: `game/assets/sprites/picker/seraphine_select_{0..5}.png` +
  strip + source.
- Expression portraits: `assets/art/expressions/` — alarmed, blush,
  celebrate, pout, wink (painterly keepers; pulled into the repo
  2026-10-06, completing the mains' sets).
- Idle/profile: `assets/art/companions/seraphine_animated.gif` (byte-duplicate of the mature GIF; quarantined 2026-10-07),
  `_animated_base.gif`, `_animated_mature.gif`, `_animated_mature.png`,
  `_profile_64.png`.
- Dialogue: `game/assets/dialogue/seraphine_en.json` (full bank).
- Music: `game/assets/music/themes/seraphine.ogg`,
  `game/assets/music/ambience/seraphine_hum.ogg`. Footage under
  `game/assets/footage/f1l*/`.
