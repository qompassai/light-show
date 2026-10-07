# Bianca — The Warehouse, Test Bench

**Role:** Co-host of the Warehouse, the game's shop-and-study hub. She
runs the test-bench side: the tool shelf, the manuals, and the grading
of the dead parts players haul back for **cores** — the currency the
shop trades in. Host record: `game/src/warehouse/mod.rs` (`HOSTS`).

## Personality and voice

Formidable, exact, and proud of it — the manual is not a suggestion,
and she has read all of them. Twice. Her greeting, verbatim from
`warehouse/mod.rs`:

> "Welcome to the Warehouse. Bring me your dead parts — every core you
> haul in gets inspected, graded, and credited, no exceptions.
> Everything on this shelf comes with a manual, and yes, I have read
> all of them. Twice. Like a good isekai protagonist reads the system
> menu."

In the Warehouse quiz she takes the correct answers: her explanations
are the manual facts, each cited to its section of the Klein Tools VDV
Scout Pro 3 instruction manual (VDV501-851) — the v1 shelf's single
tool, whose facts she vouches for.

## Appearance

The `Host.look` record in code: *"Tall. Long black hair in a blunt
hime cut, grey eyes, beauty mark under the left eye. Black tee: 'COME
AT ME PRINCESS'."* Her shipped portrait
(`art/companions/bianca_portrait.jpg`, 1376×1824) matches it: hime
cut, grey eyes, the beauty mark, and the tee, against the Warehouse's
own shelves of cable spools and test gear.

## Assets

- Portrait: `game/assets/art/companions/bianca_portrait.jpg` and
  `assets/art/companions/bianca_portrait.jpg`. (File mode normalized
  to 0644 on 2026-10-06 to match the other portraits.)
- Shared backdrop: `game/assets/art/warehouse_backdrop.jpg`.
- Sprite sheet / picker frames / expression portraits / idle GIF:
  none, and no code path expects them — the Warehouse screen renders
  hosts as static portraits over a fallback panel (the screen is
  explicitly built to tolerate the portrait missing:
  `game/src/states/warehouse.rs`).
- Dialogue: no JSON bank; her lines live in
  `game/src/warehouse/mod.rs` (greeting + quiz correct-answer
  explanations). She has no levels, footage, or music theme — the
  Warehouse is her whole stage.
