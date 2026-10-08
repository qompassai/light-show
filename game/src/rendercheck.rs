//! Development-only rendered-frame check harness.
//!
//! Activated by `--render-check <menu|picker|levels>` (debug builds only — the
//! whole module is `#[cfg(debug_assertions)]`). Builds the real app via
//! the normal `build_app()` path, optionally at a forced window size
//! (`--render-size <W>x<H>`, applied to the `WindowPlugin` in `lib.rs`),
//! settles for a bounded number of frames, captures the primary window
//! with Bevy's `Screenshot` machinery to `--render-out <path>`, and
//! exits via `AppExit`. It exists so the responsive-layout work has
//! rendered-frame evidence at window sizes a developer machine would
//! not normally sit at (phone portrait, desktop landscape, square):
//! the dimension-matrix captures under `responsive-check/` are
//! produced by this driver and asserted on by `corner_check.py`.
//!
//! Without the flag the game builds a byte-identical plugin set and
//! installs zero render-check systems: no overhead, no behavior
//! change (the same contract as `bench` and `footage`).

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

use crate::states::GameState;

/// Upper bound on the capture frame. Bounds a typo like
/// `--render-frames 999999999` from hanging the machine.
const RENDER_CHECK_FRAMES_MAX: u32 = 100_000;
/// Default frame on which the screenshot is taken: late enough for
/// asset loads, backdrop fit, and the picker's card entrance to have
/// settled at software-render frame rates.
const DEFAULT_CAPTURE_FRAME: u32 = 240;
/// Frames between triggering the screenshot and exiting: the capture
/// round-trips through the render world and `save_to_disk`'s async
/// write, so the driver drains generously before `AppExit`.
const CAPTURE_DRAIN_FRAMES: u32 = 240;
/// Frame on which a `picker` check leaves the menu for the companion
/// select screen. Early (assets are shared with the menu backdrop),
/// but after the menu's own `OnEnter` has run.
const ENTER_TARGET_FRAME: u32 = 30;
/// Frame on which a `levels` check leaves the picker for the level
/// select. The state write is direct, like the picker entry above:
/// a simulated card press races the UI focus system, which owns
/// `Interaction` in the real app (the press path itself is covered
/// end-to-end by the playthrough harness). Early enough that the
/// screen settles long before the default capture frame.
const ENTER_LEVELS_FRAME: u32 = 90;
/// Smallest window edge the size override accepts: below this the UI
/// has nothing meaningful to lay out and the capture is junk data.
const WINDOW_EDGE_MIN_PX: u32 = 64;
/// Largest window edge the size override accepts (16K): a typo guard,
/// not a product limit.
const WINDOW_EDGE_MAX_PX: u32 = 16_384;

/// Which screen a render-check run captures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderCheckTarget {
    /// The main menu over the title artwork.
    Menu,
    /// The companion select ("picker") over the title artwork.
    Picker,
    /// The level select for the default companion (Séraphine): the
    /// driver walks menu → picker → level select and captures the
    /// level list her pick opens.
    Levels,
}

/// Parsed `--render-*` flags. `target: None` means "normal game run".
#[derive(Default)]
pub struct RenderCheckArgs {
    pub target: Option<RenderCheckTarget>,
    pub size: Option<(u32, u32)>,
    pub out: Option<std::path::PathBuf>,
    pub frames: Option<u32>,
}

impl RenderCheckArgs {
    /// Reads `--render-check`, `--render-size`, `--render-out`, and
    /// `--render-frames` from the process arguments. Invalid values
    /// are reported on stderr and treated as absent; unknown flags
    /// are ignored (the same policy as `FootageArgs::from_args`).
    pub fn from_args() -> Self {
        let mut args = Self::default();
        let mut raw = std::env::args().skip(1);
        while let Some(flag) = raw.next() {
            match flag.as_str() {
                "--render-check" => match raw.next().as_deref() {
                    Some("menu") => args.target = Some(RenderCheckTarget::Menu),
                    Some("picker") => args.target = Some(RenderCheckTarget::Picker),
                    Some("levels") => args.target = Some(RenderCheckTarget::Levels),
                    other => eprintln!(
                        "[light-show] ignoring unknown --render-check {other:?} \
                         (expected menu|picker|levels)"
                    ),
                },
                "--render-size" => match raw.next().map(|s| parse_window_size(&s)) {
                    Some(Some(size)) => args.size = Some(size),
                    _ => {
                        eprintln!("[light-show] ignoring invalid --render-size (expected <W>x<H>)")
                    }
                },
                "--render-out" => match raw.next() {
                    Some(path) if !path.is_empty() => args.out = Some(path.into()),
                    _ => eprintln!("[light-show] ignoring --render-out with empty path"),
                },
                "--render-frames" => match raw.next().map(|n| n.parse::<u32>()) {
                    Some(Ok(n)) if n > 0 => args.frames = Some(n.min(RENDER_CHECK_FRAMES_MAX)),
                    Some(Ok(_)) => eprintln!("[light-show] ignoring --render-frames 0"),
                    Some(Err(_)) | None => {
                        eprintln!("[light-show] ignoring invalid --render-frames value")
                    }
                },
                _ => {}
            }
        }
        args
    }
}

