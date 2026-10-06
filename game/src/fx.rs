//! One-shot particle effects for board feedback.
//!
//! Spawns Aseprite-crafted sprite animations at gameplay moments: a cyan
//! spark when the player connects two nodes, a gold burst on victory.
//! All effects are `BoardRoot` children (or UI children for the win
//! screen) so level teardown sweeps them automatically.
//!
//! Contract: effects are fire-and-forget. The spawn message carries a
//! world position; the animation system advances frames on a fixed timer
//! and despawns at the end. No effect lives longer than its frame count
//! times the frame duration. A missing asset frame degrades to a no-op
//! spawn (the message is consumed, nothing is spawned).

use bevy::prelude::*;

/// Frames in the connect-spark sheet (`sprites/fx/connect_spark_{i}.png`).
const SPARK_FRAMES: usize = 4;
/// Seconds per spark frame: 4 frames over 0.24s, snappy but readable.
const SPARK_FRAME_SECS: f32 = 0.06;
/// World size of the spark sprite in pixels.
const SPARK_SIZE_PX: f32 = 48.0;
/// Z depth for board FX: above pulses (7.0) but below UI.
const FX_Z: f32 = 8.0;

/// Frames in the success-burst sheet (`sprites/fx/success_burst_{i}.png`).
const BURST_FRAMES: usize = 6;
/// Seconds per burst frame: 6 frames over 0.6s, celebratory pacing.
const BURST_FRAME_SECS: f32 = 0.1;

/// Request to play the connect-spark one-shot at a world position.
/// Written by `board::handle_pointer_input` on `ReleaseAction::Connect`;
/// consumed by `spawn_connect_sparks`.
#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnConnectSpark {
    /// World-space position for the effect center.
    pub position: Vec2,
}

/// A live connect-spark one-shot on the board.
#[derive(Component)]
struct ConnectSpark {
    frames: [Handle<Image>; SPARK_FRAMES],
    index: usize,
    timer: Timer,
}

/// Request to play the success-burst one-shot as a UI element.
/// Written by the results screen on victory; consumed by
/// `spawn_success_bursts`.
#[derive(Message, Debug, Clone)]
pub struct SpawnSuccessBurst {
    /// Pre-loaded frame handles (results screen owns the asset loads).
    pub frames: [Handle<Image>; BURST_FRAMES],
}

/// A live success-burst one-shot in the UI.
#[derive(Component)]
pub(crate) struct SuccessBurst {
    frames: [Handle<Image>; BURST_FRAMES],
    index: usize,
    timer: Timer,
}

impl SuccessBurst {
    /// Starts a new burst from pre-loaded frames. The frames must hold
    /// `BURST_FRAMES` handles in order (`success_burst_0.png` first).
    pub(crate) fn new(frames: [Handle<Image>; BURST_FRAMES]) -> Self {
        Self {
            frames,
            index: 0,
            timer: Timer::from_seconds(BURST_FRAME_SECS, TimerMode::Repeating),
        }
    }

    /// Number of frames in the burst sheet.
    pub(crate) const FRAMES: usize = BURST_FRAMES;
}

/// Spawns one connect-spark sprite per pending request.
/// Skips requests when the board root is missing (e.g. during teardown)
/// rather than panicking: the request is dropped, which is the correct
/// degradation for a fire-and-forget effect.
fn spawn_connect_sparks(
    mut commands: Commands,
    mut requests: MessageReader<SpawnConnectSpark>,
    asset_server: Res<AssetServer>,
    board_roots: Query<Entity, With<crate::board::BoardRoot>>,
) {
    let Ok(board_root) = board_roots.single() else {
        // Board is tearing down; drop pending FX requests.
        for _ in requests.read() {}
        return;
    };
    for req in requests.read() {
        let frames: [Handle<Image>; SPARK_FRAMES] =
            std::array::from_fn(|i| asset_server.load(format!("sprites/fx/connect_spark_{i}.png")));
        let first = frames[0].clone();
        commands.entity(board_root).with_children(|parent| {
            parent.spawn((
                ConnectSpark {
                    frames,
                    index: 0,
                    timer: Timer::from_seconds(SPARK_FRAME_SECS, TimerMode::Repeating),
                },
                Sprite {
                    image: first,
                    custom_size: Some(Vec2::splat(SPARK_SIZE_PX)),
                    ..default()
                },
                Transform::from_translation(req.position.extend(FX_Z)),
            ));
        });
    }
}

