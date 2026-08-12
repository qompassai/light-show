//! Séraphine — the anime-styled AI splice-drone companion. Purely reactive
//! and skippable: never gates puzzle solving, only comments on it and
//! offers optional favor-point hints. Keeping her fully optional is what
//! keeps this build eligible for F-Droid (no pay-to-skip, no anti-feature
//! dark patterns) while still giving Google Play a clear "fun mascot"
//! feature to market.

pub mod dialogue;
pub mod sprite;

use bevy::prelude::*;
use dialogue::DialogueBank;

pub struct SeraphinePlugin;

impl Plugin for SeraphinePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FavorPoints::default())
            .insert_resource(DialogueBank::load_default())
            .add_systems(Startup, spawn_seraphine)
            .add_systems(Update, animate_seraphine);
    }
}

/// Currency earned by clean splices / good decisions, spent only on
/// optional hints. No real-money purchase path exists anywhere in the
/// codebase — this is deliberate for store-compliance (see docs/GAME_DESIGN.md).
#[derive(Resource, Default)]
pub struct FavorPoints(pub u32);

#[derive(Component)]
pub struct Seraphine {
    pub mood: Mood,
    pub anim_timer: Timer,
    pub frame: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Idle,
    Blush,
    Wink,
    Pout,
    Celebrate,
    Alarmed,
}

impl Mood {
    /// 64x64 sprite-sheet row index for this mood (see
    /// assets/sprites/seraphine/seraphine_sheet.png layout in docs/ART_STYLE.md).
    pub fn sheet_row(&self) -> usize {
        match self {
            Mood::Idle => 0,
            Mood::Blush => 1,
            Mood::Wink => 2,
            Mood::Pout => 3,
            Mood::Celebrate => 4,
            Mood::Alarmed => 5,
        }
    }
}

fn spawn_seraphine(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture: Handle<Image> = asset_server.load("sprites/seraphine/seraphine_sheet.png");
    commands.spawn((
        Seraphine {
            mood: Mood::Idle,
            anim_timer: Timer::from_seconds(0.18, TimerMode::Repeating),
            frame: 0,
        },
        SpriteBundle {
            texture,
            transform: Transform::from_xyz(0.0, -400.0, 10.0).with_scale(Vec3::splat(4.0)),
            ..default()
        },
    ));
}

fn animate_seraphine(time: Res<Time>, mut query: Query<&mut Seraphine>) {
    for mut chan in &mut query {
        chan.anim_timer.tick(time.delta());
        if chan.anim_timer.just_finished() {
            chan.frame = (chan.frame + 1) % 4; // 4 frames per mood row
        }
    }
}
