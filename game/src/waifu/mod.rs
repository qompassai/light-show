//! The anime-styled AI companion system. Purely reactive and skippable:
//! never gates puzzle solving, only comments on it and offers optional
//! favor-point hints. Keeping the companion fully optional is what keeps
//! this build eligible for F-Droid (no pay-to-skip, no anti-feature dark
//! patterns) while still giving Google Play a clear "fun mascot" feature
//! to market.
//!
//! Four companions are offered, one per transmission medium — see
//! `Companion` and `docs/ART_STYLE.md` for each one's character brief:
//! Séraphine (fiber, the original), Ondine (coax), Linka (mobile), and
//! Lattice (Ethernet). The player picks one from the main menu
//! (`states::menu`); the choice is purely cosmetic/flavor-dialogue — the
//! link-budget math, outages, and win/fail conditions never depend on it.

pub mod dialogue;
pub mod sprite;

use bevy::prelude::*;
use dialogue::DialogueBank;

pub struct SeraphinePlugin;

impl Plugin for SeraphinePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FavorPoints::default())
            .insert_resource(SelectedCompanion::default())
            .insert_resource(DialogueBank::load_default(Companion::default()))
            .add_systems(Startup, spawn_companion)
            .add_systems(Update, (animate_companion, respawn_on_companion_change));
    }
}

/// Currency earned by clean splices / good decisions, spent only on
/// optional hints. No real-money purchase path exists anywhere in the
/// codebase — this is deliberate for store-compliance (see docs/GAME_DESIGN.md).
#[derive(Resource, Default)]
pub struct FavorPoints(pub u32);

/// Which companion the player picked on the main menu. Changing this at
/// runtime (see `states::menu::handle_companion_buttons`) triggers
/// `respawn_on_companion_change` to swap the on-screen sprite and reload
/// the matching `DialogueBank` — the only two things that vary per
/// companion.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedCompanion(pub Companion);

/// The four transmission-medium companions. See each one's character
/// brief in `docs/ART_STYLE.md` for silhouette/palette direction, and
/// `dialogue.rs` for their flavor-specific dialogue banks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Companion {
    /// Séraphine — fiber-optic splicing. The original companion.
    #[default]
    Fiber,
    /// Ondine — coax / broadband RF.
    Coax,
    /// Linka — mobile / cellular RF.
    Mobile,
    /// Lattice — Ethernet / copper LAN.
    Ethernet,
}

impl Companion {
    pub const ALL: [Companion; 4] = [
        Companion::Fiber,
        Companion::Coax,
        Companion::Mobile,
        Companion::Ethernet,
    ];

    /// The companion's in-universe name, shown in the menu's companion
    /// picker and in README/store copy.
    pub fn display_name(&self) -> &'static str {
        match self {
            Companion::Fiber => "Séraphine",
            Companion::Coax => "Ondine",
            Companion::Mobile => "Linka",
            Companion::Ethernet => "Lattice",
        }
    }

    /// Short one-line description of the medium each companion covers.
    pub fn tagline(&self) -> &'static str {
        match self {
            Companion::Fiber => "Fiber-optic OSP splicing",
            Companion::Coax => "Coax / broadband RF",
            Companion::Mobile => "Mobile / cellular RF",
            Companion::Ethernet => "Ethernet / copper LAN",
        }
    }

    /// Sprite-sheet asset path — same 6-mood-row × 4-frame 64×64 layout
    /// convention for every companion (see `docs/ART_STYLE.md`).
    pub fn sprite_path(&self) -> &'static str {
        match self {
            Companion::Fiber => "sprites/seraphine/seraphine_sheet.png",
            Companion::Coax => "sprites/ondine/ondine_sheet.png",
            Companion::Mobile => "sprites/linka/linka_sheet.png",
            Companion::Ethernet => "sprites/lattice/lattice_sheet.png",
        }
    }
}

