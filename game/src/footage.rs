//! Test-footage capture harness (debug builds only).
//!
//! Activated by `--footage <level-id>` (e.g. `--footage clara1`). Selects
//! the level by its `LevelDef::id`, drives a per-level scripted placement
//! sequence through the real game logic (`PlacedChoices` + `rebuild_live_graph`,
//! the same path the input handler uses), and exits via `AppExit` after the
//! script's frame budget. Pair with `xvfb-run` and `ffmpeg -f x11grab` to
//! capture the clip.
//!
//! `--footage-frames <N>` overrides the per-level frame budget.
//!
//! Without the flag the game builds a byte-identical plugin set and installs
//! zero footage systems: no overhead, no behavior change.

use bevy::prelude::*;

use crate::board;
use crate::level::{self, CurrentLevelIndex};
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::GameState;

/// A single scripted action, executed at a given frame.
#[derive(Clone, Copy)]
enum FootageAction {
    /// Place a component: insert `(from, to) -> slot` into `PlacedChoices`
    /// and rebuild the live graph (the same state transition the pill-tap
    /// input handler performs).
    Place { from: u32, to: u32, slot: usize },
}

/// Per-level footage script: timed actions plus the total frame budget.
struct FootageScript {
    /// `(frame_in_playing, action)` pairs, sorted by frame.
    actions: Vec<(u32, FootageAction)>,
    /// Frames in `Playing` before auto-exit.
    total_frames: u32,
}

/// Parsed `--footage` flags. `level_id: None` means "normal game run".
#[derive(Default)]
pub struct FootageArgs {
    pub level_id: Option<String>,
    pub frames: Option<u32>,
}

impl FootageArgs {
    /// Reads `--footage <level-id>` and `--footage-frames <N>` from the
    /// process arguments. Invalid values are reported on stderr and treated
    /// as absent; unknown flags are ignored.
    pub fn from_args() -> Self {
        let mut args = Self::default();
        let mut raw = std::env::args().skip(1);
        while let Some(flag) = raw.next() {
            match flag.as_str() {
                "--footage" => match raw.next() {
                    Some(id) if !id.is_empty() => args.level_id = Some(id),
                    _ => eprintln!("[light-show] ignoring --footage with empty id"),
                },
                "--footage-frames" => match raw.next().map(|n| n.parse::<u32>()) {
                    Some(Ok(n)) if n > 0 => args.frames = Some(n.min(100_000)),
                    Some(Ok(_)) => eprintln!("[light-show] ignoring --footage-frames 0"),
                    Some(Err(_)) | None => {
                        eprintln!("[light-show] ignoring invalid --footage-frames value")
                    }
                },
                _ => {}
            }
        }
        args
    }
}

/// Index into `LEVEL_SOURCES` for a level id (e.g. `"clara1"`, `"w1l1"`).
/// Matches against `LevelDef::id` parsed from the embedded JSON.
fn level_index_for_id(id: &str) -> Option<usize> {
    (0..level::LEVEL_SOURCES.len()).find(|&i| {
        let def: Result<level::LevelDef, _> = serde_json::from_str(level::LEVEL_SOURCES[i]);
        def.map(|d| d.id == id).unwrap_or(false)
    })
}

/// Frames in `Playing` before the first scripted placement. Covers asset
/// loading + board setup under software rendering.
const SETUP_FRAMES: u32 = 400;

/// Scripted placements per level: put down the level's signature component,
/// then hold for the level's signature beat (outage levels wait for the
/// scripted outage to fire on wall-clock time).
fn script_for_level(id: &str) -> Option<FootageScript> {
    // (from, to, slot, total_frames): slot selects the component choice.
    let (from, to, slot, total_frames) = match id {
        // First Light: Fusion splice, link budget succeeds.
        "w1l1" => (1, 2, 0, 900),
        // Storm Season: main splice 1->3, AerialDamage outage at 20s,
        // then protection splice 2->3. (Two placements; see below.)
        "w4l1" => (1, 3, 0, 2000),
        // Unity Gain: 5dB amplifier lands in the [0,15] dBmV window.
        "c1l1" => (1, 2, 0, 900),
        // Ingress at Night: amp placed, IngressNoise outage at 15s.
        "c1l2" => (1, 2, 1, 1400),
        // Close the Link: 20dBm repeater closes the wireless link.
        "m1l1" => (1, 2, 0, 900),
        // Ride the Storm: repeater placed, interference storm at 15s.
        "m1l2" => (1, 2, 1, 1400),
        // Hundred-Meter Wall: switch regenerates past the 100m limit.
        "e1l1" => (1, 2, 0, 900),
        // Power Budget: 60W switch covers the PoE draw.
        "e1l2" => (1, 2, 0, 900),
        // First Turn-Up: 1:16 splitter for 9 subscribers.
        "clara1" => (0, 1, 2, 1100),
        // Clara outage: 1:8 splitter for 6 subs, WaterIntrusion at 25s.
        "clara2" => (0, 1, 1, 2000),
        _ => return None,
    };
    let mut actions = vec![(SETUP_FRAMES + 10, FootageAction::Place { from, to, slot })];
    // w4l1's signature beat is the protection route: after the 20s outage
    // fires on 1->3, place the 2->3 Mechanical splice (slot 0 of its pair).
    // 1500 frames is safely past the outage at ~35fps software rendering.
    if id == "w4l1" {
        actions.push((
            1500,
            FootageAction::Place {
                from: 2,
                to: 3,
                slot: 0,
            },
        ));
    }
    Some(FootageScript {
        actions,
        total_frames,
    })
}

