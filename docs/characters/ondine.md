# Ondine — Coax

**Role:** Companion for the coax / broadband RF track (levels 11–20).
Accent: signal-teal `#2ec4b6`. Tagline: *"Coax / broadband RF."*
Picker hook: *"Gain is easy. Balance is the job."*

## Personality and voice

Precise and sweep-minded; where Séraphine swoons, Ondine measures. Her
dialogue bank (`game/assets/dialogue/ondine_en.json`) runs on RF
vocabulary — torque, return loss, ingress, MER. On a clean splice:

> "Ooh, an F-connector torqued to spec? I felt that in my signal-to-noise
> ratio."

Her first briefing teaches her track's whole thesis in one line: *"a
cascade is a balancing act."*

## Appearance

From her canonical portrait (`art/ondine/companions/ondine_portrait.jpg`): dark
skin, silver-grey coiled braids gathered in a high ponytail — the coils
are coax, tipped with F-connectors — teal eyes, a retro CATV-style
headset with a round numbered channel dial, an F-connector necklace, and
a white-and-teal hoodie under a utility vest, tool belt at the hip.
`docs/ART_STYLE.md` fixes the silhouette: coiled coax-cable ponytail,
CATV headset, F-connector necklace.

## Track

Ten coax levels (`game/assets/levels/coax1`–`coax10`): dBmV cascades and
unity gain, amplifier staging, tap values, and ingress hunts. Coax is the
one medium where components *add* level, so her puzzles punish
over-driving (distortion) as hard as under-driving (snow).

## Assets

- Portrait: `game/assets/art/ondine/companions/ondine_portrait.jpg` and
  `assets/art/ondine/companions/ondine_portrait.jpg` (1024×1536). No separate
  mature portrait — her mature art lives in the sprite sheet. Her mature
  redesign went through extra pixel-cleanup passes before it locked;
  the shipped sheet below is the result.
- Sprite sheets: `game/assets/sprites/ondine/ondine_sheet.png` (256×384),
  `ondine_sheet_fullbody.png` (384×1152), and the active
  `ondine_sheet_fullbody_mature.png` (1152×3456), each with its
  `.aseprite` source and folder READMEs. All six moods × 4 frames.
- Picker: `game/assets/sprites/picker/ondine_select_{0..5}.png` +
  strip + source.
- Expression portraits: `assets/art/ondine/expressions/` — alarmed, blush,
  celebrate, pout, wink.
- Idle/profile: `assets/art/ondine/companions/ondine_animated_mature.gif`,
  `ondine_animated_mature.png` (an APNG
  despite the extension), `ondine_profile_64.png`.
- Dialogue: `game/assets/dialogue/ondine_en.json` (full bank).
- Music: `game/assets/music/themes/ondine.ogg`,
  `game/assets/music/ambience/ondine_hum.ogg`. Footage under
  `game/assets/footage/c1l*/`.
