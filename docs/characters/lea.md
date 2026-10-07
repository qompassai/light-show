# Léa — Exam Prep

**Role:** Specialist companion for the WA 09 Telecom Admin prep track
(levels 71–80), gamified from TDS's Training Academy resources (`ta/`
— see `docs/TDS_GAMIFICATION.md` and `docs/LEA_QUIZ_IMPORT.md`).
Unlock: TRIFORCE or Konami code. Mapped origin: Switzerland
(ITU-inspired). Accent: deep red `#a63340`. Tagline: *"WA 09 Telecom
Admin prep."* Picker hook: *"Know the code. Pass the test. Own the
network."*

## Personality and voice

A warm, exacting study partner — part librarian, part exam coach. She
teaches by retrieval: every level is a quiz, and every answer comes
with its teaching moment. Her dialogue bank is the inline bank in
`game/src/waifu/dialogue.rs` (greeting and level-win lines only so
far):

> "Bonjour! I'm Léa. Know the code, pass the test, own the network."

Her study hall opens in `lea1_tutorial.json`: *"Léa's study hall is
open. Ten questions on NEC Articles 90, 100, and 110 — the general
requirements every telecom tech needs cold. Score 70% to move on."*

## Appearance

From her canonical portrait (`art/lea/companions/lea_portrait.jpg`, pulled
into the repo 2026-10-06 from the locked "librarian look" keeper):
chestnut hair in a low bun, round librarian glasses, hazel-amber eyes,
a burgundy cable-knit cardigan over a cream blouse with a Swiss flag
pin, holding a telecoms-quiz tablet and an open, heavily annotated
*Telecommunications Exam Prep* book.

## Track

Ten quiz-boss levels (`game/assets/levels/lea1`–`lea10`) imported from
real exam material: NEC general requirements, grounding and bonding
(Art. 250), wiring methods (300–398), hazardous locations (500–516),
special conditions (705–780), communications systems (800–830),
theory and calculations, Washington law (RCW 19.28) and admin code
(WAC 296-46B) as a separately scored track, and a full mock exam as
the final boss.

## Assets

- Portrait: `game/assets/art/lea/companions/lea_portrait.jpg` and
  `assets/art/lea/companions/lea_portrait.jpg` (1280×1920) — **added
  2026-10-06**. Before that, `Companion::portrait_path` pointed at
  this path as a documented placeholder with no file behind it; the
  locked keeper generation had never been pulled in. No mature
  portrait exists, and the code does not reference one for Léa.
- Sprite sheet: **missing** — the code loads
  `game/assets/sprites/lea/lea_sheet_fullbody.png`
  (`Companion::sprite_path`); no such file or folder exists.
- Picker frames: **missing** —
  `game/assets/sprites/picker/lea_select_{0..5}.png` (latent; the
  select screen fields only the base four today).
- Expression portraits: none.
- Idle GIF / profile art: none in the repo.
- Dialogue: inline bank in `game/src/waifu/dialogue.rs` — greeting +
  level-win only; the other event keys have no Léa lines.
- Music: `game/assets/music/themes/lea.ogg`,
  `game/assets/music/ambience/lea_hum.ogg`. Footage under
  `game/assets/footage/lea*/`.
