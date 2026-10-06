//! App-level playthrough tests.
//!
//! These drive the real Bevy app — the actual plugins, the true
//! `GameState` machine, the real `board::handle_pointer_input` pointer
//! pipeline — from the main menu through a companion's two-level track to
//! the results screen. Nothing here reimplements game logic:
//!
//! * pill tap positions come from `board::pill_world_pos` (the same
//!   function the renderer uses to lay pills out),
//! * taps go through `board::handle_pointer_input` with a real
//!   `ButtonInput<MouseButton>` press/release pair,
//! * menu, picker, and results buttons go through their real
//!   `Interaction`-driven handlers,
//! * outage clocks are driven by setting `LevelClock` / ticking `Outage`
//!   directly instead of sleeping.
//!
//! The app is headless: `MinimalPlugins` + asset/input/state plumbing,
//! no window, no renderer, no audio (`MusicPlugin` excluded). Outage
//! timing is deterministic — no wall-clock dependence, no display needed.

use bevy::asset::AssetApp;
use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigStore};
use bevy::gizmos::gizmos::GizmoStorage;
use bevy::input::mouse::MouseButton;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_ecs::system::RunSystemOnce;

use crate::board::{self, PlacedChoices, PointerWorld};
use crate::level::{CurrentLevelIndex, LevelDef};
use crate::states::companion_select::{BackButton, CompanionSelectPlugin, SelectButton};
use crate::states::menu::{MenuPlugin, StartButton};
use crate::states::outage::{ActiveOutage, OutagePlugin};
use crate::states::playing::{LevelClock, PlayingPlugin};
use crate::states::results::{ResultAction, ResultsPlugin};
use crate::states::{GameState, LevelOutcome};
use crate::ui::LedgerUiPlugin;
use crate::waifu::{Companion, SelectedCompanion, SeraphinePlugin};

/// Frames to run after any action so queued `NextState` transitions and
/// their `OnEnter`/`OnExit` schedules land. A transition queued during
/// `Update` applies on the next schedule run and its enter-systems on the
/// run after, so one frame is never enough.
const SETTLE_FRAMES: usize = 5;

/// World position guaranteed to be far from every pill and node on every
/// level board: tapping here must place nothing.
const EMPTY_BOARD_SPOT: Vec2 = Vec2::new(5000.0, 5000.0);

