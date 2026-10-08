# Wireless — Linka

Linka's track is point-to-point wireless, taught through free-space path loss. Distance and frequency set the loss on every hop; the far receiver accepts a window of signal levels and a minimum signal-to-noise ratio on top of it. The track's central distinction is regeneration: a repeater receives and retransmits a fresh signal, erasing the accumulated damage of earlier hops, while an amplifier merely makes the damaged signal louder — noise included. Several later levels add survey work (coverage proven at named points, with the previous survey's readings kept beside yours) and static IP configuration consoles, because a closed link that nobody configured is not a finished install.

How winning works in this game (all tracks): the live win check in `game/src/states/playing.rs` is a conjunction — the board evaluation for the level's medium must pass **and** every console or Astra gate the level carries (API sequence, alarm triage, identification, jumper swap, workbench, static config, survey, handoff) must pass. Levels with a scripted outage skip the plain win check entirely and can only be won through the outage-resolution path, which applies the same conjunction against the post-outage plant. Each level section below names every gate that applies to it.

Scoring on this track: FSPL = 20·log₁₀(distance in m) + 20·log₁₀(frequency in MHz) − 27.55 dB per hop. Every level's data sets a 10 dB minimum SNR; an unresolved interference hazard raises that requirement 1 dB per 10 s (capped at 12 dB). Survey points, where present, judge RSSI (plus SNR where authored) at named locations, and on the two anchor levels (m1l9, m1l10) every point must pass for the win.

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>m1l1 — Close the Link</strong></summary>

- **Level ID:** `m1l1` · **World (data):** 3
- **Objective:** Linka's rule: distance is the enemy, and amplifiers can't beat geometry. Two 400 m hops at 2.4 GHz stand between the sites — about 92 dB of free-space loss per hop. An amplifier just shouts the wreckage louder. Place the regenerative repeater that hands the far site a fresh signal inside the [-75, -40] dBm window. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-75, -40] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 20 dBm) → RSSI ≈ -72.10 dBm (in window); repeater (retransmits at 0 dBm) → RSSI ≈ -92.10 dBm (out of window); amplifier +30 dB.
- **Teaches:** Free-space path loss (FSPL) and regeneration: an amplifier amplifies noise with signal, while a repeater receives, decides, and retransmits a fresh signal — which is why repeaters close links amplifiers cannot.

</details>

<details>
<summary><strong>m1l2 — Ride the Storm</strong></summary>

- **Level ID:** `m1l2` · **World (data):** 3
- **Objective:** The link is up on the 20 dBm repeater, but a storm front is dragging interference across the band — the floor rises about a decibel every ten seconds. Ride it out: when your signal sinks toward the bottom of the [-75, -65] dBm window, hot-swap to the 30 dBm repeater before the clock runs out. Too much too soon cooks the receiver; too little too late loses the link. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-75, -65] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 20 dBm) → RSSI ≈ -68.01 dBm (in window); repeater (retransmits at 30 dBm) → RSSI ≈ -58.01 dBm (out of window); repeater (retransmits at 10 dBm) → RSSI ≈ -78.01 dBm (out of window).
- **Outage:** **WirelessInterference** fires 15 s into the level on edge 1→2; complaint timer 100 s. Degrading hazard: the noise floor (equivalently, the SNR requirement) climbs 1 dB per 10 s, capped at 12 dB, until resolved — rebalancing, not waiting, is the repair.
- **Notes:** Survey layer (display + diagnosis, not a win gate on this level): one point (Site B) with a historical reading of −71.0 dBm / SNR 27.0 dB beside the live reading, and a diagnosis pick — the correct cause is the risen noise floor (interference); 'wrong channel' is the named trap. Committing the correct diagnosis pays the optional objective (5 Cores).
- **Teaches:** Fade margin and interference response: a rising noise floor raises the SNR the link must carry. More transmit power helps only while the receiver stays inside its window.

</details>

<details>
<summary><strong>m1l3 — Thread the Needle</strong></summary>

- **Level ID:** `m1l3` · **World (data):** 3
- **Objective:** Linka's lesson: sometimes the window is all you've got. This link has no fade margin — the receive window is just 4 dB wide. Run the path loss for the 300 m hops at 2.4 GHz and pick the repeater that lands dead center. Close doesn't count. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-72, -68] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 15 dBm) → RSSI ≈ -74.60 dBm (out of window); repeater (retransmits at 20 dBm) → RSSI ≈ -69.60 dBm (in window); repeater (retransmits at 25 dBm) → RSSI ≈ -64.60 dBm (out of window).
- **Teaches:** Link-budget precision: with a 4 dB window there is no 'close enough' — FSPL must be computed per hop and the repeater power chosen to land mid-window.

