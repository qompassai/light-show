//! Main menu: keeper title artwork, tagline, Start (→ companion-select),
//! Warehouse, Credits, New Game (a two-step erase of all saved progress).
//! The companion picker lives on the companion-select screen
//! now (`states::companion_select`); this screen is just the title card.

use super::GameState;
use crate::anim::TransitionRequest;
use crate::board;
use crate::cheat_codes::UnlockedSpecialists;
use crate::save::SaveData;
use crate::shaders::{AtmosphereMaterial, AtmosphereSettings};
use crate::ui::neon::{spawn_neon_text, NeonText, NEON_CYAN, NEON_GOLD, NEON_INK};
use crate::ui::{styled_button, ButtonPalette, BUTTON_BORDER};
use crate::waifu::Cores;
use crate::warehouse::Loadout;
use bevy::mesh::Mesh2d;
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // `board::teardown_board` here is a safe no-op the very first time
        // (Startup) this runs, since nothing has been spawned yet. It's
        // needed for every subsequent MainMenu entry (e.g. Results ->
        // MainMenu) now that board teardown is no longer tied to
        // `OnExit(Playing)` — see `playing::PlayingPlugin::build`.
        app.insert_resource(EraseConfirm::default())
            .add_systems(
                OnEnter(GameState::MainMenu),
                (board::teardown_board, setup_menu),
            )
            .add_systems(
                Update,
                (
                    handle_start_button,
                    handle_warehouse_button,
                    handle_credits_button,
                    handle_new_game_button,
                )
                    .run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), teardown_menu);
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
pub(crate) struct StartButton;

/// Opens the in-game credits screen (`GameState::Credits`), where the
/// CC-BY music attribution is user-visible as the licenses require.
#[derive(Component)]
struct CreditsButton;

/// Opens the Warehouse (`GameState::Warehouse`): tool lessons, quizzes,
/// and the Warehouse, where cores are spent.
#[derive(Component)]
struct WarehouseButton;

/// Resets all saved progress (New Game). Two-step: the first press arms
/// `EraseConfirm` and rewrites the button's label; only the second
/// press erases (see `handle_new_game_button`).
#[derive(Component)]
struct NewGameButton;

/// The text inside `NewGameButton`. A single plain `Text` — not the
/// layered neon text the other menu labels use — so the handler can
/// rewrite it in place as the confirmation arms and disarms.
#[derive(Component)]
struct NewGameLabel;

/// Label on the New Game button at rest.
const NEW_GAME_LABEL: &str = "New Game";

/// Label while the erase confirmation is armed. The button itself
/// carries the warning, so there is no separate dialog to manage.
const NEW_GAME_CONFIRM_LABEL: &str = "Erase all progress? Press again";

/// Two-step erase confirmation state. Armed by the first New Game
/// press; disarmed by the second press (which erases) or by leaving
/// the menu, so an armed button never survives into a later visit.
#[derive(Resource, Default)]
struct EraseConfirm {
    armed: bool,
}

/// Maximum width of the menu tagline text node: the 48-character tagline
/// at 18px overflows the 720px window without a width constraint, wrapping
/// and clipping at the left edge. Kept in sync with the window resolution
/// in `lib.rs` (`WindowPlugin`).
const TAGLINE_MAX_W: f32 = 680.0;

/// Bottom padding for the menu column: the keeper title artwork carries the
/// "LIGHT SHOW" title in its center, so the interactive column (tagline,
/// buttons) lives in the lower third, over the dark night city.
const MENU_BOTTOM_PAD: f32 = 72.0;

/// Tags the fullscreen atmosphere background quad (2D mesh, behind UI).
#[derive(Debug, Component)]
struct AtmosphereBg;