/// Build the headless playthrough app: the real game plugins over headless
/// Bevy. `MusicPlugin` is excluded (needs an audio device); benches are
/// excluded (not part of the playthrough path). The `Sfx` resource is
/// present with dummy handles so the event systems under test run their
/// real `Sfx::play` call sites — no audio system runs headless, so the
/// handles are never resolved and nothing needs a device.
fn playthrough_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::input::InputPlugin,
        // Before `init_state`: it unwraps the `StateTransition` schedule
        // this plugin installs.
        StatesPlugin,
    ));
    // `SeraphinePlugin` builds the companion `TextureAtlasLayout` at
    // startup via `Assets<TextureAtlasLayout>`; the full `SpritePlugin`
    // needs the render shader store, so init just the atlas asset here.
    // Menu/board setup `asset_server.load()`s images and fonts, which
    // likewise need their asset stores even though nothing is rendered.
    app.init_asset::<bevy::image::TextureAtlasLayout>();
    app.init_asset::<bevy::image::Image>();
    app.init_asset::<bevy::text::Font>();
    app.init_asset::<bevy::mesh::Mesh>();
    // `draw_board_gizmos` buffers into the `Gizmos` system param, which
    // needs a registered config and its `()`-clear storage. The full
    // `GizmoPlugin` / `init_gizmo_group` also schedule render-coupled mesh
    // systems, so register the group manually instead: lines accumulate in
    // the store, never drawn.
    app.world_mut()
        .get_resource_or_insert_with::<GizmoConfigStore>(Default::default)
        .insert(GizmoConfig::default(), DefaultGizmoConfigGroup);
    app.init_resource::<GizmoStorage<DefaultGizmoConfigGroup, ()>>();
    app.init_state::<GameState>();
    app.insert_resource(crate::audio::Sfx::for_tests());
    // `AnimPlugin` (in the real game app) initializes this; the headless
    // app doesn't add that plugin, so init it here. Idempotent if the
    // plugin later does the same.
    app.init_resource::<crate::anim::TransitionRequest>();
    // `FxPlugin` (in the real game app) initializes these; the headless
    // app doesn't add that plugin, so init them here. The board's pill
    // systems write `SpawnConnectSpark` on placement.
    app.init_resource::<bevy::ecs::message::Messages<crate::fx::SpawnConnectSpark>>();
    app.init_resource::<bevy::ecs::message::Messages<crate::fx::SpawnSuccessBurst>>();
    // UI plugin resources (systems not added to avoid B0001)
    app.init_resource::<crate::states::api_console::ApiProgress>();
    app.init_resource::<crate::states::triage_console::TriageProgress>();
    app.init_resource::<crate::states::quiz::QuizProgress>();
    app.add_plugins((
        MenuPlugin,
        CompanionSelectPlugin,
        PlayingPlugin,
        // UI plugins (ApiConsole, TriageConsole, Quiz) are NOT added here.
        // They have conflicting &mut Text queries that cause Bevy B0001
        // when run in parallel. We init their resources manually so tests
        // can query UI state, but the systems don't run in the harness.
        // Tests needing UI interaction should add the specific plugin.
        OutagePlugin,
        ResultsPlugin,
        SeraphinePlugin,
        LedgerUiPlugin,
    ));
    // Instant transitions for logic tests: the real fade lives in
    // AnimPlugin (excluded here); this drains the request queue straight
    // into NextState so `settle()` still converges in a few frames.
    app.add_systems(Update, crate::anim::apply_transition_requests_instantly);
    app
}

/// Test app with ApiConsolePlugin for tests that press API buttons.
/// The ApiConsole system doesn't conflict with the base harness.
fn playthrough_app_with_api() -> App {
    let mut app = playthrough_app();
    app.add_plugins(crate::states::api_console::ApiConsolePlugin);
    app
}

/// Current top-level game state.
fn game_state(app: &App) -> GameState {
    *app.world().resource::<State<GameState>>().get()
}

/// Run enough frames for pending transitions and their enter/exit
/// schedules to complete.
fn settle(app: &mut App) {
    for _ in 0..SETTLE_FRAMES {
        app.update();
    }
}

/// Press the button entity carrying marker `B` through its real handler:
/// set `Interaction::Pressed`, run frames so the `Changed<Interaction>`
/// system fires, then settle. Panics if there is not exactly one such
/// button — a missing or duplicated button is a broken screen.
fn press_button<B: Component>(app: &mut App) {
    let target = {
        let world = app.world_mut();
        let mut buttons = world.query_filtered::<Entity, With<B>>();
        buttons.single(world).unwrap()
    };
    press_entity(app, target);
}

/// Press an entity's button through its real `Interaction` handler.
fn press_entity(app: &mut App, target: Entity) {
    app.world_mut()
        .entity_mut(target)
        .insert(Interaction::Pressed);
    settle(app);
}

/// Press the companion card for `companion` on the picker screen.
fn press_companion_card(app: &mut App, companion: Companion) {
    let target = {
        let world = app.world_mut();
        let mut cards = world.query::<(Entity, &SelectButton)>();
        cards
            .iter(world)
            .find(|(_, button)| button.0 == companion)
            .map(|(entity, _)| entity)
            .expect("companion card is spawned for every companion")
    };
    press_entity(app, target);
}

/// Press the results-screen button for `action`.
fn press_result_action(app: &mut App, action: ResultAction) {
    let target = {
        let world = app.world_mut();
        let mut buttons = world.query::<(Entity, &ResultAction)>();
        buttons
            .iter(world)
            .find(|(_, button)| **button == action)
            .map(|(entity, _)| entity)
            .expect("results screen spawns a button per action")
    };
    press_entity(app, target);
}