</details>

<details>
<summary><strong>m1l4 — The 5GHz Tax</strong></summary>

- **Level ID:** `m1l4` · **World (data):** 3
- **Objective:** The 5.8 GHz band is clear of interference, but physics charges a tax: free-space loss climbs with frequency. These 250 m hops lose about 96 dB each at 5.8 GHz — nearly 8 dB more than at 2.4. Don't use 2.4 GHz math on a 5.8 GHz link. Pick the repeater that closes it. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-80, -70] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 20 dBm) → RSSI ≈ -75.68 dBm (in window); repeater (retransmits at 30 dBm) → RSSI ≈ -65.68 dBm (out of window); amplifier +20 dB.
- **Teaches:** Frequency scaling of FSPL: loss grows with 20·log₁₀(f), so moving from 2.4 GHz to 5.8 GHz costs roughly 7.7 dB per hop at the same distance. Higher bands trade range for a cleaner, wider channel.

</details>

<details>
<summary><strong>m1l5 — After the Repeater</strong></summary>

- **Level ID:** `m1l5` · **World (data):** 3
- **Objective:** A short 200 m hop feeds the relay, then a long 400 m hop to the far site. Linka drills this into every tech: a regenerative repeater erases everything before it. That first hop is irrelevant — size the repeater for the 400 m hop AFTER it. That's the only math that matters. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-75, -70] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 15 dBm) → RSSI ≈ -77.10 dBm (out of window); repeater (retransmits at 20 dBm) → RSSI ≈ -72.10 dBm (in window); repeater (retransmits at 25 dBm) → RSSI ≈ -67.10 dBm (out of window).
- **Notes:** Astra gate beside the link: a static IPv4 configuration console (`states/config_console.rs`) for the office printer. Worksheet: 192.168.40.20/24, gateway 192.168.40.1, DNS 192.168.40.53. The starting configuration's only fault is the gateway (.254). Availability is judged from the address inventory and the gateway collision rule only — never from the DHCP pool (.100–.199), which is displayed precisely so it can mislead. The family must be Applied and passed to win.
- **Teaches:** Regeneration resets the budget: only the hop after the repeater determines the far-end level. What happened before the repeater is history — a core intuition for relay and mesh design.

</details>

<details>
<summary><strong>m1l6 — Loud Is Not Clear</strong></summary>

- **Level ID:** `m1l6` · **World (data):** 3
- **Objective:** Two 500 m hops, and someone left a 30 dB amplifier on the bench. Tempting — but an amplifier shouts the wreckage louder: it boosts noise with the signal, and the second 500 m hop buries it again. Only a regenerative repeater hands the far site a clean signal. Pick the power that lands in the window without overdriving it. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-75, -65] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 20 dBm) → RSSI ≈ -74.03 dBm (in window); amplifier +30 dB; repeater (retransmits at 30 dBm) → RSSI ≈ -64.03 dBm (out of window).
- **Teaches:** SNR versus RSSI: a loud signal can still be unusable. Amplifiers preserve (or worsen) SNR; only regeneration restores it. 'Loud is not clear' is the whole lesson.

</details>

<details>
<summary><strong>m1l7 — The Long Haul</strong></summary>

- **Level ID:** `m1l7` · **World (data):** 4
- **Objective:** 800 meters per hop. At 2.4 GHz that's 98 dB of free-space loss each — the edge of what this hardware can close. The window is tight because there's nowhere to hide: too little and the far site hears static, too much and you cook the receiver. Calculate, don't guess. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-80, -75] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 20 dBm) → RSSI ≈ -78.12 dBm (in window); repeater (retransmits at 25 dBm) → RSSI ≈ -73.12 dBm (out of window); repeater (retransmits at 30 dBm) → RSSI ≈ -68.12 dBm (out of window).
- **Notes:** Astra gate beside the link: the IPv6 twin of the config console. Worksheet: 2001:db8:40:1::20/64, link-local gateway fe80::1 scoped to the `business-lan` interface, DNS 2001:db8:40:1::53. The starting fault is the gateway's interface scope (`management`). A link-local gateway without the correct interface is a wrong route, and the verdict names it.
- **Teaches:** Operating at the edge of a link budget: 800 m hops at 2.4 GHz cost ≈98 dB each, leaving a narrow feasible power band between insensitivity and overload.

</details>

<details>
<summary><strong>m1l8 — Aim High</strong></summary>

