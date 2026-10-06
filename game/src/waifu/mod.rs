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
//! Lattice (Ethernet). The player picks one on the companion-select
//! screen (`states::companion_select`); the pick chooses *which* themed
//! two-level track they play (see `Companion::track_start_index`). Within
//! a level, the choice is still purely cosmetic/flavor-dialogue — the
//! link-budget math, outages, and win/fail conditions never depend on
//! which companion is on screen.

pub mod dialogue;
pub mod sprite;

use bevy::prelude::*;
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use dialogue::DialogueBank;

pub struct SeraphinePlugin;

impl Plugin for SeraphinePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FavorPoints::default())
            .insert_resource(SelectedCompanion::default())
            .insert_resource(DialogueBank::load_default(Companion::default()))
            .init_resource::<CompanionAtlasLayout>()
            .add_message::<SpliceReaction>()
            .add_systems(Startup, spawn_companion)
            .add_systems(
                Update,
                (
                    animate_companion,
                    sync_companion_atlas_index,
                    react_to_splice_events,
                    respawn_on_companion_change,
                    animate_mood_pop,
                ),
            );
    }
}

/// Fired by `board::handle_pointer_input` whenever a placement resolves
/// to a `Component::Splice` (fusion → a pleased reaction, mechanical → a
/// mildly disapproving one) so the companion sprite visibly reacts to
/// placement quality, not just outages and win/fail.
#[derive(Message, Clone, Copy)]
pub struct SpliceReaction(pub Mood);

/// Seconds for the mood-change scale pop (Finding 1).
const MOOD_POP_SECS: f32 = 0.15;
/// Peak scale of the mood pop: 1.0 -> 1.08 -> 1.0.
const MOOD_POP_PEAK: f32 = 1.08;

/// Scale-pop marker inserted on the companion sprite whenever its mood
/// changes (Finding 1). Reads as a visible reaction without needing
/// dual-sprite crossfade blending.
#[derive(Component)]
pub(crate) struct MoodPop {
    elapsed_secs: f32,
}

/// Triggers the mood-change scale pop on `entity`. Re-inserting while a
/// pop is in flight restarts it, so call sites must only call this when
/// the mood actually changes (not every frame of a steady state).
pub(crate) fn trigger_mood_pop(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).insert(MoodPop { elapsed_secs: 0.0 });
}

/// Plays the 0.15s mood pop: scale 1.0 -> 1.08 -> 1.0, both halves eased
/// with [`bevy::math::curve::EaseFunction::BackOut`] for a springy reaction feel.
fn animate_mood_pop(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut MoodPop)>,
) {
    use bevy::math::curve::{Curve, EaseFunction};
    for (entity, mut transform, mut pop) in &mut query {
        pop.elapsed_secs += time.delta_secs();
        let t = (pop.elapsed_secs / MOOD_POP_SECS).clamp(0.0, 1.0);
        let scale = if t < 0.5 {
            1.0 + (MOOD_POP_PEAK - 1.0) * EaseFunction::BackOut.sample_clamped(t * 2.0)
        } else {
            MOOD_POP_PEAK - (MOOD_POP_PEAK - 1.0) * EaseFunction::BackOut.sample_clamped((t - 0.5) * 2.0)
        };
        transform.scale = Vec3::splat(scale.max(0.01));
        if t >= 1.0 {
            transform.scale = Vec3::splat(1.0);
            commands.entity(entity).remove::<MoodPop>();
        }
    }
}

/// Applies the most recent `SpliceReaction` to every companion sprite
/// (there's only ever one on screen). Resets `frame` too so the new mood
/// starts its animation loop from the top.
pub(crate) fn react_to_splice_events(
    mut commands: Commands,
    mut events: MessageReader<SpliceReaction>,
    mut query: Query<(Entity, &mut CompanionSprite)>,
    sfx: Res<crate::audio::Sfx>,
) {
    let Some(reaction) = events.read().last() else {
        return;
    };
    // The companion visibly reacts (new mood + dialogue line): a soft
    // blip marks the moment, distinct from the splice thunk itself.
    sfx.play(&mut commands, crate::audio::SfxKind::Dialogue);
    for (entity, mut sprite) in &mut query {
        sprite.mood = reaction.0;
        sprite.frame = 0;
        trigger_mood_pop(&mut commands, entity);
    }
}

