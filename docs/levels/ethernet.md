# Ethernet — Lattice (Copper LAN)

Lattice's track replaces the decibel with the rulebook. Ethernet over twisted pair is not won by landing a signal level; it is won by satisfying three independent constraints at once: no unbroken copper segment longer than the standard allows, enough Power over Ethernet (PoE) budget at the switch for the powered device at the far end, and cabling whose category carries the required bandwidth end to end — remembering that the slowest segment sets the speed for the whole path. A switch is the track's one active tool: it terminates a segment (restarting the distance budget) and contributes PoE budget. A plain cable run does neither.

How winning works in this game (all tracks): the live win check in `game/src/states/playing.rs` is a conjunction — the board evaluation for the level's medium must pass **and** every console or Astra gate the level carries (API sequence, alarm triage, identification, jumper swap, workbench, static config, survey, handoff) must pass. Levels with a scripted outage skip the plain win check entirely and can only be won through the outage-resolution path, which applies the same conjunction against the post-outage plant. Each level section below names every gate that applies to it.

Scoring on this track — stated plainly because the data can mislead: **the Ethernet evaluator is violation-scored.** Each level's JSON still carries transmit and receive-window fields (0 dBm, −27…−8), but no Ethernet code path reads them; they are inert leftovers of the shared level format. A level passes exactly when the path is continuous and the violation list is empty: no segment over the level's maximum (100 m in every level here), PoE draw ≤ summed switch PoE budget, and minimum path bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) ≥ the level's requirement. The per-level sections therefore list constraints, not windows, as the pass thresholds.

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>e1l1 — The Hundred-Meter Wall</strong></summary>

- **Level ID:** `e1l1` · **World (data):** 4
- **Objective:** Lattice's first law: copper has a distance limit, not just a speed limit. The IDF sits 65 m from the wiring closet and the desk is 65 m past that — 130 m of unbroken copper, and Ethernet taps out at 100 m per segment. A faster cable won't fix a too-long run. Place the switch that breaks the path into two legal segments.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **0 W**, required bandwidth **1000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 0 W); Cat5e run 65 m; Cat6 run 65 m.
- **Notes:** Evaluator honesty (applies to this whole file): Ethernet levels carry `tx_dbm` 0 and a −27…−8 'window' in their data, but those fields are **inert** — nothing in the Ethernet evaluator reads them. Winning is violation-scored only: continuity plus zero violations among segment length, PoE, and bandwidth (see this file's introduction).
- **Teaches:** The TIA-568 channel limit: 100 m maximum per copper segment (conventionally 90 m horizontal + patch). A switch regenerates the signal and restarts the distance budget; faster cable does not extend it.

</details>

<details>
<summary><strong>e1l2 — Power Budget</strong></summary>

- **Level ID:** `e1l2` · **World (data):** 4
- **Objective:** The new access point needs 25 W of PoE and a 2 Gbps backhaul. The cable path is already pulled — 30 m of Cat6 to the IDF, 40 m more to the AP's jack. Cable can't conjure watts. Place the switch whose PoE budget actually covers the AP without browning out.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **25 W**, required bandwidth **2000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 60 W); switch (PoE budget 15 W); Cat6 run 10 m.
- **Teaches:** Power over Ethernet budgeting: the switch's PoE budget must cover the powered device's draw, independently of whether the cable carries the data fine. Data and power are separate budgets on the same run.

</details>

<details>
<summary><strong>e1l3 — Category Matters</strong></summary>

- **Level ID:** `e1l3` · **World (data):** 4
- **Objective:** The closet run is 60 m of Cat6 — good for 10 Gbps. But the slowest cable sets the speed for the whole path, and the far end needs 5 Gbps. Pick the 40 m run that doesn't bottleneck the link. A faster cable can't fix a slow one already in the wall.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **0 W**, required bandwidth **5000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: Cat5e run 40 m; Cat6 run 40 m; Cat6 run 50 m.
- **Teaches:** Bottleneck rule for mixed-category channels: the slowest segment sets end-to-end bandwidth (Cat5e = 1 Gbps, Cat6 = 10 Gbps in the sim's ratings).

</details>

<details>
<summary><strong>e1l4 — The Long Corridor</strong></summary>

- **Level ID:** `e1l4` · **World (data):** 4
- **Objective:** 70 m of Cat6 to the IDF, 70 m more to the far office — 140 m of copper with a gap in the middle where the old switch died. No single cable run can span it: 140 m breaks the 100 m wall. Place the switch that breaks the path into two legal segments.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **0 W**, required bandwidth **1000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 0 W); Cat6 run 10 m; Cat5e run 10 m.
- **Teaches:** Segmentation as the fix for distance: inserting a switch converts one illegal 140 m channel into two legal segments without re-pulling cable.

</details>

<details>
<summary><strong>e1l5 — Power Hungry</strong></summary>

