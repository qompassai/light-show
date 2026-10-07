# Fiber — Séraphine (OSP Splicing)

Fiber is the game's first discipline and Séraphine's home track: outside-plant (OSP) splicing, taught entirely through the optical link budget. Every level hands you a launch power, a route to complete, and a receive window; your craft choices — splice type, connector, splitter, route — each cost real decibels, and the ONT at the end accepts only a narrow band of received power. Too little light and the receiver starves; too much and it overloads. The track runs from a single training span to storm protection and a half-decibel expert budget. The three Field School scenarios that extend this track live in [field-school.md](field-school.md).

How winning works in this game (all tracks): the live win check in `game/src/states/playing.rs` is a conjunction — the board evaluation for the level's medium must pass **and** every console or Astra gate the level carries (API sequence, alarm triage, identification, jumper swap, workbench, static config, survey, handoff) must pass. Levels with a scripted outage skip the plain win check entirely and can only be won through the outage-resolution path, which applies the same conjunction against the post-outage plant. Each level section below names every gate that applies to it.

Scoring on this track: the fiber evaluator (`compute_link_budget_with_outage`) sums span attenuation (1310 nm: 0.35 dB/km; 1490 nm: 0.28 dB/km; 1550 nm: 0.21 dB/km), fusion splices (0.075 dB), mechanical splices (0.4 dB), connectors (UPC 0.35 dB, APC 0.30 dB, plus contamination), splitters (1:4 = 7.3 dB, 1:8 = 10.6 dB, 1:16 = 13.7 dB, 1:32 = 17.7 dB), and macrobends along the placed path. Received power must land inside the level's window, inclusive. Timed levels add a scripted outage with a complaint timer (see each section).

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>w1l1 — First Light</strong></summary>

- **Level ID:** `w1l1` · **Companion:** Séraphine (Fiber) · **World (data):** 1
- **Objective:** Welcome to Splice School. Route the training optic's launch power down a single buried span to the ONT. Land inside the GPON receive window — either splice gets you there, so focus on placing it cleanly.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-27, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: fusion splice → Rx ≈ -12.32 dBm (in window); mechanical splice → Rx ≈ -12.64 dBm (in window).
- **Teaches:** Reading a link budget end to end: launch power minus span loss minus splice loss must land inside the receiver's sensitivity window — neither starved nor overdriven.

</details>

<details>
<summary><strong>f1l3 — Clean Hands</strong></summary>

- **Level ID:** `f1l3` · **Companion:** Séraphine (Fiber) · **World (data):** 1
- **Objective:** Séraphine's second rule: a dirty connector is a silent killer. This 10 km buried span is clean, but the patch panel connector is up to you. A contaminated ferrule adds 2 dB you will never get back — pick a clean one.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-14, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: UPC connector → Rx ≈ -13.15 dBm (in window); UPC connector, +2 dB contamination → Rx ≈ -15.15 dBm (out of window); APC connector → Rx ≈ -13.10 dBm (in window).
- **Teaches:** Connector hygiene: inspect and clean every ferrule before mating. Contamination loss is invisible, permanent for that mating, and entirely preventable.

</details>

<details>
<summary><strong>f1l4 — Fusion or Bust</strong></summary>

- **Level ID:** `f1l4` · **Companion:** Séraphine (Fiber) · **World (data):** 1
- **Objective:** 25 km of buried fiber at 1490 nm. The budget is razor thin — a mechanical splice's 0.4 dB will sink you, but a fusion splice's 0.075 dB just barely keeps you afloat. Do the math before you pick up the cleaver.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-17.2, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: fusion splice → Rx ≈ -17.08 dBm (in window); mechanical splice → Rx ≈ -17.40 dBm (out of window).
- **Teaches:** Splice-method selection under a tight budget: fusion (≈0.075 dB) versus mechanical (≈0.4 dB) is a real engineering trade — cost and time against decibels you cannot spare.

</details>

<details>
<summary><strong>f1l5 — The Short Cut</strong></summary>

