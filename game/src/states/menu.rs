//! Main menu: the keeper title artwork (dimmed, unedited) with the
//! menu items themed to it — bare text lines in the art's circuit
//! language. Unselected items are pale unlit cyan, like dark circuit
//! traces; the hovered item lights up gold with a cyan halo, extends a
//! circuit trace from a node dot, and echoes the logo's block letters.
//!
//! Flow (playtest round 2): when a save with progress exists,
//! **Continue** is the first-class entry and **New Game** is a fresh
//! start that asks inline before discarding that progress; with no
//! progress, New Game is simply the start. Erasing a save outright is
//! a small settings-corner action at the foot of the item column, not
//! the New Game path. The companion picker lives on the
//! companion-select screen (`states::companion_select`).

use super::GameState;
use crate::anim::TransitionRequest;
use crate::board;
use crate::cheat_codes::UnlockedSpecialists;
use crate::responsive::ArtBackdrop;
use crate::save::SaveData;
use crate::shaders::{AtmosphereMaterial, AtmosphereSettings};
use crate::ui::neon::{spawn_neon_text, NeonText, NEON_CYAN, NEON_DIM, NEON_GOLD};
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
        app.insert_resource(NewGameConfirm::default())
            .insert_resource(EraseConfirm::default())
            .add_systems(
                OnEnter(GameState::MainMenu),
                (board::teardown_board, setup_menu),
            )
            .add_systems(
                Update,
                (
                    handle_continue_button,
                    handle_warehouse_button,
                    handle_credits_button,
                    handle_new_game_button,
                    handle_erase_save_button,
                    update_menu_item_fx,
                )
                    .run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), teardown_menu);
    }
}

#[derive(Component)]
struct MenuRoot;

/// Marker for the menu's primary entry item — the one that enters the
/// game keeping the current save. That is **Continue** when a save
/// with progress exists, and **New Game** when there is nothing to
/// continue. The playthrough harness presses this marker to drive
/// menu → picker, so exactly one visible item carries it at a time.
#[derive(Component)]
pub(crate) struct StartButton;

/// Continue: enter the game with the loaded save. Only spawned when
/// the save carries progress (see `menu_entries`).
#[derive(Component)]
struct ContinueButton;

/// Opens the in-game credits screen (`GameState::Credits`), where the
/// CC-BY music attribution is user-visible as the licenses require.
#[derive(Component)]
struct CreditsButton;

/// Opens the Warehouse (`GameState::Warehouse`): tool lessons, quizzes,
/// and the Warehouse, where cores are spent.
#[derive(Component)]
struct WarehouseButton;

/// Starts a fresh game. With no progress to lose it enters the picker
/// directly; with progress, the first press arms `NewGameConfirm` and
/// rewrites the item's label, and only the second press discards the
/// save and starts (see `handle_new_game_button`).
#[derive(Component)]
struct NewGameButton;

/// Erases all saved progress without starting a game: the deliberate
/// settings-corner action at the foot of the item column. Two-step,
/// like New Game's confirm, but it stays on the menu.
#[derive(Component)]
struct EraseSaveButton;

/// The text copies inside `NewGameButton` (core + halo layers of both
/// label runs — the neon spawner tags every copy with this marker), so
/// the handler can rewrite the label in place as the fresh-start
/// confirmation arms and disarms.
#[derive(Component, Clone)]
struct NewGameLabel;

/// The text inside `EraseSaveButton` — a single plain `Text`, rewritten
/// in place as the erase confirmation arms and disarms.
#[derive(Component)]
struct EraseSaveLabel;

/// Which themed items the menu column holds. Pure data so the
/// presence rules (Continue and Erase save exist only when there is
/// progress to continue or erase) are unit-testable without a UI tree.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MenuEntry {
    Continue,
    NewGame,
    Warehouse,
    Credits,
    EraseSave,
}

/// The item column's contents for the current save state, top to
/// bottom. Continue leads when a save exists; Erase save trails the
/// column, demoted from the primary flow.
fn menu_entries(has_progress: bool) -> Vec<MenuEntry> {
    let mut entries = Vec::with_capacity(5);
    if has_progress {
        entries.push(MenuEntry::Continue);
    }
    entries.push(MenuEntry::NewGame);
    entries.push(MenuEntry::Warehouse);
    entries.push(MenuEntry::Credits);
    if has_progress {
        entries.push(MenuEntry::EraseSave);
    }
    entries
}