fn setup_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cameras: Query<&Camera>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<AtmosphereMaterial>>>,
) {
    // The Credits screen reuses this camera (menu teardown only despawns
    // `MenuRoot`), so only spawn one when none exists — otherwise every
    // return from Credits would stack another camera.
    if cameras.is_empty() {
        commands.spawn(Camera2d);
    }

    // Animated background: gradient + vignette + scanlines, behind all UI.
    // Oversized quad (2000px) covers the 720x1280 view at any aspect.
    // Skipped in headless tests where the material plugin isn't registered.
    if let Some(mut mats) = materials {
        commands.spawn((
            AtmosphereBg,
            Mesh2d(meshes.add(Rectangle::new(2000.0, 2000.0))),
            MeshMaterial2d(mats.add(AtmosphereMaterial {
                settings: AtmosphereSettings::default(),
            })),
            Transform::from_xyz(0.0, 0.0, -100.0),
        ));
    }

    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                // The keeper title artwork (see `image` below) carries
                // the "LIGHT SHOW" title in its center, so the column
                // sits in the lower third, over the dark night city.
                justify_content: JustifyContent::FlexEnd,
                padding: UiRect::bottom(Val::Px(MENU_BOTTOM_PAD)),
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.12)),
        ))
        .with_children(|parent| {
            // Keeper title artwork as the full-screen menu backdrop. It is
            // the first child and absolutely positioned (out of the flex
            // flow), so it paints behind everything below. It carries the
            // "LIGHT SHOW" title baked into its center, which is why the
            // old Aseprite title logo is gone — showing both would double
            // the title.
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(asset_server.load("sprites/ui/title_artwork.png")),
            ));
            // Width-constrained and centered: without a width the
            // 48-character tagline overflows the 720px window, wraps, and
            // clips at the left edge. Gold core with a cyan halo, echoing
            // the title card's cyan-to-gold letterforms.
            spawn_neon_text(
                parent,
                NeonText {
                    marker: (),
                    value: "route the light. hit the window. survive the storm.",
                    font: asset_server.load(crate::fonts::DISPLAY),
                    font_size: 18.0,
                    core: NEON_GOLD,
                    glow: NEON_CYAN,
                    glow_px: 1.0,
                    glow_inner_alpha: 0.55,
                    glow_outer_alpha: 0.25,
                    width: Val::Px(TAGLINE_MAX_W),
                    justify: Justify::Center,
                },
            );
            parent
                .spawn((
                    StartButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(14.0)),
                        border: BUTTON_BORDER,
                        border_radius: crate::ui::BUTTON_RADIUS,
                        ..default()
                    },
                    BackgroundColor(NEON_GOLD),
                    styled_button(ButtonPalette::gold()),
                ))
                .with_children(|btn| {
                    // Ink core with a deep-gold offset halo for depth on
                    // the bright gold face.
                    spawn_neon_text(
                        btn,
                        NeonText {
                            marker: (),
                            value: "Start",
                            font: asset_server.load(crate::fonts::BODY_MEDIUM),
                            font_size: 24.0,
                            core: NEON_INK,
                            glow: Color::srgba(0.5, 0.32, 0.1, 0.7),
                            glow_px: 1.0,
                            glow_inner_alpha: 0.5,
                            glow_outer_alpha: 0.22,
                            width: Val::Auto,
                            justify: Justify::Center,
                        },
                    );
                });
            parent
                .spawn((
                    WarehouseButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(10.0)),
                        border: BUTTON_BORDER,
                        border_radius: crate::ui::BUTTON_RADIUS,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0.15, 0.22)),
                    styled_button(ButtonPalette::slate()),
                ))
                .with_children(|btn| {
                    // Gold core with a cyan halo: the Credits face with
                    // the colors swapped, so the two read as siblings.
                    spawn_neon_text(
                        btn,
                        NeonText {
                            marker: (),
                            value: "Warehouse",
                            font: asset_server.load(crate::fonts::BODY_MEDIUM),
                            font_size: 18.0,
                            core: NEON_GOLD,
                            glow: NEON_CYAN,
                            glow_px: 1.0,
                            glow_inner_alpha: 0.55,
                            glow_outer_alpha: 0.25,
                            width: Val::Auto,
                            justify: Justify::Center,
                        },
                    );
                });
            parent
                .spawn((
                    CreditsButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(10.0)),
                        border: BUTTON_BORDER,
                        border_radius: crate::ui::BUTTON_RADIUS,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0.15, 0.22)),
                    styled_button(ButtonPalette::slate()),
                ))
                .with_children(|btn| {
                    // Cyan core with a gold halo on the dark slate face:
                    // the closest thing on this screen to a neon sign.
                    spawn_neon_text(
                        btn,
                        NeonText {
                            marker: (),
                            value: "Credits",
                            font: asset_server.load(crate::fonts::BODY_MEDIUM),
                            font_size: 18.0,
                            core: NEON_CYAN,
                            glow: NEON_GOLD,
                            glow_px: 1.0,
                            glow_inner_alpha: 0.55,
                            glow_outer_alpha: 0.25,
                            width: Val::Auto,
                            justify: Justify::Center,
                        },
                    );
                });
            parent
                .spawn((
                    NewGameButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(10.0)),
                        border: BUTTON_BORDER,
                        border_radius: crate::ui::BUTTON_RADIUS,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0.15, 0.22)),
                    styled_button(ButtonPalette::slate()),
                ))
                .with_children(|btn| {
                    // Plain single text (see `NewGameLabel`): the erase
                    // handler rewrites this label in place as the
                    // two-step confirmation arms and disarms.
                    btn.spawn((
                        NewGameLabel,
                        Text::new(NEW_GAME_LABEL),
                        TextFont {
                            font: asset_server.load(crate::fonts::BODY_MEDIUM).into(),
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

/// Start goes to the companion-select screen, not straight into play:
/// the companion pick chooses which themed two-level track comes next.
fn handle_start_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
            request.0 = Some(GameState::CompanionSelect);
        }
    }
}

/// Opens the Warehouse.
fn handle_warehouse_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<WarehouseButton>)>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
            request.0 = Some(GameState::Warehouse);
        }
    }
}

