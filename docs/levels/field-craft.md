# BXE Field Craft — Hikari (FTTH/OSP)

Hikari is the field technician: FTTH drops, callbacks, certifications, and the BxE field portal that documents all of it. Her board levels revisit the fiber track's craft — clean connectors, splice choice, splitter sizing — at drop scale and under field pressure (callback clocks, night calls, water entering a closure). Her portal levels teach the BxE workflow as a fixed sequence — login, address search, device list, diagnostics — and, once the order is habit, the legitimate shortcut: with a cached device ID, the diagnostics call can be made directly. The capstone runs the full workflow, speed test and certification included, and then demands the drop be built to certification tolerance.

How winning works in this game (all tracks): the live win check in `game/src/states/playing.rs` is a conjunction — the board evaluation for the level's medium must pass **and** every console or Astra gate the level carries (API sequence, alarm triage, identification, jumper swap, workbench, static config, survey, handoff) must pass. Levels with a scripted outage skip the plain win check entirely and can only be won through the outage-resolution path, which applies the same conjunction against the post-outage plant. Each level section below names every gate that applies to it.

Scoring on this track: board levels use the standard fiber budget (1490 nm at 0.28 dB/km unless a section says otherwise) against each level's window; portal levels use exact API-sequence matching like Clara's and Aino's consoles. Honesty note: Hikari's levels carry `medium: Fiber` in their data, and her board levels genuinely are fiber-budget levels — but her portal levels (hikari2, hikari6, and the portal half of hikari10) are won via the BxE sequence logic, not the window.

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>hikari1 — First Day on the Route</strong></summary>

- **Level ID:** `hikari1` · **World (data):** 7
- **Objective:** Hikari's first solo dispatch: a fresh FTTH drop in Setagaya. The feeder span is already lit — your job is the last hundred meters. Her mentor's rule: a dirty connector is a callback waiting to happen. Pick a clean one.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-13, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: UPC connector → Rx ≈ -11.19 dBm (in window); UPC connector, +2 dB contamination → Rx ≈ -13.19 dBm (out of window); APC connector → Rx ≈ -11.14 dBm (in window).
- **Teaches:** Last-hundred-meters craft: the drop termination decides whether a good feeder becomes a good install. Clean connector selection is the first field habit.

</details>

<details>
<summary><strong>hikari2 — Address to Diagnostics</strong></summary>