/// Mutable state for one footage run.
#[derive(Resource)]
struct FootageRun {
    /// Frames spent in `GameState::Playing` so far.
    frames_in_playing: u32,
    script: FootageScript,
    /// Next action index to execute.
    next_action: usize,
    /// Whether we've entered `Playing` yet. The driver requests the
    /// transition exactly once; afterwards it leaves state alone so the
    /// win -> Results flow (which re-runs `setup_level` on re-enter) is
    /// not fought.
    entered: bool,
    /// Frames spent in `GameState::Results` (win screen hold before exit).
    frames_in_results: u32,
}

/// Installs the footage driver. Call only for a real footage run
/// (`args.level_id.is_some()` and the id resolves); normal runs must not
/// call this. Also seeds `CurrentLevelIndex` so `setup_level` loads the
/// requested level on `OnEnter(Playing)`.
pub fn add_footage_systems(app: &mut App, level_id: &str, frames_override: Option<u32>) -> bool {
    let Some(index) = level_index_for_id(level_id) else {
        eprintln!("[light-show] --footage: unknown level id {level_id:?}");
        return false;
    };
    let Some(mut script) = script_for_level(level_id) else {
        eprintln!("[light-show] --footage: no script for level id {level_id:?}");
        return false;
    };
    if let Some(n) = frames_override {
        script.total_frames = n;
    }
    eprintln!(
        "[light-show] footage: level {level_id} (index {index}), {} frames",
        script.total_frames
    );
    app.insert_resource(CurrentLevelIndex(index));
    app.insert_resource(FootageRun {
        frames_in_playing: 0,
        script,
        next_action: 0,
        entered: false,
        frames_in_results: 0,
    })
    .add_systems(Update, footage_driver);
    true
}

/// Executes the scripted placements and exits when the frame budget is
/// spent. Places components by writing `PlacedChoices` + rebuilding the
/// live graph directly — the same state transition the input handler
/// performs on a pill tap, without the fragility of synthetic cursor
/// coordinates under software rendering.
fn footage_driver(
    mut footage: ResMut<FootageRun>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
    level: Option<Res<level::LevelDef>>,
    placed: Option<ResMut<board::PlacedChoices>>,
    live: Option<ResMut<LiveGraph>>,
    active_outage: Option<Res<ActiveOutage>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !footage.entered {
        if *state.get() != GameState::Playing {
            next_state.set(GameState::Playing);
            return;
        }
        footage.entered = true;
    }
    // Win -> Results: hold the win screen briefly, then exit.
    if *state.get() == GameState::Results {
        footage.frames_in_results += 1;
        if footage.frames_in_results >= 180 {
            eprintln!("[light-show] footage: win screen held, exiting");
            exit.write(AppExit::Success);
        }
        return;
    }
    if *state.get() != GameState::Playing {
        return;
    }
    footage.frames_in_playing += 1;
    let frame = footage.frames_in_playing;

    let (Some(level), Some(mut placed), Some(mut live), Some(active_outage)) =
        (level.as_ref(), placed, live, active_outage.as_ref())
    else {
        return;
    };
    while footage.next_action < footage.script.actions.len()
        && footage.script.actions[footage.next_action].0 <= frame
    {
        let (_, action) = footage.script.actions[footage.next_action];
        footage.next_action += 1;
        match action {
            FootageAction::Place { from, to, slot } => {
                placed.0.insert((from, to), slot);
                board::rebuild_live_graph(
                    level,
                    &placed,
                    active_outage.outage.as_ref(),
                    &mut live.graph,
                );
                eprintln!("[light-show] footage: placed {from}->{to} slot {slot} at frame {frame}");
            }
        }
    }

    if frame >= footage.script.total_frames {
        eprintln!("[light-show] footage: done after {frame} frames in Playing");
        exit.write(AppExit::Success);
    }
}