/// The one shared grid layout every companion sheet uses. Built once via
/// `FromWorld` (which needs mutable access to `Assets<TextureAtlasLayout>`,
/// unavailable at plain `insert_resource` call sites) and cloned into each
/// companion's `Sprite::texture_atlas`.
#[derive(Resource)]
struct CompanionAtlasLayout(Handle<TextureAtlasLayout>);

impl FromWorld for CompanionAtlasLayout {
    fn from_world(world: &mut World) -> Self {
        let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
        Self(layouts.add(sprite::atlas_layout()))
    }
}

/// Currency earned by clean splices / good decisions, spent only on
/// optional hints. No real-money purchase path exists anywhere in the
/// codebase — this is deliberate for store-compliance (see docs/GAME_DESIGN.md).
#[derive(Resource, Default)]
pub struct FavorPoints(pub u32);

/// Which companion the player picked on the companion-select screen.
/// Changing this at runtime (see
/// `states::companion_select::handle_select_buttons`) triggers
/// `respawn_on_companion_change` to swap the on-screen sprite and reload
/// the matching `DialogueBank` — the only two things that vary per
/// companion.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedCompanion(pub Companion);

/// The transmission-medium companions (base four) plus unlockable specialists. See each one's character
/// brief in `docs/ART_STYLE.md` for silhouette/palette direction, and
/// `dialogue.rs` for their flavor-specific dialogue banks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    /// Clara — Calix CMS provisioning (USA). Unlock: JUSTINBAILEY or Konami.
    Clara,
    /// Aino — Nokia AMS network operations (Finland). Unlock: ABACABB or Konami.
    Aino,
    /// Hikari — FTTH/OSP field buildout (Japan). Unlock: BLASTPROCESSING or Konami.
    Hikari,
    /// Léa — ITU-inspired WA 09 Telecom Admin prep (Switzerland). Unlock: TRIFORCE or Konami.
    Lea,
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
            Companion::Clara => "Clara",
            Companion::Aino => "Aino",
            Companion::Hikari => "Hikari",
            Companion::Lea => "Léa",
        }
    }

    /// Short one-line description of the medium each companion covers.
    pub fn tagline(&self) -> &'static str {
        match self {
            Companion::Fiber => "Fiber-optic OSP splicing",
            Companion::Coax => "Coax / broadband RF",
            Companion::Mobile => "Mobile / cellular RF",
            Companion::Ethernet => "Ethernet / copper LAN",
            Companion::Clara => "Calix CMS provisioning",
            Companion::Aino => "Nokia AMS network ops",
            Companion::Hikari => "FTTH/OSP field buildout",
            Companion::Lea => "WA 09 Telecom Admin prep",
        }
    }

    /// One-liner on the companion-select card: the track's flavor, not
    /// the mechanics — the briefing teaches those.
    pub fn select_hook(&self) -> &'static str {
        match self {
            Companion::Fiber => "Every decibel is earned. Spend them wisely.",
            Companion::Coax => "Gain is easy. Balance is the job.",
            Companion::Mobile => "Distance always wins — unless you regenerate.",
            Companion::Ethernet => "No decibels here. Just physics and paperwork.",
            Companion::Clara => "Provision right the first time. Every ONT counts.",
            Companion::Aino => "The alarms never lie. Learn to read them.",
            Companion::Hikari => "Measure twice, splice once.",
            Companion::Lea => "Know the code. Pass the test. Own the network.",
        }
    }

    /// Levels per companion track (see `level::LEVEL_SOURCES`).
    pub const TRACK_LEN: usize = 2;

    /// First level index (into `level::LEVEL_SOURCES`) of this companion's
    /// two-level track. Tracks are laid out two at a time in
    /// `Companion::ALL` order.
    /// `None` for specialists: no track is built yet, so they are unplayable;
    /// `None` must never be turned into a phantom level index.
    pub fn track_start_index(&self) -> Option<usize> {
        match self {
            Companion::Fiber => Some(0),
            Companion::Coax => Some(2),
            Companion::Mobile => Some(4),
            Companion::Ethernet => Some(6),
            Companion::Clara | Companion::Aino | Companion::Hikari | Companion::Lea => None,
        }
    }

    /// The companion's discipline accent color — the neon outline and FX
    /// tint on the companion-select card and anywhere else her identity
    /// needs to read at a glance (see `docs/ART_STYLE.md`).
    pub fn accent(&self) -> Color {
        match self {
            Companion::Fiber => Color::srgb(1.0, 0.435, 0.682), // #ff6fae
            Companion::Coax => Color::srgb(0.180, 0.769, 0.710), // #2ec4b6
            Companion::Mobile => Color::srgb(0.220, 0.741, 0.973), // #38bdf8
            Companion::Ethernet => Color::srgb(0.918, 0.702, 0.031), // #eab308
            Companion::Clara => Color::srgb(0.0, 0.85, 0.85),
            Companion::Aino => Color::srgb(1.0, 0.65, 0.0),
            Companion::Hikari => Color::srgb(0.6, 1.0, 0.2),
            Companion::Lea => Color::srgb(0.65, 0.2, 0.25),
        }
    }

    /// File stem of the companion-select silhouette animation frames:
    /// `sprites/picker/{stem}_select_{0..6}.png` (see `gen_picker.lua`).
    pub fn picker_stem(&self) -> &'static str {
        match self {
            Companion::Fiber => "seraphine",
            Companion::Coax => "ondine",
            Companion::Mobile => "linka",
            Companion::Ethernet => "lattice",
            Companion::Clara => "clara",
            Companion::Aino => "aino",
            Companion::Hikari => "hikari",
            Companion::Lea => "lea",
        }
    }

    /// Sprite-sheet asset path — same 6-mood-row × 4-frame 288×576
    /// full-body layout convention for every companion (see
    /// `docs/ART_STYLE.md`). Loads the mature-redesign sheets; the
    /// base 96×192 sheets remain in the tree as the standard set.
    pub fn sprite_path(&self) -> &'static str {
        match self {
            Companion::Fiber => "sprites/seraphine/seraphine_sheet_fullbody_mature.png",
            Companion::Coax => "sprites/ondine/ondine_sheet_fullbody_mature.png",
            Companion::Mobile => "sprites/linka/linka_sheet_fullbody_mature.png",
            Companion::Ethernet => "sprites/lattice/lattice_sheet_fullbody_mature.png",
            Companion::Clara => "sprites/clara/clara_sheet_fullbody.png",
            Companion::Aino => "sprites/aino/aino_sheet_fullbody.png",
            Companion::Hikari => "sprites/hikari/hikari_sheet_fullbody.png",
            Companion::Lea => "sprites/lea/lea_sheet_fullbody.png",
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
    /// 96x192 sprite-sheet row index for this mood (see each companion's
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
    atlas_layout: &CompanionAtlasLayout,
    companion: Companion,
) -> (CompanionSprite, Sprite, Transform) {
    let texture: Handle<Image> = asset_server.load(companion.sprite_path());
    let mood = Mood::Idle;
    let frame = 0;
    (
        CompanionSprite {
            companion,
            mood,
            anim_timer: Timer::from_seconds(0.18, TimerMode::Repeating),
            frame,
        },
        Sprite {
            image: texture,
            texture_atlas: Some(TextureAtlas {
                layout: atlas_layout.0.clone(),
                index: sprite::atlas_index(mood.sheet_row(), frame),
            }),
            ..default()
        },
        Transform::from_xyz(0.0, -240.0, 10.0).with_scale(Vec3::splat(1.0)),
    )
}