- **Level ID:** `f1l5` · **Companion:** Séraphine (Fiber) · **World (data):** 1
- **Objective:** Two routes to the customer at 1550 nm — the long way around the ridge (45 km) or the direct shot through the valley (30 km). At 0.21 dB per km, every kilometer costs you. Pick the route your budget can afford.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1550 nm (0.21 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-17, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→3: fusion splice → Rx ≈ -16.38 dBm (in window) | edge 2→3: fusion splice → Rx ≈ -19.52 dBm (out of window).
- **Teaches:** Wavelength-dependent attenuation: at 1550 nm fiber loses ≈0.21 dB/km versus 0.28 dB/km at 1490 nm, so route length and wavelength together decide feasibility.

</details>

<details>
<summary><strong>f1l6 — Split Decision</strong></summary>

- **Level ID:** `f1l6` · **Companion:** Séraphine (Fiber) · **World (data):** 1
- **Objective:** PON turn-up. The OLT launches at +3 dBm and the feeder eats 1.4 dB before the splitter cabinet. Your split ratio is the whole budget: 1:8 leaves the ONT screaming hot, 1:32 leaves it starved. Find the Goldilocks split.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (3 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-13, -11] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: 1:8 splitter (10.6 dB) → Rx ≈ -9.00 dBm (out of window); 1:16 splitter (13.7 dB) → Rx ≈ -12.10 dBm (in window); 1:32 splitter (17.7 dB) → Rx ≈ -16.10 dBm (out of window).
- **Notes:** Numbers check: feeder 5 km at 1490 nm costs 1.4 dB; from +3 dBm the 1:16 split (13.7 dB) lands at ≈ −12.1 dBm — the only choice inside the deliberately narrow −13…−11 window. The briefing's '1:8 screams, 1:32 starves' is exact.
- **Teaches:** PON splitter budgeting: each doubling of the split costs roughly 3 dB plus excess loss (1:8 = 10.6 dB, 1:16 = 13.7 dB, 1:32 = 17.7 dB in the sim). Too little split overdrives near ONTs; too much starves them.

</details>

<details>
<summary><strong>f1l7 — Bend Don't Break</strong></summary>

- **Level ID:** `f1l7` · **Companion:** Séraphine (Fiber) · **World (data):** 1
- **Objective:** Someone kinked the drop below minimum bend radius — a 5 dB macrobend sitting right in your path. You can splice straight through it and eat the loss, or take the 12 km bypass around. A bad bend costs more than extra distance.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-16, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 2→4: fusion splice → Rx ≈ -16.48 dBm (out of window) | edge 3→4: fusion splice → Rx ≈ -14.84 dBm (in window).
- **Teaches:** Macrobend loss and bend-radius discipline: a kinked fiber radiates light out of the core. Rerouting around damage can be cheaper, in decibels, than forcing the short path.

</details>

<details>
<summary><strong>f1l8 — Contamination Event</strong></summary>

- **Level ID:** `f1l8` · **Companion:** Séraphine (Fiber) · **World (data):** 1
- **Objective:** A tech left a connector dirty and the customer's ticket just escalated — you have 60 seconds to re-terminate the patch panel. The budget is tight enough that only the lowest-loss clean connector survives. Move fast and pick right.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-14, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: UPC connector → Rx ≈ -13.99 dBm (in window); APC connector → Rx ≈ -13.94 dBm (in window); mechanical splice → Rx ≈ -14.04 dBm (out of window).
- **Outage:** **ConnectorContamination** fires 10 s into the level on edge 1→2; complaint timer 60 s. Degrading hazard on the affected edge until the connector is re-terminated clean.
- **Teaches:** Working under an escalated ticket: re-termination discipline and connector selection (APC's angled polish also buys return loss) when the budget leaves room for exactly one right answer.

</details>

<details>
<summary><strong>w4l1 — Storm Season</strong></summary>

- **Level ID:** `w4l1` · **Companion:** Séraphine (Fiber) · **World (data):** 4
- **Objective:** Standard route is up, but a storm's rolling in. If the aerial span goes down mid-shift, get the customer back on the protection path before the timer runs out.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-8 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-27, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→3: fusion splice → Rx ≈ -9.47 dBm (in window) | edge 2→3: mechanical splice → Rx ≈ -10.22 dBm (in window).
- **Outage:** **AerialDamage** fires 20 s into the level on edge 1→3; complaint timer 75 s. It is a full cut: the edge carries nothing until the player reroutes around it.
- **Teaches:** Protection switching: pre-built diverse routing is what turns a storm cut from an outage into a switchover. Aerial and buried paths fail differently, so diversity is physical, not just logical.

</details>

<details>
<summary><strong>f1l9 — Tight Budget</strong></summary>

- **Level ID:** `f1l9` · **Companion:** Séraphine (Fiber) · **World (data):** 5
- **Objective:** Expert work. 22 km at 1550 nm behind a 1:16 split — the feeder, the splitter, and your termination have to line up inside half a decibel. A mechanical splice or a connector will blow it. Only a fusion splice threads this needle.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (5 dBm) minus fiber attenuation at 1550 nm (0.21 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-13.5, -13] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 2→3: fusion splice → Rx ≈ -13.39 dBm (in window); mechanical splice → Rx ≈ -13.72 dBm (out of window); APC connector → Rx ≈ -13.62 dBm (out of window).
- **Teaches:** Half-decibel engineering: stacking feeder loss, splitter loss, and termination loss with no slack — and knowing which termination method is the only one that fits.

</details>

<details>
<summary><strong>f1l10 — Build It Right</strong></summary>

- **Level ID:** `f1l10` · **Companion:** Séraphine (Fiber) · **World (data):** 5
- **Objective:** Storm front inbound. The 6 km aerial span will not survive it — when it goes, only the 14 km buried protection path keeps the customer lit. Build the protection path NOW, and build it right: the long way around needs the low-loss splice.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-8 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-12.1, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→3: fusion splice → Rx ≈ -9.76 dBm (in window); mechanical splice → Rx ≈ -10.08 dBm (in window) | edge 2→3: fusion splice → Rx ≈ -12.00 dBm (in window); mechanical splice → Rx ≈ -12.32 dBm (out of window).
- **Outage:** **AerialDamage** fires 15 s into the level on edge 1→3; complaint timer 75 s. It is a full cut: the edge carries nothing until the player reroutes around it.
- **Teaches:** Designing for the failure you can forecast: build the protection path before the storm, and budget the longer buried route with the low-loss splice it demands.

</details>