/// The entry that carries `StartButton`: Continue when it exists,
/// otherwise New Game (with nothing saved, starting *is* continuing).
fn primary_entry(has_progress: bool) -> MenuEntry {
    if has_progress {
        MenuEntry::Continue
    } else {
        MenuEntry::NewGame
    }
}

/// One themed item's selection parts, tagged with the item they belong
/// to so `update_menu_item_fx` can light them together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MenuItem {
    Continue,
    NewGame,
    Warehouse,
    Credits,
}

/// Tags a themed item's button root (carries its `Interaction`).
#[derive(Component, Clone, Copy)]
struct MenuItemRoot(MenuItem);

/// The node dot at the left end of an item's circuit trace.
#[derive(Component, Clone, Copy)]
struct MenuItemDot(MenuItem);

/// The hairline circuit trace between the node dot and the label.
#[derive(Component, Clone, Copy)]
struct MenuItemTrace(MenuItem);

/// One of an item's two label runs (`lit` = the gold selection state);
/// the FX system toggles which run is visible.
#[derive(Component, Clone, Copy)]
struct MenuItemRun {
    item: MenuItem,
    lit: bool,
}

/// Whether the item is currently lit. The FX system rewrites colors
/// and run visibility only when this flips, so it never fights other
/// writers frame to frame.
#[derive(Component, Default)]
struct MenuItemFxApplied(bool);

/// Label on the New Game item at rest.
const NEW_GAME_LABEL: &str = "NEW GAME";

/// Label while the fresh-start confirmation is armed. The item itself
/// carries the warning, so there is no separate dialog to manage — and
/// unlike the old erase banner, it names the actual consequence.
const NEW_GAME_CONFIRM_LABEL: &str = "DISCARD SAVE & START FRESH? PRESS AGAIN";

/// Label on the settings-corner erase action at rest.
const ERASE_SAVE_LABEL: &str = "erase save";

/// Label while the erase confirmation is armed.
const ERASE_SAVE_CONFIRM_LABEL: &str = "erase ALL progress? press again";

/// Two-step fresh-start confirmation state. Armed by the first New
/// Game press (only when a save exists); disarmed by the second press
/// (which discards and starts) or by leaving the menu, so an armed
/// item never survives into a later visit.
#[derive(Resource, Default)]
struct NewGameConfirm {
    armed: bool,
}

/// Two-step erase confirmation state for the settings-corner action.
/// Same arming discipline as `NewGameConfirm`.
#[derive(Resource, Default)]
struct EraseConfirm {
    armed: bool,
}

/// Bottom padding for the menu column: the keeper title artwork carries
/// the "LIGHT SHOW" title in its center, so the interactive column
/// (items) lives in the lower third, over the dark night city.
const MENU_BOTTOM_PAD: f32 = 72.0;

/// Dim layer over the keeper artwork: darkens the art (never edits it)
/// so the item column reads at a glance, the way the companion picker
/// dims the same card behind its panels.
const MENU_ART_DIM: Color = Color::srgba(0.02, 0.03, 0.07, 0.5);

/// An unselected item's label core: pale unlit cyan, the color of a
/// dark circuit trace in the artwork.
const ITEM_UNLIT_CORE: Color = Color::srgb(0.42, 0.62, 0.68);

/// Node dot / circuit trace colors at rest (dark, barely-there) and
/// lit (the selection state: gold node, cyan trace — the logo's
/// cyan-to-gold pairing).
const ITEM_DOT_UNLIT: Color = Color::srgba(0.3, 0.42, 0.48, 0.55);
const ITEM_TRACE_UNLIT: Color = Color::srgba(0.3, 0.45, 0.5, 0.22);
const ITEM_TRACE_LIT: Color = Color::srgba(0.435, 0.949, 1.0, 0.85);

/// Font size of the primary entry item (Continue, or New Game when it
/// is the start); secondary items sit one step down. Pre-adjust units
/// (`spawn_neon_text` applies `FONT_SIZE_ADJUST`).
const ITEM_FONT_PRIMARY: f32 = 26.0;
const ITEM_FONT_SECONDARY: f32 = 20.0;

/// True when the loaded save carries anything worth continuing or
/// erasing: progress, unlocks, currency, or Warehouse stock. Settings
/// alone (e.g. a volume tweak) do not make a "save" for menu purposes.
pub(crate) fn has_progress(save: &SaveData) -> bool {
    !save.completed_levels.is_empty()
        || !save.unlocked_specialists.is_empty()
        || save.cores > 0
        || !save.owned_gear.is_empty()
        || !save.warehouse_quiz_passed.is_empty()
        || !save.consumables.is_empty()
}

