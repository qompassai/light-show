//! Interactive puzzle board: renders the current level's node graph with
//! Bevy gizmos, tracks mouse/touch pointer position, and turns drags
//! (node → node) and taps (on a component "pill") into edges placed into
//! the live `osp_sim::PathGraph`.
//!
//! Two gestures are supported:
//! - **Drag from node to node**: connects the default (first-listed)
//!   component choice for that (from, to) pair. Works even when there's
//!   only one choice — the common case — so it's the primary gesture.
//! - **Tap a pill**: explicitly picks one alternative among several
//!   choices offered for the same pair (e.g. fusion vs. mechanical
//!   splice), for players who want to compare options before connecting.
//!
//! Colors follow the palette in `docs/ART_STYLE.md` (board schematic
//! blues/greys, warm amber "light" accent, hot pink Séraphine accent).

use crate::level::LevelDef;
use crate::states::playing::LiveGraph;
use crate::test_log;
use crate::ui::LedgerText;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use osp_sim::PathGraph;
use std::collections::HashMap;

/// Visual radius of a drawn node circle.
const NODE_RADIUS: f32 = 26.0;
/// Touch/click hit-test radius for a node — larger than the visual radius
/// so the target stays finger-friendly on phone screens.
const NODE_HIT_RADIUS: f32 = 50.0;
/// Visual + hit-test radius of a component-choice "pill".
const PILL_RADIUS: f32 = 34.0;
/// Perpendicular spacing between multiple pills offered on the same edge.
const PILL_SPREAD: f32 = 70.0;

/// Board schematic palette (see `docs/ART_STYLE.md`).
const BOARD_LINE: Color = Color::srgb(0.11, 0.145, 0.255); // #1c2541
const BOARD_ACCENT: Color = Color::srgb(0.357, 0.753, 0.922); // #5bc0eb
const LIGHT_WARM: Color = Color::srgb(1.0, 0.82, 0.4); // #ffd166
const LIGHT_HOT: Color = Color::srgb(1.0, 0.435, 0.682); // #ff6fae

/// Maps a level's abstract `grid_x`/`grid_y` node coordinates onto world
/// space. `grid_x = 1` sits at world `x = 0` and each grid column is 200
/// world units wide; `grid_y = 0` sits at world `y = 300` (clear of the
/// ledger overlay at the top of the screen) descending 200 units per row
/// (clear of Séraphine's sprite anchored near `y = -400` at the bottom).
pub fn grid_to_world(grid_x: f32, grid_y: f32) -> Vec2 {
    Vec2::new((grid_x - 1.0) * 200.0, 300.0 - grid_y * 200.0)
}

fn node_world_pos(level: &LevelDef, id: u32) -> Option<Vec2> {
    level
        .nodes
        .iter()
        .find(|n| n.id == id)
        .map(|n| grid_to_world(n.grid_x, n.grid_y))
}

/// Which component-choice index (into `LevelDef::available_components`)
/// the player has picked for each open `(from, to)` node pair.
#[derive(Resource, Default)]
pub struct PlacedChoices(pub HashMap<(u32, u32), usize>);

/// Tracks an in-progress node → node drag gesture.
#[derive(Resource, Default)]
pub struct DragState {
    pub from: Option<u32>,
}

/// The pointer's current position in world space, updated every frame from
/// either the mouse cursor or the first active touch.
#[derive(Resource, Default)]
pub struct PointerWorld(pub Option<Vec2>);

/// Marks the root entity of the spawned board (node labels) so it can be
/// torn down cleanly on `OnExit(Playing)`.
#[derive(Component)]
pub struct BoardRoot;

/// Marks the root UI entity that hosts the ledger readout text.
#[derive(Component)]
pub struct LedgerRoot;

/// Groups `available_components` entries by their `(from, to)` pair,
/// preserving first-encounter order, and records each entry's original
/// index so it can be resolved back against `available_components` later.
fn grouped_choices(level: &LevelDef) -> Vec<((u32, u32), Vec<usize>)> {
    let mut order: Vec<(u32, u32)> = Vec::new();
    let mut groups: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (idx, choice) in level.available_components.iter().enumerate() {
        let key = (choice.from, choice.to);
        if !groups.contains_key(&key) {
            order.push(key);
        }
        groups.entry(key).or_default().push(idx);
    }
    order
        .into_iter()
        .map(|key| {
            let indices = groups.remove(&key).unwrap_or_default();
            (key, indices)
        })
        .collect()
}

