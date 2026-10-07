# CMS Provisioning — Clara (Calix)

Clara's track is service provisioning on a Calix PON: the physical plant shrinks to a single decision — which splitter to hang — and the real work becomes verification. Every subscriber on the cabinet has a demanded service profile, and every ONT's received power must land inside its own profile's window behind the splitter you chose. GPON profiles (GPON-100, GPON-500) accept −27…−8 dBm; XGS-1000 accepts the tighter −26…−9 dBm and overloads a decibel sooner — the difference decides several levels. Three levels leave the cabinet entirely for the management plane: the CMS northbound interface (NBI, SOAP/XML) and its SMx successor (REST/JSON), where the win is an exact sequence of console operations in the correct order.

How winning works in this game (all tracks): the live win check in `game/src/states/playing.rs` is a conjunction — the board evaluation for the level's medium must pass **and** every console or Astra gate the level carries (API sequence, alarm triage, identification, jumper swap, workbench, static config, survey, handoff) must pass. Levels with a scripted outage skip the plain win check entirely and can only be won through the outage-resolution path, which applies the same conjunction against the post-outage plant. Each level section below names every gate that applies to it.

Scoring on this track: `verify_provisioning` computes each subscriber as Rx = tx − splitter insertion loss − 0.28 dB/km × drop distance − any live outage loss, checked per profile. The splitter's port count is a hard gate (subscribers > branches fails closed), and unknown profiles or bad inputs fail closed rather than passing. Honesty note for the whole file: these levels carry `medium: Fiber` and the standard −27…−8 level window in their data, but they are **not** won through the fiber window — provisioning verification (and, where present, the API sequence) replaces the single-path check. The level window shown in each section is the data value, kept for completeness.

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>clara1 — First Turn-Up</strong></summary>

- **Level ID:** `clara1` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** Clara's first turn-up, straight off the EXOS playbook. The office already created the nine subscribers — registration IDs issued, GPON and XGS profiles assigned. Your job in the field: hang the splitter in cabinet 7+20, then verify every ONT's received power lands inside its service window before you call it done. Count the ports before you count the decibels — an 8-port splitter cannot serve nine homes.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels.
- **Pass thresholds / scoring:** All **9 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32); XGS-1000: -26…-9 dBm (1000/500 Mbps, max split 1:64). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:4 splitter (7.3 dB); 1:8 splitter (10.6 dB); 1:16 splitter (13.7 dB).
- **Teaches:** Calix CMS / EXOS turn-up discipline: count splitter ports against subscribers first, then verify every ONT's received power against its own service-profile window — GPON tiers use −27…−8 dBm; XGS-1000 uses the tighter −26…−9 dBm.

</details>

<details>
<summary><strong>clara2 — Storm Season</strong></summary>

- **Level ID:** `clara2` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** Storm season on a tired OLT. Six subscribers, water in the feeder splice, and the clock is running — this is the 'new install, no solid green LED' scenario from the EXOS field guide. Right-size the split for six homes, get everyone in window, and hold the plant together while the fault degrades. Check the NOC console (Aino's board) if the alarms start stacking.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels.
- **Pass thresholds / scoring:** All **6 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:4 splitter (7.3 dB); 1:8 splitter (10.6 dB).
- **Outage:** **WaterIntrusion** fires 25 s into the level on edge 0→1; complaint timer 120 s. Degrading hazard: extra loss climbs 1 dB per 10 s, capped at 15 dB, until resolved.
- **Teaches:** Turn-up under a degrading feeder: a right-sized split plus headroom management while water intrusion eats the budget in real time.

</details>

<details>
<summary><strong>clara3 — Dead Box Swap</strong></summary>

- **Level ID:** `clara3` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** Lightning took the ONT at 31 Oak. The swap box is on the truck — a GigaPro, so 31 Oak goes out the door as XGS-1000. Same reg ID (OAK31X), fresh activation. Seven homes on this cabinet; the truck only stocks 1:4 and 1:8 splits. Count the ports first.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels.
- **Pass thresholds / scoring:** All **7 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32); XGS-1000: -26…-9 dBm (1000/500 Mbps, max split 1:64). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:4 splitter (7.3 dB); 1:8 splitter (10.6 dB).
- **Teaches:** Like-for-like-plus swaps: a replacement ONT can change the service tier (here to XGS-1000) while keeping its registration ID — records, ports, and windows all have to be rechecked after any swap.