/// Tags the fullscreen atmosphere background quad (2D mesh, behind UI).
#[derive(Debug, Component)]
struct AtmosphereBg;

fn setup_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cameras: Query<&Camera>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<AtmosphereMaterial>>>,
    save: Option<Res<SaveData>>,
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

    let progress = save.as_deref().is_some_and(has_progress);
    let display: Handle<Font> = asset_server.load(crate::fonts::DISPLAY);
    let body: Handle<Font> = asset_server.load(crate::fonts::BODY);

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
            // the title. The art file itself is used exactly as
            // shipped — no recomposition (Matt's art decision,
            // 2026-10-06); `ArtBackdrop` cover-fits its node to the
            // real window (and swaps in the landscape variant on
            // wide windows), which is a display fit, not an edit.
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(asset_server.load("sprites/ui/title_artwork.png"))
                    .with_mode(NodeImageMode::Stretch),
                ArtBackdrop::title(),
            ));
            // Dim/vignette layer over the artwork (the art itself is not
            // edited): enough darkening for the item column to read at a
            // glance while the night city still shows through.
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(MENU_ART_DIM),
            ));
            // The item column: bare text lines, left-aligned so the node
            // dots line up like terminals on a circuit bus.
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|column| {
                    let primary = primary_entry(progress);
                    for entry in menu_entries(progress) {
                        match entry {
                            MenuEntry::Continue => spawn_menu_item(
                                column,
                                &display,
                                MenuItem::Continue,
                                ContinueButton,
                                (),
                                "CONTINUE",
                                ITEM_FONT_PRIMARY,
                                primary == MenuEntry::Continue,
                            ),
                            MenuEntry::NewGame => {
                                let font = if progress {
                                    ITEM_FONT_SECONDARY
                                } else {
                                    ITEM_FONT_PRIMARY
                                };
                                spawn_menu_item(
                                    column,
                                    &display,
                                    MenuItem::NewGame,
                                    NewGameButton,
                                    NewGameLabel,
                                    NEW_GAME_LABEL,
                                    font,
                                    primary == MenuEntry::NewGame,
                                );
                            }
                            MenuEntry::Warehouse => spawn_menu_item(
                                column,
                                &display,
                                MenuItem::Warehouse,
                                WarehouseButton,
                                (),
                                "WAREHOUSE",
                                ITEM_FONT_SECONDARY,
                                false,
                            ),
                            MenuEntry::Credits => spawn_menu_item(
                                column,
                                &display,
                                MenuItem::Credits,
                                CreditsButton,
                                (),
                                "CREDITS",
                                ITEM_FONT_SECONDARY,
                                false,
                            ),
                            MenuEntry::EraseSave => spawn_erase_save_item(column, &body),
                        }
                    }
                });
        });
}