/// World position of the Nth pill offered for a given `(from, to)` edge —
/// spaced perpendicular to the edge so multiple choices get distinct tap
/// targets instead of overlapping.
fn pill_world_pos(
    level: &LevelDef,
    from: u32,
    to: u32,
    slot_index: usize,
    slot_count: usize,
) -> Option<Vec2> {
    let a = node_world_pos(level, from)?;
    let b = node_world_pos(level, to)?;
    let mid = (a + b) * 0.5;
    let dir = (b - a).normalize_or_zero();
    let perp = Vec2::new(-dir.y, dir.x);
    // Center the row of pills on the midpoint: offsets run
    // -(n-1)/2 .. (n-1)/2 in units of PILL_SPREAD.
    let offset = (slot_index as f32) - ((slot_count as f32 - 1.0) / 2.0);
    Some(mid + perp * offset * PILL_SPREAD)
}

fn hit_test_node(level: &LevelDef, pos: Vec2) -> Option<u32> {
    level
        .nodes
        .iter()
        .filter_map(|n| {
            let world = grid_to_world(n.grid_x, n.grid_y);
            let dist = world.distance(pos);
            (dist <= NODE_HIT_RADIUS).then_some((n.id, dist))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// Returns `(from, to, slot_index)` for the pill under `pos`, if any,
/// where `slot_index` is the position within that pair's choice group
/// (not the raw index into `available_components`). Edges with only one
/// choice have no pill drawn (the drag gesture alone places them, see
/// `draw_board_gizmos`), so they're skipped here too — otherwise a tap
/// near the edge midpoint would silently register against an invisible
/// target.
fn hit_test_pill(level: &LevelDef, pos: Vec2) -> Option<(u32, u32, usize)> {
    let mut best: Option<(u32, u32, usize, f32)> = None;
    for ((from, to), indices) in grouped_choices(level) {
        let count = indices.len();
        if count <= 1 {
            continue;
        }
        for slot in 0..count {
            let Some(world) = pill_world_pos(level, from, to, slot, count) else {
                continue;
            };
            let dist = world.distance(pos);
            if dist <= PILL_RADIUS && best.as_ref().map(|b| dist < b.3).unwrap_or(true) {
                best = Some((from, to, slot, dist));
            }
        }
    }
    best.map(|(from, to, slot, _)| (from, to, slot))
}

/// What a press (mouse-down / touch-down) resolves to, given the level
/// layout and the pointer's world position. Pure decision logic, kept
/// separate from `handle_pointer_input`'s Bevy resource plumbing so it can
/// be unit-tested directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressAction {
    /// Pointer landed on a node — begin a node → node drag from it.
    StartDrag(u32),
    /// Pointer landed on a component-choice pill — select that variant
    /// immediately (no drag needed).
    SelectPill { from: u32, to: u32, slot: usize },
    /// Pointer landed on empty board space.
    None,
}

fn resolve_press(level: &LevelDef, pos: Vec2) -> PressAction {
    if let Some(node_id) = hit_test_node(level, pos) {
        PressAction::StartDrag(node_id)
    } else if let Some((from, to, slot)) = hit_test_pill(level, pos) {
        PressAction::SelectPill { from, to, slot }
    } else {
        PressAction::None
    }
}

/// What a release (mouse-up / touch-up) resolves to, given an in-progress
/// drag and the release position. Pure decision logic, unit-tested
/// directly (see `resolve_press` for why this is split out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseAction {
    /// Drag ended on a different node that has an available component for
    /// this pair — connect it (using the caller's default/first choice).
    Connect { from: u32, to: u32 },
    /// No valid connection: nothing was dragging, the release landed on
    /// empty space, on the same node the drag started from, or on a node
    /// pair with no component offered between them.
    None,
}

fn resolve_release(level: &LevelDef, drag_from: Option<u32>, pos: Option<Vec2>) -> ReleaseAction {
    let (Some(src), Some(pos)) = (drag_from, pos) else {
        return ReleaseAction::None;
    };
    let Some(dst) = hit_test_node(level, pos) else {
        return ReleaseAction::None;
    };
    if dst == src {
        return ReleaseAction::None;
    }
    let has_choice = level
        .available_components
        .iter()
        .any(|c| c.from == src && c.to == dst);
    if has_choice {
        ReleaseAction::Connect { from: src, to: dst }
    } else {
        ReleaseAction::None
    }
}

/// Rebuilds a fresh `PathGraph` from the level's nodes and fixed edges,
/// then connects whichever component choice the player has placed on each
/// open `(from, to)` pair. Plain function (not a system) so it can be
/// called both from the `OnEnter(Playing)` setup and from the pointer
/// input system whenever a placement changes.
pub fn rebuild_live_graph(level: &LevelDef, placed: &PlacedChoices, graph: &mut PathGraph) {
    let mut fresh = PathGraph::default();
    for node in &level.nodes {
        fresh.add_node(node.id, node.label.clone());
    }
    for edge in &level.fixed_edges {
        fresh.connect(edge.from, edge.to, edge.component.clone());
    }
    for ((from, to), slot) in &placed.0 {
        let component = level
            .available_components
            .iter()
            .filter(|c| c.from == *from && c.to == *to)
            .nth(*slot)
            .map(|c| c.component.clone());
        if let Some(component) = component {
            fresh.connect(*from, *to, component);
        }
    }
    *graph = fresh;
}

/// Spawns the board's node-label entities and the ledger overlay text.
/// Plain function (not a system) so it can be called synchronously from
/// the `OnEnter(Playing)` setup system, right after loading the level.
pub fn spawn_board_from_level(
    commands: &mut Commands,
    level: &LevelDef,
    asset_server: &AssetServer,
) {
    let font: Handle<Font> = asset_server.load("fonts/pixel.ttf");

    commands
        .spawn((BoardRoot, SpatialBundle::default()))
        .with_children(|parent| {
            for node in &level.nodes {
                let pos = grid_to_world(node.grid_x, node.grid_y);
                parent.spawn(Text2dBundle {
                    text: Text::from_section(
                        node.label.clone(),
                        TextStyle {
                            font: font.clone(),
                            font_size: 14.0,
                            color: BOARD_ACCENT,
                        },
                    ),
                    transform: Transform::from_translation(
                        (pos + Vec2::new(0.0, NODE_RADIUS + 18.0)).extend(5.0),
                    ),
                    text_anchor: bevy::sprite::Anchor::Center,
                    ..default()
                });
            }
        });

    commands
        .spawn((
            LedgerRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: Color::srgba(0.051, 0.051, 0.118, 0.85).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                LedgerText,
                TextBundle::from_section(
                    "Loss: -- dB",
                    TextStyle {
                        font,
                        font_size: 16.0,
                        color: LIGHT_WARM,
                    },
                ),
            ));
        });
}

