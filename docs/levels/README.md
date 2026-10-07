# Levels

Every level in Light Show, documented from the game's own data: the level JSONs under
`game/assets/levels/`, the evaluators in `game/src/level.rs` and the `osp_sim` crate, the Astra
mechanic states, and the console UIs. If a number appears here, it is computed from — or quoted
from — that data, and where the data contradicts itself, the contradiction is documented in place
rather than smoothed over.

The game ships **80 track levels** in a frozen order (`LEVEL_SOURCES` in `game/src/level.rs`):
eight companions, ten levels each. Three further **Field School scenario levels** (fj1, fj2, sp1)
live in a separate registry (`SCENARIO_SOURCES`) and never advance a track. This directory covers
all **83**.

## The discipline taxonomy

A discipline is one companion's answer to one question: *what does competence look like on this
medium?* The four main companions each own a transmission medium and its native evaluation —
a link budget, a balanced cascade, a path-loss calculation, a constraint check. The four
specialists own the work that surrounds the plant: provisioning it, operating it, installing it,
and being examined on it.

| Discipline | Companion | What it teaches | Evaluation core | File |
|---|---|---|---|---|
| Fiber (OSP splicing) | Séraphine | Optical link budgets: attenuation, splices, splitters, bend loss, protection routing | Received power inside a dBm window | [fiber.md](fiber.md) |
| G.fast over coax plant | Ondine | Cascade balance on the copper distribution plant: gain staging, taps, ingress and CNR, plus termination and identification craft | Carrier level in a dBmV window **and** CNR ≥ 25 dB, plus Astra gates | [gfast.md](gfast.md) |
| Wireless | Linka | Free-space path loss, regeneration vs amplification, SNR, surveys, static IP configuration | RSSI in a dBm window **and** SNR ≥ 10 dB, plus survey/config gates | [wireless.md](wireless.md) |
| Ethernet (copper LAN) | Lattice | TIA-568 segment limits, PoE budgets, category bandwidth — simultaneous constraints | **Violation-scored**: zero violations (segment, PoE, bandwidth) | [ethernet.md](ethernet.md) |
| CMS provisioning | Clara | Calix PON turn-up: splitter port counting, per-profile window verification, NBI (SOAP) and SMx (REST) session discipline | Every subscriber inside its profile window; exact API sequences | [provisioning.md](provisioning.md) |
| AMS NOC triage | Aino | Alarm-board triage by severity and root cause; AMS NBI queries, supervision, maintenance mode | Exact triage order; exact API sequences | [noc-triage.md](noc-triage.md) |
| BXE field craft | Hikari | FTTH drop craft under field conditions; the BxE portal workflow and its direct-diagnostics fast path | Fiber budget at drop scale; exact BxE sequences | [field-craft.md](field-craft.md) |
| TA study & quiz | Léa | NEC articles, theory, and Washington law/admin code for the 09 telecom administrator study | Quiz: ≥ 70% correct per level | [nec-study.md](nec-study.md) |
| Field School (scenarios) | Séraphine | A commercial fiber repair call in two legs, and a TIA-598 splice-tray repair | Fiber budget against verified repair readings | [field-school.md](field-school.md) |

Shared shape worth knowing before reading any file: the live win check is a **conjunction**. The
board evaluation for the level's medium must pass, and so must every console or Astra gate the
level carries. Wrong picks on the consoles (API, triage, workbench, config) are recorded and
taught from, but never fail a level outright; the board and the Astra hard gates are what fail.

Evaluator honesty, up front (each file repeats the part that governs it):

- **Ethernet** levels carry transmit/window fields in their data, but those fields are inert —
  the track is violation-scored.
- **Léa's** track is quiz-scored; the board is unused by design.
- **Clara, Aino, and Hikari** levels carry `medium: Fiber` in their data, but Clara's are won via
  provisioning verification, Aino's via triage/NBI logic, and Hikari's portal levels via BxE
  sequence logic — not via the fiber window as a skill test.