- **Level ID:** `hikari2` · **World (data):** 7
- **Objective:** Dispatch hands you a street address in Nakano and nothing else. The BxE portal is slow, but the workflow is fixed: log in, search the address, list the devices, read the diagnostics. Learn the order — you will run it a hundred times.
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `bxe`. Choices offered: Login, SearchAddress, ListDevices, ReadDiagnostics, RunSpeedTest, CertifyInstall (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **Login → SearchAddress → ListDevices → ReadDiagnostics**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Teaches:** The BxE field-portal workflow: login → address search → device list → diagnostics. A fixed order, run the same way every time, is what makes field data trustworthy.

</details>

<details>
<summary><strong>hikari3 — Beat the Callback</strong></summary>

- **Level ID:** `hikari3` · **World (data):** 7
- **Objective:** The customer was promised a callback in thirty seconds and the dispatcher is already counting. The aerial drop is up — get the splice closed and the light verified before water gets into the span. Fusion is cleanest, but mechanical is faster to place. Your call.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-27, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: fusion splice → Rx ≈ -11.76 dBm (in window); mechanical splice → Rx ≈ -12.08 dBm (in window).
- **Outage:** **WaterIntrusion** fires 30 s into the level on edge 0→1; complaint timer 120 s. Degrading hazard: extra loss climbs 1 dB per 10 s, capped at 15 dB, until resolved.
- **Teaches:** Speed-versus-quality under a callback clock: fusion is cleaner, mechanical is faster to place; with water entering the span, the tech owns that trade explicitly.

</details>

<details>
<summary><strong>hikari4 — The Dirty Drop</strong></summary>

- **Level ID:** `hikari4` · **World (data):** 7
- **Objective:** Callback in Suginami: 'internet slow since the move-in.' BxE shows the light level 3 dB low at the ONT. You know this smell — someone touched the ferrule. Re-terminate it clean and watch the numbers come back.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (2 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-11, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 2→3: UPC connector → Rx ≈ -9.51 dBm (in window); UPC connector, +3 dB contamination → Rx ≈ -12.51 dBm (out of window); APC connector → Rx ≈ -9.46 dBm (in window).
- **Teaches:** Reading diagnostics quantitatively: a 3 dB-low ONT reading points at the termination, and re-terminating clean restores the budget — measure, fix, re-measure.

</details>

<details>
<summary><strong>hikari5 — Certify It</strong></summary>

- **Level ID:** `hikari5` · **World (data):** 7
- **Objective:** Install day in Kichijoji. BxE certification does not accept 'close enough' — the checklist wants every reading inside a 1 dB certification window, and the record follows you. A mechanical splice will not survive this. Fuse it.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-13.6, -13] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: fusion splice → Rx ≈ -13.44 dBm (in window); mechanical splice → Rx ≈ -13.76 dBm (out of window).
- **Teaches:** Certification discipline: BxE certification windows are tighter than service windows (here 0.6 dB). 'Works' and 'certified' are different bars, and the record follows the tech.

</details>

<details>
<summary><strong>hikari6 — Slow Portal</strong></summary>

- **Level ID:** `hikari6` · **World (data):** 7
- **Objective:** The portal is crawling — forty-five seconds a page, and the customer is watching. You cached this device ID last visit. Skip the search entirely: log in and hit the diagnostics API direct. The slow path is for people with time.
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `bxe`. Choices offered: Login, SearchAddress, ListDevices, ReadDiagnostics, RunSpeedTest, CertifyInstall (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **Login → ReadDiagnostics**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Teaches:** The direct-API fast path: with a cached device ID, login → diagnostics skips the slow portal search. Knowing when the shortcut is legitimate is workflow fluency, not corner-cutting.

</details>

<details>
<summary><strong>hikari7 — Bend in the Wall</strong></summary>

- **Level ID:** `hikari7` · **World (data):** 7
- **Objective:** High loss on an in-wall drop in an old Machida house, and the portal cannot tell you where. Two suspects: a kink where someone stapled the cable too tight, or a bad termination at the wall box. Diagnose it like a tech, not a guesser — re-terminate clean.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-10 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-13, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: fusion splice → Rx ≈ -10.36 dBm (in window); macrobend (4 dB excess loss) → Rx ≈ -14.28 dBm (out of window).
- **Notes:** Data note: the briefing names two suspects, but the board's choices on the open edge are a fusion splice or the 4 dB macrobend itself — there is no separate 'replace the kinked section' component. The winnable play is the clean fusion splice; treat the level as re-termination practice with the bend as the failure you refuse to ship.
- **Teaches:** Fault isolation with two suspects: a kink (macrobend) versus a bad termination present the same symptom. The level's honest limit: the board offers a clean splice or the bend itself — re-termination is the test the level can actually run.

</details>

<details>
<summary><strong>hikari8 — The Splitter Closet</strong></summary>

- **Level ID:** `hikari8` · **World (data):** 7
- **Objective:** Eight units in a Shinjuku apartment block, one closet, one shot. Too small a split and you overdrive the ONTs into distortion; too big and the far units starve. Size the splitter for exactly what the building needs.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (1 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-13, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: 1:4 splitter (7.3 dB) → Rx ≈ -7.70 dBm (out of window); 1:8 splitter (10.6 dB) → Rx ≈ -11.00 dBm (in window); 1:16 splitter (13.7 dB) → Rx ≈ -14.10 dBm (out of window); 1:32 splitter (17.7 dB) → Rx ≈ -18.10 dBm (out of window).
- **Teaches:** Splitter sizing for a building: match the split to the unit count — undersized overdrives, oversized starves. The same PON math as the fiber track, at closet scale.

</details>

<details>
<summary><strong>hikari9 — Night Trouble Call</strong></summary>

- **Level ID:** `hikari9` · **World (data):** 7
- **Objective:** 2 AM in Chofu. A business customer is dark, rain is starting, and the splice closure up the pole is taking water. Get the drop re-terminated and verified before the intrusion climbs — the clock is real and the margin is thin.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (-8 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-13.5, -8] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 1→2: UPC connector → Rx ≈ -12.55 dBm (in window); UPC connector, +2 dB contamination → Rx ≈ -14.55 dBm (out of window); APC connector → Rx ≈ -12.50 dBm (in window).
- **Outage:** **WaterIntrusion** fires 45 s into the level on edge 0→1; complaint timer 120 s. Degrading hazard: extra loss climbs 1 dB per 10 s, capped at 15 dB, until resolved.
- **Teaches:** Night trouble-call work: aerial plant, incoming weather, a closure taking water, and a thin margin — clean termination choice under time pressure.

</details>

<details>
<summary><strong>hikari10 — Master Tech</strong></summary>

- **Level ID:** `hikari10` · **World (data):** 7
- **Objective:** The master-tech practical: a full install in Yokohama, start to finish. Run the complete BxE workflow — log in, find the customer, read the diagnostics, prove the speed, certify it. Then build the drop to certification tolerance. Nothing left unproven.
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `bxe`. Choices offered: Login, SearchAddress, ListDevices, ReadDiagnostics, RunSpeedTest, CertifyInstall, Logout (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **Login → SearchAddress → ListDevices → ReadDiagnostics → RunSpeedTest → CertifyInstall**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Player choices:** edge 1→2: fusion splice → Rx ≈ -12.88 dBm (in window); mechanical splice → Rx ≈ -13.20 dBm (out of window).
- **Teaches:** The complete field workflow as a practical exam: full BxE sequence (through speed test and certification) plus a drop built to certification tolerance.

</details>