type BoardOrLedgerRoot = Or<(With<BoardRoot>, With<LedgerRoot>)>;

pub fn teardown_board(mut commands: Commands, query: Query<Entity, BoardOrLedgerRoot>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

/// Updates `PointerWorld` every frame from the mouse cursor (desktop) or
/// the first active touch (Android), converting viewport coordinates to
/// world space via the active 2D camera.
pub fn track_pointer(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    touches: Res<Touches>,
    mut pointer: ResMut<PointerWorld>,
) {
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        pointer.0 = None;
        return;
    };

    let viewport_pos = windows
        .get_single()
        .ok()
        .and_then(|w| w.cursor_position())
        .or_else(|| touches.iter().next().map(|t| t.position()));

    pointer.0 = viewport_pos.and_then(|p| camera.viewport_to_world_2d(camera_transform, p));
}

/// Turns pointer gestures into placements: drag node → node connects the
/// default choice for that pair; tapping a pill explicitly picks one
/// alternative among several offered choices. All decision logic lives in
/// `resolve_press`/`resolve_release` (see their unit tests below); this
/// system is just the thin ECS-resource glue around them.
pub fn handle_pointer_input(
    level: Res<LevelDef>,
    pointer: Res<PointerWorld>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut drag: ResMut<DragState>,
    mut placed: ResMut<PlacedChoices>,
    mut live: ResMut<LiveGraph>,
) {
    let just_pressed = mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed();
    let just_released = mouse.just_released(MouseButton::Left) || touches.any_just_released();

    if just_pressed {
        if let Some(pos) = pointer.0 {
            match resolve_press(&level, pos) {
                PressAction::StartDrag(node_id) => drag.from = Some(node_id),
                PressAction::SelectPill { from, to, slot } => {
                    placed.0.insert((from, to), slot);
                    rebuild_live_graph(&level, &placed, &mut live.graph);
                    test_log!("select from={} to={} slot={}", from, to, slot);
                }
                PressAction::None => {}
            }
        }
    }

    if just_released {
        if let ReleaseAction::Connect { from, to } = resolve_release(&level, drag.from, pointer.0) {
            placed.0.entry((from, to)).or_insert(0);
            rebuild_live_graph(&level, &placed, &mut live.graph);
            test_log!("connect from={} to={}", from, to);
        }
        drag.from = None;
    }
}