- Two Aino levels (aino6, aino10) gate on a board budget: a +3 dBm launch into a −27…−8
  window that only a splitter swap on the second leg can land — the arithmetic is shown in
  their sections.

## Level index

Track order below is the frozen `LEVEL_SOURCES` order; scenario levels follow.

| # | ID | Title | Discipline |
|---|---|---|---|
| 0 | `w1l1` | First Light | [Fiber](fiber.md) |
| 1 | `f1l3` | Clean Hands | [Fiber](fiber.md) |
| 2 | `f1l4` | Fusion or Bust | [Fiber](fiber.md) |
| 3 | `f1l5` | The Short Cut | [Fiber](fiber.md) |
| 4 | `f1l6` | Split Decision | [Fiber](fiber.md) |
| 5 | `f1l7` | Bend Don't Break | [Fiber](fiber.md) |
| 6 | `f1l8` | Contamination Event | [Fiber](fiber.md) |
| 7 | `w4l1` | Storm Season | [Fiber](fiber.md) |
| 8 | `f1l9` | Tight Budget | [Fiber](fiber.md) |
| 9 | `f1l10` | Build It Right | [Fiber](fiber.md) |
| 10 | `c1l1` | Unity Gain | [G.fast](gfast.md) |
| 11 | `c1l2` | Ingress at Night | [G.fast](gfast.md) |
| 12 | `c1l3` | Longer Run | [G.fast](gfast.md) |
| 13 | `c1l4` | Tap Dance | [G.fast](gfast.md) |
| 14 | `c1l5` | Ingress Returns | [G.fast](gfast.md) |
| 15 | `c1l6` | Backup Plan | [G.fast](gfast.md) |
| 16 | `c1l7` | Hot Headend | [G.fast](gfast.md) |
| 17 | `c1l8` | The Long Cascade | [G.fast](gfast.md) |
| 18 | `c1l9` | Precision Run | [G.fast](gfast.md) |
| 19 | `c1l10` | Ingress Storm | [G.fast](gfast.md) |
| 20 | `m1l1` | Close the Link | [Wireless](wireless.md) |
| 21 | `m1l2` | Ride the Storm | [Wireless](wireless.md) |
| 22 | `m1l3` | Thread the Needle | [Wireless](wireless.md) |
| 23 | `m1l4` | The 5GHz Tax | [Wireless](wireless.md) |
| 24 | `m1l5` | After the Repeater | [Wireless](wireless.md) |
| 25 | `m1l6` | Loud Is Not Clear | [Wireless](wireless.md) |
| 26 | `m1l7` | The Long Haul | [Wireless](wireless.md) |
| 27 | `m1l8` | Aim High | [Wireless](wireless.md) |
| 28 | `m1l9` | Mixed Bands | [Wireless](wireless.md) |
| 29 | `m1l10` | Linka's Gauntlet | [Wireless](wireless.md) |
| 30 | `e1l1` | The Hundred-Meter Wall | [Ethernet](ethernet.md) |
| 31 | `e1l2` | Power Budget | [Ethernet](ethernet.md) |
| 32 | `e1l3` | Category Matters | [Ethernet](ethernet.md) |
| 33 | `e1l4` | The Long Corridor | [Ethernet](ethernet.md) |
| 34 | `e1l5` | Power Hungry | [Ethernet](ethernet.md) |
| 35 | `e1l6` | Every Constraint | [Ethernet](ethernet.md) |
| 36 | `e1l7` | The Campus | [Ethernet](ethernet.md) |
| 37 | `e1l8` | No Slack | [Ethernet](ethernet.md) |
| 38 | `e1l9` | Retrofit | [Ethernet](ethernet.md) |
| 39 | `e1l10` | The Data Center | [Ethernet](ethernet.md) |
| 40 | `clara1` | First Turn-Up | [Provisioning](provisioning.md) |
| 41 | `clara2` | Storm Season | [Provisioning](provisioning.md) |
| 42 | `clara3` | Dead Box Swap | [Provisioning](provisioning.md) |
| 43 | `clara4` | Profile Audit | [Provisioning](provisioning.md) |
| 44 | `clara5` | Drive the NBI | [Provisioning](provisioning.md) |
| 45 | `clara6` | Bulk Turn-Up | [Provisioning](provisioning.md) |
| 46 | `clara7` | SMx Lifecycle | [Provisioning](provisioning.md) |
| 47 | `clara8` | Building Turn-Up | [Provisioning](provisioning.md) |
| 48 | `clara9` | Tight Budget | [Provisioning](provisioning.md) |
| 49 | `clara10` | Night Cutover | [Provisioning](provisioning.md) |
| 50 | `aino1` | First Shift | [NOC triage](noc-triage.md) |
| 51 | `aino2` | Triage Order | [NOC triage](noc-triage.md) |
| 52 | `aino3` | The Cascade | [NOC triage](noc-triage.md) |
| 53 | `aino4` | Bulk Ack | [NOC triage](noc-triage.md) |
| 54 | `aino5` | Query the NBI | [NOC triage](noc-triage.md) |
| 55 | `aino6` | Supervision | [NOC triage](noc-triage.md) |
| 56 | `aino7` | Night Shift | [NOC triage](noc-triage.md) |
| 57 | `aino8` | Maintenance Window | [NOC triage](noc-triage.md) |
| 58 | `aino9` | SOAP Fault | [NOC triage](noc-triage.md) |
| 59 | `aino10` | All Clear | [NOC triage](noc-triage.md) |
| 60 | `hikari1` | First Day on the Route | [Field craft](field-craft.md) |
| 61 | `hikari2` | Address to Diagnostics | [Field craft](field-craft.md) |
| 62 | `hikari3` | Beat the Callback | [Field craft](field-craft.md) |
| 63 | `hikari4` | The Dirty Drop | [Field craft](field-craft.md) |
| 64 | `hikari5` | Certify It | [Field craft](field-craft.md) |
| 65 | `hikari6` | Slow Portal | [Field craft](field-craft.md) |
| 66 | `hikari7` | Bend in the Wall | [Field craft](field-craft.md) |
| 67 | `hikari8` | The Splitter Closet | [Field craft](field-craft.md) |
| 68 | `hikari9` | Night Trouble Call | [Field craft](field-craft.md) |
| 69 | `hikari10` | Master Tech | [Field craft](field-craft.md) |
| 70 | `lea1` | Study Hall Warm-Up | [TA study](nec-study.md) |
| 71 | `lea2` | Grounded: Article 250 | [TA study](nec-study.md) |
| 72 | `lea3` | Wiring Methods: Articles 300-398 | [TA study](nec-study.md) |
| 73 | `lea4` | Hazardous Locations: Articles 500-516 | [TA study](nec-study.md) |
| 74 | `lea5` | Special Conditions: Articles 705-780 | [TA study](nec-study.md) |
| 75 | `lea6` | Comms Systems Boss: Articles 800-830 | [TA study](nec-study.md) |
| 76 | `lea7` | Theory Workshop | [TA study](nec-study.md) |
| 77 | `lea8` | Washington Law: RCW 19.28 | [TA study](nec-study.md) |
| 78 | `lea9` | Washington Admin Code: WAC 296-46B | [TA study](nec-study.md) |
| 79 | `lea10` | Mock Exam: Final Boss | [TA study](nec-study.md) |
| S1 | `fj1` | The Brewhouse Job — Replace the 1x4 | [Field School](field-school.md) |
| S2 | `fj2` | The Brewhouse Job — Prove the SB | [Field School](field-school.md) |
| S3 | `sp1` | True to the Color | [Field School](field-school.md) |

Per-level detail — objective, evaluator, pass thresholds, and the real-world skill each level teaches — lives in the discipline files above, one collapsible section per level.