/// Tap a world position through the real pointer pipeline: set
/// `PointerWorld`, press and release the left mouse button around
/// `board::handle_pointer_input` — the same path real cursor input takes.
fn tap_world_pos(app: &mut App, world_pos: Vec2) {
    app.world_mut().resource_mut::<PointerWorld>().0 = Some(world_pos);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_system_once(board::handle_pointer_input);
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.clear();
    buttons.release(MouseButton::Left);
    app.world_mut().run_system_once(board::handle_pointer_input);
    settle(app);
}

/// Tap the pill for choice `slot` (within its edge group) of edge
/// (`from`, `to`) through the real pointer pipeline. The pill position
/// comes from `board::pill_world_pos` — the layout function the renderer
/// uses — so the test taps where the player taps.
fn tap_pill(app: &mut App, from: u32, to: u32, slot: usize) {
    let world_pos = {
        let level = app.world().resource::<LevelDef>();
        let slot_count = board::grouped_choices(level)
            .iter()
            .find(|(edge, _)| *edge == (from, to))
            .map(|(_, slots)| slots.len())
            .expect("tapped edge must have a choice group");
        board::pill_world_pos(level, from, to, slot, slot_count)
            .expect("tapped slot must be within its group")
    };
    tap_world_pos(app, world_pos);
}

/// Drag from node `from` to node `to` through the real pointer pipeline:
/// press on the source node's world position (begins the drag), release on
/// the target node's (connects the route). Single-choice pairs are
/// drag-only by design — no pill is drawn for them — so this is the real
/// input path for levels like w4l1.
fn drag_route(app: &mut App, from: u32, to: u32) {
    let (from_pos, to_pos) = {
        let level = app.world().resource::<LevelDef>();
        (
            board::node_world_pos(level, from).expect("drag source node exists"),
            board::node_world_pos(level, to).expect("drag target node exists"),
        )
    };
    app.world_mut().resource_mut::<PointerWorld>().0 = Some(from_pos);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().run_system_once(board::handle_pointer_input);
    app.world_mut().resource_mut::<PointerWorld>().0 = Some(to_pos);
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.clear();
    buttons.release(MouseButton::Left);
    app.world_mut().run_system_once(board::handle_pointer_input);
    settle(app);
    assert_eq!(
        app.world().resource::<PlacedChoices>().0.get(&(from, to)),
        Some(&0),
        "the drag must lay the route's default component"
    );
}

/// Drive a fresh app from boot to the first level of `companion`'s track
/// through the real UI: menu → Start → companion picker → card.
fn start_track(app: &mut App, companion: Companion) {
    settle(app);
    assert_eq!(
        game_state(app),
        GameState::MainMenu,
        "a fresh app boots to the main menu"
    );
    press_button::<StartButton>(app);
    assert_eq!(
        game_state(app),
        GameState::CompanionSelect,
        "Start opens the companion picker"
    );
    press_companion_card(app, companion);
    assert_eq!(
        game_state(app),
        GameState::Playing,
        "picking a companion starts their track"
    );
    assert_eq!(
        app.world().resource::<SelectedCompanion>().0,
        companion,
        "the picked companion is the selected one"
    );
    assert_eq!(
        Some(app.world().resource::<CurrentLevelIndex>().0),
        companion.track_start_index(),
        "the track starts at the companion's first level"
    );
}

/// Assert the app reached the results screen with the expected outcome.
/// Returns the reported level index.
fn expect_results(app: &App, won: bool) -> usize {
    assert_eq!(
        game_state(app),
        GameState::Results,
        "expected the results screen"
    );
    assert_eq!(
        app.world().resource::<LevelOutcome>().won,
        won,
        "results outcome mismatch"
    );
    app.world().resource::<CurrentLevelIndex>().0
}