fn draw_dashed_line(gizmos: &mut Gizmos, from: Vec2, to: Vec2, color: Color) {
    const SEGMENTS: i32 = 12;
    for i in 0..SEGMENTS {
        if i % 2 != 0 {
            continue;
        }
        let t0 = i as f32 / SEGMENTS as f32;
        let t1 = (i + 1) as f32 / SEGMENTS as f32;
        gizmos.line_2d(from.lerp(to, t0), from.lerp(to, t1), color);
    }
}

/// Draws the board every frame: node circles, fixed edges (solid), open
/// choice edges (dashed, bright once placed), component pills, and an
/// active drag-preview line following the pointer.
pub fn draw_board_gizmos(
    mut gizmos: Gizmos,
    level: Res<LevelDef>,
    placed: Res<PlacedChoices>,
    drag: Res<DragState>,
    pointer: Res<PointerWorld>,
) {
    for edge in &level.fixed_edges {
        if let (Some(a), Some(b)) = (
            node_world_pos(&level, edge.from),
            node_world_pos(&level, edge.to),
        ) {
            gizmos.line_2d(a, b, LIGHT_WARM);
        }
    }

    for (from, to) in grouped_choices(&level).into_iter().map(|(k, _)| k) {
        if let (Some(a), Some(b)) = (node_world_pos(&level, from), node_world_pos(&level, to)) {
            let color = if placed.0.contains_key(&(from, to)) {
                LIGHT_WARM
            } else {
                BOARD_LINE
            };
            draw_dashed_line(&mut gizmos, a, b, color);
        }
    }

    for node in &level.nodes {
        let pos = grid_to_world(node.grid_x, node.grid_y);
        let is_endpoint = node.id == level.source_node || node.id == level.target_node;
        let color = if is_endpoint {
            LIGHT_WARM
        } else {
            BOARD_ACCENT
        };
        gizmos.circle_2d(pos, NODE_RADIUS, color);
    }

    for ((from, to), indices) in grouped_choices(&level) {
        let count = indices.len();
        if count <= 1 {
            // A single unambiguous choice needs no pill — the drag
            // gesture alone places it.
            continue;
        }
        let selected_slot = placed.0.get(&(from, to)).copied();
        for slot in 0..count {
            if let Some(pos) = pill_world_pos(&level, from, to, slot, count) {
                let color = if selected_slot == Some(slot) {
                    LIGHT_HOT
                } else {
                    BOARD_ACCENT
                };
                gizmos.circle_2d(pos, PILL_RADIUS, color);
            }
        }
    }

    if let (Some(from_id), Some(pointer_pos)) = (drag.from, pointer.0) {
        if let Some(from_pos) = node_world_pos(&level, from_id) {
            gizmos.line_2d(from_pos, pointer_pos, LIGHT_HOT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{ComponentChoice, LevelEdge, LevelNode, WavelengthDef};
    use crate::states::playing::WavelengthWrapper;
    use osp_sim::component::PlantType;
    use osp_sim::{Component, SpliceType};

    /// Builds a minimal but fully-valid `LevelDef` from just the parts a
    /// given test cares about, filling in flavor/metadata fields with
    /// placeholder values.
    fn fixture(
        nodes: Vec<LevelNode>,
        fixed_edges: Vec<LevelEdge>,
        available_components: Vec<ComponentChoice>,
        source_node: u32,
        target_node: u32,
    ) -> LevelDef {
        LevelDef {
            id: "test".into(),
            title: "Test Level".into(),
            world: 0,
            briefing: String::new(),
            tx_dbm: 3.0,
            wavelength: WavelengthDef::Nm1490,
            window_min_dbm: -27.0,
            window_max_dbm: -8.0,
            nodes,
            fixed_edges,
            available_components,
            source_node,
            target_node,
            scripted_outage: None,
            on_enter_line: None,
            on_win_line: None,
            on_fail_line: None,
        }
    }

    fn node(id: u32, grid_x: f32, grid_y: f32) -> LevelNode {
        LevelNode {
            id,
            label: format!("Node {id}"),
            grid_x,
            grid_y,
        }
    }

    fn fusion_splice() -> Component {
        Component::Splice {
            kind: SpliceType::Fusion,
            degradation_db: 0.0,
        }
    }

    fn mechanical_splice() -> Component {
        Component::Splice {
            kind: SpliceType::Mechanical,
            degradation_db: 0.0,
        }
    }

    /// Mirrors `assets/levels/world1_level1.json`'s shape: two node ids
    /// (1, 2) on the same row, with a fusion-vs-mechanical splice choice
    /// between them — used to exercise pill selection.
    fn two_choice_level() -> LevelDef {
        fixture(
            vec![node(1, 1.0, 0.0), node(2, 2.0, 0.0)],
            vec![],
            vec![
                ComponentChoice {
                    from: 1,
                    to: 2,
                    component: fusion_splice(),
                },
                ComponentChoice {
                    from: 1,
                    to: 2,
                    component: mechanical_splice(),
                },
            ],
            1,
            2,
        )
    }

    /// Same layout, but with only one component offered between (1, 2) —
    /// used to confirm no invisible pill target exists for single choices.
    fn single_choice_level() -> LevelDef {
        fixture(
            vec![node(1, 1.0, 0.0), node(2, 2.0, 0.0)],
            vec![],
            vec![ComponentChoice {
                from: 1,
                to: 2,
                component: fusion_splice(),
            }],
            1,
            2,
        )
    }

    // -- grid_to_world / grouped_choices -----------------------------------

    #[test]
    fn grid_to_world_matches_expected_layout() {
        assert_eq!(grid_to_world(1.0, 0.0), Vec2::new(0.0, 300.0));
        assert_eq!(grid_to_world(2.0, 0.0), Vec2::new(200.0, 300.0));
        assert_eq!(grid_to_world(1.0, 1.0), Vec2::new(0.0, 100.0));
    }

    #[test]
    fn grouped_choices_preserves_encounter_order_and_indices() {
        let level = fixture(
            vec![node(1, 1.0, 0.0), node(2, 2.0, 0.0), node(3, 3.0, 0.0)],
            vec![],
            vec![
                ComponentChoice {
                    from: 1,
                    to: 2,
                    component: fusion_splice(),
                },
                ComponentChoice {
                    from: 2,
                    to: 3,
                    component: fusion_splice(),
                },
                ComponentChoice {
                    from: 1,
                    to: 2,
                    component: mechanical_splice(),
                },
            ],
            1,
            3,
        );
        let groups = grouped_choices(&level);
        assert_eq!(groups, vec![((1, 2), vec![0, 2]), ((2, 3), vec![1])]);
    }

    // -- hit_test_node ------------------------------------------------------

    #[test]
    fn hit_test_node_finds_node_within_radius() {
        let level = two_choice_level();
        // Node 1 sits at world (0, 300); comfortably within NODE_HIT_RADIUS.
        assert_eq!(hit_test_node(&level, Vec2::new(5.0, 302.0)), Some(1));
    }

    #[test]
    fn hit_test_node_returns_none_when_out_of_range() {
        let level = two_choice_level();
        assert_eq!(hit_test_node(&level, Vec2::new(5000.0, 5000.0)), None);
    }

    #[test]
    fn hit_test_node_picks_nearest_when_two_are_in_range() {
        // Two nodes close enough together that both fall within
        // NODE_HIT_RADIUS of a point between them; the nearer one wins.
        let level = fixture(
            vec![node(1, 1.0, 0.0), node(2, 1.05, 0.0)],
            vec![],
            vec![],
            1,
            2,
        );
        // Node 1 world pos (0, 300); node 2 world pos (10, 300).
        assert_eq!(hit_test_node(&level, Vec2::new(3.0, 300.0)), Some(1));
        assert_eq!(hit_test_node(&level, Vec2::new(7.0, 300.0)), Some(2));
    }

    // -- hit_test_pill --------------------------------------------------------

    #[test]
    fn hit_test_pill_finds_each_pill_in_a_multi_choice_group() {
        let level = two_choice_level();
        // Pill 0 (Fusion) at (100, 265), pill 1 (Mechanical) at (100, 335)
        // per pill_world_pos's midpoint + perpendicular-spread formula.
        assert_eq!(
            hit_test_pill(&level, Vec2::new(100.0, 265.0)),
            Some((1, 2, 0))
        );
        assert_eq!(
            hit_test_pill(&level, Vec2::new(100.0, 335.0)),
            Some((1, 2, 1))
        );
    }

    #[test]
    fn hit_test_pill_returns_none_for_single_choice_edge() {
        let level = single_choice_level();
        // No pill is drawn when there's only one choice, so a tap at the
        // edge midpoint must not silently register against an invisible
        // target.
        assert_eq!(hit_test_pill(&level, Vec2::new(100.0, 300.0)), None);
    }

    #[test]
    fn hit_test_pill_returns_none_far_from_any_pill() {
        let level = two_choice_level();
        assert_eq!(hit_test_pill(&level, Vec2::new(-500.0, -500.0)), None);
    }

    // -- resolve_press / resolve_release (the actual gesture logic) --------

    #[test]
    fn resolve_press_starts_drag_when_pressing_a_node() {
        let level = two_choice_level();
        assert_eq!(
            resolve_press(&level, Vec2::new(0.0, 300.0)),
            PressAction::StartDrag(1)
        );
    }

    #[test]
    fn resolve_press_selects_pill_when_pressing_a_pill() {
        let level = two_choice_level();
        assert_eq!(
            resolve_press(&level, Vec2::new(100.0, 335.0)),
            PressAction::SelectPill {
                from: 1,
                to: 2,
                slot: 1
            }
        );
    }

    #[test]
    fn resolve_press_none_on_empty_board_space() {
        let level = two_choice_level();
        assert_eq!(
            resolve_press(&level, Vec2::new(-900.0, -900.0)),
            PressAction::None
        );
    }

    #[test]
    fn resolve_release_connects_a_valid_drag() {
        let level = two_choice_level();
        let action = resolve_release(&level, Some(1), Some(Vec2::new(200.0, 300.0)));
        assert_eq!(action, ReleaseAction::Connect { from: 1, to: 2 });
    }

    #[test]
    fn resolve_release_rejects_dropping_on_the_same_node() {
        let level = two_choice_level();
        let action = resolve_release(&level, Some(1), Some(Vec2::new(0.0, 300.0)));
        assert_eq!(action, ReleaseAction::None);
    }

    #[test]
    fn resolve_release_rejects_pair_with_no_available_component() {
        let level = two_choice_level();
        // Reversed direction: available_components only defines 1 -> 2, not
        // 2 -> 1, so dragging node 2 onto node 1 must not connect.
        let action = resolve_release(&level, Some(2), Some(Vec2::new(0.0, 300.0)));
        assert_eq!(action, ReleaseAction::None);
    }

    #[test]
    fn resolve_release_none_when_nothing_was_dragging() {
        let level = two_choice_level();
        let action = resolve_release(&level, None, Some(Vec2::new(200.0, 300.0)));
        assert_eq!(action, ReleaseAction::None);
    }

    #[test]
    fn resolve_release_none_when_dropped_on_empty_space() {
        let level = two_choice_level();
        let action = resolve_release(&level, Some(1), Some(Vec2::new(-900.0, -900.0)));
        assert_eq!(action, ReleaseAction::None);
    }

    // -- rebuild_live_graph --------------------------------------------------

    #[test]
    fn rebuild_live_graph_includes_fixed_and_placed_edges() {
        let level = fixture(
            vec![node(0, 0.0, 0.0), node(1, 1.0, 0.0), node(2, 2.0, 0.0)],
            vec![LevelEdge {
                from: 0,
                to: 1,
                component: Component::Span {
                    length_km: 8.0,
                    plant: PlantType::Buried,
                },
            }],
            vec![
                ComponentChoice {
                    from: 1,
                    to: 2,
                    component: fusion_splice(),
                },
                ComponentChoice {
                    from: 1,
                    to: 2,
                    component: mechanical_splice(),
                },
            ],
            0,
            2,
        );
        let mut placed = PlacedChoices::default();
        placed.0.insert((1, 2), 1); // pick the mechanical alternative
        let mut graph = PathGraph::default();
        rebuild_live_graph(&level, &placed, &mut graph);

        assert_eq!(graph.edges.len(), 2);
        assert!(graph
            .edges
            .iter()
            .any(|e| e.from == 0 && e.to == 1 && matches!(e.component, Component::Span { .. })));
        assert!(graph
            .edges
            .iter()
            .any(|e| e.from == 1 && e.to == 2 && e.component == mechanical_splice()));
    }

    #[test]
    fn rebuild_live_graph_ignores_an_out_of_range_slot() {
        // Defensive regression test: if `PlacedChoices` ever ends up with a
        // slot index beyond what the level offers for that pair, rebuild
        // must skip it instead of panicking or connecting the wrong thing.
        let level = two_choice_level();
        let mut placed = PlacedChoices::default();
        placed.0.insert((1, 2), 7);
        let mut graph = PathGraph::default();
        rebuild_live_graph(&level, &placed, &mut graph);
        assert!(graph.edges.is_empty());
    }

    // -- end-to-end interaction scenario -------------------------------------

    #[test]
    fn drag_then_pill_tap_updates_the_live_graph_and_link_budget() {
        let level = two_choice_level();
        let mut placed = PlacedChoices::default();
        let mut live = LiveGraph {
            graph: PathGraph::default(),
            wavelength: WavelengthWrapper(level.wavelength.into()),
            tx_dbm: level.tx_dbm,
        };
        rebuild_live_graph(&level, &placed, &mut live.graph);

        // 1. Drag from node 1 to node 2 -- places the default (first,
        //    fusion) choice, exactly like `handle_pointer_input` would on
        //    a real press-then-release.
        let press = resolve_press(&level, Vec2::new(0.0, 300.0));
        assert_eq!(press, PressAction::StartDrag(1));
        let drag_from = match press {
            PressAction::StartDrag(id) => Some(id),
            _ => None,
        };
        let release = resolve_release(&level, drag_from, Some(Vec2::new(200.0, 300.0)));
        assert_eq!(release, ReleaseAction::Connect { from: 1, to: 2 });
        if let ReleaseAction::Connect { from, to } = release {
            placed.0.entry((from, to)).or_insert(0);
            rebuild_live_graph(&level, &placed, &mut live.graph);
        }

        let result_fusion = live
            .graph
            .compute_link_budget(1, 2, live.tx_dbm, live.wavelength.0, level.receive_window())
            .expect("fusion splice alone forms a valid path");
        assert!((result_fusion.total_loss_db - SpliceType::Fusion.typical_loss_db()).abs() < 1e-9);

        // 2. Tap the mechanical pill directly -- overrides the drag's
        //    default choice with the explicitly selected alternative.
        let press = resolve_press(&level, Vec2::new(100.0, 335.0));
        assert_eq!(
            press,
            PressAction::SelectPill {
                from: 1,
                to: 2,
                slot: 1
            }
        );
        if let PressAction::SelectPill { from, to, slot } = press {
            placed.0.insert((from, to), slot);
            rebuild_live_graph(&level, &placed, &mut live.graph);
        }

        let result_mechanical = live
            .graph
            .compute_link_budget(1, 2, live.tx_dbm, live.wavelength.0, level.receive_window())
            .expect("mechanical splice alone forms a valid path");
        assert!(
            (result_mechanical.total_loss_db - SpliceType::Mechanical.typical_loss_db()).abs()
                < 1e-9
        );
        assert!(result_mechanical.total_loss_db > result_fusion.total_loss_db);
    }

    // -- handle_pointer_input as an actual Bevy system -----------------------
    //
    // The tests above exercise `resolve_press`/`resolve_release` directly.
    // These run the real `handle_pointer_input` system through a `World`
    // via `run_system_once`, covering the ECS resource-extraction glue
    // (`Res<ButtonInput<MouseButton>>`, `ResMut<DragState>`, etc.) itself,
    // not just the pure decision logic it delegates to.

    fn test_world(level: LevelDef) -> World {
        let mut world = World::new();
        world.insert_resource(ButtonInput::<MouseButton>::default());
        world.insert_resource(Touches::default());
        world.insert_resource(PointerWorld::default());
        world.insert_resource(DragState::default());
        world.insert_resource(PlacedChoices::default());
        world.insert_resource(LiveGraph {
            graph: PathGraph::default(),
            wavelength: WavelengthWrapper(level.wavelength.into()),
            tx_dbm: level.tx_dbm,
        });
        world.insert_resource(level);
        world
    }

    #[test]
    fn system_drag_from_node_to_node_connects_default_choice() {
        use bevy_ecs::system::RunSystemOnce;

        let mut world = test_world(two_choice_level());

        // Frame 1: press over node 1 (world (0, 300)) -- starts a drag.
        world.resource_mut::<PointerWorld>().0 = Some(Vec2::new(0.0, 300.0));
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        world.run_system_once(handle_pointer_input);
        assert_eq!(world.resource::<DragState>().from, Some(1));

        // Frame 2: move over node 2 (world (200, 300)) and release --
        // connects the default (first-listed, fusion) choice.
        world.resource_mut::<ButtonInput<MouseButton>>().clear();
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        world.resource_mut::<PointerWorld>().0 = Some(Vec2::new(200.0, 300.0));
        world.run_system_once(handle_pointer_input);

        assert_eq!(world.resource::<DragState>().from, None);
        assert_eq!(world.resource::<PlacedChoices>().0.get(&(1, 2)), Some(&0));
        let live = world.resource::<LiveGraph>();
        assert_eq!(live.graph.edges.len(), 1);
        assert_eq!(live.graph.edges[0].component, fusion_splice());
    }

    #[test]
    fn system_tapping_a_pill_selects_it_without_starting_a_drag() {
        use bevy_ecs::system::RunSystemOnce;

        let mut world = test_world(two_choice_level());

        // Press directly on the Mechanical pill (slot 1) -- no drag
        // involved, selection should apply immediately on press.
        world.resource_mut::<PointerWorld>().0 = Some(Vec2::new(100.0, 335.0));
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        world.run_system_once(handle_pointer_input);

        assert_eq!(world.resource::<DragState>().from, None);
        assert_eq!(world.resource::<PlacedChoices>().0.get(&(1, 2)), Some(&1));
        let live = world.resource::<LiveGraph>();
        assert_eq!(live.graph.edges.len(), 1);
        assert_eq!(live.graph.edges[0].component, mechanical_splice());
    }

    #[test]
    fn system_press_and_release_on_empty_space_places_nothing() {
        use bevy_ecs::system::RunSystemOnce;

        let mut world = test_world(two_choice_level());

        world.resource_mut::<PointerWorld>().0 = Some(Vec2::new(-900.0, -900.0));
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        world.run_system_once(handle_pointer_input);

        world.resource_mut::<ButtonInput<MouseButton>>().clear();
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        world.run_system_once(handle_pointer_input);

        assert_eq!(world.resource::<DragState>().from, None);
        assert!(world.resource::<PlacedChoices>().0.is_empty());
        assert!(world.resource::<LiveGraph>().graph.edges.is_empty());
    }
}