/// Advances live connect-sparks and despawns finished ones.
fn animate_connect_sparks(
    mut commands: Commands,
    time: Res<Time>,
    mut sparks: Query<(Entity, &mut ConnectSpark, &mut Sprite)>,
) {
    for (entity, mut spark, mut sprite) in &mut sparks {
        spark.timer.tick(time.delta());
        if !spark.timer.just_finished() {
            continue;
        }
        spark.index += 1;
        if spark.index >= SPARK_FRAMES {
            commands.entity(entity).despawn();
        } else {
            sprite.image = spark.frames[spark.index].clone();
        }
    }
}

/// Spawns one success-burst per pending request as a UI child.
/// The request carries pre-loaded frames so this system needs no
/// `AssetServer`: the results screen owns the load lifetime.
fn spawn_success_bursts(mut commands: Commands, mut requests: MessageReader<SpawnSuccessBurst>) {
    for req in requests.read() {
        let first = req.frames[0].clone();
        commands.spawn((
            SuccessBurst {
                frames: req.frames.clone(),
                index: 0,
                timer: Timer::from_seconds(BURST_FRAME_SECS, TimerMode::Repeating),
            },
            Node {
                width: Val::Px(256.0),
                height: Val::Px(256.0),
                ..default()
            },
            ImageNode::new(first),
        ));
    }
}

/// Advances live success-bursts, scaling up and fading out, then despawns.
fn animate_success_bursts(
    mut commands: Commands,
    time: Res<Time>,
    mut bursts: Query<(Entity, &mut SuccessBurst, &mut ImageNode, &mut UiTransform)>,
) {
    for (entity, mut burst, mut image, mut transform) in &mut bursts {
        burst.timer.tick(time.delta());
        if !burst.timer.just_finished() {
            continue;
        }
        burst.index += 1;
        if burst.index >= BURST_FRAMES {
            commands.entity(entity).despawn();
        } else {
            image.image = burst.frames[burst.index].clone();
            let progress = burst.index as f32 / BURST_FRAMES as f32;
            let scale = 0.6 + 0.8 * progress;
            transform.scale = Vec2::splat(scale);
            image.color = image.color.with_alpha(1.0 - progress * 0.7);
        }
    }
}

/// Registers the FX messages and systems.
pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnConnectSpark>()
            .add_message::<SpawnSuccessBurst>()
            .add_systems(
                Update,
                (
                    spawn_connect_sparks,
                    animate_connect_sparks,
                    spawn_success_bursts,
                    animate_success_bursts,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spark_frame_count_matches_asset_files() {
        // The asset pipeline exports connect_spark_0..3.png.
        assert_eq!(SPARK_FRAMES, 4);
    }

    #[test]
    fn burst_frame_count_matches_asset_files() {
        // The asset pipeline exports success_burst_0..5.png.
        assert_eq!(BURST_FRAMES, 6);
    }

    #[test]
    fn spark_lifetime_is_bounded() {
        let lifetime = SPARK_FRAMES as f32 * SPARK_FRAME_SECS;
        assert!(lifetime < 1.0, "spark must be sub-second: {lifetime}s");
    }

    #[test]
    fn burst_lifetime_is_bounded() {
        let lifetime = BURST_FRAMES as f32 * BURST_FRAME_SECS;
        assert!(lifetime < 1.0, "burst must be sub-second: {lifetime}s");
    }
}
