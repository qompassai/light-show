# Light Show — Level Test Footage

Short gameplay clips for each level, captured headless via `xvfb-run` +
`ffmpeg x11grab` using the `--footage <level-id>` harness (see
`game/src/footage.rs`, debug builds only). Each clip shows the level's
signature mechanic in action. All footage uses mature (young-adult)
companion models per policy.

## Clips

| Level | File | What it shows |
|-------|------|---------------|
| w1l1 — First Light (Séraphine) | `world1_level1/w1l1.mp4` | Fusion splice placed 1→2; GPON link budget lands in the receive window; win screen. |
| w4l1 — Storm Season (Séraphine) | `world4_level1_outage/w4l1.mp4` | Main route placed; scripted AerialDamage outage fires at 20s; protection splice 2→3 reroutes the customer. |
| c1l1 — Unity Gain (Ondine) | `coax1_unity_gain/c1l1.mp4` | 5 dB amplifier selected from three pills; carrier lands in the [0, 15] dBmV window; win screen. |
| c1l2 — Ingress at Night (Ondine) | `coax2_ingress/c1l2.mp4` | Amplifier placed; scripted IngressNoise outage fires at 15s on the 1→2 edge. |
| m1l1 — Close the Link (Linka) | `wireless1_close_the_link/m1l1.mp4` | 20 dBm repeater placed 1→2; wireless link closes; win screen. |
| m1l2 — Ride the Storm (Linka) | `wireless2_ride_the_storm/m1l2.mp4` | Repeater placed; scripted WirelessInterference storm fires at 15s. |
| e1l1 — The Hundred-Meter Wall (Lattice) | `ethernet1_hundred_meter_wall/e1l1.mp4` | Switch placed 1→2 to regenerate past the 100 m Ethernet limit (65 m + 65 m would exceed it); win screen. |
| e1l2 — Power Budget (Lattice) | `ethernet2_power_budget/e1l2.mp4` | 60 W switch placed 1→2 covering the endpoint PoE draw; win screen. |
| clara1 — First Turn-Up (Clara) | `clara1_provisioning/clara1.mp4` | 1:16 splitter selected for 9 subscribers; provisioning verification passes; win screen. |
| clara2 — Storm Season (Clara) | `clara2_outage/clara2.mp4` | 1:8 splitter for 6 subscribers; scripted WaterIntrusion outage fires at 25s on the 0→1 edge. |
| clara3 — Dead Box Swap (Clara) | `clara3_dead_box_swap/clara3.mp4` | 1:8 splitter for 7 subscribers (XGS); provisioning verification passes. |
| clara4 — Profile Audit (Clara) | `clara4_profile_audit/clara4.mp4` | 1:16 splitter beats the hidden XGS overload trap. |
| clara5 — Drive the NBI (Clara) | `clara5_drive_the_nbi/clara5.mp4` | NBI API sequence: Login → ShowOnt → CreateService → VerifyService. |
| clara6 — Bulk Turn-Up (Clara) | `clara6_bulk_turn_up/clara6.mp4` | 1:8 splitter + NBI API sequence for bulk provisioning. |
| clara7 — SMx Lifecycle (Clara) | `clara7_smx_lifecycle/clara7.mp4` | SMx REST API: Login → CreateSubscriber → CreateOnt → CreateService → VerifyService. |
| clara8 — Building Turn-Up (Clara) | `clara8_building_turn_up/clara8.mp4` | 1:16 splitter for 12 subscribers (SmartMDU). |
| clara9 — Tight Budget (Clara) | `clara9_tight_budget/clara9.mp4` | 1:32 splitter for hot OLT (+5 dBm); tight power budget. |
| clara10 — Night Cutover (Clara) | `clara10_night_cutover/clara10.mp4` | 1:16 splitter + NBI API; WaterIntrusion outage at 30s. |
| aino1 — First Shift (Aino) | `aino1_first_shift/aino1.mp4` | Alarm triage: single critical alarm acked. |
| aino2 — Triage Order (Aino) | `aino2_triage_order/aino2.mp4` | Alarm triage: 3 alarms acked in priority order. |
| aino3 — The Cascade (Aino) | `aino3_the_cascade/aino3.mp4` | Alarm triage: 5-alarm cascade, correct priority sequence. |
| aino4 — Bulk Ack (Aino) | `aino4_bulk_ack/aino4.mp4` | Alarm triage: 6 alarms, bulk acknowledgment workflow. |
| aino5 — Query the NBI (Aino) | `aino5_query_the_nbi/aino5.mp4` | AMS API: Login → GetAllManagedElements → GetManagedElement → GetNeStatus. |
| aino6 — Supervision (Aino) | `aino6_supervision/aino6.mp4` | Board spans + AMS supervision API (Stop/Execute/Start). |
| aino7 — Night Shift (Aino) | `aino7_night_shift/aino7.mp4` | Alarm triage: 8 alarms with priority inversion. |
| aino8 — Maintenance Window (Aino) | `aino8_maintenance_window/aino8.mp4` | Triage + AMS maintenance mode API sequence. |
| aino9 — SOAP Fault (Aino) | `aino9_soap_fault/aino9.mp4` | AMS API with SOAP fault handling. |
| aino10 — All Clear (Aino) | `aino10_all_clear/aino10.mp4` | Triage + AMS API: full NOC shift workflow. |
| hikari1 — First Day (Hikari) | `hikari1_first_day/hikari1.mp4` | Clean UPC connector (slot 0); dirty 2 dB pill loses. |
| hikari2 — Address to Diagnostics (Hikari) | `hikari2_address_to_diagnostics/hikari2.mp4` | BxE API: Login → SearchAddress → ListDevices → ReadDiagnostics. |
| hikari3 — Beat the Callback (Hikari) | `hikari3_beat_the_callback/hikari3.mp4` | Fusion splice; WaterIntrusion outage at 30s. |
| hikari4 — Dirty Drop (Hikari) | `hikari4_dirty_drop/hikari4.mp4` | Clean UPC connector at the ONT; 3 dB dirty pill loses. |
| hikari5 — Certify It (Hikari) | `hikari5_certify_it/hikari5.mp4` | Fusion splice for 0.6 dB certification window. |
| hikari6 — Slow Portal (Hikari) | `hikari6_slow_portal/hikari6.mp4` | BxE API shortcut: Login → ReadDiagnostics (skip search). |
| hikari7 — Bend in the Wall (Hikari) | `hikari7_bend_in_the_wall/hikari7.mp4` | Fusion splice beats the macrobend; re-terminate clean. |
| hikari8 — Splitter Closet (Hikari) | `hikari8_splitter_closet/hikari8.mp4` | 1:8 splitter for 8 units; Goldilocks sizing. |
| hikari9 — Night Trouble Call (Hikari) | `hikari9_night_trouble/hikari9.mp4` | Clean connector; WaterIntrusion outage at 45s. |
| hikari10 — Master Tech (Hikari) | `hikari10_master_tech/hikari10.mp4` | Fusion splice + full 6-step BxE workflow. |
| m1l3 — Thread the Needle (Linka) | `wireless3_thread_the_needle/m1l3.mp4` | 20 dBm repeater placed 1→2; tight receive window threaded; win screen. |
| m1l4 — The 5GHz Tax (Linka) | `wireless4_the_5ghz_tax/m1l4.mp4` | 20 dBm repeater placed 1→2; 5.8 GHz free-space tax paid; win screen. |
| m1l5 — After the Repeater (Linka) | `wireless5_after_the_repeater/m1l5.mp4` | 20 dBm repeater placed 1→2; two-hop relay closed; win screen. |
| m1l6 — Loud Is Not Clear (Linka) | `wireless6_loud_is_not_clear/m1l6.mp4` | 20 dBm repeater placed 1→2; 30 dB amp trap avoided; win screen. |
| m1l7 — The Long Haul (Linka) | `wireless7_the_long_haul/m1l7.mp4` | 20 dBm repeater placed 1→2; 800 m hop at the edge; win screen. |
| m1l8 — Aim High (Linka) | `wireless8_aim_high/m1l8.mp4` | 20 dBm repeater placed 1→2; cuts through 2.4 GHz noise; win screen. |
| m1l9 — Mixed Bands (Linka) | `wireless9_mixed_bands/m1l9.mp4` | 20 dBm repeater placed 1→2; 2.4/5.8 GHz hops balanced; win screen. |
| m1l10 — Linka's Gauntlet (Linka) | `wireless10_linkas_gauntlet/m1l10.mp4` | 25 dBm repeater placed 1→2; final exam conquered; win screen. |
| e1l3 — Category Matters (Lattice) | `ethernet3_category_matters/e1l3.mp4` | Cat6 40 m run placed 1→2; beats Cat5e bottleneck; win screen. |
| e1l4 — The Long Corridor (Lattice) | `ethernet4_the_long_corridor/e1l4.mp4` | Switch placed 1→2; 140 m corridor regenerated; win screen. |
| e1l5 — Power Hungry (Lattice) | `ethernet5_power_hungry/e1l5.mp4` | 60 W switch placed 1→2; feeds 45 W camera array; win screen. |
| e1l6 — Every Constraint (Lattice) | `ethernet6_every_constraint/e1l6.mp4` | 30 W switch placed 1→2; power + bandwidth met; win screen. |
| e1l7 — The Campus (Lattice) | `ethernet7_the_campus/e1l7.mp4` | 60 W switch placed 1→2; feeds 50 W access point; win screen. |
| e1l8 — No Slack (Lattice) | `ethernet8_no_slack/e1l8.mp4` | Switch placed 1→2; past Cat5e bottleneck; win screen. |
| e1l9 — Retrofit (Lattice) | `ethernet9_retrofit/e1l9.mp4` | Switch placed 1→2; bridges legacy Cat5e runs; win screen. |
| e1l10 — The Data Center (Lattice) | `ethernet10_the_data_center/e1l10.mp4` | Top-tier switch placed 1→2; 10 Gbps + 55 W PoE; win screen. |
| f1l3 — Clean Hands (Séraphine) | `f1l3/f1l3.mp4` | Clean UPC connector placed 1→2; 2 dB dirty pill loses; win screen. |
| f1l4 — Fusion or Bust (Séraphine) | `f1l4/f1l4.mp4` | Fusion splice placed 1→2; 25 km tight budget survives; win screen. |
| f1l5 — The Short Cut (Séraphine) | `f1l5/f1l5.mp4` | 30 km route via Valley Junction placed 1→3; beats 45 km ridge; win screen. |
| f1l6 — Split Decision (Séraphine) | `f1l6/f1l6.mp4` | Goldilocks 1:16 splitter placed 1→2; win screen. |
| f1l7 — Bend Don't Break (Séraphine) | `f1l7/f1l7.mp4` | Bypass splice placed 3→4; avoids 5 dB kinked drop; win screen. |
| f1l8 — Contamination Event (Séraphine) | `f1l8/f1l8.mp4` | Clean connector placed 1→2; ConnectorContamination outage fires at 10s; win screen. |
| f1l9 — Tight Budget (Séraphine) | `f1l9/f1l9.mp4` | Fusion splice placed 2→3; 22 km + 1:16 split, 0.5 dB window; win screen. |
| f1l10 — Build It Right (Séraphine) | `f1l10/f1l10.mp4` | Protection route fusion splice placed 2→3; AerialDamage outage fires at 15s on 1→3; win screen. |
| c1l3 — Longer Run (Ondine) | `c1l3/c1l3.mp4` | 10 dB amplifier placed 1→2; lands in [5, 12] dBmV; win screen. |
| c1l4 — Tap Dance (Ondine) | `c1l4/c1l4.mp4` | 8 dB tap placed 2→3; tap value is the budget; win screen. |
| c1l5 — Ingress Returns (Ondine) | `c1l5/c1l5.mp4` | 5 dB amplifier placed 1→2; IngressNoise outage fires at 15s; mid-level rebalance wins. |
| c1l6 — Backup Plan (Ondine) | `c1l6/c1l6.mp4` | 1 dB amp placed 2→3 on backup path; AmplifierFailure outage fires at 10s on 1→3; win screen. |
| c1l7 — Hot Headend (Ondine) | `c1l7/c1l7.mp4` | 500 m coax span placed 1→2 as a pad; tames +45 dBmV headend; win screen. |
| c1l9 — Precision Run (Ondine) | `c1l9/c1l9.mp4` | 12 dB amplifier placed 1→2; threads [3, 9] window; win screen. |
| c1l10 — Ingress Storm (Ondine) | `c1l10/c1l10.mp4` | 10 dB amplifier placed 1→2; IngressNoise storm fires at 10s; win screen. |
| lea1 — Study Hall Warm-Up (Léa) | `lea1/lea1.mp4` | Quiz: question display, answer selection, Léa's explanation; 3 questions scripted. |
| lea2 — Grounded: Article 250 (Léa) | `lea2/lea2.mp4` | Quiz: NEC Article 250 grounding questions; answer feedback + explanation. |
| lea3 — Wiring Methods: Articles 300-398 (Léa) | `lea3/lea3.mp4` | Quiz: wiring method questions; answer selection with Léa's guidance. |
| lea4 — Hazardous Locations: Articles 500-516 (Léa) | `lea4/lea4.mp4` | Quiz: hazloc classification questions; explanation after each answer. |
| lea5 — Special Conditions: Articles 705-780 (Léa) | `lea5/lea5.mp4` | Quiz: special conditions questions; Léa explains the code rationale. |
| lea6 — Comms Systems Boss: Articles 800-830 (Léa) | `lea6/lea6.mp4` | Quiz: communications systems questions; 12 questions, boss level. |
| lea7 — Theory Workshop (Léa) | `lea7/lea7.mp4` | Quiz: electrical theory questions; answer feedback + explanation. |
| lea8 — Washington Law: RCW 19.28 (Léa) | `lea8/lea8.mp4` | Quiz: Washington state law questions; Léa's explanation. |
| lea9 — Washington Admin Code: WAC 296-46B (Léa) | `lea9/lea9.mp4` | Quiz: WAC admin code questions; answer selection + explanation. |
| lea10 — Mock Exam: Final Boss (Léa) | `lea10/lea10.mp4` | Quiz: 20-question mock exam; includes a wrong-answer correction path. |

## Capture method

```bash
# One level (inside xvfb-run):
xvfb-run -a -s "-screen 0 720x1280x24" \
  env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  ./target/debug/light-show --footage w1l1 --bench-backend vulkan
# With ffmpeg x11grab running alongside for the actual recording.
```

The `--footage` harness is debug-only (`#[cfg(debug_assertions)]`); release
builds contain none of it. Frame budgets assume software rendering
(~35 fps via lavapipe); outage timers run on wall-clock time.