/// Spawns one themed menu item: a bare text line (no box, no border)
/// preceded by its circuit punctuation — a node dot and a hairline
/// trace. The label exists twice (an unlit run and a lit run of the
/// same neon text); `update_menu_item_fx` swaps which run shows and
/// lights the dot/trace as the item's `Interaction` changes.
#[allow(clippy::too_many_arguments)]
fn spawn_menu_item<B, M>(
    parent: &mut ChildSpawnerCommands,
    display: &Handle<Font>,
    item: MenuItem,
    button_marker: B,
    label_marker: M,
    label: &'static str,
    font_size: f32,
    primary: bool,
) where
    B: Bundle,
    M: Bundle + Clone,
{
    let mut entity = parent.spawn((
        button_marker,
        MenuItemRoot(item),
        MenuItemFxApplied::default(),
        Button,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(4.0), Val::Px(7.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
    ));
    if primary {
        entity.insert(StartButton);
    }
    entity.with_children(|btn| {
        // Node dot: the terminal the item's circuit trace runs from.
        btn.spawn((
            MenuItemDot(item),
            Node {
                width: Val::Px(10.0),
                height: Val::Px(10.0),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(ITEM_DOT_UNLIT),
        ));
        // Circuit trace: a hairline of (so far unlit) fiber running
        // from the node into the label.
        btn.spawn((
            MenuItemTrace(item),
            Node {
                width: Val::Px(40.0),
                height: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(ITEM_TRACE_UNLIT),
        ));
        btn.spawn(Node {
            position_type: PositionType::Relative,
            ..default()
        })
        .with_children(|label_box| {
            // Unlit run: pale cyan core, the faintest halo — a dark
            // trace in the artwork's circuit pattern.
            label_box
                .spawn((MenuItemRun { item, lit: false }, Node::default()))
                .with_children(|run| {
                    spawn_neon_text(
                        run,
                        NeonText {
                            marker: label_marker.clone(),
                            value: label,
                            font: display.clone(),
                            font_size,
                            core: ITEM_UNLIT_CORE,
                            glow: NEON_CYAN,
                            glow_px: 1.0,
                            glow_inner_alpha: 0.15,
                            glow_outer_alpha: 0.06,
                            width: Val::Auto,
                            justify: Justify::Left,
                        },
                    );
                });
            // Lit run: gold core with a cyan halo — the logo's
            // cyan-to-gold letterforms. Hidden until selected.
            label_box
                .spawn((
                    MenuItemRun { item, lit: true },
                    Node::default(),
                    Visibility::Hidden,
                ))
                .with_children(|run| {
                    spawn_neon_text(
                        run,
                        NeonText {
                            marker: label_marker,
                            value: label,
                            font: display.clone(),
                            font_size,
                            core: NEON_GOLD,
                            glow: NEON_CYAN,
                            glow_px: 1.0,
                            glow_inner_alpha: 0.55,
                            glow_outer_alpha: 0.25,
                            width: Val::Auto,
                            justify: Justify::Left,
                        },
                    );
                });
        });
    });
}

/// The settings-corner erase action: deliberately *not* a themed item —
/// small plain text at the foot of the column, no node, no trace, no
/// glow. Erasing is a settings-level destructive action, not part of
/// the menu's primary flow.
fn spawn_erase_save_item(parent: &mut ChildSpawnerCommands, body: &Handle<Font>) {
    parent
        .spawn((
            EraseSaveButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(4.0), Val::Px(6.0)),
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|btn| {
            btn.spawn((
                EraseSaveLabel,
                Text::new(ERASE_SAVE_LABEL),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(NEON_DIM),
            ));
        });
}

/// Lights the hovered/pressed item and dims the rest: the selection
/// state is the artwork's gradient moment — gold node dot, lit cyan
/// trace, and the label's gold run swapped in for the unlit one.
fn update_menu_item_fx(
    mut items: Query<(
        &Interaction,
        &MenuItemRoot,
        &Children,
        &mut MenuItemFxApplied,
    )>,
    children_of: Query<&Children>,
    mut dots: Query<(&MenuItemDot, &mut BackgroundColor), Without<MenuItemTrace>>,
    mut traces: Query<(&MenuItemTrace, &mut BackgroundColor), Without<MenuItemDot>>,
    mut runs: Query<(&MenuItemRun, &mut Visibility)>,
) {
    for (interaction, root, children, mut applied) in &mut items {
        let selected = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        if applied.0 == selected {
            continue;
        }
        applied.0 = selected;
        for child in children.iter() {
            if let Ok((dot, mut bg)) = dots.get_mut(child) {
                if dot.0 == root.0 {
                    bg.0 = if selected { NEON_GOLD } else { ITEM_DOT_UNLIT };
                }
            }
            if let Ok((trace, mut bg)) = traces.get_mut(child) {
                if trace.0 == root.0 {
                    bg.0 = if selected {
                        ITEM_TRACE_LIT
                    } else {
                        ITEM_TRACE_UNLIT
                    };
                }
            }
            // The label box is the third child; its children are the two
            // label runs.
            if let Ok(grandchildren) = children_of.get(child) {
                for grandchild in grandchildren.iter() {
                    if let Ok((run, mut vis)) = runs.get_mut(grandchild) {
                        if run.item == root.0 {
                            *vis = if run.lit == selected {
                                Visibility::Visible
                            } else {
                                Visibility::Hidden
                            };
                        }
                    }
                }
            }
        }
    }
}

/// Continue goes to the companion-select screen with the loaded save
/// intact: the companion pick chooses which themed track comes next.
fn handle_continue_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<ContinueButton>)>,
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

/// New Game starts a fresh game:
/// - No progress to lose: the press simply starts (picker).
/// - Progress exists: the first press arms the confirmation and
///   rewrites the item's own label; the second press discards the
///   save via `reset_progress` and starts. Leaving the menu disarms
///   without erasing (see `teardown_menu`).
///
/// The persistence resources are optional parameters: partial app
/// builds (the playthrough harness installs `MenuPlugin` without the
/// save stack) must not panic on a missing resource. Without the
/// stack there is nothing persisted to discard; an armed confirmation
/// just disarms and logs, as before.
#[allow(clippy::too_many_arguments)]
fn handle_new_game_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<NewGameButton>)>,
    mut confirm: ResMut<NewGameConfirm>,
    mut labels: Query<&mut Text, With<NewGameLabel>>,
    mut save: Option<ResMut<SaveData>>,
    mut cores: Option<ResMut<Cores>>,
    mut specialists: Option<ResMut<UnlockedSpecialists>>,
    mut loadout: Option<ResMut<Loadout>>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        sfx.play(&mut commands, crate::audio::SfxKind::Click);
        if confirm.armed {
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
                    request.0 = Some(GameState::CompanionSelect);
                }
                _ => {
                    warn!(
                        "New Game confirmed without the save stack installed; nothing to discard"
                    );
                }
            }
            continue;
        }
        if save.as_deref().is_some_and(has_progress) {
            confirm.armed = true;
            for mut label in &mut labels {
                label.0 = NEW_GAME_CONFIRM_LABEL.to_owned();
            }
            continue;
        }
        // Nothing saved, nothing to lose: New Game is the start.
        request.0 = Some(GameState::CompanionSelect);
    }
}

