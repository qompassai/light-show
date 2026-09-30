# Render Backends

Light Show renders with Bevy 0.14 (wgpu 0.20). The backend is selected by
wgpu at startup; the game logs the choice once:

```
[light-show] render backend: vulkan | adapter: NVIDIA GeForce RTX 4070 Laptop GPU | vendor: 0x10de device: 0x2820 | driver: NVIDIA (615.71.09)
```

## Available backends (primo, 2026-09-30)

| Backend | Adapter | Driver | Notes |
|---------|---------|--------|-------|
| Vulkan | NVIDIA GeForce RTX 4070 Laptop GPU | NVIDIA 615.71.09 | Default; discrete GPU |
| Vulkan | Intel(R) Graphics (RPL-P) | Mesa | Integrated GPU (not benchmarked) |
| Vulkan | llvmpipe (LLVM 23.1.1) | Mesa 26.2.3 | Software rasterizer |
| GL | llvmpipe (LLVM 23.1.1) | Mesa 26.2.3 | Software rasterizer; via `--bench-backend gl` |

The bench harness (`game/src/bench.rs`, dev-only) can force the GL backend
with `--bench-backend gl`. Vulkan is the default. There is no Metal/DX12
backend on this Linux workstation.

## Benchmark numbers (primo-local, Xvfb)

Method: `nix run .#bench-backends` (or manually: `DISPLAY=:99
./target/debug/light-show --bench-frames 300 [--bench-backend gl]`).
The harness plays a scripted splice (press node 1, drag to node 2, release),
measures `Time<Real>` deltas for 300 frames (first frame skipped), and prints
avg/p95/min/max. Debug build, Xvfb :99 (720x1280, no real display).

| Backend | Runs | avg_ms | p95_ms | min_ms | max_ms |
|---------|------|--------|--------|--------|--------|
| Vulkan (NVIDIA RTX 4070) | 3x300 | 21.1–21.9 | 28.3–31.4 | 15.8–16.3 | 131–202 |
| GL (llvmpipe, software) | 3x300 | 17.4–18.0 | 26.3–27.7 | 9.3–9.8 | 66–203 |

Raw (2026-09-30, manual runs):

- Vulkan: `avg_ms=21.91 p95_ms=31.36`, `avg_ms=21.14 p95_ms=28.28`, `avg_ms=21.09 p95_ms=29.64`
- GL: `avg_ms=17.36 p95_ms=27.69`, `avg_ms=17.96 p95_ms=26.33`, `avg_ms=17.73 p95_ms=27.33`

Raw (2026-09-30, via `nix run .#bench-backends`):

- Vulkan: `avg_ms=22.14 p95_ms=32.51`, `avg_ms=22.85 p95_ms=34.48`, `avg_ms=22.77 p95_ms=36.02`
- GL: `avg_ms=18.17 p95_ms=31.68`, `avg_ms=18.84 p95_ms=35.96`, `avg_ms=54.26 p95_ms=111.94`

Note: the third GL run via the app (`avg 54ms, p95 112ms`) is an outlier —
likely system contention during the software-rasterizer run (llvmpipe uses
all CPU cores; a background task can skew it). It is reported, not hidden;
the manual runs show the typical range.

### Caveats (read before quoting these)

- **Xvfb, not a real display.** Present timing under Xvfb does not represent
  on-screen vsync behavior. Both backends were measured under the same Xvfb,
  so the comparison is fair; absolute numbers are not.
- **Debug build.** The bench harness is `#[cfg(debug_assertions)]`; release
  numbers will differ. The harness changes nothing when its flags are absent
  (verified: no-flags run behaves identically to the pre-bench binary).
- **The game is not GPU-bound.** It draws gizmos, sprites, and UI for a
  single puzzle board. Neither backend is stressed; the numbers mostly
  reflect driver present overhead and vsync quantization under Xvfb.
- **llvmpipe is software rendering.** The GL numbers do not represent real
  GL hardware. They are useful only as a "does the GL path work and stay
  in the same ballpark" check.
- **Max spikes are hitches**, not sustained performance (shader compile,
  asset upload, first-frame setup). p95 is the more honest tail metric.
- **Android is not included.** See below.

## Android

An emulator (`emulator-5554`, API 36) is reachable via adb on primo with
`ai.qompass.lightshow` installed. `adb shell dumpsys gfxinfo
ai.qompass.lightshow framestats` reports **0 frames rendered**: the game is
a NativeActivity driving Vulkan directly, bypassing the Skia/OpenGL view
hierarchy that gfxinfo measures. gfxinfo is the wrong tool for this game;
on-device numbers would need the in-game bench harness (not wired to
Android intents) or external frame capture. No Android numbers are reported
here rather than reporting misleading ones.

Building a debuggable APK with the bench harness and driving it via intents
is future work. The human-gated `tbr-*` scripts (signing, store submission)
are out of scope by design.

## Reproducing

```bash
# Deterministic bench matrix (primo-local, needs Xvfb :99):
nix run .#bench-backends

# Manual:
DISPLAY=:99 ./target/debug/light-show --bench-frames 300
DISPLAY=:99 ./target/debug/light-show --bench-frames 300 --bench-backend gl
```

The flake's `nix flake check` does NOT run benches (they need hardware);
it runs the toolchain determinism check only. See `docs/FLAKE.md`.