/// Assert the fired outage was resolved — i.e. the win came through
/// `check_outage_resolution`, the only win path outage levels have
/// (`check_win_condition` skips them while in `Playing`).
fn expect_outage_resolved(app: &App) {
    assert!(
        app.world()
            .resource::<ActiveOutage>()
            .outage
            .as_ref()
            .is_some_and(|o| o.resolved),
        "the win must resolve the fired outage"
    );
}

/// Advance the level clock to `seconds`, letting scripted outages fire
/// through the real `check_scripted_outage` system. Deterministic: no
/// wall-clock wait.
fn advance_clock(app: &mut App, seconds: f64) {
    app.world_mut().resource_mut::<LevelClock>().elapsed_seconds = seconds;
    settle(app);
}

/// Assert an outage is currently active.
fn expect_outage_active(app: &App) {
    assert_eq!(
        game_state(app),
        GameState::OutageActive,
        "expected the outage screen"
    );
    assert!(
        app.world().resource::<ActiveOutage>().outage.is_some(),
        "an outage must be registered while OutageActive"
    );
}

/// Tick the active outage forward by `dt_seconds` through the real
/// `Outage::tick`, then settle so resolution/expiry is evaluated.
fn tick_outage(app: &mut App, dt_seconds: f64) {
    {
        let mut active_outage = app.world_mut().resource_mut::<ActiveOutage>();
        let active = active_outage
            .outage
            .as_mut()
            .expect("no active outage to tick");
        active.tick(dt_seconds);
    }
    settle(app);
}

// ---------------------------------------------------------------------------
// Winning playthroughs: all four companions, both levels of each track.
// ---------------------------------------------------------------------------

/// Séraphine (fiber): w1l1 splice tutorial, then f1l3 where the player
/// picks a clean connector (the dirty one adds 2 dB and fails).
#[test]
fn fiber_track_wins_both_levels_through_real_input() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Fiber);

    // w1l1: splice node 1 -> 2 shut. Either splice wins the tutorial.
    tap_pill(&mut app, 1, 2, 0);
    assert_eq!(expect_results(&app, true), 0);

    // f1l3 (index 1 in the 50-level layout): clean UPC connector wins;
    // slot 0 is the clean UPC, slot 1 is dirty, slot 2 is clean APC.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(game_state(&app), GameState::Playing);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 1);
    tap_pill(&mut app, 1, 2, 0);
    expect_results(&app, true);

    // The track loop closes: back to the picker for another medium.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(game_state(&app), GameState::Playing);
}

/// Ondine (coax): c1l1 unity-gain balance, then c1l2's ingress storm.
///
/// DESIGN NOTE: the 3 dB amp is still in window when the storm fires, so
/// the level resolves as a win immediately with no mid-storm hot-swap.
/// This test pins the shipped behavior; the intended repair path is
/// covered by `coax_storm_hot_swap_repair_wins_through_real_input`, and
/// the bypass is reported as a first-class finding.
#[test]
fn coax_track_wins_both_levels_through_real_input() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Coax);

    // c1l1: the 5 dB amp lands the 35 dBmV plant in [0, 15] dBmV.
    tap_pill(&mut app, 1, 2, 0);
    assert_eq!(expect_results(&app, true), 10);

    // c1l2: the 3 dB amp starts in window; the storm fires at 15 s and the
    // pre-placed answer wins on the first resolution check.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 11);
    tap_pill(&mut app, 1, 2, 0);
    advance_clock(&mut app, 20.0);
    expect_results(&app, true);
    expect_outage_resolved(&app);
}

