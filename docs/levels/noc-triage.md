# AMS NOC Triage — Aino (Nokia)

Aino works the network operations center: a Nokia AMS (Access Management System) alarm board and its SOAP northbound interface (NBI). The track teaches the two NOC disciplines in turn. Triage: acknowledge alarms in exact priority order — Critical, then Major, then Minor, with informational Warnings last — and learn to ack the root cause of a cascade before its downstream symptoms. Query: drive the NBI in the correct order (authenticate, enumerate managed elements, pull the element, read its status), including supervision and maintenance-mode discipline around planned work.

How winning works in this game (all tracks): the live win check in `game/src/states/playing.rs` is a conjunction — the board evaluation for the level's medium must pass **and** every console or Astra gate the level carries (API sequence, alarm triage, identification, jumper swap, workbench, static config, survey, handoff) must pass. Levels with a scripted outage skip the plain win check entirely and can only be won through the outage-resolution path, which applies the same conjunction against the post-outage plant. Each level section below names every gate that applies to it.

Scoring on this track: triage and API sequences are exact ordered matches (`verify_triage_order`, `verify_api_sequence`), fail-closed on empty or partial sequences; a wrong pick raises the console's wrong-pick counter but never fails the level outright — the lesson is the order, not punishment. Honesty note: these levels carry `medium: Fiber` and −27…−8 windows in their data, but they are won through triage/board logic, not the fiber window as a skill test — most boards are a single token 0.1 km span that is in-window by construction once the consoles are satisfied. Two levels (aino6, aino10) carry a genuine board-placement gate: a +3 dBm launch into a −27…−8 window that only a splitter swap on the second leg can land — see their sections, and the surprises note in the levels index.

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>aino1 — First Shift</strong></summary>

- **Level ID:** `aino1` · **World (data):** 6
- **Objective:** Welcome to the Nokia NOC, Aino's desk. One alarm on the board: a fiber cut. Click it to acknowledge -- that's the whole job tonight. The alarms never lie. Learn to read them.
- **Mechanic / evaluator:** Alarm-triage console (`states/triage_console.rs`): alarms must be acknowledged in exactly the expected priority order (`verify_triage_order` — exact ordered match, fail-closed). Alarms: #0 [Critical] Fiber cut: edge 3-7 (backhoe).
- **Pass thresholds / scoring:** Win = ack order **[0]** exactly, over the (already-lit) board. A wrong pick bumps the wrong-pick counter but never fails the level outright.
- **Teaches:** Alarm literacy: a NOC board's alarms are the primary source. Acknowledging in the console is the operator's record that the fault is owned.

</details>

<details>
<summary><strong>aino2 — Triage Order</strong></summary>

- **Level ID:** `aino2` · **World (data):** 6
- **Objective:** Three alarms, one pair of hands. Critical outranks Major outranks Minor -- ack them in severity order. The board can wait; triage cannot.
- **Mechanic / evaluator:** Alarm-triage console (`states/triage_console.rs`): alarms must be acknowledged in exactly the expected priority order (`verify_triage_order` — exact ordered match, fail-closed). Alarms: #0 [Critical] Fiber cut: edge 3-7; #1 [Major] Signal degrade: ONT-4412 (-25 dBm); #2 [Minor] Syslog noise: auth failures on NE-7.
- **Pass thresholds / scoring:** Win = ack order **[0, 1, 2]** exactly, over the (already-lit) board. A wrong pick bumps the wrong-pick counter but never fails the level outright.
- **Teaches:** Severity-first triage: Critical before Major before Minor. Order is the skill — the board is worked by priority, not by arrival or by convenience.

</details>

<details>
<summary><strong>aino3 — The Cascade</strong></summary>

- **Level ID:** `aino3` · **World (data):** 6
- **Objective:** A backhoe found our fiber. One cut, five alarms -- the downstream ONTs are screaming LOS but they're symptoms, not causes. Ack the root cause first, then work down by severity. Never chase symptoms.
- **Mechanic / evaluator:** Alarm-triage console (`states/triage_console.rs`): alarms must be acknowledged in exactly the expected priority order (`verify_triage_order` — exact ordered match, fail-closed). Alarms: #0 [Critical] Fiber cut: edge 3-7 (ROOT CAUSE); #1 [Major] LOS: ONT-4412 (downstream of cut); #2 [Major] LOS: ONT-4413 (downstream of cut); #3 [Minor] Syslog: flap on NE-7; #4 [Warning] Info: backup path active.
- **Pass thresholds / scoring:** Win = ack order **[0, 1, 2, 3, 4]** exactly, over the (already-lit) board. A wrong pick bumps the wrong-pick counter but never fails the level outright.
- **Teaches:** Root cause versus symptoms: one fiber cut raises many downstream LOS alarms. Acking the root cause first is correlation — the discipline that keeps a cascade readable.

</details>

<details>
<summary><strong>aino4 — Bulk Ack</strong></summary>