fn spawn_companion(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas_layout: Res<CompanionAtlasLayout>,
    selected: Res<SelectedCompanion>,
) {
    commands.spawn(companion_bundle(&asset_server, &atlas_layout, selected.0));
}

/// Swaps the on-screen sprite and reloads the dialogue bank whenever
/// `SelectedCompanion` no longer matches the currently-spawned companion
/// (i.e. the player picked a different one on the main menu). Compares by
/// value rather than `Res::is_changed()` so this can't loop or double-fire
/// across the despawn/respawn it performs.
fn respawn_on_companion_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    atlas_layout: Res<CompanionAtlasLayout>,
    selected: Res<SelectedCompanion>,
    mut dialogue: ResMut<DialogueBank>,
    query: Query<(Entity, &CompanionSprite)>,
) {
    let Ok((entity, sprite)) = query.single() else {
        return;
    };
    if sprite.companion == selected.0 {
        return;
    }
    commands.entity(entity).despawn();
    *dialogue = DialogueBank::load_default(selected.0);
    commands.spawn(companion_bundle(&asset_server, &atlas_layout, selected.0));
}

fn animate_companion(time: Res<Time>, mut query: Query<&mut CompanionSprite>) {
    for mut chan in &mut query {
        chan.anim_timer.tick(time.delta());
        if chan.anim_timer.just_finished() {
            chan.frame = (chan.frame + 1) % 4; // 4 frames per mood row
        }
    }
}

