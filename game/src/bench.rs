//! Development-only render-backend benchmark harness.
//!
//! Activated by the `--bench-frames <N>` CLI flag (debug builds only —
//! the whole module is `#[cfg(debug_assertions)]`). Drives deterministic
//! synthetic input through the real gesture pipeline (`PointerWorld` +
//! `ButtonInput<MouseButton>`, the same resources the unit tests poke via
//! `run_system_once`), measures wall-clock frame times with `Time<Real>`,
//! prints a one-line summary, and exits via `AppExit`.
//!
//! `--bench-backend <vulkan|gl>` forces the wgpu backend through
//! `WgpuSettings { backends: Some(..) }`, plumbed into `RenderPlugin`
//! (where bevy 0.14 actually reads it — `RenderPlugin::default()` bakes
//! in `WgpuSettings::default()`, it does not read a world resource).
//!
//! Without either flag the game builds a byte-identical plugin set and
//! installs zero bench systems: no overhead, no behavior change.

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::time::Real;

use crate::board;
use crate::level::LevelDef;
use crate::states::GameState;

/// Upper bound on measured frames per run. Bounds the `frame_ms`
/// allocation and keeps a typo like `--bench-frames 1000000000` from
/// hanging the machine or OOMing it.
const BENCH_FRAMES_MAX: u32 = 100_000;

/// Scripted gesture, by count of frames spent in `GameState::Playing`:
/// press node 1, drag to node 2, release (connects the default splice).
/// Afterwards the board just renders the placed splice and the live
/// ledger for the remaining frames.
const SCRIPT_PRESS_FRAME: u32 = 1;
const SCRIPT_DRAG_FRAME: u32 = 2;
const SCRIPT_RELEASE_FRAME: u32 = 3;

/// Parsed `--bench-*` flags. `frames: None` means "normal game run".
#[derive(Default)]
pub struct BenchArgs {
    pub frames: Option<u32>,
    pub backend: Option<Backends>,
}

impl BenchArgs {
    /// Reads `--bench-frames <N>` and `--bench-backend <vulkan|gl>` from
    /// the process arguments. Invalid values are reported on stderr and
    /// treated as absent; unknown flags are ignored.
    pub fn from_args() -> Self {
        let mut args = Self::default();
        let mut raw = std::env::args().skip(1);
        while let Some(flag) = raw.next() {
            match flag.as_str() {
                "--bench-frames" => match raw.next().map(|n| n.parse::<u32>()) {
                    Some(Ok(n)) if n > 0 => args.frames = Some(n.min(BENCH_FRAMES_MAX)),
                    Some(Ok(_)) => eprintln!("[light-show] ignoring --bench-frames 0"),
                    Some(Err(_)) | None => {
                        eprintln!("[light-show] ignoring invalid --bench-frames value")
                    }
                },
                "--bench-backend" => match raw.next().as_deref() {
                    Some("vulkan") => args.backend = Some(Backends::VULKAN),
                    Some("gl") => args.backend = Some(Backends::GL),
                    other => eprintln!(
                        "[light-show] ignoring unknown --bench-backend {other:?} (expected vulkan|gl)"
                    ),
                },
                _ => {}
            }
        }
        args
    }
}

/// Swaps a backend-forcing `RenderPlugin` into the plugin builder when
/// `--bench-backend` was given. Returns the builder untouched otherwise,
/// so non-bench runs assemble a byte-identical plugin set.
pub fn maybe_force_backend(plugins: PluginGroupBuilder, args: &BenchArgs) -> PluginGroupBuilder {
    match args.backend {
        Some(backends) => plugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: Some(backends),
                ..default()
            }),
            ..default()
        }),
        None => plugins,
    }
}

/// Mutable state for one bench run.
#[derive(Resource)]
struct BenchRun {
    /// Frames spent in `GameState::Playing` so far.
    frames_in_playing: u32,
    /// How many frame times to measure before printing stats and exiting.
    target_frames: u32,
    /// Measured wall-clock frame times, milliseconds.
    frame_ms: Vec<f32>,
}

