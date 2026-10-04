//! In-game credits screen: code/art/font credits plus the shipped music
//! attribution (CC-BY compliance), reachable from the main menu.
//!
//! The music attribution text is `include_str!`ed from
//! `game/assets/music/ATTRIBUTION.txt` — the same file that ships inside
//! the APK's assets — so the on-screen credits can never drift from the
//! shipped attribution. One source of truth: edit `ATTRIBUTION.txt` and
//! both update together.

use super::GameState;
use crate::anim::TransitionRequest;
use crate::fonts::FONT_SIZE_ADJUST;
use bevy::prelude::*;

pub struct CreditsPlugin;

impl Plugin for CreditsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Credits), setup_credits)
            .add_systems(
                Update,
                handle_back_button.run_if(in_state(GameState::Credits)),
            )
            .add_systems(OnExit(GameState::Credits), teardown_credits);
    }
}

/// Shipped music attribution, embedded at compile time. `ATTRIBUTION.txt`
/// is hard-wrapped at ~54 columns so it reads on a phone screen without
/// depending on the text renderer's wrapping behavior.
const ATTRIBUTION: &str = include_str!("../../assets/music/ATTRIBUTION.txt");

/// Code/art/font credits shown above the music block. Kept short: the
/// full licensing detail lives in `docs/CREDITS.md` and
/// `game/assets/music/CREDITS.md`.
const CODE_ART_FONT_CREDITS: &str = "Code: Qompass AI (GPL-3.0-or-later).\nArt: original tool-generated sprites; companion\nportraits AI-generated (rights cleared).\nFonts: Monaspace Neon + Inter (SIL OFL 1.1).";

#[derive(Component)]
struct CreditsRoot;

#[derive(Component)]
struct BackButton;

fn setup_credits(mut commands: Commands, asset_server: Res<AssetServer>) {
    // No camera spawn here: MainMenu's camera is still alive (menu
    // teardown only despawns `MenuRoot`), so this screen reuses it.
    commands
        .spawn((
            CreditsRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                row_gap: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(48.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.12)),
        ))
        .with_children(|parent| {
            let display_bold: Handle<Font> = asset_server.load(crate::fonts::DISPLAY_BOLD);
            let body: Handle<Font> = asset_server.load(crate::fonts::BODY);
            let body_medium: Handle<Font> = asset_server.load(crate::fonts::BODY_MEDIUM);
            // Parley (0.19) renders the same point size larger than the
            // 0.14 stack; FONT_SIZE_ADJUST keeps the visual size identical
            // (flagged for Matt's visual review).
            let text = |font: &Handle<Font>, content: &str, size: f32, color: Color| {
                (
                    Text::new(content),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(size * FONT_SIZE_ADJUST),
                        ..default()
                    },
                    TextColor(color),
                )
            };
            parent.spawn(text(
                &display_bold,
                "CREDITS",
                40.0,
                Color::srgb(0.6, 0.95, 1.0),
            ));
            parent.spawn(text(
                &body,
                CODE_ART_FONT_CREDITS,
                14.0,
                Color::srgb(0.8, 0.8, 0.9),
            ));
            // Back sits above the long music attribution: the attribution
            // text can overflow the viewport (Bevy UI has no scrolling),
            // and anything spawned after it would be unreachable.
            parent
                .spawn((
                    BackButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.28)),
                ))
                .with_children(|btn| {
                    btn.spawn(text(&body_medium, "Back", 20.0, Color::WHITE));
                });
            parent.spawn(text(
                &body,
                ATTRIBUTION,
                13.0,
                Color::srgb(0.75, 0.75, 0.85),
            ));
        });
}

fn handle_back_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut request: ResMut<TransitionRequest>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            request.0 = Some(GameState::MainMenu);
        }
    }
}

fn teardown_credits(mut commands: Commands, query: Query<Entity, With<CreditsRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    #[test]
    fn pressing_the_back_button_returns_to_the_main_menu() {
        let mut world = World::new();
        world.init_resource::<TransitionRequest>();
        world.spawn((BackButton, Interaction::Pressed));

        world.run_system_once(handle_back_button);

        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::MainMenu)
        );
    }

    #[test]
    fn hovering_the_back_button_requests_no_state_change() {
        let mut world = World::new();
        world.init_resource::<TransitionRequest>();
        world.spawn((BackButton, Interaction::Hovered));

        world.run_system_once(handle_back_button);

        assert_eq!(world.resource::<TransitionRequest>().0, None);
    }

    #[test]
    fn embedded_attribution_names_every_cc_by_artist() {
        // The compile-time embed is the compliance mechanism: if a
        // CC-BY artist is missing here, the in-game screen is missing
        // the legally required attribution.
        for artist in ["Eric Skiff", "Kevin MacLeod", "TeknoAXE"] {
            assert!(
                ATTRIBUTION.contains(artist),
                "in-game credits missing CC-BY artist: {artist}"
            );
        }
    }
}