/// Opens the credits screen. The CC-BY music attribution must be
/// user-visible, so credits are one tap from the main menu.
fn handle_credits_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<CreditsButton>)>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
            request.0 = Some(GameState::Credits);
        }
    }
}

/// New Game is a two-step erase, not a one-tap accident: the first
/// press arms the confirmation and rewrites the button's own label;
/// the second press wipes all progress via `reset_progress`. Leaving
/// the menu disarms without erasing (see `teardown_menu`), so the
/// confirmation can never linger into a later visit.
///
/// The persistence resources are optional parameters: partial app
/// builds (the playthrough harness installs `MenuPlugin` without the
/// save stack) must not panic on a missing resource. Without the
/// stack there is nothing persisted to erase, so the second press
/// just disarms and logs.
#[allow(clippy::too_many_arguments)]
fn handle_new_game_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<NewGameButton>)>,
    mut confirm: ResMut<EraseConfirm>,
    mut labels: Query<&mut Text, With<NewGameLabel>>,
    mut save: Option<ResMut<SaveData>>,
    mut cores: Option<ResMut<Cores>>,
    mut specialists: Option<ResMut<UnlockedSpecialists>>,
    mut loadout: Option<ResMut<Loadout>>,
    sfx: Res<crate::audio::Sfx>,
) {
    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        sfx.play(&mut commands, crate::audio::SfxKind::Click);
        if !confirm.armed {
            confirm.armed = true;
            for mut label in &mut labels {
                label.0 = NEW_GAME_CONFIRM_LABEL.to_owned();
            }
            continue;
        }
        confirm.armed = false;
        for mut label in &mut labels {
            label.0 = NEW_GAME_LABEL.to_owned();
        }
        match (
            save.as_deref_mut(),
            cores.as_deref_mut(),
            specialists.as_deref_mut(),
        ) {
            (Some(save), Some(cores), Some(specialists)) => {
                reset_progress(save, cores, specialists, loadout.as_deref_mut());
            }
            _ => {
                warn!("New Game confirmed without the save stack installed; nothing to erase");
            }
        }
    }
}