- **Level ID:** `e1l5` · **World (data):** 4
- **Objective:** The new camera array draws 45 W of PoE — more than the old 30 W switches can feed. Cable can't conjure watts. The 50 m Cat6 run is already pulled; place the switch whose PoE budget actually covers the load without browning out.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **45 W**, required bandwidth **1000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 30 W); switch (PoE budget 60 W); switch (PoE budget 15 W).
- **Teaches:** PoE sizing for high-draw devices: camera-class loads (45 W) exceed entry switch budgets; the fix is a switch that can feed the load, not a different cable.

</details>

<details>
<summary><strong>e1l6 — Every Constraint</strong></summary>

- **Level ID:** `e1l6` · **World (data):** 4
- **Objective:** 80 m of Cat6 to the IDF, then a 40 m Cat6 drop — but the far end needs 8 Gbps AND 20 W of PoE. Distance, bandwidth, power: the win needs all three. The switch in the middle must break the segments, feed the watts, and not bottleneck the 10-gigabit path.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **20 W**, required bandwidth **8000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 30 W); switch (PoE budget 15 W); Cat6 run 5 m.
- **Teaches:** Simultaneous constraint satisfaction: segment length, bandwidth, and PoE must all hold at once — optimizing one while violating another is a failed design.

</details>

<details>
<summary><strong>e1l7 — The Campus</strong></summary>

- **Level ID:** `e1l7` · **World (data):** 5
- **Objective:** Three buildings share one IDF, and the far access point draws 50 W of PoE — more than a standard switch feeds. The 80 m Cat6 runs are already pulled on both sides. Cable can't conjure watts: place the switch whose PoE budget actually covers the load.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **50 W**, required bandwidth **1000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 30 W); switch (PoE budget 60 W); switch (PoE budget 45 W).
- **Teaches:** Campus distribution: one intermediate switch serving multiple buildings must still satisfy the far device's full PoE draw; shared infrastructure does not average away a budget.

</details>

<details>
<summary><strong>e1l8 — No Slack</strong></summary>

- **Level ID:** `e1l8` · **World (data):** 5
- **Objective:** 90 m of Cat6, then 90 m of legacy Cat5e — both near the 100 m wall, and the Cat5e caps the whole path at 1 Gbps. The far end needs exactly that: 1 Gbps and 25 W of PoE. Every parameter is tight. The switch must split the segments, feed the watts, and live with the Cat5e bottleneck.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **25 W**, required bandwidth **1000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 30 W); switch (PoE budget 20 W); Cat6 run 5 m.
- **Teaches:** Designing with legacy plant: near-limit segment lengths and a Cat5e bottleneck can still be a valid design when the requirement is exactly what the legacy plant can deliver.

</details>

<details>
<summary><strong>e1l9 — Retrofit</strong></summary>

- **Level ID:** `e1l9` · **World (data):** 5
- **Objective:** The building's old 60 m Cat5e runs are in the walls and the owner won't pay to replace them — but 120 m of unbroken copper breaks the 100 m wall. You can't fix the cable; work around it. Place the switch that turns two illegal spans into two legal segments.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **0 W**, required bandwidth **1000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 0 W); Cat5e run 5 m; Cat6 run 5 m.
- **Teaches:** Retrofit economics: when re-cabling is off the table, active electronics (a switch) work around a distance violation the passive plant cannot fix.

</details>

<details>
<summary><strong>e1l10 — The Data Center</strong></summary>

- **Level ID:** `e1l10` · **World (data):** 5
- **Objective:** The final exam. 40 m of Cat6 to the IDF, 40 m more to the rack — and the rack needs 10 Gbps AND 55 W of PoE. Cat5e anywhere would bottleneck the ten-gigabit path, so the cable is all Cat6. But watts don't ride on cable: place the switch that feeds 55 W without browning out.
- **Mechanic / evaluator:** Ethernet constraint evaluation (`evaluate_ethernet` in osp_sim): the placed path is checked for violations, not for a signal level — longest unbroken copper segment against the level's maximum, summed switch PoE budget against the endpoint draw, and the slowest run's category bandwidth (Cat5e 1 Gbps, Cat6 10 Gbps) against the required bandwidth. A switch breaks a segment; a plain run does not. Verification states: Continuity + Application. The level's transmit/window data fields are inert on this medium (see e1l1's note).
- **Pass thresholds / scoring:** **Zero violations** with: max segment **100 m**, endpoint PoE draw **55 W**, required bandwidth **10000 Mbps**. Any single violation fails the level.
- **Player choices:** edge 1→2: switch (PoE budget 30 W); switch (PoE budget 50 W); switch (PoE budget 60 W).
- **Teaches:** Data-center access design at full demand: 10 Gbps end to end (no Cat5e anywhere) plus a 55 W PoE draw, satisfied simultaneously.

</details>
