# Aino — NOC Operations

**Role:** Specialist companion for the Nokia AMS network-operations
track (levels 51–60), gamified from TDS's AMS resources (`ams/` — see
`docs/TDS_GAMIFICATION.md`). Unlock: ABACABB or Konami code. Mapped
origin: Finland. Accent: amber-orange `#ffa600`. Tagline: *"Nokia AMS
network ops."* Picker hook: *"The alarms never lie. Learn to read
them."*

## Personality and voice

Terse, calm, and shift-lead disciplined: faults are information, and
panic is a process failure. Her dialogue bank is the inline bank in
`game/src/waifu/dialogue.rs` (greeting and level-win lines only so
far):

> "Aino, NOC shift lead. The alarms never lie — let's learn to read
> them."

Her first briefing (`aino1_first_shift.json`) opens at her desk:
*"Welcome to the Nokia NOC, Aino's desk. One alarm on the board: a
fiber cut. Click it to acknowledge — that's the whole job tonight."*

## Appearance

From her shipped portrait (`art/aino/companions/aino_portrait.jpg`):
platinum-blonde hair in a long side braid, blue eyes, light freckles,
a NOC headset with boom mic, and a dark navy operations uniform with
cyan circuit accents, arms crossed, against the line's magenta
backdrop. (The art pipeline's keeper record describes amber accents
and a Finnish flag pin; the shipped JPG's accents are cyan and no pin
is visible — noted in the [index](README.md).)

## Track

Ten NOC levels (`game/assets/levels/aino1`–`aino10`): alarm triage on
the NOC console — a live alarm list across all four media with an
ack/dispatch/clear workflow and severity that ages while you work.
Bulk acknowledgements, NBI/SOAP queries, supervision, and maintenance
windows. Design detail in `docs/NOC_CONSOLE.md`.

## Assets

- Portraits: `game/assets/art/aino/companions/aino_portrait.jpg` +
  `aino_portrait_mature.jpg`, mirrored under `assets/art/aino/companions/`
  (1280×1920; the dialogue box loads the mature portrait by default).
- Sprite sheet: **missing** — the code loads
  `game/assets/sprites/aino/aino_sheet_fullbody.png`
  (`Companion::sprite_path`); no such file or folder exists.
- Picker frames: **missing** —
  `game/assets/sprites/picker/aino_select_{0..5}.png` (latent; the
  select screen fields only the base four today).
- Expression portraits: none.
- Idle GIF / profile art: none in the repo.
- Dialogue: inline bank in `game/src/waifu/dialogue.rs` — greeting +
  level-win only; the other event keys have no Aino lines.
- Music: `game/assets/music/themes/aino.ogg`,
  `game/assets/music/ambience/aino_hum.ogg`. Footage under
  `game/assets/footage/aino*/`.
