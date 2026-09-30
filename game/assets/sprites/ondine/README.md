# Ondine — Full-Body Sprite Sheet

Coax / broadband RF companion. Coax-coil ponytail, CATV headset with
channel dial, F-connector necklace, white/orange field jacket.

## Sheet layout

`ondine_sheet_fullbody.png` — 384×1152, 4 columns × 6 rows of 96×192
cells. `ondine_sheet_fullbody.aseprite` is the editable project (24
frames @ 180 ms, one tag per mood row).

## Mood → row → animation

| Row | Mood      | Frames | Animation                              |
|-----|-----------|--------|----------------------------------------|
| 0   | Idle      | 1–4    | Breathing bob + blink (frame 3)        |
| 1   | Blush     | 5–8    | Idle + cheek blush overlay             |
| 2   | Wink      | 9–12   | Left-eye wink + sparkle, frames 2–3    |
| 3   | Pout      | 13–16  | Frown + half-lidded eyes               |
| 4   | Celebrate | 17–20  | **Single modest hop** — full-body      |
|     |           |        | rises 6 px on frames 2–3, no spin      |
|     |           |        | (headset stays put, as it should)      |
| 5   | Alarmed   | 21–24  | Red wash + wide eyes + `o` mouth       |

## In-game triggers

- Celebrate: level win, clean connector torque
- Pout: loose connector (ingress risk)
- Blush: (reserved) Alarmed: outage start
- Wink: outage resolved

## Notes

- 6 px mood bar at cell bottom (teal `#2ec4b6`; red for alarmed).
- Right eye is half-hidden by the headset: blink/wink use lash-line-only
  ops there (no skin fill) to avoid smudging the dial.
