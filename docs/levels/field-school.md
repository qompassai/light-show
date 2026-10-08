# Field School — Séraphine (Scenario Levels)

Field School is Séraphine's scenario shelf: three levels that live outside the 80-level track freeze, in their own registry (`SCENARIO_SOURCES` in `game/src/level.rs`, addressed by id — fj1, fj2, sp1 — rather than by track position), and reached from the companion-select Field School row. They are longer, more literal jobs than the track levels: a commercial fiber repair call worked in two legs (fj1, fj2), and a splice-tray repair scored against a TIA-598 work order (sp1). Scenario results never advance a companion track; they stand as practical exams for the fiber discipline. The scenarios are described generically here, as commercial field work — the readings quoted are the game's own verified values.

How winning works in this game (all tracks): the live win check in `game/src/states/playing.rs` is a conjunction — the board evaluation for the level's medium must pass **and** every console or Astra gate the level carries (API sequence, alarm triage, identification, jumper swap, workbench, static config, survey, handoff) must pass. Levels with a scripted outage skip the plain win check entirely and can only be won through the outage-resolution path, which applies the same conjunction against the post-outage plant. Each level section below names every gate that applies to it.

Scoring: all three scenarios use the standard fiber budget at 1490 nm. Their windows are narrower than the early track's because the plant is mostly fixed — the player's few choices (which splice, which splitter leg) carry the whole outcome, and partial repairs land measurably short, as the verified readings in each section show.

Navigation: [Levels index](README.md) · [Characters](../characters/README.md)

---

<details>
<summary><strong>fj1 — The Brewhouse Job — Replace the 1x4</strong></summary>

- **Level ID:** `fj1` · **World (data):** 1
- **Objective:** Field school, real job. A brewhouse customer is up on a dying plant: the meter reads about -25 dBm everywhere it matters. The 1x4 at the SB is in poor condition, port 4's yellow tail is smashed and cut, and the VFL shows light bleeding at the splice and in the white feed into the splitter's center block. Two repairs decide this job: re-splice the bleed, and replace the 1x4. Do both and prove it at the ONT — about -17.65 dBm is what a healthy port reads here. Do only one and the customer stays dark.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (3 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-19, -16.5] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 6→7: mechanical splice → Rx ≈ -22.17 dBm (out of window); fusion splice → Rx ≈ -17.65 dBm (in window) | edge 7→8: 1:4 splitter (7.3 dB) | edge 7→9: 1:4 splitter (7.3 dB).
- **Notes:** Verified readings (game data): full repair — fusion re-splice at the bleed **and** the new 1×4 — reads **−17.65 dBm** at the ONT, inside the −19.0…−16.5 window. Either repair alone leaves the reading below the window (TOO LOW): the partial fixes fail on level, not on effort. Routing through the old 1×4's smashed port-4 tail instead of the new splitter also fails. This is a scenario level (`SCENARIO_SOURCES`, id-addressed), not a track level: it does not advance any companion track.
- **Teaches:** Commercial fiber repair method: prove the plant with measurements (the as-found reading is about −25 dBm), replace the damaged splitter rather than working around it, re-splice the light bleed found with a visual fault locator, and prove the repair at the ONT.

</details>

<details>
<summary><strong>fj2 — The Brewhouse Job — Prove the SB</strong></summary>

