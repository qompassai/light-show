# Lattice — Full-Body Sprite Sheet (v3, 2026-10-03)

Ethernet / copper LAN companion. v3 redesign from the locked model
(`workspace/user/files/2434_9_7owg.jpg`): very long dark navy braids with
gold bands, braid ends tipped with clear RJ45 plugs, gold RJ45 hair clip,
brown eyes, white grid-pattern jacket with Taegeukgi patches on BOTH
shoulders, black shirt, black cargo pants, black combat boots.

## Sheet layout

`lattice_sheet_fullbody.png` — 384×1152, 4 columns × 6 rows of 96×192
cells. `lattice_sheet_fullbody.aseprite` is the editable project (24
frames @ 180 ms, one tag per mood row: idle, blush, wink, pout,
celebrate, alarmed). Rebuilt 2026-10-03 from six magenta-keyed v3
expression illustrations; each row is breathing-bob offsets of its
expression.

## Mood → row → animation

| Row | Mood      | Frames | Animation                              |
|-----|-----------|--------|----------------------------------------|
| 0   | Idle      | 1–4    | Breathing bob                          |
| 1   | Blush     | 5–8    | Idle + cheek blush                     |
| 2   | Wink      | 9–12   | Left-eye wink                          |
| 3   | Pout      | 13–16  | Frown + half-lidded eyes               |
| 4   | Celebrate | 17–20  | Fist pump, crimp tool in hand          |
| 5   | Alarmed   | 21–24  | Red wash + wide eyes + `o` mouth       |

## In-game triggers

- Celebrate: level win, clean punch-down
- Pout: high attenuation (bad termination)
- Blush: (reserved) Alarmed: outage start
- Wink: outage resolved

## Build notes

- Source illustrations generated on flat magenta `#FF00FF`, keyed with
  the chrominance matting pipeline (YCbCr distance, soft alpha, despill).
  Validated 0.0000 magenta residue on all six expressions.
- Breathing bob: vertical offsets [0, -4, -8, -4] px at 288×576 scale.
- Alarmed row carries the classic red wash (red channel lifted).
- Sheet sources: `~/workspace/light-show/portraits/lattice-v3/sheet_src/`
