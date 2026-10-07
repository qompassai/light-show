# Linka — Wireless

**Role:** Companion for the mobile / cellular RF track (levels 21–30).
Accent: electric blue `#38bdf8`. Tagline: *"Mobile / cellular RF."*
Picker hook: *"Distance always wins — unless you regenerate."*

## Personality and voice

Bright, fast, and geometry-obsessed: her enemy is never the equipment,
it's the distance. Her dialogue bank (`game/assets/dialogue/linka_en.json`)
speaks fluent cellular — RSRP, SINR, handoffs, baseband. On a clean
splice:

> "Ooh, a clean handoff with zero dropped calls? Be still my baseband,
> senpai."

Her first briefing states the track's core lesson: *"distance is the
enemy, and amplifiers can't beat geometry... An amplifier just shouts
the wreckage louder."*

## Appearance

From her canonical portrait (`art/linka/companions/linka_portrait.jpg`):
purple hair with an antenna-fin side ponytail, signal-bar hair clips,
purple eyes, a holographic visor with a signal-bar HUD overlay, and a
white-and-blue tech jacket patched with CELL LINK and LINKA insignia.
`docs/ART_STYLE.md` fixes the silhouette: antenna-fin ponytail,
signal-bar hair clips, holographic visor.

## Track

Ten wireless levels (`game/assets/levels/wireless1`–`wireless10`):
free-space path loss in dBm (RSSI), hop geometry, antennas, and
regenerative repeaters that retransmit at their own power instead of
amplifying noise. Interference is her outage hazard.

## Assets

- Portrait: `game/assets/art/linka/companions/linka_portrait.jpg` and
  `assets/art/linka/companions/linka_portrait.jpg` (1024×1536). No separate
  mature portrait — her mature art lives in the sprite sheet.
- Sprite sheets: `game/assets/sprites/linka/linka_sheet.png` (256×384),
  `linka_sheet_fullbody.png` (384×1152), and the active
  `linka_sheet_fullbody_mature.png` (1152×3456), each with its
  `.aseprite` source and folder READMEs. All six moods × 4 frames.
- Picker: `game/assets/sprites/picker/linka_select_{0..5}.png` +
  strip + source.
- Expression portraits: `assets/art/linka/expressions/` — alarmed, blush,
  celebrate, pout, wink.
- Idle/profile: `assets/art/linka/companions/linka_animated_base.gif`,
  `linka_profile_64.png`.
- Dialogue: `game/assets/dialogue/linka_en.json` (full bank).
- Music: `game/assets/music/themes/linka.ogg`,
  `game/assets/music/ambience/linka_hum.ogg`. Footage under
  `game/assets/footage/` (wireless track folders).