- **Level ID:** `fj2` · **World (data):** 1
- **Objective:** Same job, second leg — and the survey lesson that saves the next tech. The SB drop reads -25.45 dBm as found. Do not let the site fool you: the legacy carrier's box on the south side of this building is NOT the demarc — the real demarc is on the roof, up the aerial path, and this MST is aerial, not the buried can in the vault. Brick and mortar everywhere: the follow-up install runs a 75-100 ft aerial drop along the legacy path, masonry bits required, and lands on an unmanaged switch inside because the customer runs their own routers. Your work here: same discipline as the ports. Clear the bleed, replace what is damaged, and prove the SB leg at about -18.03 dBm.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (3 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-19.5, -17] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 7→8: mechanical splice → Rx ≈ -22.63 dBm (out of window); fusion splice → Rx ≈ -18.03 dBm (in window) | edge 8→9: 1:4 splitter (7.3 dB) | edge 8→10: 1:4 splitter (7.3 dB).
- **Notes:** Verified reading (game data): the fully repaired SB leg reads **−18.03 dBm**, inside the −19.5…−17.0 window; partial repairs read below it. The as-found SB drop reads −25.45 dBm. Scenario level, Séraphine / Field School — see fj1's note on scenario status.
- **Teaches:** Site-survey discipline: identify the real demarcation point and the real plant type (aerial versus buried) before designing the follow-up work — the obvious box on the building is not always the demarc, and the follow-up drop (75–100 ft of aerial plant, masonry fixings, customer-owned routers behind an unmanaged switch) is planned from the survey, not assumed.

</details>

<details>
<summary><strong>sp1 — True to the Color</strong></summary>

- **Level ID:** `sp1` · **World (data):** 1
- **Objective:** Repair ticket: the splice at the secondary splitter is open on the tray and the drops downstream are dark. The plant: primary 1x4 upstream, this secondary 1x8 feeding the building. Your repair is only correct when every leg is spliced true to the TIA-598 color code — a neat splice to the wrong strand lights the wrong home and leaves yours dark. Take the chart (it lives in the work order, and it stays open): fiber identity is a PAIR, tube color x strand color, number = (tube - 1) x 12 + strand. Verify the pair, then verify the light. A clean fusion splice at the tray keeps the ONT in window at about -18.1 dBm; a mechanical gets you home poorer; forcing a damaged strand kinks 3.5 dB out of the budget and fails.
- **Mechanic / evaluator:** Optical link budget (`compute_link_budget_with_outage` in osp_sim): Rx = tx (3 dBm) minus fiber attenuation at 1490 nm (0.28 dB/km), splice losses (fusion 0.075 dB, mechanical 0.4 dB), connector losses (UPC 0.35 dB, APC 0.30 dB, plus any contamination), splitter and macrobend losses along the placed source→target path. Verification states: Continuity + Carrier Level.
- **Pass thresholds / scoring:** Rx inside **[-19.5, -16.5] dBm**, inclusive. Below the floor reads TOO LOW (starved receiver); above the ceiling reads too hot (overload). An open path fails Continuity and cannot pass.
- **Player choices:** edge 3→4: fusion splice → Rx ≈ -18.12 dBm (in window); mechanical splice → Rx ≈ -18.45 dBm (in window); macrobend (3.5 dB excess loss) → Rx ≈ -21.55 dBm (out of window).
- **Notes:** Verified readings (game data): **two** repair methods win. A fusion splice at the tray reads **−18.125 dBm**; a mechanical splice reads **−18.45 dBm** — both inside the −19.5…−16.5 window. Forcing the damaged strand (the 3.5 dB macrobend option) fails. Scoring honesty from the work-order data itself: the `splice_work_orders` block (TIA-598 chart, three beats, damage beats naming spares fiber 60 and fiber 120) is fully typed, parsed, and validated against the color rule today, but the strand-by-strand splicing UI that would score it interactively is future work — the shipped win check is the light-path budget above. The beats are documented here because they are the repair's work order: Beat 1 'One tube, twelve strands' (strand color alone is identity in the blue tube), Beat 2 'Three tubes — color alone starts lying' (fiber 2 vs fiber 13 transposition pair), Beat 3 'Work-order mode — numbers only' (decode tube/strand from the fiber number; exclude damaged fibers 55 and 118, splice spares 60 and 120). Scenario level, Séraphine / Field School.
- **Teaches:** TIA-598 color-code fluency: fiber identity is the (tube, strand) color pair, numbered (tube − 1) × 12 + strand across a 144-fiber chart. A mis-splice does not corrupt data — it lights the wrong premises and leaves the ordered one dark. Damaged strands are excluded and replaced with the designated spare, never forced.

</details>