#[derive(Component)]
pub struct CompanionSprite {
    pub companion: Companion,
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
    /// 64x64 sprite-sheet row index for this mood (see each companion's
    /// sprite sheet layout in docs/ART_STYLE.md — identical for all four).
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

fn companion_bundle(
    asset_server: &AssetServer,
    companion: Companion,
) -> (CompanionSprite, SpriteBundle) {
    let texture: Handle<Image> = asset_server.load(companion.sprite_path());
    (
        CompanionSprite {
            companion,
            mood: Mood::Idle,
            anim_timer: Timer::from_seconds(0.18, TimerMode::Repeating),
            frame: 0,
        },
        SpriteBundle {
            texture,
            transform: Transform::from_xyz(0.0, -400.0, 10.0).with_scale(Vec3::splat(4.0)),
            ..default()
        },
    )
}

fn spawn_companion(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected: Res<SelectedCompanion>,
) {
    commands.spawn(companion_bundle(&asset_server, selected.0));
}

/// Swaps the on-screen sprite and reloads the dialogue bank whenever
/// `SelectedCompanion` no longer matches the currently-spawned companion
/// (i.e. the player picked a different one on the main menu). Compares by
/// value rather than `Res::is_changed()` so this can't loop or double-fire
/// across the despawn/respawn it performs.
fn respawn_on_companion_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected: Res<SelectedCompanion>,
    mut dialogue: ResMut<DialogueBank>,
    query: Query<(Entity, &CompanionSprite)>,
) {
    let Ok((entity, sprite)) = query.get_single() else {
        return;
    };
    if sprite.companion == selected.0 {
        return;
    }
    commands.entity(entity).despawn();
    *dialogue = DialogueBank::load_default(selected.0);
    commands.spawn(companion_bundle(&asset_server, selected.0));
}

fn animate_companion(time: Res<Time>, mut query: Query<&mut CompanionSprite>) {
    for mut chan in &mut query {
        chan.anim_timer.tick(time.delta());
        if chan.anim_timer.just_finished() {
            chan.frame = (chan.frame + 1) % 4; // 4 frames per mood row
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    #[test]
    fn all_four_companions_have_distinct_names_and_sprite_paths() {
        let names: Vec<_> = Companion::ALL.iter().map(|c| c.display_name()).collect();
        let paths: Vec<_> = Companion::ALL.iter().map(|c| c.sprite_path()).collect();
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(names[i], names[j], "duplicate display name");
                assert_ne!(paths[i], paths[j], "duplicate sprite path");
            }
        }
    }

    #[test]
    fn selected_companion_defaults_to_fiber() {
        assert_eq!(SelectedCompanion::default().0, Companion::Fiber);
        assert_eq!(Companion::default(), Companion::Fiber);
    }

    fn test_app(selected: Companion) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.insert_resource(SelectedCompanion(selected));
        app.insert_resource(DialogueBank::load_default(selected));
        app
    }

    #[test]
    fn spawn_companion_creates_one_sprite_matching_the_selection() {
        let mut app = test_app(Companion::Coax);
        let world = app.world_mut();

        world.run_system_once(spawn_companion);

        let mut query = world.query::<&CompanionSprite>();
        let spawned: Vec<_> = query.iter(world).collect();
        assert_eq!(spawned.len(), 1);
        assert_eq!(spawned[0].companion, Companion::Coax);
        assert_eq!(spawned[0].mood, Mood::Idle);
    }

    #[test]
    fn respawn_on_companion_change_swaps_sprite_and_dialogue_when_selection_differs() {
        let mut app = test_app(Companion::Fiber);
        let world = app.world_mut();
        world.run_system_once(spawn_companion);

        // Player picks a different companion on the menu.
        world.resource_mut::<SelectedCompanion>().0 = Companion::Ethernet;
        world.run_system_once(respawn_on_companion_change);

        let mut query = world.query::<&CompanionSprite>();
        let spawned: Vec<_> = query.iter(world).collect();
        assert_eq!(
            spawned.len(),
            1,
            "old sprite should be despawned, not duplicated"
        );
        assert_eq!(spawned[0].companion, Companion::Ethernet);
        assert_eq!(
            world.resource::<DialogueBank>().random_line("clean_splice"),
            DialogueBank::load_default(Companion::Ethernet).random_line("clean_splice")
        );
    }

    #[test]
    fn respawn_on_companion_change_is_a_no_op_when_selection_is_unchanged() {
        let mut app = test_app(Companion::Mobile);
        let world = app.world_mut();
        world.run_system_once(spawn_companion);

        world.run_system_once(respawn_on_companion_change);

        let mut query = world.query::<&CompanionSprite>();
        assert_eq!(query.iter(world).count(), 1, "should not spawn a duplicate");
    }

    #[test]
    fn mood_sheet_rows_are_unique_and_within_bounds() {
        let moods = [
            Mood::Idle,
            Mood::Blush,
            Mood::Wink,
            Mood::Pout,
            Mood::Celebrate,
            Mood::Alarmed,
        ];
        let rows: Vec<_> = moods.iter().map(|m| m.sheet_row()).collect();
        for &row in &rows {
            assert!(row < sprite::MOOD_ROWS as usize);
        }
        assert_eq!(
            rows.len(),
            rows.iter().collect::<std::collections::HashSet<_>>().len()
        );
    }
}