/// Keeps each companion's on-screen `TextureAtlas` cell in sync with its
/// `CompanionSprite.{mood, frame}`. Split out from `animate_companion` so
/// mood changes fired from other states (e.g. `states::outage::alarm_companion`
/// on `OnEnter(OutageActive)`) are reflected the instant they happen rather
/// than waiting on the animation timer.
fn sync_companion_atlas_index(
    mut query: Query<(&CompanionSprite, &mut Sprite), Changed<CompanionSprite>>,
) {
    for (companion, mut sprite) in &mut query {
        // `texture_atlas` is always `Some`: `companion_bundle` sets it and
        // nothing ever removes it, so `expect` upholds the invariant.
        let atlas = sprite
            .texture_atlas
            .as_mut()
            .expect("companion sprite must keep its texture atlas");
        atlas.index = sprite::atlas_index(companion.mood.sheet_row(), companion.frame);
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
        app.init_asset::<bevy::image::TextureAtlasLayout>();
        app.insert_resource(SelectedCompanion(selected));
        app.insert_resource(DialogueBank::load_default(selected));
        app.init_resource::<CompanionAtlasLayout>();
        app
    }

    #[test]
    fn spawn_companion_creates_one_sprite_matching_the_selection() {
        let mut app = test_app(Companion::Coax);
        let world = app.world_mut();

        world.run_system_once(spawn_companion);

        let mut query = world.query::<(&CompanionSprite, &Sprite)>();
        let spawned: Vec<_> = query.iter(world).collect();
        assert_eq!(spawned.len(), 1);
        assert_eq!(spawned[0].0.companion, Companion::Coax);
        assert_eq!(spawned[0].0.mood, Mood::Idle);
        assert_eq!(
            spawned[0]
                .1
                .texture_atlas
                .as_ref()
                .expect("companion sprite must keep its texture atlas")
                .index,
            sprite::atlas_index(0, 0)
        );
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
        world
            .run_system_once(spawn_companion)
            .expect("spawn_companion system should run in test");

        world
            .run_system_once(respawn_on_companion_change)
            .expect("respawn_on_companion_change system should run in test");

        let mut query = world.query::<&CompanionSprite>();
        assert_eq!(query.iter(world).count(), 1, "should not spawn a duplicate");
    }

    #[test]
    fn every_companion_has_a_non_empty_tagline_distinct_from_its_display_name() {
        for companion in Companion::ALL {
            assert!(!companion.tagline().is_empty());
            assert_ne!(companion.tagline(), companion.display_name());
        }
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