/// The settings-corner erase: two-step like New Game's confirm, but it
/// never leaves the menu — erasing is housekeeping, not a way into the
/// game. With no progress there is nothing to erase and the press is
/// ignored entirely.
#[allow(clippy::too_many_arguments)]
fn handle_erase_save_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<EraseSaveButton>)>,
    mut confirm: ResMut<EraseConfirm>,
    mut labels: Query<&mut Text, With<EraseSaveLabel>>,
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
        if !confirm.armed && !save.as_deref().is_some_and(has_progress) {
            continue;
        }
        sfx.play(&mut commands, crate::audio::SfxKind::Click);
        if !confirm.armed {
            confirm.armed = true;
            for mut label in &mut labels {
                label.0 = ERASE_SAVE_CONFIRM_LABEL.to_owned();
            }
            continue;
        }
        confirm.armed = false;
        for mut label in &mut labels {
            label.0 = ERASE_SAVE_LABEL.to_owned();
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
                warn!("Erase save confirmed without the save stack installed; nothing to erase");
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
    mut new_game_confirm: ResMut<NewGameConfirm>,
    mut erase_confirm: ResMut<EraseConfirm>,
) {
    // Armed confirmations never survive leaving the menu: the next
    // visit starts disarmed, with the resting labels.
    new_game_confirm.armed = false;
    erase_confirm.armed = false;
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
        world.insert_resource(NewGameConfirm::default());
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

    /// A world with the save stack installed but no progress in it.
    fn world_fresh() -> World {
        let mut world = world_with_next_state();
        world.insert_resource(NewGameConfirm::default());
        world.insert_resource(EraseConfirm::default());
        world.insert_resource(SaveData::default());
        world.insert_resource(Cores(0));
        world.insert_resource(UnlockedSpecialists::default());
        world
    }

    // -- save / entry presence rules -------------------------------------

    #[test]
    fn a_default_save_has_no_progress() {
        assert!(!has_progress(&SaveData::default()));
    }

    #[test]
    fn any_real_progress_counts_but_settings_do_not() {
        let mut by_level = SaveData::default();
        by_level.complete_level("c1l1");
        assert!(has_progress(&by_level));

        let mut by_cores = SaveData::default();
        by_cores.cores = 1;
        assert!(has_progress(&by_cores));

        let mut by_specialist = SaveData::default();
        by_specialist.unlock_specialist(Companion::Clara);
        assert!(has_progress(&by_specialist));

        let mut by_gear = SaveData::default();
        by_gear.owned_gear.push("headlamp".to_owned());
        assert!(has_progress(&by_gear));

        // A volume tweak alone must not conjure a Continue item: there
        // is still nothing to continue.
        let mut by_settings = SaveData::default();
        by_settings.settings.master_volume = 0.5;
        assert!(!has_progress(&by_settings));
    }

    #[test]
    fn without_a_save_the_menu_starts_with_new_game() {
        assert_eq!(
            menu_entries(false),
            vec![MenuEntry::NewGame, MenuEntry::Warehouse, MenuEntry::Credits]
        );
        assert_eq!(primary_entry(false), MenuEntry::NewGame);
    }

    #[test]
    fn with_a_save_continue_leads_and_erase_save_trails() {
        assert_eq!(
            menu_entries(true),
            vec![
                MenuEntry::Continue,
                MenuEntry::NewGame,
                MenuEntry::Warehouse,
                MenuEntry::Credits,
                MenuEntry::EraseSave
            ]
        );
        assert_eq!(primary_entry(true), MenuEntry::Continue);
    }

    /// The tagline Matt ordered removed ("illegible over the art") must
    /// never come back. The needle is built from fragments so this
    /// test's own source doesn't match itself.
    #[test]
    fn the_removed_tagline_stays_removed() {
        let src = include_str!("menu.rs");
        for fragments in [
            &["route the ", "light. hit the ", "window."][..],
            &["survive the ", "storm."][..],
        ] {
            let needle: String = fragments.concat();
            assert!(
                !src.contains(&needle),
                "the removed menu tagline is back: {needle:?}"
            );
        }
    }

    // -- Continue ---------------------------------------------------------

    #[test]
    fn pressing_continue_requests_the_companion_select_state() {
        let mut world = world_with_next_state();
        world.spawn((ContinueButton, Interaction::Pressed));

        world.run_system_once(handle_continue_button);

        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::CompanionSelect)
        );
    }

    #[test]
    fn hovering_continue_requests_no_state_change() {
        let mut world = world_with_next_state();
        world.spawn((ContinueButton, Interaction::Hovered));

        world.run_system_once(handle_continue_button);

        assert_eq!(world.resource::<TransitionRequest>().0, None);
    }

    // -- Warehouse / Credits (unchanged destinations) ---------------------

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

    // -- New Game ----------------------------------------------------------

    /// With nothing saved, New Game is the start: one press enters the
    /// picker, nothing is armed, nothing is erased.
    #[test]
    fn new_game_without_a_save_starts_immediately() {
        let mut world = world_fresh();
        world.spawn((NewGameButton, Interaction::Pressed));

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(!world.resource::<NewGameConfirm>().armed);
        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::CompanionSelect)
        );
    }

    /// With a save, the first New Game press only arms the inline
    /// confirmation: every label copy flips to the warning and every
    /// piece of progress survives; no transition is requested.
    #[test]
    fn first_new_game_press_arms_confirmation_without_erasing() {
        let mut world = world_with_progress();
        world.spawn((NewGameButton, Interaction::Pressed));
        // Both label runs carry copies tagged NewGameLabel; use two
        // stand-ins to prove the handler rewrites every copy.
        let label_a = world.spawn((NewGameLabel, Text::new(NEW_GAME_LABEL))).id();
        let label_b = world.spawn((NewGameLabel, Text::new(NEW_GAME_LABEL))).id();

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(world.resource::<NewGameConfirm>().armed);
        assert_eq!(
            world.get::<Text>(label_a).unwrap().0,
            NEW_GAME_CONFIRM_LABEL
        );
        assert_eq!(
            world.get::<Text>(label_b).unwrap().0,
            NEW_GAME_CONFIRM_LABEL
        );
        assert_eq!(world.resource::<TransitionRequest>().0, None);
        assert!(world.resource::<SaveData>().is_completed("c1l1"));
        assert_eq!(world.resource::<Cores>().0, 30);
        assert!(world
            .resource::<UnlockedSpecialists>()
            .is_unlocked(&Companion::Clara));
    }

    /// Adversarial: hovering New Game must neither arm the
    /// confirmation nor touch progress.
    #[test]
    fn hovering_new_game_neither_arms_nor_erases() {
        let mut world = world_with_progress();
        world.spawn((NewGameButton, Interaction::Hovered));
        world.spawn((NewGameLabel, Text::new(NEW_GAME_LABEL)));

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(!world.resource::<NewGameConfirm>().armed);
        assert_eq!(world.resource::<TransitionRequest>().0, None);
        assert!(world.resource::<SaveData>().is_completed("c1l1"));
        assert_eq!(world.resource::<Cores>().0, 30);
    }

    /// The second press discards everything (save resource, runtime
    /// mirrors, on-disk file), restores the resting label, disarms —
    /// and starts the fresh game by entering the picker.
    #[test]
    fn second_new_game_press_discards_and_starts_fresh() {
        let _guard = crate::save::tests::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-menu-new-game");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let mut on_disk = SaveData::default();
        on_disk.complete_level("c1l1");
        crate::save::save(&on_disk).expect("seed save should succeed");

        let mut world = world_with_progress();
        world.resource_mut::<NewGameConfirm>().armed = true;
        world.spawn((NewGameButton, Interaction::Pressed));
        let label = world
            .spawn((NewGameLabel, Text::new(NEW_GAME_CONFIRM_LABEL)))
            .id();

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(!world.resource::<NewGameConfirm>().armed);
        assert_eq!(world.get::<Text>(label).unwrap().0, NEW_GAME_LABEL);
        let save = world.resource::<SaveData>();
        assert!(save.completed_levels.is_empty());
        assert_eq!(save.cores, 0);
        assert!(save.unlocked_specialists.is_empty());
        assert_eq!(world.resource::<Cores>().0, 0);
        assert!(!world
            .resource::<UnlockedSpecialists>()
            .is_unlocked(&Companion::Clara));
        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::CompanionSelect),
            "a confirmed New Game must start the fresh game, not sit on the menu"
        );
        // The file is gone too: a relaunch loads defaults, not the
        // discarded progress.
        assert!(!dir.join(crate::save::SAVE_FILENAME).exists());
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Adversarial: a partial app without the save stack (the shape
    /// the playthrough harness builds — `MenuPlugin` alone) must not
    /// panic when the fresh start is confirmed; it just disarms.
    #[test]
    fn confirming_new_game_without_the_save_stack_does_not_panic() {
        let mut world = world_with_next_state();
        world.insert_resource(NewGameConfirm { armed: true });
        world.spawn((NewGameButton, Interaction::Pressed));
        let label = world
            .spawn((NewGameLabel, Text::new(NEW_GAME_CONFIRM_LABEL)))
            .id();

        world.run_system_once(handle_new_game_button).unwrap();

        assert!(!world.resource::<NewGameConfirm>().armed);
        assert_eq!(world.get::<Text>(label).unwrap().0, NEW_GAME_LABEL);
        assert_eq!(world.resource::<TransitionRequest>().0, None);
    }

    // -- Erase save (settings-corner) --------------------------------------

    /// The first erase press only arms: the label flips to the warning
    /// and every piece of progress survives.
    #[test]
    fn first_erase_press_arms_confirmation_without_erasing() {
        let mut world = world_with_progress();
        world.spawn((EraseSaveButton, Interaction::Pressed));
        let label = world
            .spawn((EraseSaveLabel, Text::new(ERASE_SAVE_LABEL)))
            .id();

        world.run_system_once(handle_erase_save_button).unwrap();

        assert!(world.resource::<EraseConfirm>().armed);
        assert_eq!(
            world.get::<Text>(label).unwrap().0,
            ERASE_SAVE_CONFIRM_LABEL
        );
        assert!(world.resource::<SaveData>().is_completed("c1l1"));
        assert_eq!(world.resource::<Cores>().0, 30);
    }

    /// The second erase press wipes everything (including the disk
    /// file) and restores the resting label — and never asks for a
    /// state transition: erasing is housekeeping, not a way into the
    /// game. (`handle_erase_save_button` doesn't even take the
    /// transition resource; this pins the progress side.)
    #[test]
    fn second_erase_press_erases_all_progress() {
        let _guard = crate::save::tests::ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("light-show-menu-erase");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("LIGHTSHOW_SAVE_DIR", &dir);
        let mut on_disk = SaveData::default();
        on_disk.complete_level("c1l1");
        crate::save::save(&on_disk).expect("seed save should succeed");

        let mut world = world_with_progress();
        world.resource_mut::<EraseConfirm>().armed = true;
        world.spawn((EraseSaveButton, Interaction::Pressed));
        let label = world
            .spawn((EraseSaveLabel, Text::new(ERASE_SAVE_CONFIRM_LABEL)))
            .id();

        world.run_system_once(handle_erase_save_button).unwrap();

        assert!(!world.resource::<EraseConfirm>().armed);
        assert_eq!(world.get::<Text>(label).unwrap().0, ERASE_SAVE_LABEL);
        let save = world.resource::<SaveData>();
        assert!(save.completed_levels.is_empty());
        assert_eq!(save.cores, 0);
        assert!(save.unlocked_specialists.is_empty());
        assert_eq!(world.resource::<Cores>().0, 0);
        assert!(!dir.join(crate::save::SAVE_FILENAME).exists());
        std::env::remove_var("LIGHTSHOW_SAVE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With no progress there is nothing to erase: the press doesn't
    /// even arm (the button normally isn't spawned in that state; this
    /// pins the handler's own guard).
    #[test]
    fn erase_press_without_progress_does_not_arm() {
        let mut world = world_fresh();
        world.spawn((EraseSaveButton, Interaction::Pressed));
        let label = world
            .spawn((EraseSaveLabel, Text::new(ERASE_SAVE_LABEL)))
            .id();

        world.run_system_once(handle_erase_save_button).unwrap();

        assert!(!world.resource::<EraseConfirm>().armed);
        assert_eq!(world.get::<Text>(label).unwrap().0, ERASE_SAVE_LABEL);
    }

    /// Leaving the menu with either confirmation armed disarms both:
    /// the next visit must start from the resting state, not one press
    /// away from a discard the player may no longer intend.
    #[test]
    fn leaving_the_menu_disarms_both_confirmations() {
        let mut world = world_with_progress();
        world.resource_mut::<NewGameConfirm>().armed = true;
        world.resource_mut::<EraseConfirm>().armed = true;

        world.run_system_once(teardown_menu).unwrap();

        assert!(!world.resource::<NewGameConfirm>().armed);
        assert!(!world.resource::<EraseConfirm>().armed);
        assert!(world.resource::<SaveData>().is_completed("c1l1"));
    }

    // -- Selection FX --------------------------------------------------------

    /// Hovering an item lights it: gold node dot, lit trace, and the
    /// lit label run visible while the unlit run hides. Unhovering
    /// reverses all of it.
    #[test]
    fn hovering_an_item_lights_its_circuit_and_label() {
        let mut world = World::new();
        let item = MenuItem::Continue;
        let (mut dot, mut trace, mut unlit_run, mut lit_run) = (None, None, None, None);
        world
            .spawn((
                MenuItemRoot(item),
                MenuItemFxApplied::default(),
                Interaction::Hovered,
            ))
            .with_children(|btn| {
                dot = Some(
                    btn.spawn((
                        MenuItemDot(item),
                        Node::default(),
                        BackgroundColor(ITEM_DOT_UNLIT),
                    ))
                    .id(),
                );
                trace = Some(
                    btn.spawn((
                        MenuItemTrace(item),
                        Node::default(),
                        BackgroundColor(ITEM_TRACE_UNLIT),
                    ))
                    .id(),
                );
                btn.spawn(Node::default()).with_children(|label_box| {
                    unlit_run = Some(
                        label_box
                            .spawn((MenuItemRun { item, lit: false }, Node::default()))
                            .id(),
                    );
                    lit_run = Some(
                        label_box
                            .spawn((
                                MenuItemRun { item, lit: true },
                                Node::default(),
                                Visibility::Hidden,
                            ))
                            .id(),
                    );
                });
            });
        let (dot, trace, unlit_run, lit_run) = (
            dot.unwrap(),
            trace.unwrap(),
            unlit_run.unwrap(),
            lit_run.unwrap(),
        );

        world.run_system_once(update_menu_item_fx).unwrap();

        assert_eq!(world.get::<BackgroundColor>(dot).unwrap().0, NEON_GOLD);
        assert_eq!(
            world.get::<BackgroundColor>(trace).unwrap().0,
            ITEM_TRACE_LIT
        );
        assert_eq!(
            world.get::<Visibility>(unlit_run).unwrap(),
            &Visibility::Hidden
        );
        assert_eq!(
            world.get::<Visibility>(lit_run).unwrap(),
            &Visibility::Visible
        );

        // Unhover: everything dims back to the unlit circuit.
        let mut items = world.query::<&mut Interaction>();
        for mut interaction in items.iter_mut(&mut world) {
            *interaction = Interaction::None;
        }
        world.run_system_once(update_menu_item_fx).unwrap();

        assert_eq!(world.get::<BackgroundColor>(dot).unwrap().0, ITEM_DOT_UNLIT);
        assert_eq!(
            world.get::<BackgroundColor>(trace).unwrap().0,
            ITEM_TRACE_UNLIT
        );
        assert_eq!(
            world.get::<Visibility>(unlit_run).unwrap(),
            &Visibility::Visible
        );
        assert_eq!(
            world.get::<Visibility>(lit_run).unwrap(),
            &Visibility::Hidden
        );
    }
}
