# OSP / DFN Construction Guide (for level designers)

Hikari's track. How to build Outside Plant levels that teach real
distribution-network design: MSTs, tap plans, and the geometry of getting
fiber past every home without blowing the loss budget.

## The plant you're modeling

A **DFN (Distribution Fiber Network)**: feeder fiber from the central office
to a **MST (Multiport Service Terminal)** — the hardened tap box on the
strand or pedestal — then drop fibers from MST ports to individual homes
(ONTs). The level designer's job: lay out a believable neighborhood and let
the player plan the taps.

## Board pieces for OSP levels

| Piece | What it is | Game behavior |
|---|---|---|
| CO / OLT | Signal source | Fixed launch power, fixed location |
| Feeder span | Trunk fiber | dB per km; player chooses the route |
| MST | Multiport tap (4/8/12 port) | Each used port taps ~1 dB; unused ports are free but cost capex |
| Tap plan | Which home feeds from which MST port | The core puzzle: assign drops to ports |
| Drop | Home run fiber | dB per km, shorter than feeder |
| Home / ONT | Subscriber | Must land inside its receive window |

## Designing a tap plan puzzle

1. **Draw the neighborhood.** Place 6–12 homes at varied distances from 2–3
   candidate MST spots. Distances must differ enough that the choice matters
   (≥0.5 km spread is a good rule of thumb).
2. **Set the budget.** Pick OLT launch and ONT window so that the *obvious*
   plan (nearest MST for every home) fails for 1–2 homes — too much drop
   length, or one MST oversubscribed past its port count.
3. **Force the tradeoff.** The winning plan uses a farther MST (more feeder
   loss) to relieve the oversubscribed one, or accepts a longer drop on a
   low-loss branch. There should be exactly one clean solution and two
   near-misses that fail by <2 dB — that's where the learning lives.
4. **Add the outage.** Mid-level: a backhoe cuts the feeder to one MST.
   The player re-homes its subscribers to the surviving MST(s) under outage
   time pressure. Some homes will go dark — the puzzle is choosing *which*,
   then restoring them.

## Rendering (`game/src/board.rs`)

MST/tap plans render as a distinct board layer: MSTs as tap-box glyphs with
port pips (filled = assigned, hollow = free), drops as thin colored runs
from port to home, hover on a home highlights its full path back to the CO
with the cumulative loss annotated per segment. Keep the existing node/edge
vocabulary — an MST is a node, a drop is an edge, the tap plan is metadata.

## Difficulty knobs

- Home count and distance spread.
- MST port counts (scarcity forces hard choices).
- Receive window tightness.
- Whether the outage severs a feeder (hard) or degrades one MST's ports
  with water intrusion (diagnostic).

## Checklist before shipping a Hikari level

- [ ] Exactly one clean solution; near-misses fail by <2 dB.
- [ ] Every home's path loss is hand-verified against the budget table.
- [ ] The outage has a unique best re-home plan.
- [ ] The tap-plan layer reads at a glance (no overlapping drop runs).
