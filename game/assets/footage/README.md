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
