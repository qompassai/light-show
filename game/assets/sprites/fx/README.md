# FX Sprites — `game/assets/sprites/fx/`

One-shot particle effects for in-game feedback. All crafted in Aseprite
1.3 (`.aseprite` sources kept alongside the PNG exports), RGBA, neon
palette (cyan `#6ff2ff`, gold `#ffd166`, error red `#ff5577`).

Generation: `gen_fx.lua` (Aseprite batch Lua, deterministic pixel art).
Run: `aseprite -b --script-param outdir=<dir> --script gen_fx.lua`

## Assets

| File | Size | Frames | Used where |
|------|------|--------|------------|
| `connect_spark` | 32×32 | 4 | Board: cyan burst at the midpoint when the player connects two nodes (`board.rs` → `fx::SpawnConnectSpark`) |
| `success_burst` | 64×64 | 6 | Results: gold expanding rings on victory, layered over `win_ring` (`states/results.rs` → `fx::SuccessBurst`) |
| `error_flicker` | 32×32 | 4 | Reserved: red X flicker for invalid actions (not yet wired — asset ready) |
| `fiber_pulse` | 32×32 | 4 | Reserved: animated glow pulse, upgrade path for `pulse_dot` (not yet wired — asset ready) |
| `pulse_dot` | 24×24 | 1 | (pre-existing) Signal pulses traveling along lit edges |
| `storm_streak` | 16×64 | 1 | (pre-existing) Outage rain streaks |
| `win_ring` | — | 6 | (pre-existing) Victory ring expansion |

## Wiring

`game/src/fx.rs` (`FxPlugin`):
- `SpawnConnectSpark { position: Vec2 }` message → `spawn_connect_sparks`
  spawns the 4-frame one-shot as a `BoardRoot` child (auto-swept on teardown).
- `SuccessBurst::new(frames)` component → `animate_success_bursts` scales
  0.6→1.4 and fades across 0.6s, then despawns.
- All effects are sub-second and fire-and-forget. Missing assets degrade
  to no-op (message consumed, nothing spawned).

## Conventions

- Numbered frames: `<name>_<i>.png` (e.g. `connect_spark_0.png`).
- `.aseprite` source kept for every asset; PNGs are the runtime files.
- New effects: add the `.aseprite` + PNGs here, add the component/system
  to `game/src/fx.rs`, write a unit test for the frame count.