- **Level ID:** `m1l8` · **World (data):** 4
- **Objective:** The 2.4 GHz band here is noisy — interference comes and goes, and every decibel of fade eats your margin. Linka's rule: in a noisy band, land HIGH in the window, not in the middle. High enough to survive a fade, low enough to stay legal. Find the repeater that does both. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-80, -66] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 20 dBm) → RSSI ≈ -69.60 dBm (in window); repeater (retransmits at 25 dBm) → RSSI ≈ -64.60 dBm (out of window); repeater (retransmits at 28 dBm) → RSSI ≈ -61.60 dBm (out of window).
- **Notes:** Survey layer (not a win gate here): one point at Site B, required −75 dBm, historical −77.5 dBm — the history says this point failed before you arrived, which is the argument for landing high in the window.
- **Teaches:** Margin placement: in a noisy band, aim high in the window to bank fade margin — deliberately trading headroom against interference and weather fades.

</details>

<details>
<summary><strong>m1l9 — Mixed Bands</strong></summary>

- **Level ID:** `m1l9` · **World (data):** 4
- **Objective:** First hop: 300 m at 2.4 GHz. Second hop: 200 m at 5.8 GHz. Different bands, different loss — you can't use one number for both. Run each hop's path loss separately, then pick the repeater that closes the 5.8 GHz leg into the window. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-75, -70] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 15 dBm) → RSSI ≈ -78.74 dBm (out of window); repeater (retransmits at 20 dBm) → RSSI ≈ -73.74 dBm (in window); repeater (retransmits at 25 dBm) → RSSI ≈ -68.74 dBm (out of window).
- **Notes:** Survey is a **win gate** on this anchor level (`required_for_win`): both points must pass — Front Desk ≥ −75 dBm and Stockroom ≥ −79 dBm (the stockroom carries 4.0 dB of authored obstruction loss on RSSI only; SNR is never adjusted). Verified winning reading: the survey winner measures **−73.74 dBm**. History at both points (−79.0 and −83.0 dBm, SNR 14.0 dB) shows why the previous install failed.
- **Teaches:** Per-band, per-point survey work: each band's FSPL is computed separately, and coverage is verified at named points — including a stockroom point carrying extra obstruction loss — not just at the far endpoint.

</details>

<details>
<summary><strong>m1l10 — Linka's Gauntlet</strong></summary>

- **Level ID:** `m1l10` · **World (data):** 5
- **Objective:** The final exam. Two 600 m hops at 5.8 GHz — over 103 dB of loss per hop, the hardest path in the track. The window is 5 dB wide. An amplifier is on the bench to tempt you, and it will fail exactly the way amplifiers always fail on long hops. Combine everything: frequency math, tight windows, repeater discipline. Close the link. Service acceptance: SNR at or above 10 dB at the far site, measured — not assumed.
- **Mechanic / evaluator:** Wireless link evaluation (`evaluate_wireless` in osp_sim): each hop loses free-space path loss, FSPL = 20·log₁₀(distance) + 20·log₁₀(frequency) − 147.55 dB. A repeater regenerates — the far-end RSSI is the repeater's transmit power minus the final hop's FSPL — while an amplifier boosts signal and noise together and cannot restore SNR. Verification states: Continuity, Link Level, Link SNR, Noise Channel, plus per-point Coverage states where a survey block is present.
- **Pass thresholds / scoring:** RSSI inside **[-80, -75] dBm** **and** carried SNR ≥ **10 dB** (the requirement rises 1 dB per 10 s under an unresolved interference hazard).
- **Player choices:** edge 1→2: repeater (retransmits at 25 dBm) → RSSI ≈ -78.28 dBm (in window); repeater (retransmits at 30 dBm) → RSSI ≈ -73.28 dBm (out of window); amplifier +30 dB.
- **Notes:** Survey is a **win gate** here too: Site B Desk ≥ −80 dBm and Far Corner Office ≥ −81 dBm (corner carries 1.5 dB extra loss). Diagnosis pick required for the optional objective (10 Cores): the true cause is a noise-limited path; 'wrong band selection' is the trap. Verified winning SNR: **16.72 dB**. Both static-config families (V4: 192.168.50.20/24, gateway .1, DNS .53; V6: 2001:db8:50:1::20/64, fe80::1 on `business-lan`) must also be Applied and passed — dual-stack means two independent verdicts, never a shared one.
- **Teaches:** Full link-budget fluency under survey and dual-stack configuration load: mixed hops, tight windows, per-point coverage acceptance, and diagnosing a noise-limited path from survey evidence.

</details>
