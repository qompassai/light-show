# Lattice — Full-Body Sprite Sheet

Ethernet / copper LAN companion. Blue hair with patch-cable braids ending
in RJ45 connectors, grid-pattern jacket.

## Sheet layout

`lattice_sheet_fullbody.png` — 384×1152, 4 columns × 6 rows of 96×192
cells. `lattice_sheet_fullbody.aseprite` is the editable project (24
frames @ 180 ms, one tag per mood row).

## Mood → row → animation

| Row | Mood      | Frames | Animation                              |
|-----|-----------|--------|----------------------------------------|
| 0   | Idle      | 1–4    | Breathing bob + blink (frame 3)        |
| 1   | Blush     | 5–8    | Idle + cheek blush overlay             |
| 2   | Wink      | 9–12   | Left-eye wink + sparkle, frames 2–3    |
| 3   | Pout      | 13–16  | Frown + half-lidded eyes               |
| 4   | Celebrate | 17–20  | **Fist-pump rhythm** — sharp upward   |
|     |           |        | arm pump on frames 2 and 4, the most   |
|     |           |        | staccato celebrate of the four         |
| 5   | Alarmed   | 21–24  | Red wash + wide eyes + `o` mouth       |

## In-game triggers

- Celebrate: level win, clean punch-down
- Pout: high attenuation (bad termination)
- Blush: (reserved) Alarmed: outage start
- Wink: outage resolved

## Notes

- 6 px mood bar at cell bottom (blue `#4cc9f0`; red for alarmed).
- Non-visor: eyes use skin-fill close ops; the saturation-filtered sampler
  keeps blue hair out of the fill.