/// Installs the bench driver. Call only for a real bench run
/// (`args.frames.is_some()`); normal runs must not call this.
pub fn add_bench_systems(app: &mut App, frames: u32) {
    let target = frames.clamp(1, BENCH_FRAMES_MAX);
    // `usize` conversion of a u32 clamped to 100_000 cannot fail, but
    // `try_into` keeps the bound explicit instead of trusting it.
    let capacity: usize = target.try_into().unwrap_or(BENCH_FRAMES_MAX as usize);
    app.insert_resource(BenchRun {
        frames_in_playing: 0,
        target_frames: target,
        frame_ms: Vec::with_capacity(capacity),
    })
    .add_systems(
        Update,
        bench_driver
            .after(board::track_pointer)
            .before(board::handle_pointer_input),
    );
}

/// World-space position of a level node, via the same `grid_to_world`
/// mapping the board renderer uses. `None` when the level has no such
/// node (the bench still measures render times, just without the splice).
fn node_world_pos(level: &LevelDef, id: u32) -> Option<Vec2> {
    level
        .nodes
        .iter()
        .find(|n| n.id == id)
        .map(|n| board::grid_to_world(n.grid_x, n.grid_y))
}

/// Drives the scripted gesture and records frame times. Runs after
/// `track_pointer` so the synthetic `PointerWorld` wins over the (absent)
/// real cursor, and before `handle_pointer_input` so the synthetic
/// press/release is observed the same frame it is set.
///
/// Frame 1 in `Playing` is not measured: it carries the `OnEnter` level
/// setup cost, not steady-state rendering.
#[allow(clippy::too_many_arguments)]
fn bench_driver(
    mut bench: ResMut<BenchRun>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
    level: Option<Res<LevelDef>>,
    mut pointer: ResMut<board::PointerWorld>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    mut exit: EventWriter<AppExit>,
    adapter_info: Option<Res<RenderAdapterInfo>>,
) {
    if *state.get() != GameState::Playing {
        next_state.set(GameState::Playing);
        return;
    }
    bench.frames_in_playing += 1;

    if bench.frames_in_playing > 1 {
        bench.frame_ms.push(time.delta().as_secs_f32() * 1000.0);
    }

    match bench.frames_in_playing {
        SCRIPT_PRESS_FRAME => {
            if let Some(pos) = level.as_ref().and_then(|l| node_world_pos(l, 1)) {
                pointer.0 = Some(pos);
                mouse.press(MouseButton::Left);
            }
        }
        SCRIPT_DRAG_FRAME => {
            if let Some(pos) = level.as_ref().and_then(|l| node_world_pos(l, 2)) {
                pointer.0 = Some(pos);
            }
        }
        SCRIPT_RELEASE_FRAME => {
            if let Some(pos) = level.as_ref().and_then(|l| node_world_pos(l, 2)) {
                pointer.0 = Some(pos);
            }
            mouse.release(MouseButton::Left);
        }
        _ => {}
    }

    if bench.frame_ms.len() >= bench.target_frames as usize {
        print_bench_stats(&bench, adapter_info.as_deref());
        exit.send(AppExit::Success);
    }
}

/// Nearest-rank percentile over an ascending-sorted slice.
fn percentile_sorted(sorted: &[f32], p: f32) -> f32 {
    let idx = (p * sorted.len() as f32).ceil() as usize;
    sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
}

/// Sorts the measured frame times in place, prints the one-line summary,
/// and leaves the process exit to the already-sent `AppExit`.
fn print_bench_stats(bench: &BenchRun, adapter_info: Option<&RenderAdapterInfo>) {
    let mut sorted = bench.frame_ms.clone();
    // `total_cmp` gives a total order on f32 (NaN sorts last); frame
    // times are never NaN, this just satisfies the sort contract.
    sorted.sort_by(|a, b| a.total_cmp(b));
    let n = sorted.len();
    let sum: f32 = sorted.iter().sum();
    let backend = adapter_info
        .map(|info| info.0.backend.to_str())
        .unwrap_or("unknown");
    eprintln!(
        "BENCH frames={n} avg_ms={:.2} p95_ms={:.2} min_ms={:.2} max_ms={:.2} backend={backend}",
        sum / n as f32,
        percentile_sorted(&sorted, 0.95),
        sorted[0],
        sorted[n - 1],
    );
}