/// Linka (wireless): m1l1 link closure, then m1l2's interference storm.
/// Same design note as the coax track: the pre-storm repeater wins at
/// storm-fire with no hot-swap.
#[test]
fn wireless_track_wins_both_levels_through_real_input() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Mobile);

    // m1l1: the 20 dBm repeater closes the 400 m hop into [-75, -40] dBm.
    tap_pill(&mut app, 1, 2, 0);
    assert_eq!(expect_results(&app, true), 20); // wireless1 in 50-level layout

    // m1l2: the 20 dBm repeater starts in window; the storm fires at 15 s
    // and the pre-placed answer wins on the first resolution check.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 21);
    tap_pill(&mut app, 1, 2, 0);
    advance_clock(&mut app, 20.0);
    expect_results(&app, true);
    expect_outage_resolved(&app);
}

/// Lattice (ethernet): e1l1's hundred-meter wall, then e1l2's PoE budget.
#[test]
fn ethernet_track_wins_both_levels_through_real_input() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Ethernet);

    // e1l1: only the switch beats the 100 m wall — 130 m of copper can't.
    tap_pill(&mut app, 1, 2, 0);
    assert_eq!(expect_results(&app, true), 30); // ethernet1 in 50-level layout

    // e1l2: the 60 W switch feeds the 25 W AP with headroom to spare.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 31); // ethernet2 in 50-level layout
    tap_pill(&mut app, 1, 2, 0);
    expect_results(&app, true);

    press_result_action(&mut app, ResultAction::ReturnToMenu);
    assert_eq!(game_state(&app), GameState::MainMenu);
}

// ---------------------------------------------------------------------------
// Mid-storm repairs: the intended hot-swap gameplay, driven by placing
// nothing before the storm and repairing once the floor has climbed.
// ---------------------------------------------------------------------------

/// c1l2 repair path: no pre-placement, storm fires at 15 s, ride the
/// climbing noise floor for 90 s, then hot-swap to the 12 dB amp.
#[test]
fn coax_storm_hot_swap_repair_wins_through_real_input() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Coax);
    tap_pill(&mut app, 1, 2, 0); // c1l1 win
    expect_results(&app, true);
    press_result_action(&mut app, ResultAction::ContinueNextLevel);

    advance_clock(&mut app, 20.0);
    expect_outage_active(&app);
    // IngressNoise timer is 110 s: at 90 s the storm still rages and the
    // floor has climbed ~9 dB, sinking the (unplaced) 3 dB answer.
    tick_outage(&mut app, 90.0);
    assert_eq!(
        game_state(&app),
        GameState::OutageActive,
        "the storm must still be active at 90 s"
    );
    tap_pill(&mut app, 1, 2, 1); // 12 dB amp: back in window
    expect_results(&app, true);
}

/// m1l2 repair path: no pre-placement, storm fires at 15 s, ride the
/// interference for 80 s, then hot-swap to the 30 dBm repeater.
#[test]
fn wireless_storm_hot_swap_repair_wins_through_real_input() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Mobile);
    tap_pill(&mut app, 1, 2, 0); // m1l1 win
    expect_results(&app, true);
    press_result_action(&mut app, ResultAction::ContinueNextLevel);

    advance_clock(&mut app, 20.0);
    expect_outage_active(&app);
    // WirelessInterference timer is 100 s: at 80 s the 20 dBm answer has
    // sunk out of the window and the storm still rages.
    tick_outage(&mut app, 80.0);
    assert_eq!(
        game_state(&app),
        GameState::OutageActive,
        "the storm must still be active at 80 s"
    );
    tap_pill(&mut app, 1, 2, 1); // 30 dBm repeater: back in window
    expect_results(&app, true);
}

// ---------------------------------------------------------------------------
// Losing lines: one per medium, through the real fail paths.
// ---------------------------------------------------------------------------

