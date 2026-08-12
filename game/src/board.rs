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
/// (not the raw index into `available_components`).
fn hit_test_pill(level: &LevelDef, pos: Vec2) -> Option<(u32, u32, usize)> {
    let mut best: Option<(u32, u32, usize, f32)> = None;
    for ((from, to), indices) in grouped_choices(level) {
        let count = indices.len();
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
/// alternative among several offered choices.
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

    let Some(pos) = pointer.0 else {
        if just_released {
            drag.from = None;
        }
        return;
    };

    if just_pressed {
        if let Some(node_id) = hit_test_node(&level, pos) {
            drag.from = Some(node_id);
        } else if let Some((from, to, slot)) = hit_test_pill(&level, pos) {
            placed.0.insert((from, to), slot);
            rebuild_live_graph(&level, &placed, &mut live.graph);
        }
    }

    if just_released {
        if let Some(src) = drag.from {
            if let Some(dst) = hit_test_node(&level, pos) {
                let has_choice = level
                    .available_components
                    .iter()
                    .any(|c| c.from == src && c.to == dst);
                if dst != src && has_choice {
                    placed.0.entry((src, dst)).or_insert(0);
                    rebuild_live_graph(&level, &placed, &mut live.graph);
                }
            }
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
