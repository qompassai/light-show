# Light Show — Game Design Document

## Premise

You're a rookie Outside Plant (OSP) tech at Qompass Networks. Your job: restore
service on broken networks — route the signal from Point A to Point B and land
the received level inside spec. Four AI-hologram companions live in your OTDR
tablet, one per transmission discipline: **Séraphine** (fiber-optic splicing),
**Ondine** (coax / broadband RF), **Linka** (mobile / cellular RF), and
**Lattice** (Ethernet / copper LAN). Picking a companion picks *what you learn*:
each one teaches her own medium through a two-level track of real
engineering problems wearing a puzzle-game costume.

## Game Flow

1. **Title.** Keeper title artwork (night city, neon LIGHT SHOW logo), tagline,
   Start → companion picker. Credits button.
2. **Companion picker.** Four cards, one per companion/medium. Each card shows
   the companion's dark silhouette, her name, discipline, and a one-line hook;
   hovering or pressing a card plays her discipline animation and lights an
   accent-glow outline. Choosing a card sets her as your companion and jumps
   you to the first level of her track. A Back button returns to the title.
3. **Board play.** The puzzle itself — see Core Loop.
4. **Results.** Win banner ("SERVICE RESTORED") with an animated neon ring.
   From here: **Continue** plays the next level in the current track, or
   **Companion Select** returns to the picker (mid-track or at the end).

The menu-to-picker-to-track flow replaced the earlier single-companion design;
the picker is the game's central hub.

## Core Loop (per level)

1. **Survey the board.** A grid-based plant map: endpoints, taps, enclosures,
   spans, and component pills you place or select. An art-directed dark
   circuit-board backdrop with glowing neon node rings replaces the old
   text-only presentation.
2. **Route the signal.** Drag jumpers / choose paths / place components to
   connect A to B. Every component you place consumes budget (or, on coax,
   contributes gain — see The Four Disciplines).
3. **Hit the window.** The received level at B must land inside the level's
   target window. Too hot can be as wrong as too much loss — just like real
   plant engineering.
4. **Survive the outage event.** Mid-level, a fault fires (fiber cut, water
   intrusion, ingress, interference, a severed copper run). You must reroute
   or repair before the "customer complaint" timer expires.
5. **Your companion reacts.** Dialogue/animation triggers on key actions: clean
   splices, messy choices, level clear, outage resolved.

## The Four Disciplines

Each medium gets genuinely different simulation in `osp_sim` — not a reskin.
The dB arithmetic is shared (a decibel is a decibel); what differs is the
component set, the units on the receive window, and — for Ethernet — the win
condition itself.

| Companion | Medium | Units | What you learn |
|---|---|---|---|
| Séraphine | Fiber | dBm | Link budgets, fusion vs mechanical splices, protected routing. Loss-only path: every component eats optical budget; route resolution uses DFS/backtracking. |
| Ondine | Coax | dBmV | Amplifiers, dBmV cascades, **unity gain**. Cascades include *gain*, so the puzzle punishes both under-drive (snow) and over-drive (distortion). Ingress hazards. |
| Linka | Wireless | dBm (RSSI) | Path loss, antennas/repeaters. Hops lose power to free-space path loss (geometry, not cable); regenerative repeaters reset the level by retransmitting at their own power. Interference hazards. |
| Lattice | Ethernet | — | **No dB budget at all.** Structured-cabling constraints: 100 m segment limits, switches, PoE power budget, bandwidth. Win = every constraint satisfied. |

## The Optical Model (real formulas, simplified UI)

Fiber levels use an additive dB loss budget, matching real OSP link-budget
practice:

```
P_rx (dBm) = P_tx (dBm) − Σ(loss contributors, dB) + Σ(gain, dB from amps, rare)
```

Loss contributors modeled as game pieces:

| Component            | Typical loss (dB) | Puzzle behavior |
|-----------------------|-------------------|------------------|
| Fusion splice (good)  | 0.05–0.1          | Cheap, but costs "splice time" resource |
| Mechanical splice     | 0.3–0.5           | Fast, worse loss, used under outage time pressure |
| Connector (UPC)       | 0.3–0.5           | Placed at patch panels |
| Connector (APC)       | 0.3 (angled, low reflectance) | Required for high-bandwidth / PON levels to avoid "reflectance fail" |
| Dirty/contaminated connector | +2 to +5 extra | Random hazard; "clean it" mini-interaction |
| Splitter 1×2          | ~3.5              | Required to reach multiple ONTs (PON levels) |
| Splitter 1×4          | ~7.2              | |
| Splitter 1×8          | ~10.5             | |
| Splitter 1×16         | ~13.5             | |
| Splitter 1×32         | ~17.5             | |
| Fiber span (per km, SMF-28) | 0.35 (1310nm) / 0.25 (1550nm) | Distance slider per span; wavelength choice matters |
| Macrobend (tight coil/staple strike) | 0.5–3+ variable | Outage hazard — visually a kinked line |
| Water intrusion in splice closure | rises over "time" stat | Outage hazard that worsens if ignored |

Distances and losses are pulled onto a running ledger UI (styled like an OTDR
trace) so players visually learn to read a loss budget the way a real tech
reads OTDR output.

