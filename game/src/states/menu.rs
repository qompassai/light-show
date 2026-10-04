//! Main menu: keeper title artwork, tagline, Start (→ companion-select),
//! Credits. The companion picker lives on the companion-select screen
//! now (`states::companion_select`); this screen is just the title card.

use super::GameState;
use crate::anim::TransitionRequest;
use crate::board;
use crate::ui::neon::{spawn_neon_text, NeonText, NEON_CYAN, NEON_GOLD, NEON_INK};
use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // `board::teardown_board` here is a safe no-op the very first time
        // (Startup) this runs, since nothing has been spawned yet. It's
        // needed for every subsequent MainMenu entry (e.g. Results ->
        // MainMenu) now that board teardown is no longer tied to
        // `OnExit(Playing)` — see `playing::PlayingPlugin::build`.
        app.add_systems(
            OnEnter(GameState::MainMenu),
            (board::teardown_board, setup_menu),
        )
        .add_systems(
            Update,
            (handle_start_button, handle_credits_button).run_if(in_state(GameState::MainMenu)),
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

/// Maximum width of the menu tagline text node: the 48-character tagline
/// at 18px overflows the 720px window without a width constraint, wrapping
/// and clipping at the left edge. Kept in sync with the window resolution
/// in `lib.rs` (`WindowPlugin`).
const TAGLINE_MAX_W: f32 = 680.0;

/// Bottom padding for the menu column: the keeper title artwork carries the
/// "LIGHT SHOW" title in its center, so the interactive column (tagline,
/// buttons) lives in the lower third, over the dark night city.
const MENU_BOTTOM_PAD: f32 = 72.0;

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>, cameras: Query<&Camera>) {
    // The Credits screen reuses this camera (menu teardown only despawns
    // `MenuRoot`), so only spawn one when none exists — otherwise every
    // return from Credits would stack another camera.
    if cameras.is_empty() {
        commands.spawn(Camera2d);
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
                        ..default()
                    },
                    BackgroundColor(NEON_GOLD),
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
                    CreditsButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0.15, 0.22)),
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

fn teardown_menu(mut commands: Commands, query: Query<Entity, With<MenuRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    fn world_with_next_state() -> World {
        let mut world = World::new();
        world.init_resource::<TransitionRequest>();
        world.insert_resource(crate::audio::Sfx::for_tests());
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
}
