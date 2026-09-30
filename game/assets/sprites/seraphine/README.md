# Séraphine — Full-Body Sprite Sheet

Fiber-optic splicing companion. Magenta fiber-braid ponytail, cyan
light-pipe visor, white/orange field jacket.

## Sheet layout

`seraphine_sheet_fullbody.png` — 384×1152, 4 columns × 6 rows of 96×192
cells. `seraphine_sheet_fullbody.aseprite` is the editable project (24
frames @ 180 ms, one tag per mood row).

## Mood → row → animation

| Row | Mood      | Frames | Animation                              |
|-----|-----------|--------|----------------------------------------|
| 0   | Idle      | 1–4    | Breathing bob + blink (frame 3)        |
| 1   | Blush     | 5–8    | Idle + cheek blush overlay             |
| 2   | Wink      | 9–12   | Left-eye wink + sparkle, frames 2–3    |
| 3   | Pout      | 13–16  | Frown + half-lidded eyes               |
| 4   | Celebrate | 17–20  | **Wink + spin** — frame 4 is the       |
|     |           |        | horizontally flipped pose, reads as a  |
|     |           |        | twirl mid-celebration                  |
| 5   | Alarmed   | 21–24  | Red wash + wide eyes + `o` mouth       |

## In-game triggers

- Celebrate: level win, clean fusion splice
- Pout: mechanical splice (high loss)
- Blush: (reserved) Alarmed: outage start
- Wink: outage resolved

## Notes

- 6 px mood bar at cell bottom (magenta `#ff4dd2`; red for alarmed).
- Visor characters use lash-line-only eye ops (no skin fill under visor).