/// Fiber loss: light ONLY the aerial span in w4l1. The storm severs it at
/// 20 s, there is no protection path, and the outage times out.
#[test]
fn fiber_unprotected_span_loses_the_storm() {
    let mut app = playthrough_app();
    // Jump directly to the outage level (index 7 in 50-level layout).
    // The test verifies outage mechanics, not track progression.
    settle(&mut app);
    app.world_mut().resource_mut::<SelectedCompanion>().0 = Companion::Fiber;
    app.world_mut().resource_mut::<CurrentLevelIndex>().0 = 7;
    app.world_mut()
        .resource_mut::<crate::anim::TransitionRequest>()
        .0 = Some(GameState::Playing);
    settle(&mut app);
    assert_eq!(game_state(&mut app), GameState::Playing);

    // Light ONLY the aerial span (1 -> 3) via the drag gesture — no backup
    // route. The storm severs it at 20 s and the outage must time out.
    drag_route(&mut app, 1, 3);
    advance_clock(&mut app, 25.0);
    expect_outage_active(&app);
    tick_outage(&mut app, 80.0); // AerialDamage timer is 75 s
    expect_results(&app, false);
}

/// Coax loss: the 21 dB amp overshoots the window even before the storm;
/// nothing placed during the outage can land in window, so it times out.
#[test]
fn coax_overdriven_amp_loses_the_storm() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Coax);
    tap_pill(&mut app, 1, 2, 0); // c1l1 win
    expect_results(&app, true);
    press_result_action(&mut app, ResultAction::ContinueNextLevel);

    tap_pill(&mut app, 1, 2, 2); // 21 dB amp: over-gain, out of window
    advance_clock(&mut app, 20.0);
    expect_outage_active(&app);
    tick_outage(&mut app, 120.0); // IngressNoise timer is 110 s
    expect_results(&app, false);
}

/// Wireless loss (no outage level): the 30 dB amplifier can't fix a
/// geometry problem — gain is not the missing piece, a repeater is. The
/// level must stay open, and the board must stay solvable afterwards.
#[test]
fn wireless_distractor_pill_never_wins() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Mobile);

    tap_pill(&mut app, 1, 2, 2); // 30 dB amp: the distractor
    settle(&mut app);
    assert_eq!(
        game_state(&app),
        GameState::Playing,
        "a wrong pill must not end the level"
    );
    // Tapping the right pill replaces it on the same edge — and wins.
    tap_pill(&mut app, 1, 2, 0);
    expect_results(&app, true);
}

/// Ethernet loss (no outage level): 130 m of Cat6 breaks the 100 m
/// segment wall. The level must stay open and recoverable.
#[test]
fn ethernet_long_copper_run_never_wins() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Ethernet);

    tap_pill(&mut app, 1, 2, 2); // Cat6 65 m: 130 m total, over the wall
    settle(&mut app);
    assert_eq!(
        game_state(&app),
        GameState::Playing,
        "a wrong pill must not end the level"
    );
    tap_pill(&mut app, 1, 2, 0); // the switch replaces it — and wins
    expect_results(&app, true);
}

// ---------------------------------------------------------------------------
// Adversarial: malformed input and navigation edges must not break the app.
// ---------------------------------------------------------------------------

/// Tapping empty board places nothing, ends nothing, breaks nothing.
#[test]
fn tapping_empty_board_places_nothing() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Fiber);

    tap_world_pos(&mut app, EMPTY_BOARD_SPOT);
    assert!(
        app.world().resource::<PlacedChoices>().0.is_empty(),
        "tapping empty board must not place anything"
    );
    assert_eq!(game_state(&app), GameState::Playing);
}

/// Retry replays the failed level with a clean board, not the next one.
/// (The Retry button only spawns on a loss — wins offer Continue — so this
/// drives a real timeout loss first.)
#[test]
fn retry_button_replays_the_failed_level() {
    let mut app = playthrough_app();
    start_track(&mut app, Companion::Coax);
    tap_pill(&mut app, 1, 2, 0); // c1l1 win
    expect_results(&app, true);
    press_result_action(&mut app, ResultAction::ContinueNextLevel);

    // c1l2: overdrive the amp and ride out the storm to a timeout loss.
    tap_pill(&mut app, 1, 2, 2);
    advance_clock(&mut app, 20.0);
    expect_outage_active(&app);
    tick_outage(&mut app, 120.0);
    expect_results(&app, false);

    press_result_action(&mut app, ResultAction::RetrySameLevel);
    assert_eq!(game_state(&app), GameState::Playing);
    assert_eq!(
        app.world().resource::<CurrentLevelIndex>().0,
        11,
        "retry must replay c1l2, not advance"
    );
    assert!(
        app.world().resource::<PlacedChoices>().0.is_empty(),
        "retry must reset placements"
    );
}