### Win condition
`P_rx` within the level's target window AND path is a single continuous
path A→B AND (if present) outage resolved before timer end. On Ethernet
tracks: all structured-cabling constraints satisfied instead of a level
window.

### Fail conditions
- `P_rx` too low (link down, "customer can't stream anime").
- `P_rx` too high / reflectance too high on APC-required segment (receiver
  saturation / return-loss fail).
- Outage timer expires before reroute.
- Physical impossibility (bend radius violation — visualized as a snapped
  fiber if a player forces too tight a corner in the routing grid).

### Simulation honesty notes
Two of the original fiber levels shipped with power values that made them
unwinnable under the real model; both were corrected so every bundled level
is winnable through legitimate play — verified by the playthrough test
suite, not by eyeballing the numbers.

## Tracks (shipped content)

Eight levels, two per companion, laid out in `game/src/level.rs`
(`LEVEL_SOURCES`) in companion order:

| # | File | Companion | Level |
|---|---|---|---|
| 1 | `world1_level1.json` | Séraphine | Fiber basics: single span, single splice, the dB ledger |
| 2 | `world4_level1_outage.json` | Séraphine | Storm season: timed fiber cut, protected-path reroute |
| 3 | `coax1_unity_gain.json` | Ondine | Unity gain: balance amplifier gain against cascade loss |
| 4 | `coax2_ingress.json` | Ondine | Ingress: find and clear the noise source |
| 5 | `wireless1_close_the_link.json` | Linka | Path loss: close the link over distance |
| 6 | `wireless2_ride_the_storm.json` | Linka | Storm: ride out interference with regeneration |
| 7 | `ethernet1_hundred_meter_wall.json` | Lattice | The 100 m wall: segment-length discipline |
| 8 | `ethernet2_power_budget.json` | Lattice | PoE budget: power every device without oversubscribing |

## Advanced track (Clara / Aino / Hikari)

Unlocked by completing the Séraphine/Ondine/Linka level sets (see the
unlock system in `game/TODO.md`). Three new companions, three new
disciplines — the game graduates from single-board puzzles to
network-scale thinking.

| Companion | Discipline | What changes |
|---|---|---|
| Clara | PON provisioning | Service profiles and split ratios: every ONT on a shared PON must land inside its tier's optical window. Bulk turn-ups, worst-case path verification. See `docs/CALIX_PROVISIONING.md`. |
| Aino | NOC triage | The NOC console (`GameState::NocConsole`): a live alarm list across all four media, ack/dispatch/clear workflow, severity that ages. Triage under pressure instead of one fault at a time. See `docs/NOC_CONSOLE.md`. |
| Hikari | OSP construction | MST/tap-plan puzzles: assign homes to multiport terminals, mind the port counts and the drop lengths, survive the backhoe. See `docs/OSP_DFN_GUIDE.md`. |

Design rule for the advanced track: every level must have exactly one clean
solution and at least one near-miss that fails by a small, legible margin.
The lesson is always in the near-miss.

## Companion System

- Four anime AI-hologram companions, one per access technology. Each has a
  full-body 96×192 sprite sheet: 6 mood rows (Idle, Blush, Wink, Pout,
  Celebrate, Alarmed) × 4 animation frames.
- Mood rows are identical across companions so gameplay code treats them
  uniformly (`game/src/waifu/sprite.rs` atlas indexing).
- Flirty-but-wholesome dialogue bank keyed to game events (see
  `src/waifu/dialogue.rs` and `assets/dialogue/seraphine_en.json`).
- Gives **optional hints** (costs in-game "favor points" earned by clean
  splices) — never required to solve a level, keeps it skippable/SFW-safe for
  both storefronts.
- No purchasable currency tied to dialogue — avoids loot-box/gacha
  anti-features so the build stays F-Droid eligible.

## Testing Contract

The game is verified by three layers, all run on real hardware (Primo):

- **Playthrough tests** (`game/src/playthrough.rs`, 13 tests): drive the
  real Bevy app — real mouse press/release resources, the actual board
  pointer-input system, real `Interaction::Pressed` buttons. All four tracks'
  winning paths, one or more losing/adversarial paths per medium, storm
  repair timing, retry-after-failure, back navigation, and distractor pills
  that must never win.
- **Unit tests** (`osp_sim` + game): medium models, graph evaluation,
  component math — 34 in `osp_sim` alone.
- **GUI smoke runs**: real Xvfb + xdotool sessions (software rendering)
  completing full tracks end to end, with screenshots as evidence. Note
  honestly: Xvfb is not a physical phone GPU and not real-device acceptance.

Gate numbers at last green: 126 tests passed, 0 failed; `cargo clippy
--workspace --all-targets -- -D warnings` clean.

## Content & Store Compliance Notes

- No real-money transactions, ads, trackers, or proprietary network calls —
  keeps the build free of F-Droid "anti-features."
- All art/audio original or CC0/CC-BY, tracked in `docs/CREDITS.md`.
- Dialogue reviewed for PEGI 12 / Google Play "Teen" rating: flirtation and
  light innuendo only, no explicit content, no fan-service nudity.
- Educational framing (real dB math, real component names) supports Google
  Play's "Educational" secondary category.
