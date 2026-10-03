//! Shared animation easing helpers and the fullscreen state-transition fade.
//!
//! Bevy 0.14 does not ship `bevy::math::curve::EaseFunction` (it arrived
//! in 0.15), so [`Ease`] provides the small subset of easing curves the
//! game needs, using the exact formulas from Bevy 0.18's
//! `easing_functions` module for future upgrade compatibility.
//!
//! [`AnimPlugin`] owns the fullscreen fade overlay ([`TransitionFade`]):
//! gameplay systems request state changes through [`TransitionRequest`]
//! instead of writing `NextState<GameState>` directly, and the plugin
//! fades out (0.3s), applies the state swap at full black, then fades
//! back in (0.3s). See Finding 2 of `docs/ANIMATION_AUDIT.md`.

use bevy::prelude::*;

use crate::states::GameState;

/// Seconds to fade from the old state to black.
pub const FADE_OUT_SECS: f32 = 0.3;
/// Seconds to fade from black into the new state.
pub const FADE_IN_SECS: f32 = 0.3;

/// Easing curves used across the game's animation systems. `sample(t)`
/// takes a normalized time `t` in `[0, 1]` and returns the eased progress.
/// Formulas match Bevy 0.18's `easing_functions` exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ease {
    /// Accelerate then decelerate: smooth start and stop.
    CubicInOut,
    /// Fast start, gentle landing.
    CubicOut,
    /// Sinusoidal accelerate/decelerate: organic drift.
    SineInOut,
    /// Overshoot past 1.0 then settle: springy "pop".
    BackOut,
}

impl Ease {
    /// Samples the easing curve at normalized time `t`.
    ///
    /// Contract: `t` is clamped to `[0, 1]` by the caller; the return is
    /// `0.0` at `t = 0.0` and `1.0` at `t = 1.0`, except [`Ease::BackOut`]
    /// which overshoots above `1.0` mid-flight by design.
    pub fn sample(self, t: f32) -> f32 {
        match self {
            Ease::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Ease::CubicOut => 1.0 - (1.0 - t).powi(3),
            Ease::SineInOut => -((std::f32::consts::PI * t).cos() - 1.0) / 2.0,
            Ease::BackOut => {
                let c = 1.70158;
                1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
            }
        }
    }
}

/// Phase of the fullscreen transition fade.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FadePhase {
    /// No transition in flight; the overlay is transparent.
    Idle,
    /// Fading to black; the state swap fires when `elapsed_secs`
    /// reaches [`FADE_OUT_SECS`].
    FadingOut { target: GameState, elapsed_secs: f32 },
    /// Fading back in from black after the swap.
    FadingIn { elapsed_secs: f32 },
}

/// Fullscreen state-transition fade state (Finding 2).
///
/// `alpha` is the current overlay opacity (`0.0` = transparent,
/// `1.0` = full black). It is driven by [`drive_transition_fade`]; the
/// overlay node itself is spawned by [`spawn_fade_overlay`].
#[derive(Debug, Resource)]
pub struct TransitionFade {
    phase: FadePhase,
    alpha: f32,
}

impl Default for TransitionFade {
    fn default() -> Self {
        Self {
            phase: FadePhase::Idle,
            alpha: 0.0,
        }
    }
}

/// Request a state change through the fade overlay.
///
/// Contract: gameplay systems set `request.0 = Some(target)` instead of
/// `next_state.set(target)`. The fade driver picks the request up when
/// idle, fades out, applies the swap at full black, and fades back in.
/// A request written while a fade is already in flight waits in this
/// resource until the current fade completes (natural queue, last
/// write wins — mirroring `NextState` semantics).
///
/// The bench driver (`bench.rs`) bypasses this and writes `NextState`
/// directly: synthetic frame timing must not include the fade.
#[derive(Debug, Default, Resource)]
pub struct TransitionRequest(pub Option<GameState>);

/// Marker for the fullscreen fade overlay node.
#[derive(Debug, Component)]
struct FadeOverlay;

/// Spawns the fullscreen black overlay at startup. It sits above all UI
/// (`ZIndex::Global(i32::MAX)`) and starts fully transparent.
fn spawn_fade_overlay(mut commands: Commands) {
    commands.spawn((
        FadeOverlay,
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
            z_index: ZIndex::Global(i32::MAX),
            ..default()
        },
    ));
}