/// Backing out of the picker returns to the menu without starting a track.
#[test]
fn back_button_returns_to_menu_without_starting_a_track() {
    let mut app = playthrough_app();
    settle(&mut app);
    press_button::<StartButton>(&mut app);
    assert_eq!(game_state(&app), GameState::CompanionSelect);

    press_button::<BackButton>(&mut app);
    assert_eq!(game_state(&app), GameState::MainMenu);
    // No track started: no level loaded, no selections made.
    assert!(
        app.world().get_resource::<LevelDef>().is_none(),
        "backing out must not load a level"
    );
}

/// Start Clara's track at `index` without going through the picker
/// (Clara has no picker card yet — see `Companion::ALL`). Mirrors
/// `handle_select_buttons`: select companion, set level, request Playing.
fn start_clara_level(app: &mut App, index: usize) {
    settle(app);
    assert_eq!(
        game_state(app),
        GameState::MainMenu,
        "a fresh app boots to the main menu"
    );
    app.world_mut().resource_mut::<SelectedCompanion>().0 = Companion::Clara;
    app.world_mut().resource_mut::<CurrentLevelIndex>().0 = index;
    app.world_mut()
        .resource_mut::<crate::anim::TransitionRequest>()
        .0 = Some(GameState::Playing);
    settle(app);
    assert_eq!(game_state(app), GameState::Playing);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, index);
}

/// Press an API console button for `op` through its real `Interaction`
/// handler.
fn press_api_op(app: &mut App, op: crate::level::ApiOp) {
    let target = {
        let world = app.world_mut();
        let mut buttons = world.query::<(Entity, &crate::states::api_console::ApiButton)>();
        buttons
            .iter(world)
            .find(|(_, button)| button.0 == op)
            .map(|(entity, _)| entity)
            .expect("API console spawns a button per choice")
    };
    press_entity(app, target);
}

