# Linka — Full-Body Sprite Sheet

Mobile / cellular RF companion. Violet antenna-fin ponytail, signal-bar
hair clips, HUD visor, white/orange field jacket.

## Sheet layout

`linka_sheet_fullbody.png` — 384×1152, 4 columns × 6 rows of 96×192
cells. `linka_sheet_fullbody.aseprite` is the editable project (24
frames @ 180 ms, one tag per mood row).

## Mood → row → animation

| Row | Mood      | Frames | Animation                              |
|-----|-----------|--------|----------------------------------------|
| 0   | Idle      | 1–4    | **Bouncier breathing** + blink — Linka |
|     |           |        | idles at higher energy than the others |
| 1   | Blush     | 5–8    | Idle + cheek blush overlay             |
| 2   | Wink      | 9–12   | Left-eye wink + sparkle, frames 2–3    |
| 3   | Pout      | 13–16  | Frown + half-lidded eyes               |
| 4   | Celebrate | 17–20  | **Rapid double bounce** — two quick   |
|     |           |        | hops (frames 2 and 4), the most kinetic |
|     |           |        | celebrate of the four                  |
| 5   | Alarmed   | 21–24  | Red wash + wide eyes + `o` mouth       |

## In-game triggers

- Celebrate: level win, clean handoff
- Pout: high path loss (dropped signal)
- Blush: (reserved) Alarmed: outage start
- Wink: outage resolved

## Notes

- 6 px mood bar at cell bottom (violet `#9d4edd`; red for alarmed).
- Visor character: lash-line-only eye ops (no skin fill under visor).
- Celebrate `^^` eyes are drawn as arcs above the visor line.