/// Parses a `<W>x<H>` window-size override, each edge bounded to
/// `WINDOW_EDGE_MIN_PX..=WINDOW_EDGE_MAX_PX`. `None` on any malformed
/// or out-of-range input.
fn parse_window_size(raw: &str) -> Option<(u32, u32)> {
    let (w, h) = raw.split_once('x')?;
    let w: u32 = w.parse().ok()?;
    let h: u32 = h.parse().ok()?;
    if (WINDOW_EDGE_MIN_PX..=WINDOW_EDGE_MAX_PX).contains(&w)
        && (WINDOW_EDGE_MIN_PX..=WINDOW_EDGE_MAX_PX).contains(&h)
    {
        Some((w, h))
    } else {
        None
    }
}

/// Mutable state for one render-check run.
#[derive(Resource)]
struct RenderCheckRun {
    target: RenderCheckTarget,
    out: std::path::PathBuf,
    capture_frame: u32,
    frame: u32,
    captured: bool,
}

/// Installs the render-check driver. Call only for a real check run
/// (`args.target.is_some()`); normal runs must not call this. A run
/// without `--render-out` writes `render-check.png` in the working
/// directory so the capture is never silently dropped.
pub fn add_rendercheck_systems(app: &mut App, args: &RenderCheckArgs) {
    let Some(target) = args.target else {
        return;
    };
    app.insert_resource(RenderCheckRun {
        target,
        out: args
            .out
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("render-check.png")),
        capture_frame: args.frames.unwrap_or(DEFAULT_CAPTURE_FRAME),
        frame: 0,
        captured: false,
    })
    .add_systems(Update, rendercheck_driver);
}

/// The driver: walks to the target screen if needed, triggers the
/// screenshot at the capture frame, and exits once the drain has
/// elapsed. Total frames per run are bounded by
/// `capture_frame + CAPTURE_DRAIN_FRAMES`.
fn rendercheck_driver(
    mut commands: Commands,
    mut run: ResMut<RenderCheckRun>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    run.frame += 1;
    if matches!(
        run.target,
        RenderCheckTarget::Picker | RenderCheckTarget::Levels
    ) && run.frame == ENTER_TARGET_FRAME
    {
        next_state.set(GameState::CompanionSelect);
    }
    if run.target == RenderCheckTarget::Levels && run.frame == ENTER_LEVELS_FRAME {
        next_state.set(GameState::LevelSelect);
    }
    if !run.captured && run.frame >= run.capture_frame {
        run.captured = true;
        eprintln!(
            "[light-show] render-check: capturing {:?} at frame {} -> {}",
            run.target,
            run.frame,
            run.out.display()
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(run.out.clone()));
    }
    if run.captured && run.frame >= run.capture_frame + CAPTURE_DRAIN_FRAMES {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_size_parses_bounded_wxh() {
        assert_eq!(parse_window_size("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_window_size("720x1600"), Some((720, 1600)));
        assert_eq!(parse_window_size("1024x1024"), Some((1024, 1024)));
    }

    #[test]
    fn window_size_rejects_malformed_and_out_of_range() {
        assert_eq!(parse_window_size("1920"), None);
        assert_eq!(parse_window_size("1920x"), None);
        assert_eq!(parse_window_size("axb"), None);
        assert_eq!(parse_window_size("0x1080"), None);
        assert_eq!(parse_window_size("32x1080"), None);
        assert_eq!(parse_window_size("1920x99999"), None);
        assert_eq!(parse_window_size("19 20x1080"), None);
    }
}
