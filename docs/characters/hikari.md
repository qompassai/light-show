# Hikari — Field Buildout

**Role:** Specialist companion for the FTTH/OSP field-buildout track
(levels 61–70), gamified from TDS's BxE field-portal resources (`bxe/`
— see `docs/TDS_GAMIFICATION.md`). Unlock: BLASTPROCESSING or Konami
code. Mapped origin: Japan. Accent: lime `#99ff33`. Tagline:
*"FTTH/OSP field buildout."* Picker hook: *"Measure twice, splice
once."*

## Personality and voice

A field tech's field tech: methodical, cleanliness-obsessed, and
callback-averse — a dirty connector is a return trip. Her dialogue
bank is the inline bank in `game/src/waifu/dialogue.rs` (greeting and
level-win lines only so far):

> "Hikari here. Measure twice, splice once — that's the field rule."

Her first briefing (`hikari1_first_day.json`) sends her out solo in
Setagaya and repeats her mentor's rule: *"a dirty connector is a
callback waiting to happen. Pick a clean one."*

## Appearance

From her shipped portrait (`art/hikari/companions/hikari_portrait.jpg`):
black hair in a high ponytail threaded with glowing fiber strands,
brown eyes, clear safety glasses, and an orange hi-vis OSP vest
(patched HIKARI — OSP FIBER TECH) over a dark shirt, holding a small
open splicer/test unit with a coiled cable over her shoulder, against
the line's magenta backdrop. (The art pipeline's keeper record
describes a beanie and tool belt; the shipped JPG has the ponytail
and vest — noted in the [index](README.md).)

## Track

Ten field levels (`game/assets/levels/hikari1`–`hikari10`):
address-to-diagnostics workflows against the callback timer, MST/tap
plans (assign homes to multiport terminals, mind port counts and drop
lengths), certification runs, and hazards like the slow portal and
the backhoe. Design detail in `docs/OSP_DFN_GUIDE.md`.

## Assets

- Portraits: `game/assets/art/hikari/companions/hikari_portrait.jpg` +
  `hikari_portrait_mature.jpg`, mirrored under
  `assets/art/hikari/companions/` (1280×1920; the dialogue box loads the
  mature portrait by default).
- Sprite sheet: **missing** — the code loads
  `game/assets/sprites/hikari/hikari_sheet_fullbody.png`
  (`Companion::sprite_path`); no such file or folder exists.
- Picker frames: **missing** —
  `game/assets/sprites/picker/hikari_select_{0..5}.png` (latent; the
  select screen fields only the base four today).
- Expression portraits: none.
- Idle GIF / profile art: none in the repo.
- Dialogue: inline bank in `game/src/waifu/dialogue.rs` — greeting +
  level-win only; the other event keys have no Hikari lines.
- Music: `game/assets/music/themes/hikari.ogg`,
  `game/assets/music/ambience/hikari_hum.ogg`. Footage under
  `game/assets/footage/hikari*/`.