</details>

<details>
<summary><strong>clara4 — Profile Audit</strong></summary>

- **Level ID:** `clara4` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** The ticket says six GPON homes. The ticket is wrong — the Clinic was upgraded to XGS-1000 last month and nobody updated the record. XGS overloads a full dB sooner than GPON. The close ONT on a hot split will cook. Audit the profiles, then pick the split that keeps every window happy — including the one the ticket forgot.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels.
- **Pass thresholds / scoring:** All **6 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32); XGS-1000: -26…-9 dBm (1000/500 Mbps, max split 1:64). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:8 splitter (10.6 dB); 1:16 splitter (13.7 dB).
- **Teaches:** Trust records, verify anyway: an unrecorded tier upgrade changes the overload ceiling by 1 dB and flips the correct splitter choice. Profile audits precede design.

</details>

<details>
<summary><strong>clara5 — Drive the NBI</strong></summary>

- **Level ID:** `clara5` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** The fiber's already lit — this one's all keyboard. Forty ONTs to turn up and the CMS desktop would take all day. Drive the NBI instead: log in, read the ONT, create the service, verify it stuck. Wrong calls raise alarms on Aino's board, so get the order right. (CMS NBI: XML/SOAP on :18080.)
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `nbi`. Choices offered: Login, ShowOnt, CreateService, VerifyService, RebootOnt, Logout (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **Login → ShowOnt → CreateService → VerifyService**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Notes:** Honesty note: this level has no subscribers and no splitter choice — the board underneath is a token 0.1 km span and the level is won entirely through the API console. Its `medium: Fiber` data and −27…−8 window do not decide anything.
- **Teaches:** Driving a northbound interface (NBI): the CMS NBI speaks SOAP/XML, and bulk work follows a strict session discipline — authenticate, read, create, verify. The read-before-write and verify-after-write steps are what make automation safe.

</details>

<details>
<summary><strong>clara6 — Bulk Turn-Up</strong></summary>

- **Level ID:** `clara6` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** Four homes, one cabinet, and the office wants them all live through the NBI — no desktop clicking. Hang the right split, then drive the bulk sequence: log in, read each ONT, create the services, verify, log out. A reboot mid-sequence will alarm the NOC, so keep your clicks clean.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels. Additionally the API console sequence must match exactly (`verify_api_sequence`, nbi): Login → ShowOnt → CreateService → VerifyService → Logout. Wrong picks raise alarms but never fail the level outright.
- **Pass thresholds / scoring:** All **4 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:4 splitter (7.3 dB); 1:8 splitter (10.6 dB).
- **Teaches:** Bulk turn-up: the same NBI session discipline applied to a whole cabinet, combined with the physical splitter choice — console correctness does not excuse a wrong split.

</details>

<details>
<summary><strong>clara7 — SMx Lifecycle</strong></summary>

- **Level ID:** `clara7` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** AXOS OLT in the cabinet — that means SMx, not the old CMS NBI. REST/JSON on :18443, no sessions, no SOAP. Walk the subscriber lifecycle: log in, create the subscriber, pre-provision the ONT, create the service, verify. (Careful: SMx is stateless — hitting Logout here is a tell that you're thinking in SOAP.)
- **Mechanic / evaluator:** API-sequence console (`states/api_console.rs`): the player issues calls from the offered choices and the sequence must match the expected order exactly (`verify_api_sequence` — exact ordered match, fail-closed on empty). API tag: `smx`. Choices offered: Login, CreateSubscriber, CreateOnt, CreateService, DeleteSubscriber, VerifyService, Logout (includes distractors).
- **Pass thresholds / scoring:** Win = exact sequence **Login → CreateSubscriber → CreateOnt → CreateService → VerifyService**, with the board win check still applying underneath. Wrong picks increment an alarm counter and give feedback; they never fail the level outright.
- **Notes:** Honesty note: same shape as clara5 — a pure console level (SMx). No subscribers, no board decision; the sequence is the level.
- **Teaches:** SMx versus NBI: SMx is REST/JSON and stateless (Basic auth per request, no session, no logout). Using the wrong mental model — reaching for Logout — is itself the diagnostic.

</details>

<details>
<summary><strong>clara8 — Building Turn-Up</strong></summary>

- **Level ID:** `clara8` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** SmartMDU property: twelve units, one GigaSpire per door, all backhauled on PON. The Field Service app has every system associated to its unit — your job is the headend math. Twelve homes, one split: count the ports before you count the decibels.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels.
- **Pass thresholds / scoring:** All **12 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:8 splitter (10.6 dB); 1:16 splitter (13.7 dB).
- **Teaches:** MDU turn-up: twelve units on one split is port counting plus per-unit budget verification at building scale (SmartMDU-style association of systems to units).

</details>

<details>
<summary><strong>clara9 — Tight Budget</strong></summary>

- **Level ID:** `clara9` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** Sixteen drops on one split and the OLT is running hot at +5 dBm. The lobby AP is XGS, one kilometer out — on a small split it will overload before the far units even wake up. This is the backwards one: you need MORE loss, not less. Size the split for the hottest ONT, then check the farthest.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels.
- **Pass thresholds / scoring:** All **16 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32); XGS-1000: -26…-9 dBm (1000/500 Mbps, max split 1:64). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:16 splitter (13.7 dB); 1:32 splitter (17.7 dB).
- **Teaches:** Overload-first design: with a hot OLT, the binding constraint is the nearest, hottest ONT — size the split for it, then confirm the farthest ONT still clears its floor. Sometimes the correct design adds loss.

</details>

<details>
<summary><strong>clara10 — Night Cutover</strong></summary>

- **Level ID:** `clara10` · **Companion:** Clara (CMS provisioning) · **World (data):** 5
- **Objective:** The expert cutover. Fourteen drops on one split, the OLT running hot at +6 dBm, and a storm cell sitting over the feeder route. The NOC uplink is XGS at two kilometers — on a small split it will overload before the far units wake up. Hang the split that tames the hot ONT, drive the NBI bulk turn-up, and hold the plant together when the water gets in. Everything you've learned, one level.
- **Mechanic / evaluator:** Provisioning verification (`verify_provisioning` in `game/src/level.rs`): the player places one splitter; every subscriber's received power is computed as Rx = tx − splitter insertion loss − 0.28 dB/km × distance − outage loss (1490 nm plant) and checked against that subscriber's own service-profile window. Port count is a hard gate: a splitter with fewer branches than subscribers fails regardless of levels. Additionally the API console sequence must match exactly (`verify_api_sequence`, nbi): Login → ShowOnt → CreateService → VerifyService → Logout. Wrong picks raise alarms but never fail the level outright.
- **Pass thresholds / scoring:** All **14 subscribers** in window simultaneously. Profiles: GPON-100: -27…-8 dBm (100/25 Mbps, max split 1:32); XGS-1000: -26…-9 dBm (1000/500 Mbps, max split 1:64). A single subscriber too hot or too cold fails the level.
- **Player choices:** edge 0→1: 1:16 splitter (13.7 dB); 1:32 splitter (17.7 dB).
- **Outage:** **WaterIntrusion** fires 30 s into the level on edge 0→1; complaint timer 120 s. Degrading hazard: extra loss climbs 1 dB per 10 s, capped at 15 dB, until resolved.
- **Teaches:** Night-cutover synthesis: fourteen subscribers, a hot OLT, a bulk NBI sequence, and a live water-intrusion fault — provisioning, API discipline, and plant judgment in one window of work.

</details>