/// Clara (provisioning): all nine levels through real input.
///
/// * clara1 (8): 9 subs, 1:16 wins on ports (1:4/1:8 too few).
/// * clara2 (9): 6 subs + WaterIntrusion outage, 1:8 wins.
/// * clara3 (10): dead-box swap, 7 subs, 1:8 wins (1:4 too few ports).
/// * clara4 (11): profile audit — hidden XGS at 2 km overloads on 1:8, so 1:16 wins.
/// * clara5 (12): pure NBI API sequence.
/// * clara6 (13): hybrid — 1:8 splitter + NBI bulk sequence.
/// * clara7 (14): pure SMx REST lifecycle.
/// * clara8 (15): SmartMDU 12 subs, 1:16 wins on ports.
/// * clara9 (16): hot OLT — close XGS overloads on 1:16, so 1:32 wins.
/// * clara10 (17): night cutover — 14 subs + outage + API on a hot OLT.
///   Expert capstone: 1:32, the NBI bulk sequence, survive the intrusion.
#[test]
fn clara_track_wins_all_ten_levels_through_real_input() {
    use crate::level::ApiOp;
    let mut app = playthrough_app_with_api();

    // clara1: 9 subscribers — only the 1:16 (slot 2 of 3) has the ports.
    start_clara_level(&mut app, 40);
    tap_pill(&mut app, 0, 1, 2);
    assert_eq!(expect_results(&app, true), 40);

    // clara2: storm level — 1:8 (slot 1 of 2), then survive the intrusion.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 41);
    tap_pill(&mut app, 0, 1, 1);
    advance_clock(&mut app, 30.0);
    expect_results(&app, true);
    expect_outage_resolved(&app);

    // clara3: dead-box swap — 1:8 (slot 1 of [1:4, 1:8]).
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 42);
    tap_pill(&mut app, 0, 1, 1);
    assert_eq!(expect_results(&app, true), 42);

    // clara4: profile audit — 1:16 (slot 1 of [1:8, 1:16]).
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 43);
    tap_pill(&mut app, 0, 1, 1);
    assert_eq!(expect_results(&app, true), 43);

    // clara5: pure NBI — drive the console sequence.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 44);
    for op in [
        ApiOp::Login,
        ApiOp::ShowOnt,
        ApiOp::CreateService,
        ApiOp::VerifyService,
    ] {
        press_api_op(&mut app, op);
    }
    assert_eq!(expect_results(&app, true), 44);

    // clara6: hybrid — 1:8 splitter, then the NBI bulk sequence.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 45);
    tap_pill(&mut app, 0, 1, 1);
    for op in [
        ApiOp::Login,
        ApiOp::ShowOnt,
        ApiOp::CreateService,
        ApiOp::VerifyService,
        ApiOp::Logout,
    ] {
        press_api_op(&mut app, op);
    }
    assert_eq!(expect_results(&app, true), 45);

    // clara7: pure SMx — the REST lifecycle.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 46);
    for op in [
        ApiOp::Login,
        ApiOp::CreateSubscriber,
        ApiOp::CreateOnt,
        ApiOp::CreateService,
        ApiOp::VerifyService,
    ] {
        press_api_op(&mut app, op);
    }
    assert_eq!(expect_results(&app, true), 46);

    // clara8: SmartMDU 12 subs — 1:16 (slot 1 of [1:8, 1:16]).
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 47);
    tap_pill(&mut app, 0, 1, 1);
    assert_eq!(expect_results(&app, true), 47);

    // clara9: hot OLT — 1:32 (slot 1 of [1:16, 1:32]).
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 48);
    tap_pill(&mut app, 0, 1, 1);
    assert_eq!(expect_results(&app, true), 48);

    // clara10: night cutover — 1:32 (slot 1 of [1:16, 1:32]), then the
    // NBI bulk sequence, then survive the WaterIntrusion at 30 s.
    press_result_action(&mut app, ResultAction::ContinueNextLevel);
    assert_eq!(app.world().resource::<CurrentLevelIndex>().0, 49);
    tap_pill(&mut app, 0, 1, 1);
    for op in [
        ApiOp::Login,
        ApiOp::ShowOnt,
        ApiOp::CreateService,
        ApiOp::VerifyService,
        ApiOp::Logout,
    ] {
        press_api_op(&mut app, op);
    }
    advance_clock(&mut app, 35.0);
    expect_results(&app, true);
    expect_outage_resolved(&app);

    // The track loop closes: back to the picker for another companion.
    press_result_action(&mut app, ResultAction::ReturnToSelect);
    assert_eq!(game_state(&app), GameState::CompanionSelect);
}

/// A wrong API pick raises an alarm and does not advance the sequence:
/// the player must still produce the exact expected order to win.
#[test]
fn clara_api_wrong_pick_raises_alarm_without_advancing() {
    use crate::level::ApiOp;
    use crate::states::api_console::ApiProgress;
    let mut app = playthrough_app_with_api();
    // clara5 is index 44.
    start_clara_level(&mut app, 44);

    // Wrong first pick: RebootOnt instead of Login.
    press_api_op(&mut app, ApiOp::RebootOnt);
    let progress = app.world().resource::<ApiProgress>();
    assert_eq!(progress.alarms_raised, 1);
    assert!(progress.placed.is_empty());

    // Now the correct sequence still wins.
    for op in [
        ApiOp::Login,
        ApiOp::ShowOnt,
        ApiOp::CreateService,
        ApiOp::VerifyService,
    ] {
        press_api_op(&mut app, op);
    }
    assert_eq!(expect_results(&app, true), 44);
}
