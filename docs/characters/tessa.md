# Tessa — The Warehouse, Field Side

**Role:** Co-host of the Warehouse, the game's shop-and-study hub. She
runs the field side: use cases, war stories, and the crawlspace work
that digs out half the dead parts players trade in for **cores**. Host
record: `game/src/warehouse/mod.rs` (`HOSTS`).

## Personality and voice

Warm, funny, and field-worn — the counterweight to Bianca's manual.
Bianca tells you what the book says; Tessa tells you when you'll
actually reach for it. Her greeting, verbatim from
`warehouse/mod.rs`:

> "Howdy, partner. Bianca'll tell you what the book says. I'll tell
> you when you actually reach for it, usually at the bottom of a
> crawlspace — which is also where I dig out half the dead parts
> you'll be trading us."

In the Warehouse quiz she takes the wrong answers: her explanations
give the field consequence first, then the answer — the lesson lands
because it sounds like a callback she has already lived through.

## Appearance

The `Host.look` record in code: *"Short. Brown ponytail, cowgirl
boots, jeans with more holes than a punch-down block."* Her shipped
portrait (`art/tessa/companions/tessa_portrait.jpg`, 1280×1920) matches the
ponytail and the jeans, and adds what the record doesn't mention:
freckles, a red plaid flannel over a black tee, and a belt-mounted
tester, against the Warehouse shelves.

## Assets

- Portrait: `game/assets/art/tessa/companions/tessa_portrait.jpg` and
  `assets/art/tessa/companions/tessa_portrait.jpg`. (File mode normalized
  to 0644 on 2026-10-06 to match the other portraits.)
- Shared backdrop: `game/assets/art/warehouse_backdrop.jpg`.
- Sprite sheet / picker frames / expression portraits / idle GIF:
  none, and no code path expects them — the Warehouse screen renders
  hosts as static portraits over a fallback panel (the screen is
  explicitly built to tolerate the portrait missing:
  `game/src/states/warehouse.rs`).
- Dialogue: no JSON bank; her lines live in
  `game/src/warehouse/mod.rs` (greeting + quiz wrong-answer
  explanations). She has no levels, footage, or music theme — the
  Warehouse is her whole stage.