/// Wipe all progression, in memory and on disk. The runtime mirrors
/// are reset alongside `SaveData` because other systems read them
/// directly — and `SavePlugin`'s sync would otherwise write their
/// stale values straight back into the fresh save. A disk failure is
/// logged, never panicked on: the in-memory reset has already
/// happened, and the next successful save rewrites the file anyway.
fn reset_progress(
    save: &mut SaveData,
    cores: &mut Cores,
    specialists: &mut UnlockedSpecialists,
    loadout: Option<&mut Loadout>,
) {
    *save = SaveData::default();
    cores.0 = 0;
    specialists.unlocked.clear();
    if let Some(loadout) = loadout {
        *loadout = Loadout::from_save(save);
    }
    if let Err(e) = crate::save::erase() {
        warn!("failed to erase the save file: {e}");
    }
}

fn teardown_menu(
    mut commands: Commands,
    query: Query<Entity, With<MenuRoot>>,
    bg: Query<Entity, With<AtmosphereBg>>,
    mut confirm: ResMut<EraseConfirm>,
) {
    // An armed erase confirmation never survives leaving the menu:
    // the next visit starts disarmed, with the resting label.
    confirm.armed = false;
    for entity in &query {
        commands.entity(entity).despawn();
    }
    for entity in &bg {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waifu::Companion;
    use bevy_ecs::system::RunSystemOnce;

    fn world_with_next_state() -> World {
        let mut world = World::new();
        world.init_resource::<TransitionRequest>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world
    }

    /// A world whose save and runtime mirrors carry real progress:
    /// one completed level, a core balance, one specialist unlock.
    fn world_with_progress() -> World {
        let mut world = world_with_next_state();
        world.insert_resource(EraseConfirm::default());
        let mut save = SaveData::default();
        save.complete_level("c1l1");
        save.cores = 30;
        save.unlock_specialist(Companion::Clara);
        world.insert_resource(save);
        world.insert_resource(Cores(30));
        let mut specialists = UnlockedSpecialists::default();
        specialists.unlock(Companion::Clara);
        world.insert_resource(specialists);
        world
    }

    #[test]
    fn pressing_start_requests_the_companion_select_state() {
        let mut world = world_with_next_state();
        world.spawn((StartButton, Interaction::Pressed));

        world.run_system_once(handle_start_button);

        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::CompanionSelect)
        );
    }

    #[test]
    fn hovering_start_requests_no_state_change() {
        let mut world = world_with_next_state();
        world.spawn((StartButton, Interaction::Hovered));

        world.run_system_once(handle_start_button);

        assert_eq!(world.resource::<TransitionRequest>().0, None);
    }

    #[test]
    fn pressing_the_credits_button_requests_the_credits_state() {
        let mut world = world_with_next_state();
        world.spawn((CreditsButton, Interaction::Pressed));

        world.run_system_once(handle_credits_button);

        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Credits)
        );
    }

    #[test]
    fn hovering_the_credits_button_requests_no_state_change() {
        let mut world = world_with_next_state();
        world.spawn((CreditsButton, Interaction::Hovered));

        world.run_system_once(handle_credits_button);

        assert_eq!(world.resource::<TransitionRequest>().0, None);
    }

    #[test]
    fn pressing_the_warehouse_button_requests_the_warehouse_state() {
        let mut world = world_with_next_state();
        world.spawn((WarehouseButton, Interaction::Pressed));

        world.run_system_once(handle_warehouse_button).unwrap();

        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Warehouse)
        );
    }

    #[test]
    fn hovering_the_warehouse_button_requests_no_state_change() {
        let mut world = world_with_next_state();
        world.spawn((WarehouseButton, Interaction::Hovered));

        world.run_system_once(handle_warehouse_button).unwrap();

        assert_eq!(world.resource::<TransitionRequest>().0, None);
    }

    /// The first New Game press only arms the confirmation: the label
    /// flips to the warning and every piece of progress survives.
    #[test]
    fn first_new_game_press_arms_confirmation_without_erasing() {
        let mut world = world_with_progress();
        world.spawn((NewGameButton, Interaction::Pressed));
        let label = world.spawn((NewGameLabel, Text::new(NEW_GAME_LABEL))).id();

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(world.resource::<EraseConfirm>().armed);
        assert_eq!(world.get::<Text>(label).unwrap().0, NEW_GAME_CONFIRM_LABEL);
        assert!(world.resource::<SaveData>().is_completed("c1l1"));
        assert_eq!(world.resource::<Cores>().0, 30);
        assert!(world
            .resource::<UnlockedSpecialists>()
            .is_unlocked(&Companion::Clara));
    }

    /// Adversarial: hovering the New Game button must neither arm the
    /// confirmation nor touch progress.
    #[test]
    fn hovering_new_game_neither_arms_nor_erases() {
        let mut world = world_with_progress();
        world.spawn((NewGameButton, Interaction::Hovered));
        world.spawn((NewGameLabel, Text::new(NEW_GAME_LABEL)));

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(!world.resource::<EraseConfirm>().armed);
        assert!(world.resource::<SaveData>().is_completed("c1l1"));
        assert_eq!(world.resource::<Cores>().0, 30);
    }

    /// The second press erases everything: the save resource, the
    /// runtime mirrors (Cores, specialist unlocks), the on-disk save
    /// file, and the button returns to its resting label, disarmed.
    #[test]
    fn second_new_game_press_erases_all_progress() {
        let _guard = crate::save::tests::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-menu-erase");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let mut on_disk = SaveData::default();
        on_disk.complete_level("c1l1");
        crate::save::save(&on_disk).expect("seed save should succeed");

        let mut world = world_with_progress();
        world.resource_mut::<EraseConfirm>().armed = true;
        world.spawn((NewGameButton, Interaction::Pressed));
        let label = world
            .spawn((NewGameLabel, Text::new(NEW_GAME_CONFIRM_LABEL)))
            .id();

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(!world.resource::<EraseConfirm>().armed);
        assert_eq!(world.get::<Text>(label).unwrap().0, NEW_GAME_LABEL);
        let save = world.resource::<SaveData>();
        assert!(save.completed_levels.is_empty());
        assert_eq!(save.cores, 0);
        assert!(save.unlocked_specialists.is_empty());
        assert_eq!(world.resource::<Cores>().0, 0);
        assert!(!world
            .resource::<UnlockedSpecialists>()
            .is_unlocked(&Companion::Clara));
        // The file is gone too: a relaunch loads defaults, not the
        // pre-erase progress.
        assert!(!dir.join(crate::save::SAVE_FILENAME).exists());
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Leaving the menu with the confirmation armed disarms it: the
    /// next visit must start from the resting state, not one press
    /// away from an erase the player may no longer intend.
    #[test]
    fn leaving_the_menu_disarms_the_confirmation() {
        let mut world = world_with_progress();
        world.resource_mut::<EraseConfirm>().armed = true;

        world.run_system_once(teardown_menu).unwrap();

        assert!(!world.resource::<EraseConfirm>().armed);
        assert!(world.resource::<SaveData>().is_completed("c1l1"));
    }

    /// Adversarial: a partial app without the save stack (the shape
    /// the playthrough harness builds — `MenuPlugin` alone) must not
    /// panic when the erase is confirmed; it just disarms.
    #[test]
    fn confirming_new_game_without_the_save_stack_does_not_panic() {
        let mut world = world_with_next_state();
        world.insert_resource(EraseConfirm { armed: true });
        world.spawn((NewGameButton, Interaction::Pressed));
        let label = world
            .spawn((NewGameLabel, Text::new(NEW_GAME_CONFIRM_LABEL)))
            .id();

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(!world.resource::<EraseConfirm>().armed);
        assert_eq!(world.get::<Text>(label).unwrap().0, NEW_GAME_LABEL);
    }
}