- **Level ID:** `aino4` · **World (data):** 6
- **Objective:** Storm rolled through and the board lit up like a Christmas tree. Six alarms. Bulk-ack them in priority order -- Criticals, then Majors, then everything else. Speed matters; order matters more.
- **Mechanic / evaluator:** Alarm-triage console (`states/triage_console.rs`): alarms must be acknowledged in exactly the expected priority order (`verify_triage_order` — exact ordered match, fail-closed). Alarms: #0 [Critical] Fiber cut: edge 3-7; #1 [Critical] Power alarm: headend UPS on battery; #2 [Major] LOS: ONT-4412; #3 [Major] LOS: ONT-4413; #4 [Minor] High temp: NE-7; #5 [Warning] Info: config backup complete.
- **Pass thresholds / scoring:** Win = ack order **[0, 1, 2, 3, 4, 5]** exactly, over the (already-lit) board. A wrong pick bumps the wrong-pick counter but never fails the level outright.
- **Teaches:** Bulk triage under storm load: six simultaneous alarms still sort by severity; two Criticals lead, informational items close the order.

</details>

<details>
<summary><strong>aino5 — Query the NBI</strong></summary>

- **Level ID:** `aino5` · **World (data):** 6
- **Objective:** The board is quiet but a customer says they're down. Drive the AMS NBI (SOAP on :8443): log in, list the NEs, pull the suspect record, read its status. The right call order finds the fault.
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `ams`. Choices offered: Login, GetAllManagedElements, GetManagedElement, GetNeStatus, StartSupervision, DeleteSubscriber (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **Login → GetAllManagedElements → GetManagedElement → GetNeStatus**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Notes:** Honesty note: a pure AMS console level — no triage block and no board decision beyond a token span. The API sequence is the level.
- **Teaches:** Driving the AMS NBI: Nokia AMS exposes a SOAP northbound interface — authenticate, enumerate managed elements, pull the suspect element, read its status. Enumeration before inspection.

</details>

<details>
<summary><strong>aino6 — Supervision</strong></summary>

- **Level ID:** `aino6` · **World (data):** 6
- **Objective:** NE-12 needs a card swap. Stop supervision first (never work on a supervised NE), run the test action, then start supervision again. And the backup path needs a span placed -- route it.
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `ams`. Choices offered: StopSupervision, ExecuteNeAction, StartSupervision, Login, GetNeStatus, RebootOnt (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **StopSupervision → ExecuteNeAction → StartSupervision**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Player choices:** edge 0→1: span 1 km (buried) — the feeder leg, always placed | edge 1→2: span 1 km (buried) → Rx ≈ +2.30 dBm (TOO HOT) | splitter 1:4 → Rx ≈ −4.65 dBm (TOO HOT) | splitter 1:8 → Rx ≈ −7.95 dBm (TOO HOT — 0.05 dB over the ceiling) | splitter 1:16 → Rx ≈ −11.05 dBm (in window).
- **Notes:** Combined level: the AMS sequence (stop supervision → execute action → start supervision; note there is deliberately no Login step in the expected sequence) plus a board gate — the backup path's two 1 km buried spans must both be placed. **Board-budget fix (2026-10-07):** as originally authored the only components offered were the two spans, and the completed path budgeted to +3 − 2 × 0.35 = **+2.30 dBm** — above the level's own −27…−8 dBm window ceiling, so the board half could not pass. The second leg now also offers splitter ratios (1:4 / 1:8 / 1:16): choosing split ratio to land a hot launch is the PON design skill the level teaches. Only the 1:16 lands (−11.05 dBm); the 1:8 misses the ceiling by 0.05 dB — close is not in-window. The console sequence is unaffected.
- **Teaches:** Supervision discipline: stop supervision before working on a network element, run the action, restore supervision after. Working on a supervised element — or forgetting to restore supervision — are both real-world faults.

</details>

<details>
<summary><strong>aino7 — Night Shift</strong></summary>

- **Level ID:** `aino7` · **World (data):** 6
- **Objective:** 02:00 and the board won't stop. Eight alarms -- but two are just informational Warnings. Ack the real problems first (Critical, Major, Minor), leave the info noise for last. Triage is telling signal from noise.
- **Mechanic / evaluator:** Alarm-triage console (`states/triage_console.rs`): alarms must be acknowledged in exactly the expected priority order (`verify_triage_order` — exact ordered match, fail-closed). Alarms: #0 [Critical] Fiber cut: edge 12-3; #1 [Major] Aerial damage: storm branch; #2 [Major] LOS: ONT-8821; #3 [Minor] Water intrusion: closure 7; #4 [Minor] High temp: NE-12; #5 [Warning] Info: nightly backup started; #6 [Warning] Info: firmware check OK; #7 [Minor] Syslog: flap on NE-3.
- **Pass thresholds / scoring:** Win = ack order **[0, 1, 2, 3, 4, 7, 5, 6]** exactly, over the (already-lit) board. A wrong pick bumps the wrong-pick counter but never fails the level outright.
- **Teaches:** Signal versus noise at scale: eight alarms where two are informational Warnings that sort last — and one Minor that still outranks them. Triage is ranking everything, not just the loud items.

</details>

<details>
<summary><strong>aino8 — Maintenance Window</strong></summary>

- **Level ID:** `aino8` · **World (data):** 6
- **Objective:** Planned work on edge 5-2. Put the NE in maintenance mode through the NBI (silences the alarms while you work), check status, then take it back out. Four alarms came in anyway -- triage them too.
- **Mechanic / evaluator:** Combined console level: alarm triage (`verify_triage_order`) **and** an API sequence (`verify_api_sequence`, ams) must both complete, over the board check. Alarms: #0 [Major] Planned work: edge 5-2; #1 [Minor] Syslog: maint window open; #2 [Warning] Info: supervision paused; #3 [Minor] High temp: NE-5.
- **Pass thresholds / scoring:** Triage order **[0, 1, 3, 2]** exactly, and API sequence **EnableMaintenanceMode → GetNeStatus → DisableMaintenanceMode** exactly. Both gates, plus the board where the level has open edges.
- **Notes:** Combined level: AMS maintenance-mode sequence and a four-alarm triage. Note the triage order is not pure severity order: the expected order is [0, 1, 3, 2] — the two Minor items are acked before the informational Warning, which always sorts last.
- **Teaches:** Maintenance mode: silencing alarms deliberately during planned work (enable, verify status, disable) while still triaging what arrives — planned work does not suspend judgment.

</details>

<details>
<summary><strong>aino9 — SOAP Fault</strong></summary>

- **Level ID:** `aino9` · **World (data):** 6
- **Objective:** Your first query faulted: EXCPTENTITYNOTFOUND -- the NE record isn't where you looked. Recover like a pro: log in, pull the right managed element, read its status. The distractors are real fault codes; don't click the fault, fix it.
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `ams`. Choices offered: Login, GetManagedElement, GetNeStatus, GetAllManagedElements, CreateService, RebootOnt (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **Login → GetManagedElement → GetNeStatus**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Teaches:** Fault recovery: an entity-not-found SOAP fault means the query targeted the wrong element, not that the network is down. Recover by re-querying the right managed element.

</details>

<details>
<summary><strong>aino10 — All Clear</strong></summary>

- **Level ID:** `aino10` · **World (data):** 6
- **Objective:** The big one: a fiber cut cascaded five alarms, the NOC needs the faulty NE identified through the NBI, and the protection path needs routing. Triage, query, repair -- in that order. Clear the board and go home.
- **Mechanic / evaluator:** Combined console level: alarm triage (`verify_triage_order`) **and** an API sequence (`verify_api_sequence`, ams) must both complete, over the board check. Alarms: #0 [Critical] Fiber cut: edge 9-1 (ROOT CAUSE); #1 [Major] LOS: ONT-9901 (downstream); #2 [Major] LOS: ONT-9902 (downstream); #3 [Minor] Aerial damage: branch on span; #4 [Warning] Info: protection path available; #5 [Minor] Syslog: flap on NE-9.
- **Pass thresholds / scoring:** Triage order **[0, 1, 2, 3, 5, 4]** exactly, and API sequence **Login → GetAllManagedElements → GetNeStatus** exactly. Both gates, plus the board where the level has open edges.
- **Player choices:** edge 0→1: span 2 km (aerial) — the feeder leg, always placed | edge 1→2: span 2 km (aerial) → Rx ≈ +2.16 dBm (TOO HOT) | splitter 1:4 → Rx ≈ −4.72 dBm (TOO HOT) | splitter 1:8 → Rx ≈ −8.02 dBm (in window, no headroom) | splitter 1:16 → Rx ≈ −11.12 dBm (in window — the engineered answer).
- **Notes:** Triage subtlety, documented because it teaches: the expected order [0, 1, 2, 3, 5, 4] places a Minor syslog flap (id 5) ahead of the Warning 'protection path available' (id 4). Severity order governs, and Warning/informational always closes the board. **Board-budget fix (2026-10-07):** the board half originally had the same defect as aino6 — at tx +3 dBm, 1550 nm, the only completable path (two 2 km spans) budgeted to +3 − 4 × 0.21 = **+2.16 dBm**, above the −27…−8 dBm window ceiling, so the board check could not pass as authored. The second leg now also offers splitter ratios (1:4 / 1:8 / 1:16). Two land: the 1:8 at −8.02 dBm (in window with no headroom to spare) and the 1:16 at −11.12 dBm (the engineered answer). The triage and NBI halves are unaffected.
- **Teaches:** Capstone NOC work: triage a five-alarm cascade (expected order is severity with the root cause first — note one Minor syslog item outranks the informational Warning), identify the faulty element through the NBI, and route the protection path.

</details>
