# Clara — Provisioning

**Role:** Specialist companion for the Calix CMS provisioning track
(levels 41–50), gamified from TDS's CMS resources (`cms/` — see
`docs/TDS_GAMIFICATION.md`). Unlock: JUSTINBAILEY or Konami code.
Mapped origin: USA. Accent: cyan `#00d9d9`. Tagline: *"Calix CMS
provisioning."* Picker hook: *"Provision right the first time. Every
ONT counts."*

## Personality and voice

Brisk and process-first — a turn-up is a playbook, and the playbook is
right. Her dialogue bank is the inline bank in
`game/src/waifu/dialogue.rs` (greeting and level-win lines only so far):

> "Clara here. Let's get these ONTs provisioned right the first time."

Her first briefing (`clara1_provisioning.json`) teaches the field habit
that defines her track: *"Count the ports before you count the decibels
— an 8-port splitter cannot serve nine homes."*

## Appearance

From her shipped portrait (`art/clara/companions/clara_portrait.jpg`):
shoulder-length auburn waves, amber eyes, clear safety glasses, and a
grey-white provisioning uniform with gold-orange accents, holding a
holographic network tablet against the line's magenta backdrop.

*Provenance note:* the art pipeline's locked keeper record describes a
different look (dark blue-black bob, teal eyes, navy tech uniform with
cyan circuit accents). The shipped JPG does not match that description,
and the two repo copies of `clara_portrait_mature.jpg` are different
images from each other. Which generation is canonical is undecided —
flagged in the [index](README.md), untouched during the 2026-10-06
inventory.

## Track

Ten provisioning levels (`game/assets/levels/clara1`–`clara10`):
subscribers, service profiles, and split ratios — every ONT on a shared
PON must land inside its tier's optical window. Bulk turn-ups, MDU
buildings, worst-case path verification, and a night cutover finale.
Design detail in `docs/CALIX_PROVISIONING.md`.

## Assets

- Portraits: `game/assets/art/clara/companions/clara_portrait.jpg` +
  `clara_portrait_mature.jpg`, mirrored under `assets/art/clara/companions/`
  (1280×1920; the dialogue box loads the mature portrait by default).
- Sprite sheet: **missing** — the code loads
  `game/assets/sprites/clara/clara_sheet_fullbody.png`
  (`Companion::sprite_path`); no such file or folder exists.
- Picker frames: **missing** —
  `game/assets/sprites/picker/clara_select_{0..5}.png` (latent; the
  select screen fields only the base four today).
- Expression portraits: none.
- Idle GIF / profile art: none in the repo.
- Dialogue: inline bank in `game/src/waifu/dialogue.rs` — greeting +
  level-win only; the other event keys (clean/messy splice, fail, outage,
  hint) have no Clara lines.
- Music: `game/assets/music/themes/clara.ogg`,
  `game/assets/music/ambience/clara_hum.ogg`. Footage under
  `game/assets/footage/clara*/`.