/// Drives the transition fade state machine every frame:
///
/// 1. When idle and a [`TransitionRequest`] is pending, begin fading out.
/// 2. While fading out, tick the 0.3s timer with [`Ease::CubicInOut`];
///    at full black, write the target into `NextState` (the state swap
///    runs at the end of this frame) and begin fading in.
/// 3. While fading in, tick the 0.3s timer in reverse; back to idle at
///    transparent.
///
/// A request that arrives mid-fade is left in the resource and picked
/// up when the current fade returns to idle.
fn drive_transition_fade(
    time: Res<Time>,
    mut fade: ResMut<TransitionFade>,
    mut request: ResMut<TransitionRequest>,
    mut next_state: ResMut<NextState<GameState>>,
    mut overlay: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    let delta_secs = time.delta_seconds();
    if matches!(fade.phase, FadePhase::Idle) {
        if let Some(target) = request.0.take() {
            fade.phase = FadePhase::FadingOut {
                target,
                elapsed_secs: 0.0,
            };
        }
    }
    // The phase is moved out, stepped, and written back: this keeps the
    // borrow of `fade` disjoint from the `request`/`next_state` accesses
    // and avoids a double-mutable-borrow inside the match arms.
    let phase = std::mem::replace(&mut fade.phase, FadePhase::Idle);
    // The swap target is staged (not applied inside the `match`) so the
    // `NextState` write happens after all resource borrows end.
    let mut swap_to: Option<GameState> = None;
    let (next_phase, alpha) = match phase {
        FadePhase::Idle => (FadePhase::Idle, 0.0),
        FadePhase::FadingOut {
            mut target,
            elapsed_secs,
        } => {
            // Last write wins, mirroring NextState semantics.
            if let Some(newer) = request.0.take() {
                target = newer;
            }
            let elapsed_secs = elapsed_secs + delta_secs;
            let t = (elapsed_secs / FADE_OUT_SECS).clamp(0.0, 1.0);
            let alpha = Ease::CubicInOut.sample(t);
            if t >= 1.0 {
                swap_to = Some(target);
                (FadePhase::FadingIn { elapsed_secs: 0.0 }, alpha)
            } else {
                (FadePhase::FadingOut { target, elapsed_secs }, alpha)
            }
        }
        FadePhase::FadingIn { elapsed_secs } => {
            let elapsed_secs = elapsed_secs + delta_secs;
            let t = (elapsed_secs / FADE_IN_SECS).clamp(0.0, 1.0);
            let alpha = 1.0 - Ease::CubicInOut.sample(t);
            if t >= 1.0 {
                (FadePhase::Idle, 0.0)
            } else {
                (FadePhase::FadingIn { elapsed_secs }, alpha)
            }
        }
    };
    fade.phase = next_phase;
    fade.alpha = alpha;
    if let Some(target) = swap_to {
        next_state.set(target);
    }
    for mut background in &mut overlay {
        background.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
}

/// Applies pending [`TransitionRequest`]s instantly, bypassing the fade.
///
/// Test-only helper for the playthrough suite (`playthrough.rs`): those
/// tests assert on game logic (win conditions, outage resolution), not
/// on the presentation-layer fade timing, so they drain the request
/// queue straight into `NextState` instead of running the 0.6s fade.
#[cfg(test)]
pub fn apply_transition_requests_instantly(
    mut request: ResMut<TransitionRequest>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(target) = request.0.take() {
        next_state.set(target);
    }
}

/// Registers the transition-fade resources and systems.
pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransitionFade>()
            .init_resource::<TransitionRequest>()
            .add_systems(Startup, spawn_fade_overlay)
            .add_systems(Update, drive_transition_fade);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_endpoints_hold() {
        for ease in [
            Ease::CubicInOut,
            Ease::CubicOut,
            Ease::SineInOut,
            Ease::BackOut,
        ] {
            assert!((ease.sample(0.0) - 0.0).abs() < 1e-6, "{ease:?} at 0");
            assert!((ease.sample(1.0) - 1.0).abs() < 1e-6, "{ease:?} at 1");
        }
    }

    #[test]
    fn cubic_in_out_is_symmetric() {
        let eased = Ease::CubicInOut.sample(0.25);
        assert!((eased + Ease::CubicInOut.sample(0.75) - 1.0).abs() < 1e-6);
        assert!(eased < 0.25, "slow start: {eased}");
    }

    #[test]
    fn back_out_overshoots() {
        let peak = (0..=100)
            .map(|i| Ease::BackOut.sample(i as f32 / 100.0))
            .fold(0.0_f32, f32::max);
        assert!(peak > 1.0, "expected overshoot, got {peak}");
    }

    #[test]
    fn sine_in_out_midpoint_is_half() {
        assert!((Ease::SineInOut.sample(0.5) - 0.5).abs() < 1e-6);
    }

    /// Drives `drive_transition_fade` through a full request cycle and
    /// asserts the fade-out → swap-at-black → fade-in sequence.
    #[test]
    fn transition_fade_swaps_state_at_full_black() {
        let mut world = World::new();
        world.init_resource::<TransitionFade>();
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.init_resource::<Time>();
        // Spawn the overlay so the system has something to tint.
        world.spawn((
            FadeOverlay,
            NodeBundle {
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
                ..default()
            },
        ));
        let mut schedule = Schedule::new(Update);
        schedule.add_systems(drive_transition_fade);

        // Request a transition; the first tick starts the fade-out.
        world.resource_mut::<TransitionRequest>().0 = Some(GameState::Results);
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(0.1));
        schedule.run(&mut world);
        let fade = world.resource::<TransitionFade>();
        assert!(matches!(fade.phase, FadePhase::FadingOut { .. }));
        assert!(fade.alpha > 0.0 && fade.alpha < 1.0);
        // No swap yet: NextState must stay Unchanged until full black.
        assert!(matches!(
            *world.resource::<NextState<GameState>>(),
            NextState::Unchanged
        ));

        // Finish the fade-out: the swap fires at full black.
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(1.0));
        schedule.run(&mut world);
        assert!(matches!(
            *world.resource::<NextState<GameState>>(),
            NextState::Pending(GameState::Results)
        ));
        let fade = world.resource::<TransitionFade>();
        assert!(matches!(fade.phase, FadePhase::FadingIn { .. }));

        // Finish the fade-in: back to idle and transparent.
        schedule.run(&mut world);
        let fade = world.resource::<TransitionFade>();
        assert!(matches!(fade.phase, FadePhase::Idle));
        assert_eq!(fade.alpha, 0.0);
    }

    #[test]
    fn mid_fade_out_request_overwrites_target() {
        let mut world = World::new();
        world.init_resource::<TransitionFade>();
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.init_resource::<Time>();
        let mut schedule = Schedule::new(Update);
        schedule.add_systems(drive_transition_fade);

        world.resource_mut::<TransitionRequest>().0 = Some(GameState::Playing);
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(0.1));
        schedule.run(&mut world);
        // A second request lands mid-fade-out: last write wins (mirrors
        // NextState), so the swap goes to the newer target, not the first.
        world.resource_mut::<TransitionRequest>().0 = Some(GameState::MainMenu);
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(1.0));
        schedule.run(&mut world);
        assert!(matches!(
            *world.resource::<NextState<GameState>>(),
            NextState::Pending(GameState::MainMenu)
        ));
    }

    #[test]
    fn mid_fade_in_request_waits_for_idle() {
        let mut world = World::new();
        world.init_resource::<TransitionFade>();
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.init_resource::<Time>();
        let mut schedule = Schedule::new(Update);
        schedule.add_systems(drive_transition_fade);

        // Complete the fade-out so we're in FadingIn.
        world.resource_mut::<TransitionRequest>().0 = Some(GameState::Playing);
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(1.0));
        schedule.run(&mut world);
        assert!(matches!(
            world.resource::<TransitionFade>().phase,
            FadePhase::FadingIn { .. }
        ));
        // A request during fade-in waits in the resource.
        world.resource_mut::<TransitionRequest>().0 = Some(GameState::MainMenu);
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(0.1));
        schedule.run(&mut world);
        // Still fading in to Playing; the MainMenu request is queued.
        assert!(matches!(
            world.resource::<TransitionFade>().phase,
            FadePhase::FadingIn { .. }
        ));
        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::MainMenu)
        );
        // Finish the fade-in: back to idle, then the queued request starts
        // a new fade-out.
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(1.0));
        schedule.run(&mut world);
        assert!(matches!(
            world.resource::<TransitionFade>().phase,
            FadePhase::Idle
        ));
        // Fresh small delta (the previous advance_by's 1.0s delta is stale
        // and would instantly complete the new fade-out).
        world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(0.01));
        schedule.run(&mut world);
        assert!(matches!(
            world.resource::<TransitionFade>().phase,
            FadePhase::FadingOut { .. }
        ));
    }
}
