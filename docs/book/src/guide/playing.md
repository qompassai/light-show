# Playing the Game

## In plain terms: the loop

**Menu → pick your companion → Start → route the light → land in the
window (or survive the outage) → results screen → next level.**
Win by getting the received power inside the target window. Lose by
running out the outage clock. That's the whole game; this chapter is
the full version.

## 1. The main menu

The menu shows the title, a **companion picker** (four buttons), a
**Start** button, and a **Credits** button.

The companions are anime-styled AI sidekicks, each tied to a real
access technology:

| Companion | Technology | Default? |
|---|---|---|
| Séraphine | Fiber-optic OSP splicing | **yes** (the original) |
| Ondine | Coax / broadband RF | |
| Linka | Mobile / cellular RF | |
| Lattice | Ethernet / copper LAN | |

Picking one swaps the on-screen sprite and the whole dialogue bank —
she reacts to your splices, outages, and results in her own voice
(`waifu::respawn_on_companion_change`). The choice is flavor, not
physics: the simulation doesn't care who you picked.

**Credits** opens the in-game credits screen: code/art/font credits
plus the full shipped music attribution. The attribution is
`include_str!`ed from `game/assets/music/ATTRIBUTION.txt` — the same
file that ships in the APK — so the on-screen text can never drift
from what actually ships. (CC-BY licensed tracks *require* visible
attribution, which is why credits is a real game state, not a popup.)

## 2. Playing: reading the board

Pressing Start enters `Playing` and loads the current level
(`playing::setup_level`): placements reset, the clock resets, the live
graph rebuilds, the companion resets to idle, and a dedicated board
camera spawns centered so the board clears the briefing text at top
and the companion sprite at bottom.

The board is drawn from the level definition:

- **Nodes** — circles. The two endpoints (source/OLT and
  target/ONT) draw warm gold (`#ffd166`); intermediate nodes draw
  accent blue (`#5bc0eb`).
- **Fixed edges** — solid warm lines. Pre-built plant you don't place.
- **Open choice edges** — dashed lines, dim blue until you place a
  component on them, then solid warm.
- **Pills** — one small circle per component choice on multi-choice
  edges, accent blue, the selected one hot pink (`#ff6fae`).

At the top, the **ledger UI** — styled like an OTDR trace readout —
shows the live link budget, recomputed every frame:

```text
Loss: 12.35 dB  |  Rx: -9.35 dBm  |  Window: [-27, -8] dBm  |  IN WINDOW  |  Favor: 10
```

`IN WINDOW` / `OUT OF WINDOW` is the whole puzzle in two words: win
the instant the computed receive power lands inside the window.

### The physics behind the ledger

This is the mechanism, and it's genuinely how fiber works:

```text
received_dBm = transmit_dBm − Σ(component losses in dB)
```

Light leaves the transmitter at the level's `tx_dbm` (default
+3 dBm, the standard GPON downstream launch per ITU-T G.984.2 class
B+). Every component on the path eats some of it. Win when the result
is inside `[window_min_dbm, window_max_dbm]` — typically `[−27, −8]`
dBm, the GPON ONT sensitivity range. The results screen also shows
**margin**: how deep inside (or how far outside) the window you are.

The loss figures are real midpoints from field practice:

| Component | Loss |
|---|---|
| Fusion splice | 0.075 dB |
| Mechanical splice | 0.40 dB |
| UPC connector | 0.35 dB |
| APC connector | 0.30 dB |
| Splitter 1:2 / 1:4 / 1:8 / 1:16 / 1:32 | 3.6 / 7.3 / 10.6 / 13.7 / 17.7 dB |
| Fiber span (1310 / 1490 / 1550 nm) | 0.35 / 0.28 / 0.21 dB per km |
| Macrobend (kink hazard) | its excess loss, as seeded |

So the puzzle is a real engineering tradeoff: the fusion splice is
cheaper in dB but the mechanical splice places faster — which matters
when the outage clock is ticking. Splitters divide your light among
many customers, costing dearly. Picking the wrong wavelength wastes
dB per kilometer.

## 3. Winning a normal level

Levels *without* a scripted outage win the moment `check_win_condition`
sees the live budget in-window: the companion flips to celebrating,
and the game transitions to `Results`. There is no "submit" button —
the win is continuous and immediate.

## 4. Outages: the boss fight

Levels *with* a scripted outage play differently. A `fires_after_seconds`
clock runs during normal play; when it elapses, `check_scripted_outage`
fires: the `Outage` is stored, the graph rebuilds (a full cut shows in
the ledger *instantly*), and the game enters `OutageActive`.

What you see: a full-width red alarm banner with the hazard's flavor
text and a live **"REPAIR NOW — Ns"** countdown; your companion snaps
to alarmed; severed edges draw hazard red. The board stays fully
interactive — you reroute around the fault or repair it with the same
drag-and-tap gestures.

The five outage kinds, with their base timers:

| Outage | What it is | Timer |
|---|---|---|
| FiberCut | Backhoe severs a buried span — **full cut**, reroute or emergency-splice | 90 s |
| AerialDamage | Storm limb across aerial span — **full cut** | 75 s |
| WaterIntrusion | Splice closure floods; loss **grows 1 dB per 10 s**, up to +15 dB | 120 s |
| ConnectorContamination | Dirty/scratched connector — degraded, not severed | 60 s |
| Macrobend | Drop cable kinked under minimum bend radius — degraded | 60 s |

Resolution is checked every frame by `check_outage_resolution`:

- **Repaired budget back in-window** → outage marked resolved, a win
  ("nice save" — the companion winks instead of the plain celebrate).
- **Timer expires** → loss.

Either way the game goes **straight to `Results`** — deliberately
*never* back to `Playing`, because re-entering `Playing` would run the
full level reset and wipe the in-progress repair.

Note: levels with a scripted outage *skip* the normal win check
entirely, so a player who pre-builds the eventual protection route
can't win before the storm — the outage content is the point of the
level.

## 5. Results and favor

The results screen shows a banner — **SERVICE RESTORED** (green) or
**OUTAGE TIMED OUT** (red) — the level name, the final ledger
(`Loss / Rx / Margin`, or "Link disconnected — no route completed"),
and a companion reaction line picked from the dialogue bank
(`level_win` / `level_fail_cold`, overridable per level).

A clean win earns **+10 favor points** (`FAVOR_PER_WIN`); losses earn
nothing. Favor is spendable later on optional hints, and the running
total is always visible in the ledger. Buttons: **Continue** (win only,
and only if another bundled level exists — advances the level index),
**Retry** (loss), **Main Menu** (resets the level index to 0).

## 6. The full state map

```text
MainMenu ──Start──▶ Playing ──in window──▶ Results ──Continue──▶ Playing (next level)
   │                   │                       │  └─Retry──▶ Playing (same level)
   │                   │                       └─Main Menu──▶ MainMenu (index → 0)
   │                   └──outage fires──▶ OutageActive ──repaired/timeout──▶ Results
   └──Credits──▶ Credits ──Back──▶ MainMenu
```

Music follows the states: menu loop → world-tiered level tracks →
boss-fight outage track → victory/defeat jingle → back to menu. One
track at a time, always — the old state's exit runs before the new
state's enter, so two tracks never overlap.
