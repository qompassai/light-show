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
//! blues/greys, warm amber "light" accent, hot pink Séraphine accent),
//! restyled to the keeper title artwork's neon-circuit look: fibers are
//! drawn with a layered glow pass (`neon_line_2d`) and the node/pill
//! circles are hand-crafted Aseprite ring sprites under
//! `assets/sprites/ui/` (same visual radii as the old gizmo circles, so
//! the hit-test contract is unchanged).

use crate::fonts::FONT_SIZE_ADJUST;
use crate::level::{LevelDef, MediumDef};
use crate::shaders::{PulseMaterial, PulseSettings};
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::test_log;
use crate::ui::LedgerText;
use bevy::input::touch::Touches;
use bevy::math::curve::{Curve, EaseFunction};
use bevy::mesh::Mesh2d;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::sprite_render::MeshMaterial2d;
use bevy::window::PrimaryWindow;
use osp_sim::{Component, Outage, PathGraph};
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
const HAZARD_RED: Color = Color::srgb(1.0, 0.302, 0.302); // #ff4d4d

/// Canvas edge length (px) of the Aseprite-crafted ring sprites. One
/// world unit maps to one sprite pixel, so each drawn ring band sits
/// exactly on `NODE_RADIUS` / `PILL_RADIUS`; the 14 px margin around it
/// holds the glow halo and the diagonal circuit ticks.
const NODE_RING_SIZE: f32 = 80.0;
const PILL_RING_SIZE: f32 = 96.0;

/// Which Aseprite node-ring flavor a node gets — gold for the level's
/// source/target endpoints, the tap-leg ring for nodes whose label names
/// tap plant, the broadcast-arcs ring for non-endpoint nodes on wireless
/// levels, cyan for everything else. Pure, so tests can pin the mapping
/// without spawning anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeRingFlavor {
    Endpoint,
    Tap,
    Site,
    Standard,
}

fn node_ring_flavor(level: &LevelDef, node_id: u32, label: &str) -> NodeRingFlavor {
    if node_id == level.source_node || node_id == level.target_node {
        NodeRingFlavor::Endpoint
    } else if label.contains("Tap") {
        NodeRingFlavor::Tap
    } else if level.medium == MediumDef::Wireless {
        NodeRingFlavor::Site
    } else {
        NodeRingFlavor::Standard
    }
}

/// Asset path of the Aseprite-crafted node ring sprite for a flavor (see
/// `node_ring_flavor`).
fn node_ring_path(flavor: NodeRingFlavor) -> &'static str {
    match flavor {
        NodeRingFlavor::Endpoint => "sprites/ui/node_ring_gold.png",
        NodeRingFlavor::Tap => "sprites/ui/node_ring_tap.png",
        NodeRingFlavor::Site => "sprites/ui/node_ring_site.png",
        NodeRingFlavor::Standard => "sprites/ui/node_ring_cyan.png",
    }
}

/// Asset path of the Aseprite-crafted pill ring sprite — hot pink once
/// the player has picked that slot, cyan while it is only an offer.
fn pill_ring_path(selected: bool) -> &'static str {
    if selected {
        "sprites/ui/pill_ring_selected.png"
    } else {
        "sprites/ui/pill_ring_normal.png"
    }
}

/// Marks a node-ring sprite (see `node_ring_path`).
#[derive(Component)]
struct NodeRing;

/// Marks a pill-ring sprite so `update_pill_rings` can swap its texture
/// when the player's pick changes. `slot` is the position within the
/// pair's choice group (same indexing as `pill_world_pos`).
#[derive(Component)]
pub(crate) struct PillRing {
    pub(crate) from: u32,
    pub(crate) to: u32,
    pub(crate) slot: usize,
}

/// Maps a level's abstract `grid_x`/`grid_y` node coordinates onto world
/// space. `grid_x = 1` sits at world `x = 0` and each grid column is 200
/// world units wide; `grid_y = 0` sits at world `y = 300` (clear of the
/// ledger overlay at the top of the screen) descending 200 units per row
/// (clear of Séraphine's sprite anchored near `y = -400` at the bottom).
pub fn grid_to_world(grid_x: f32, grid_y: f32) -> Vec2 {
    Vec2::new((grid_x - 1.0) * 200.0, 300.0 - grid_y * 200.0)
}

pub(crate) fn node_world_pos(level: &LevelDef, id: u32) -> Option<Vec2> {
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

/// Marks the root UI entity that hosts the level-intro briefing text
/// (world/title heading and the level's instructional briefing).
#[derive(Component)]
pub struct BriefingRoot;

/// Widest measure the briefing text column may take, in px. Narrower
/// windows wrap at the window width instead (the panel is full-width).
const BRIEFING_TEXT_MAX_W: f32 = 880.0;

/// Resolves which concrete `Component` a `(from, to, slot)` placement
/// refers to — the same lookup `rebuild_live_graph` does per placed edge,
/// factored out so `handle_pointer_input` can inspect what was just
/// placed (e.g. to fire a `waifu::SpliceReaction`) without duplicating it.
fn resolve_placed_component(
    level: &LevelDef,
    from: u32,
    to: u32,
    slot: usize,
) -> Option<Component> {
    level
        .available_components
        .iter()
        .filter(|c| c.from == from && c.to == to)
        .nth(slot)
        .map(|c| c.component.clone())
}

/// Maps a just-placed component to the companion's reaction, if any —
/// only splices carry a quality judgement in this game (fusion = clean,
/// mechanical = messy); every other component type is reaction-neutral.
fn splice_reaction_for(component: &Component) -> Option<crate::waifu::SpliceReaction> {
    match component {
        Component::Splice { kind, .. } => Some(crate::waifu::SpliceReaction(match kind {
            osp_sim::SpliceType::Fusion => crate::waifu::Mood::Blush,
            osp_sim::SpliceType::Mechanical => crate::waifu::Mood::Pout,
        })),
        _ => None,
    }
}

/// Which `game/assets/sprites/components/*.png` icon represents a given
/// component, per the iconography table in `docs/ART_STYLE.md`. `Span`
/// has no dedicated icon (it's fixed background plant, already drawn as
/// a solid line by `draw_board_gizmos`, never offered as a pill choice).
fn component_icon_path(component: &Component) -> Option<&'static str> {
    match component {
        Component::Splice {
            kind: osp_sim::SpliceType::Fusion,
            ..
        } => Some("sprites/components/fusion_splice.png"),
        Component::Splice {
            kind: osp_sim::SpliceType::Mechanical,
            ..
        } => Some("sprites/components/mechanical_splice.png"),
        Component::Connector {
            kind: osp_sim::ConnectorType::Upc,
            ..
        } => Some("sprites/components/upc_connector.png"),
        Component::Connector {
            kind: osp_sim::ConnectorType::Apc,
            ..
        } => Some("sprites/components/apc_connector.png"),
        Component::Splitter { .. } => Some("sprites/components/splitter.png"),
        Component::Macrobend { .. } => Some("sprites/components/macrobend.png"),
        Component::Span { .. } => None,
        Component::Amplifier { .. } => Some("sprites/components/amplifier.png"),
        Component::Tap { .. } => Some("sprites/components/tap.png"),
        Component::CoaxSpan { .. } => Some("sprites/components/coax_span.png"),
        Component::WirelessHop { .. } => Some("sprites/components/wireless_hop.png"),
        Component::Repeater { .. } => Some("sprites/components/repeater.png"),
        Component::EthernetRun { .. } => Some("sprites/components/ethernet_run.png"),
        Component::Switch { .. } => Some("sprites/components/switch.png"),
    }
}

/// Three-letter pill label for a component, drawn as `Text2d` when it has
/// no icon sprite (see `component_icon_path`) — today only `Span`, which
/// is never offered as a pill choice, so this is a safety net rather
/// than a render path.
fn component_short_label(component: &Component) -> &'static str {
    match component {
        Component::Splice {
            kind: osp_sim::SpliceType::Fusion,
            ..
        } => "FUS",
        Component::Splice {
            kind: osp_sim::SpliceType::Mechanical,
            ..
        } => "MEC",
        Component::Connector {
            kind: osp_sim::ConnectorType::Upc,
            ..
        } => "UPC",
        Component::Connector {
            kind: osp_sim::ConnectorType::Apc,
            ..
        } => "APC",
        Component::Splitter { .. } => "SPL",
        Component::Macrobend { .. } => "BEND",
        Component::Span { .. } => "SPAN",
        Component::Amplifier { .. } => "AMP",
        Component::Tap { .. } => "TAP",
        Component::CoaxSpan { .. } => "COAX",
        Component::WirelessHop { .. } => "AIR",
        Component::Repeater { .. } => "RPT",
        Component::EthernetRun { .. } => "CAT",
        Component::Switch { .. } => "SW",
    }
}

/// Compact number formatting for pill captions: whole values print
/// without a decimal point ("15"), fractional ones keep one decimal
/// ("2.5") — captions are read at a glance, mid-puzzle.
fn fmt_compact(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// Value caption drawn under a pill's icon: the spec that distinguishes
/// this choice from its siblings on the same edge. Sibling pills share
/// one icon (the three Unity Gain amps differ only in `gain_db`), so
/// the icon alone cannot carry the choice. `None` where the icon
/// already is the distinction (splice/connector kinds) or where the
/// remaining fields are hidden state rather than a player-facing spec
/// (splice degradation, connector contamination).
fn component_value_label(component: &Component) -> Option<String> {
    match component {
        Component::Amplifier { gain_db } => Some(format!("+{} dB", fmt_compact(*gain_db))),
        Component::Tap { tap_loss_db } => Some(format!("-{} dB", fmt_compact(*tap_loss_db))),
        Component::Splitter { ratio } => Some(format!("1x{}", ratio.branch_count())),
        Component::Span { length_km, .. } => Some(format!("{} km", fmt_compact(*length_km))),
        Component::CoaxSpan { length_m } => Some(format!("{} m", fmt_compact(*length_m))),
        Component::WirelessHop { distance_m, .. } => {
            Some(format!("{} m", fmt_compact(*distance_m)))
        }
        Component::Repeater { tx_dbm } => Some(format!("{} dBm", fmt_compact(*tx_dbm))),
        Component::EthernetRun {
            length_m,
            category,
        } => Some(format!(
            "{} m {}",
            fmt_compact(*length_m),
            match category {
                osp_sim::component::CableCategory::Cat5e => "Cat5e",
                osp_sim::component::CableCategory::Cat6 => "Cat6",
            }
        )),
        Component::Switch { poe_budget_w } => Some(format!("{} W", fmt_compact(*poe_budget_w))),
        Component::Macrobend { excess_loss_db } => {
            Some(format!("+{} dB", fmt_compact(*excess_loss_db)))
        }
        Component::Splice { .. } | Component::Connector { .. } => None,
    }
}

/// Marks a pill's icon sprite so it can be found/despawned alongside the
/// rest of the board (it's spawned as a `BoardRoot` child, so normal
/// `despawn_recursive` teardown already covers it — this marker exists
/// for test/inspector legibility, not cleanup).
#[derive(Component)]
struct ComponentIcon;

/// Groups `available_components` entries by their `(from, to)` pair,
/// preserving first-encounter order, and records each entry's original
/// index so it can be resolved back against `available_components` later.
pub(crate) fn grouped_choices(level: &LevelDef) -> Vec<((u32, u32), Vec<usize>)> {
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
pub(crate) fn pill_world_pos(
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

/// Whether `(from, to)` should be treated as physically disconnected
/// because of `outage` — true only for an *unresolved, full-cut* hazard
/// on that exact edge (see `osp_sim::OutageKind::is_full_cut`).
/// Degrade-type hazards (water intrusion, connector contamination,
/// macrobend) keep the edge connected here; their extra loss is instead
/// applied on top of the budget by
/// `osp_sim::PathGraph::compute_link_budget_with_outage`.
fn is_edge_severed(outage: Option<&Outage>, from: u32, to: u32) -> bool {
    outage.is_some_and(|o| {
        !o.resolved && o.kind.is_full_cut() && o.edge_from == from && o.edge_to == to
    })
}

/// Rebuilds a fresh `PathGraph` from the level's nodes and fixed edges,
/// then connects whichever component choice the player has placed on each
/// open `(from, to)` pair — skipping any edge currently severed by a
/// full-cut `outage` (see `is_edge_severed`), which is exactly what forces
/// `compute_link_budget`/`compute_link_budget_with_outage` to report
/// `Disconnected` until the player reroutes around it. Plain function
/// (not a system) so it can be called both from the `OnEnter(Playing)`
/// setup and from the pointer input system whenever a placement changes.
pub fn rebuild_live_graph(
    level: &LevelDef,
    placed: &PlacedChoices,
    outage: Option<&Outage>,
    graph: &mut PathGraph,
) {
    let mut fresh = PathGraph::default();
    for node in &level.nodes {
        fresh.add_node(node.id, node.label.clone());
    }
    for edge in &level.fixed_edges {
        if is_edge_severed(outage, edge.from, edge.to) {
            continue;
        }
        fresh.connect(edge.from, edge.to, edge.component.clone());
    }
    for ((from, to), slot) in &placed.0 {
        if is_edge_severed(outage, *from, *to) {
            continue;
        }
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

/// Font size of node labels (world units == px at the default camera zoom).
const LABEL_FONT_SIZE: f32 = 14.0;
/// Vertical gap between a node ring and a "near" label plate.
const LABEL_NEAR_DY: f32 = NODE_RADIUS + 18.0;
/// Vertical gap used when the near slot is blocked by pills or another
/// label — one full pill spread further out.
const LABEL_FAR_DY: f32 = LABEL_NEAR_DY + PILL_SPREAD;
/// Horizontal gap between a node ring and a side label plate.
const LABEL_SIDE_GAP: f32 = 10.0;
/// Obstacle radius treated as solid when placing labels: the ring band
/// sits at `NODE_RADIUS` / `PILL_RADIUS` with a soft glow halo around it,
/// so a plate may touch the halo but must not cover the band.
const LABEL_NODE_OBSTACLE_R: f32 = NODE_RING_SIZE / 2.0;
const LABEL_PILL_OBSTACLE_R: f32 = PILL_RADIUS + 6.0;
/// Minimum clearance kept between a label plate and the screen edge.
const LABEL_EDGE_MARGIN: f32 = 8.0;
/// Padding added around already-placed plates when testing new ones.
const LABEL_PLATE_PAD: f32 = 4.0;

/// Dark nameplate size behind a node label. Width is estimated from the
/// fixed Monaspace Neon advance (0.6x font size, monospace); the 24 px
/// padding absorbs the estimate. Keep in sync with the label `TextStyle`
/// in `spawn_board_from_level` — a wider font needs a larger advance here.
fn label_plate_size(label: &str) -> Vec2 {
    Vec2::new(
        label.len() as f32 * LABEL_FONT_SIZE * 0.6 + 24.0,
        LABEL_FONT_SIZE + 20.0,
    )
}

/// One node's label placement request: world position, text, and which
/// side the row stagger prefers (alternates above/below within a grid row
/// so same-row neighbors don't start on top of each other).
struct LabelRequest<'a> {
    pos: Vec2,
    label: &'a str,
    prefer_above: bool,
}

/// Chosen plate center and size for one node label, in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LabelPlacement {
    center: Vec2,
    plate: Vec2,
}

fn rect_hits_circle(center: Vec2, half: Vec2, circle: Vec2, radius: f32) -> bool {
    let closest = Vec2::new(
        circle.x.clamp(center.x - half.x, center.x + half.x),
        circle.y.clamp(center.y - half.y, center.y + half.y),
    );
    closest.distance_squared(circle) < radius * radius
}

fn rects_overlap(a_center: Vec2, a_half: Vec2, b_center: Vec2, b_half: Vec2) -> bool {
    (a_center.x - b_center.x).abs() < a_half.x + b_half.x
        && (a_center.y - b_center.y).abs() < a_half.y + b_half.y
}

/// Picks a non-overlapping plate position for every node label. Pure so
/// the w1l1/c1l1 layouts can be pinned by unit tests: for each node in
/// order, the stagger-preferred near side is tried first, then the other
/// near side, then the far sides, then the flanks; the first candidate
/// that doesn't cover another node's ring, a pill ring, or an
/// already-placed plate wins (each candidate is clamped horizontally
/// first, so the winner also fixes the c1l1 "Customer Drop" label
/// clipping past the right screen edge on phones).
fn layout_node_labels(
    requests: &[LabelRequest],
    pill_centers: &[Vec2],
    half_w: f32,
    view_center_x: f32,
) -> Vec<LabelPlacement> {
    let mut placed: Vec<LabelPlacement> = Vec::with_capacity(requests.len());
    for (i, req) in requests.iter().enumerate() {
        let plate = label_plate_size(req.label);
        let half = plate * 0.5;
        let side_dx = half.x + NODE_RADIUS + LABEL_SIDE_GAP;
        // Preference order keeps the historical stagger look; the far
        // slots only trigger when pills crowd the near ones (w1l1's
        // splice pair sits between two pill rings).
        let mut candidates = if req.prefer_above {
            vec![
                Vec2::new(0.0, LABEL_NEAR_DY),
                Vec2::new(0.0, -LABEL_NEAR_DY),
                Vec2::new(0.0, LABEL_FAR_DY),
                Vec2::new(0.0, -LABEL_FAR_DY),
            ]
        } else {
            vec![
                Vec2::new(0.0, -LABEL_NEAR_DY),
                Vec2::new(0.0, LABEL_NEAR_DY),
                Vec2::new(0.0, -LABEL_FAR_DY),
                Vec2::new(0.0, LABEL_FAR_DY),
            ]
        };
        candidates.push(Vec2::new(-side_dx, 0.0));
        candidates.push(Vec2::new(side_dx, 0.0));

        let mut best_center = req.pos;
        let mut best_score = usize::MAX;
        for offset in &candidates {
            // Clamp before scoring, into the visible band around the
            // camera's center (the fit framing can center the view away
            // from x = 0 on wide levels): a plate wider than the
            // visible area centers on the view instead of clipping
            // one edge.
            let max_dx = (half_w - LABEL_EDGE_MARGIN - half.x).max(0.0);
            let raw = req.pos + *offset;
            let center = Vec2::new(
                raw.x.clamp(view_center_x - max_dx, view_center_x + max_dx),
                raw.y,
            );
            let mut score = 0;
            // Other nodes' rings are solid; the node's own ring is
            // excluded — every candidate clears its visible band by
            // construction (near dy and side dx both exceed NODE_RADIUS).
            for (j, other) in requests.iter().enumerate() {
                if j != i && rect_hits_circle(center, half, other.pos, LABEL_NODE_OBSTACLE_R) {
                    score += 1;
                }
            }
            for pill in pill_centers {
                if rect_hits_circle(center, half, *pill, LABEL_PILL_OBSTACLE_R) {
                    score += 1;
                }
            }
            for prev in &placed {
                if rects_overlap(
                    center,
                    half + LABEL_PLATE_PAD,
                    prev.center,
                    prev.plate * 0.5 + LABEL_PLATE_PAD,
                ) {
                    score += 1;
                }
            }
            if score < best_score {
                best_score = score;
                best_center = center;
                if score == 0 {
                    break;
                }
            }
        }
        placed.push(LabelPlacement {
            center: best_center,
            plate,
        });
    }
    placed
}
/// Spawns the board's node-label entities and the ledger overlay text.
/// Plain function (not a system) so it can be called synchronously from
/// the `OnEnter(Playing)` setup system, right after loading the level.
/// `half_w` is half the visible width in world units (window width x
/// the fit framing's camera scale) and `view_center_x` is the world x
/// the camera centers on — together they keep label plates on screen.
///
/// The briefing panel carries the level's instructional text ONLY.
/// Companion flavor lines (the level's `on_enter_line` key into the
/// dialogue bank) used to be appended to it in quotes; the first
/// playtest showed a random quip landing inside the instructions
/// ("Ooh, a punch-down with zero crosstalk?…") — chatter belongs to
/// the dialogue system, not the briefing, so it is no longer folded in.
pub fn spawn_board_from_level(
    commands: &mut Commands,
    level: &LevelDef,
    asset_server: &AssetServer,
    half_w: f32,
    view_center_x: f32,
) {
    let label_font: Handle<Font> = asset_server.load(crate::fonts::DISPLAY);
    let body_font: Handle<Font> = asset_server.load(crate::fonts::BODY);

    let mut briefing_text = format!(
        "World {} — {}\n{}",
        level.world, level.title, level.briefing
    );
    // The first level carries the how-to: the playtest showed the two
    // placement gestures are not discoverable on their own, so they are
    // taught here in the briefing and reinforced live by the ledger's
    // empty-state line (see `LevelDef::partial_ledger`).
    if level.id == "w1l1" {
        briefing_text.push_str(
            "\nHow to play: tap a pill beside a link to place that exact component — the caption under each pill shows its value. Or drag from one glowing node to the next to place the standard part. Watch the ledger at the bottom: land Rx inside the window to light the link.",
        );
    }
    commands
        .spawn((
            BriefingRoot,
            // Tags the entity with the level's data-file id (e.g.
            // "world4_level1_outage") so it's identifiable in the Bevy
            // entity inspector / scene dumps during development — the
            // player never sees this, only the rendered briefing text.
            Name::new(level.id.clone()),
            Node {
                width: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                padding: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.051, 0.051, 0.118, 0.7)),
        ))
        .with_children(|parent| {
            // The briefing is the level's primary instruction text: it
            // renders at briefing size (17 px, not the 14 px fine print
            // of the first playtest) in a centered column capped at a
            // readable measure, so lines wrap inside the panel instead
            // of stretching edge to edge, and the auto-height panel
            // hugs the wrapped text.
            parent.spawn((
                Text::new(briefing_text),
                TextFont {
                    font: body_font.clone().into(),
                    font_size: FontSize::Px(17.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(BOARD_ACCENT),
                Node {
                    max_width: Val::Px(BRIEFING_TEXT_MAX_W),
                    ..default()
                },
            ));
        });

    // `Visibility` on the root is load-bearing: every board child is a
    // sprite or a text (both carry `Visibility` through their required
    // components), and a parent without it trips Bevy's B0004 hierarchy
    // warning once per child. The signal pulses spawn about once a
    // second, so the warning spammed the log for a whole session on
    // the first playtest.
    commands
        .spawn((BoardRoot, Transform::default(), Visibility::default()))
        .with_children(|parent| {
            // Circuit-texture backdrop fitted to the level's node extents,
            // far behind everything else (z = -10).
            let (backdrop_center, backdrop_size) = board_backdrop_frame(level);
            parent.spawn((
                BoardBackdrop,
                Sprite {
                    image: asset_server.load("sprites/ui/board_bg.png"),
                    custom_size: Some(backdrop_size),
                    ..default()
                },
                Transform::from_translation(backdrop_center.extend(-10.0)),
            ));
            // Pill rings are the main thing labels collide with on crowded
            // rows (w1l1's splice pair sits between two pill rings), so
            // their centers feed the label placer alongside the nodes.
            let pill_centers: Vec<Vec2> = grouped_choices(level)
                .into_iter()
                .filter(|(_, indices)| indices.len() > 1)
                .flat_map(|((from, to), indices)| {
                    let count = indices.len();
                    (0..count).filter_map(move |slot| pill_world_pos(level, from, to, slot, count))
                })
                .collect();
            // Row-parity stagger preserved as the placer's preference
            // order: deterministic per level file (node-id order).
            let label_requests: Vec<LabelRequest> = level
                .nodes
                .iter()
                .map(|node| {
                    let row_index = level
                        .nodes
                        .iter()
                        .filter(|n| n.grid_y == node.grid_y && n.id < node.id)
                        .count();
                    LabelRequest {
                        pos: grid_to_world(node.grid_x, node.grid_y),
                        label: &node.label,
                        prefer_above: row_index % 2 == 0,
                    }
                })
                .collect();
            let label_layouts =
                layout_node_labels(&label_requests, &pill_centers, half_w, view_center_x);
            for ((node, req), layout) in level
                .nodes
                .iter()
                .zip(label_requests.iter())
                .zip(label_layouts.iter())
            {
                let pos = req.pos;
                // Dark nameplate behind the label, sized by
                // `label_plate_size`; the plate keeps every label readable
                // where text crosses a ring or pill.
                parent.spawn((
                    Sprite {
                        color: Color::srgba(0.015, 0.02, 0.05, 0.88),
                        custom_size: Some(layout.plate),
                        ..default()
                    },
                    Transform::from_translation(layout.center.extend(4.9)),
                ));
                parent.spawn((
                    Text2d::new(node.label.clone()),
                    TextFont {
                        font: label_font.clone().into(),
                        font_size: FontSize::Px(LABEL_FONT_SIZE * FONT_SIZE_ADJUST),
                        ..default()
                    },
                    TextColor(BOARD_ACCENT),
                    Anchor::CENTER,
                    Transform::from_translation(layout.center.extend(5.0)),
                ));
                // Aseprite-crafted neon ring under the label; the band
                // sits exactly on NODE_RADIUS so the hit-test contract
                // (NODE_HIT_RADIUS) is unchanged. The flavor follows the
                // node: gold endpoints, tap leg for tap plant, broadcast
                // arcs for wireless sites, cyan otherwise.
                let flavor = node_ring_flavor(level, node.id, &node.label);
                parent.spawn((
                    NodeRing,
                    Sprite {
                        image: asset_server.load(node_ring_path(flavor)),
                        custom_size: Some(Vec2::splat(NODE_RING_SIZE)),
                        ..default()
                    },
                    Transform::from_translation(pos.extend(4.0)),
                ));
            }

            // One icon sprite per offered component choice, positioned at
            // the same spot `draw_board_gizmos` draws that pill's circle
            // (`pill_world_pos`) so the icon sits centered inside it.
            // Single-choice pairs place their component via drag alone
            // and never draw a pill circle either (see the `count <= 1`
            // skip in `draw_board_gizmos`), so icons are skipped there too
            // for visual consistency.
            for ((from, to), indices) in grouped_choices(level) {
                let count = indices.len();
                if count <= 1 {
                    continue;
                }
                for (slot, idx) in indices.into_iter().enumerate() {
                    let Some(pos) = pill_world_pos(level, from, to, slot, count) else {
                        continue;
                    };
                    let component = &level.available_components[idx].component;
                    // Aseprite-crafted neon ring under the icon; the band
                    // sits exactly on PILL_RADIUS so the hit-test contract
                    // is unchanged. The texture swaps to hot pink via
                    // `update_pill_rings` once the player picks this slot.
                    parent.spawn((
                        PillRing { from, to, slot },
                        Sprite {
                            image: asset_server.load(pill_ring_path(false)),
                            custom_size: Some(Vec2::splat(PILL_RING_SIZE)),
                            ..default()
                        },
                        Transform::from_translation(pos.extend(4.0)),
                    ));
                    // Value caption under the pill: the spec that
                    // tells sibling choices apart (three amps, one
                    // icon, three gains).
                    if let Some(caption) = component_value_label(component) {
                        parent.spawn((
                            Text2d::new(caption),
                            TextFont {
                                font: label_font.clone().into(),
                                font_size: FontSize::Px(13.0 * FONT_SIZE_ADJUST),
                                ..default()
                            },
                            TextColor(LIGHT_WARM),
                            Anchor::CENTER,
                            Transform::from_translation(
                                (pos + Vec2::new(0.0, -(PILL_RADIUS + 12.0))).extend(6.0),
                            ),
                        ));
                    }
                    if let Some(icon_path) = component_icon_path(component) {
                        parent.spawn((
                            ComponentIcon,
                            Sprite {
                                image: asset_server.load(icon_path),
                                custom_size: Some(Vec2::splat(48.0)),
                                ..default()
                            },
                            Transform::from_translation(pos.extend(6.0)),
                        ));
                    } else {
                        // Icon-less components (only `Span`, never offered
                        // as a choice) render a three-letter text pill
                        // instead — same ring, same position, no
                        // missing-texture magenta.
                        parent.spawn((
                            ComponentIcon,
                            Text2d::new(component_short_label(component)),
                            TextFont {
                                font: asset_server.load(crate::fonts::DISPLAY_BOLD).into(),
                                font_size: FontSize::Px(28.0 * FONT_SIZE_ADJUST),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Transform::from_translation(pos.extend(6.0)),
                        ));
                    }
                }
            }
        });

    commands
        .spawn((
            LedgerRoot,
            Node {
                width: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                padding: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent.spawn((
                LedgerText,
                Text::new("Loss: -- dB"),
                TextFont {
                    font: label_font.into(),
                    font_size: FontSize::Px(16.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(LIGHT_WARM),
            ));
        });
}

type BoardOrLedgerRoot = Or<(With<BoardRoot>, With<LedgerRoot>, With<BriefingRoot>)>;

pub fn teardown_board(mut commands: Commands, query: Query<Entity, BoardOrLedgerRoot>) {
    for entity in &query {
        commands.entity(entity).despawn();
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
    let Ok((camera, camera_transform)) = camera_q.single() else {
        pointer.0 = None;
        return;
    };

    let viewport_pos = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .or_else(|| touches.iter().next().map(|t| t.position()));

    pointer.0 = viewport_pos.and_then(|p| camera.viewport_to_world_2d(camera_transform, p).ok());
}

/// Turns pointer gestures into placements: drag node → node connects the
/// default choice for that pair; tapping a pill explicitly picks one
/// alternative among several offered choices. All decision logic lives in
/// `resolve_press`/`resolve_release` (see their unit tests below); this
/// system is just the thin ECS-resource glue around them.
// One parameter per input/state resource this glue needs to read or
// mutate; splitting it up would just move the same resource list into an
// artificial bag type for no clarity gain.
#[allow(clippy::too_many_arguments)]
pub fn handle_pointer_input(
    mut commands: Commands,
    level: Res<LevelDef>,
    pointer: Res<PointerWorld>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut drag: ResMut<DragState>,
    mut placed: ResMut<PlacedChoices>,
    mut live: ResMut<LiveGraph>,
    active_outage: Res<ActiveOutage>,
    mut splice_reactions: MessageWriter<crate::waifu::SpliceReaction>,
    sfx: Res<crate::audio::Sfx>,
    mut spark_requests: MessageWriter<crate::fx::SpawnConnectSpark>,
) {
    let just_pressed = mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed();
    let just_released = mouse.just_released(MouseButton::Left) || touches.any_just_released();

    if just_pressed {
        if let Some(pos) = pointer.0 {
            match resolve_press(&level, pos) {
                PressAction::StartDrag(node_id) => drag.from = Some(node_id),
                PressAction::SelectPill { from, to, slot } => {
                    sfx.play(&mut commands, crate::audio::SfxKind::Pick);
                    placed.0.insert((from, to), slot);
                    rebuild_live_graph(
                        &level,
                        &placed,
                        active_outage.outage.as_ref(),
                        &mut live.graph,
                    );
                    if let Some(component) = resolve_placed_component(&level, from, to, slot) {
                        if let Some(reaction) = splice_reaction_for(&component) {
                            splice_reactions.write(reaction);
                        }
                    }
                    test_log!("select from={} to={} slot={}", from, to, slot);
                }
                PressAction::None => {}
            }
        }
    }

    if just_released {
        if let ReleaseAction::Connect { from, to } = resolve_release(&level, drag.from, pointer.0) {
            sfx.play(&mut commands, crate::audio::SfxKind::Place);
            // Cyan spark burst at the connection midpoint.
            if let (Some(from_pos), Some(to_pos)) =
                (node_world_pos(&level, from), node_world_pos(&level, to))
            {
                spark_requests.write(crate::fx::SpawnConnectSpark {
                    position: from_pos.midpoint(to_pos),
                });
            }
            let slot = *placed.0.entry((from, to)).or_insert(0);
            rebuild_live_graph(
                &level,
                &placed,
                active_outage.outage.as_ref(),
                &mut live.graph,
            );
            if let Some(component) = resolve_placed_component(&level, from, to, slot) {
                if let Some(reaction) = splice_reaction_for(&component) {
                    splice_reactions.write(reaction);
                }
            }
            test_log!("connect from={} to={}", from, to);
        }
        drag.from = None;
    }
}

/// Draws a fiber line with a neon-tube glow: a wide faint halo pass on
/// both sides plus the bright core, echoing the light streams in the
/// keeper title artwork.
fn neon_line_2d(gizmos: &mut Gizmos, from: Vec2, to: Vec2, core: Color) {
    let dir = (to - from).normalize_or_zero();
    let perp = Vec2::new(-dir.y, dir.x);
    let halo = core.with_alpha(0.22);
    gizmos.line_2d(from + perp * 3.0, to + perp * 3.0, halo);
    gizmos.line_2d(from - perp * 3.0, to - perp * 3.0, halo);
    gizmos.line_2d(from, to, core);
}

fn draw_dashed_line(gizmos: &mut Gizmos, from: Vec2, to: Vec2, color: Color) {
    const SEGMENTS: i32 = 12;
    for i in 0..SEGMENTS {
        if i % 2 != 0 {
            continue;
        }
        let t0 = i as f32 / SEGMENTS as f32;
        let t1 = (i + 1) as f32 / SEGMENTS as f32;
        neon_line_2d(gizmos, from.lerp(to, t0), from.lerp(to, t1), color);
    }
}

/// Draws the board's fibers every frame with a neon glow: fixed edges
/// (solid), open choice edges (dashed, bright once placed), and an active
/// drag-preview line following the pointer. Node and pill rings are
/// Aseprite-crafted sprites spawned once by `spawn_board_from_level`, and
/// their selection glow is updated by `update_pill_rings` — no circles
/// are drawn here.
pub fn draw_board_gizmos(
    mut gizmos: Gizmos,
    level: Res<LevelDef>,
    placed: Res<PlacedChoices>,
    drag: Res<DragState>,
    pointer: Res<PointerWorld>,
    active_outage: Res<ActiveOutage>,
) {
    let outage = active_outage.outage.as_ref();

    for edge in &level.fixed_edges {
        if let (Some(a), Some(b)) = (
            node_world_pos(&level, edge.from),
            node_world_pos(&level, edge.to),
        ) {
            if is_edge_severed(outage, edge.from, edge.to) {
                draw_dashed_line(&mut gizmos, a, b, HAZARD_RED);
            } else {
                neon_line_2d(&mut gizmos, a, b, LIGHT_WARM);
            }
        }
    }

    for (from, to) in grouped_choices(&level).into_iter().map(|(k, _)| k) {
        if let (Some(a), Some(b)) = (node_world_pos(&level, from), node_world_pos(&level, to)) {
            let color = if is_edge_severed(outage, from, to) {
                HAZARD_RED
            } else if placed.0.contains_key(&(from, to)) {
                LIGHT_WARM
            } else {
                BOARD_LINE
            };
            draw_dashed_line(&mut gizmos, a, b, color);
        }
    }

    if let (Some(from_id), Some(pointer_pos)) = (drag.from, pointer.0) {
        if let Some(from_pos) = node_world_pos(&level, from_id) {
            neon_line_2d(&mut gizmos, from_pos, pointer_pos, LIGHT_HOT);
        }
    }
}

/// Seconds for the pill-ring pick pop (Finding 7).
const PILL_POP_SECS: f32 = 0.2;
/// Peak scale of the pill pop: 1.0 -> 1.25 -> 1.0.
const PILL_POP_PEAK: f32 = 1.25;

/// Scale-pop marker on a pill ring, inserted when its slot becomes the
/// picked one (Finding 7). The highest-frequency player interaction in
/// the game gets a visible pop per occurrence.
#[derive(Component)]
pub struct PillPop {
    pub(crate) elapsed_secs: f32,
}

/// Swaps pill-ring sprite textures when the player's picks change: the
/// chosen slot glows hot pink, the rest stay cyan. Runs only on the
/// frames `PlacedChoices` actually changed, and only touches the texture
/// handle — the sprites themselves are spawned once per level.
///
/// The freshly-picked ring also gets a 0.2s scale pop (Finding 7).
pub fn update_pill_rings(
    mut commands: Commands,
    placed: Res<PlacedChoices>,
    asset_server: Res<AssetServer>,
    mut rings: Query<(Entity, &PillRing, &mut Sprite)>,
) {
    if !placed.is_changed() {
        return;
    }
    for (entity, ring, mut sprite) in &mut rings {
        let selected = placed.0.get(&(ring.from, ring.to)).copied() == Some(ring.slot);
        let new_texture: Handle<Image> = asset_server.load(pill_ring_path(selected));
        // `AssetServer::load` is idempotent per path, so a changed handle
        // means this ring's pick state actually flipped.
        if sprite.image != new_texture {
            sprite.image = new_texture;
            if selected {
                commands
                    .entity(entity)
                    .insert(PillPop { elapsed_secs: 0.0 });
            }
        }
    }
}

/// Plays the 0.2s pill-ring pop: scale 1.0 -> 1.25 -> 1.0, both halves
/// eased with [`bevy::math::curve::EaseFunction::BackOut`] (Finding 7).
pub fn animate_pill_pops(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut PillPop)>,
) {
    for (entity, mut transform, mut pop) in &mut query {
        pop.elapsed_secs += time.delta_secs();
        let t = (pop.elapsed_secs / PILL_POP_SECS).clamp(0.0, 1.0);
        let scale = if t < 0.5 {
            1.0 + (PILL_POP_PEAK - 1.0) * EaseFunction::BackOut.sample_clamped(t * 2.0)
        } else {
            PILL_POP_PEAK
                - (PILL_POP_PEAK - 1.0) * EaseFunction::BackOut.sample_clamped((t - 0.5) * 2.0)
        };
        transform.scale = Vec3::splat(scale.max(0.01));
        if t >= 1.0 {
            transform.scale = Vec3::splat(1.0);
            commands.entity(entity).remove::<PillPop>();
        }
    }
}

/// Marks the circuit-texture backdrop sprite (`sprites/ui/board_bg.png`)
/// fitted to the level's node extents, far behind the board (z = -10).
#[derive(Component)]
struct BoardBackdrop;

/// Native pixel size of `sprites/ui/board_bg.png`.
const BOARD_BG_NATIVE: Vec2 = Vec2::new(1200.0, 1800.0);

/// Frame (center, world size) for the board backdrop: the level's node
/// extents plus a 200-unit margin on each side, scaled up from the
/// texture's native size just enough to cover (aspect preserved). Pure,
/// so tests can pin the fit without spawning anything.
fn board_backdrop_frame(level: &LevelDef) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for node in &level.nodes {
        let p = grid_to_world(node.grid_x, node.grid_y);
        min = min.min(p);
        max = max.max(p);
    }
    if level.nodes.is_empty() {
        min = Vec2::new(-600.0, -800.0);
        max = Vec2::new(600.0, 1000.0);
    }
    let center = (min + max) / 2.0;
    let span = (max - min).max(Vec2::splat(400.0)) + Vec2::splat(400.0);
    let scale = (span.x / BOARD_BG_NATIVE.x).max(span.y / BOARD_BG_NATIVE.y);
    (center, BOARD_BG_NATIVE * scale)
}

/// Horizontal clearance kept on each side of the node extents in the
/// fit frame: node rings (26) plus room for side-placed label plates.
const FIT_SIDE_PAD: f32 = 160.0;
/// Clearance below the lowest node: its below-labels (~128) plus the
/// ledger overlay and companion zone at the screen's foot.
const FIT_BOTTOM_PAD: f32 = 260.0;
/// Clearance above the highest node before the briefing allowance:
/// its above-labels at the far stagger (~128) plus a small gap.
const FIT_TOP_LABEL_PAD: f32 = 140.0;
/// Screen-space height (px) the briefing panel may occupy; converted
/// to world units at the candidate camera scale, since the panel is
/// UI and does not shrink when the camera zooms out. Long briefings
/// wrap to roughly this height at phone widths.
const FIT_BRIEFING_PX: f32 = 240.0;

/// The camera framing a level needs: where to center, and the
/// orthographic scale (world units per screen pixel, >= 1.0) at which
/// the whole board fits the window.
///
/// Why this exists: the board is laid out in fixed world coordinates
/// (200-unit grid) and the camera used to sit at 1:1 on a fixed point,
/// so any level wider than the window bled off the edges — most
/// levels span 800 world units against a 720-wide design window, and
/// Clara's span 1200. The playtest saw the bleed as clipped node
/// labels at the screen edges (worst behind the briefing panel, which
/// overlays the top of the view). Zooming out to fit — never in —
/// keeps every node, pill, and label reachable at any window size;
/// levels that already fit keep the historical 1:1 framing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardFraming {
    pub center: Vec2,
    pub scale: f32,
}

/// The orthographic scale at which `required` (world units) fits
/// `window` (screen px) on both axes. Never below 1.0: fitting only
/// ever zooms out.
pub fn fit_scale(required: Vec2, window: Vec2) -> f32 {
    (required.x / window.x).max(required.y / window.y).max(1.0)
}

/// Computes the fit framing for `level` at the given window size.
///
/// The required frame is the node extents padded by the fit margins;
/// the briefing allowance is scale-dependent (the panel is fixed-size
/// UI eating world space), so the scale is solved in two passes: a
/// first scale from label-only headroom, then the final scale with
/// the panel's world-space height folded into the top pad.
pub fn board_framing(level: &LevelDef, window: Vec2) -> BoardFraming {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for node in &level.nodes {
        let p = grid_to_world(node.grid_x, node.grid_y);
        min = min.min(p);
        max = max.max(p);
    }
    if level.nodes.is_empty() {
        min = Vec2::new(-600.0, -800.0);
        max = Vec2::new(600.0, 1000.0);
    }
    let frame = |top_pad: f32| {
        let lo = Vec2::new(min.x - FIT_SIDE_PAD, min.y - FIT_BOTTOM_PAD);
        let hi = Vec2::new(max.x + FIT_SIDE_PAD, max.y + top_pad);
        ((lo + hi) / 2.0, hi - lo)
    };
    let (_, size) = frame(FIT_TOP_LABEL_PAD);
    let first = fit_scale(size, window);
    let top_pad = FIT_TOP_LABEL_PAD + FIT_BRIEFING_PX * first;
    let (center, size) = frame(top_pad);
    BoardFraming {
        center,
        scale: first.max(fit_scale(size, window)),
    }
}

/// Largest number of signal pulses alive at once — one per lit edge in
/// the worst case. A dense level can't grow an unbounded sprite fleet.
const MAX_SIGNAL_PULSES: usize = 24;
/// Seconds between pulse spawns: each lit edge gets a fresh pulse about
/// twice a second, staggered by the repeating timer.
const PULSE_SPAWN_INTERVAL_SECS: f32 = 0.45;
/// World units a pulse travels per second.
const PULSE_SPEED: f32 = 260.0;

/// A light packet traveling a placed (lit) edge. `progress` runs 0.0 at
/// `from` to 1.0 at `to`; the system despawns it on arrival.
#[derive(Component)]
pub(crate) struct SignalPulse {
    from_id: u32,
    to_id: u32,
    from: Vec2,
    to: Vec2,
    progress: f32,
}

/// Repeating timer that staggers signal-pulse spawns.
#[derive(Resource)]
pub struct PulseSpawnTimer(Timer);

impl Default for PulseSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            PULSE_SPAWN_INTERVAL_SECS,
            TimerMode::Repeating,
        ))
    }
}

/// Spawns one pulse per lit edge (a placed component in `LiveGraph`) each
/// timer tick, skipping edges that already carry a pulse and stopping at
/// `MAX_SIGNAL_PULSES`. Pulses are `BoardRoot` children so level teardown
/// sweeps them with the rest of the board.
// One parameter per input/state resource this spawn pass needs; the same
// shape as `handle_pointer_input` below would only move the list around.
#[allow(clippy::too_many_arguments)]
pub fn spawn_signal_pulses(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<PulseSpawnTimer>,
    live_graph: Res<LiveGraph>,
    level: Res<LevelDef>,
    asset_server: Res<AssetServer>,
    pulses: Query<&SignalPulse>,
    board_roots: Query<Entity, With<BoardRoot>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    let Ok(board_root) = board_roots.single() else {
        return;
    };
    let mut live_count = pulses.iter().count();
    for edge in &live_graph.graph.edges {
        if live_count >= MAX_SIGNAL_PULSES {
            break;
        }
        if pulses
            .iter()
            .any(|p| p.from_id == edge.from && p.to_id == edge.to)
        {
            continue;
        }
        let (Some(from), Some(to)) = (
            node_world_pos(&level, edge.from),
            node_world_pos(&level, edge.to),
        ) else {
            continue;
        };
        commands.entity(board_root).with_children(|parent| {
            parent.spawn((
                SignalPulse {
                    from_id: edge.from,
                    to_id: edge.to,
                    from,
                    to,
                    progress: 0.0,
                },
                Sprite {
                    image: asset_server.load("sprites/fx/pulse_dot.png"),
                    custom_size: Some(Vec2::splat(24.0)),
                    ..default()
                },
                Transform::from_translation(from.extend(7.0)),
            ));
        });
        live_count += 1;
    }
}

/// Advances every signal pulse along its edge and despawns it on arrival.
pub fn move_signal_pulses(
    mut commands: Commands,
    time: Res<Time>,
    mut pulses: Query<(Entity, &mut SignalPulse, &mut Transform)>,
) {
    for (entity, mut pulse, mut transform) in &mut pulses {
        let edge_len = pulse.from.distance(pulse.to).max(1.0);
        pulse.progress += PULSE_SPEED * time.delta_secs() / edge_len;
        if pulse.progress >= 1.0 {
            // 0.16+ auto-detaches children on despawn; the manual
            // detach-before-despawn workaround is gone.
            commands.entity(entity).despawn();
            continue;
        }
        // Ease the progress so pulses accelerate out of the source and
        // decelerate into the target (Finding 8). PULSE_SPEED is unchanged;
        // only the curve changes.
        let eased = EaseFunction::SineInOut.sample_clamped(pulse.progress.clamp(0.0, 1.0));
        transform.translation = pulse.from.lerp(pulse.to, eased).extend(7.0);
    }
}

/// Largest number of rain streaks alive during an outage.
const MAX_STORM_STREAKS: usize = 40;
/// Streaks spawned per frame until the sky is full — avoids a 40-sprite
/// burst in a single frame when the outage hits.
const STREAKS_PER_FRAME: usize = 2;
/// Fall speed of a streak, world units per second.
const STREAK_FALL_SPEED: f32 = 520.0;
/// Rain spawns across this x band, from above the camera view, and wraps
/// back to the top once it falls past the bottom.
const RAIN_SPAWN_HALF_WIDTH: f32 = 620.0;
const RAIN_TOP_Y: f32 = 1200.0;
const RAIN_BOTTOM_Y: f32 = -900.0;

/// A diagonal storm streak, alive only while an outage is active.
#[derive(Component)]
pub(crate) struct StormStreak;

/// Holds the storm sky at `MAX_STORM_STREAKS` falling streaks while
/// `ActiveOutage.outage` is `Some`, and sweeps every streak the moment it
/// clears. Spawn lanes spread deterministically (golden-ratio stride, no
/// RNG resource in the hot path); streaks are `BoardRoot` children so
/// level teardown covers them too.
pub fn update_storm_rain(
    mut commands: Commands,
    time: Res<Time>,
    active_outage: Res<ActiveOutage>,
    asset_server: Res<AssetServer>,
    board_roots: Query<Entity, With<BoardRoot>>,
    mut streaks: Query<(Entity, &mut Transform), With<StormStreak>>,
) {
    if active_outage.outage.is_none() {
        for (entity, _) in &streaks {
            // 0.16+ auto-detaches children on despawn.
            commands.entity(entity).despawn();
        }
        return;
    }
    let Ok(board_root) = board_roots.single() else {
        return;
    };
    let mut live_count = streaks.iter().count();
    if live_count < MAX_STORM_STREAKS {
        commands.entity(board_root).with_children(|parent| {
            for _ in 0..STREAKS_PER_FRAME.min(MAX_STORM_STREAKS - live_count) {
                // 0.61803… stride spreads lanes without clumping.
                let lane = (live_count as f32 * 0.61803) % 1.0;
                let x = (lane * 2.0 - 1.0) * RAIN_SPAWN_HALF_WIDTH;
                let y = RAIN_TOP_Y + (live_count as f32 * 37.0) % 160.0;
                parent.spawn((
                    StormStreak,
                    Sprite {
                        image: asset_server.load("sprites/fx/storm_streak.png"),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(x, y, 3.0)),
                ));
                live_count += 1;
            }
        });
    }
    for (_, mut transform) in &mut streaks {
        transform.translation.y -= STREAK_FALL_SPEED * time.delta_secs();
        if transform.translation.y < RAIN_BOTTOM_Y {
            transform.translation.y = RAIN_TOP_Y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{ComponentChoice, LevelEdge, LevelNode, MediumDef, WavelengthDef};
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
            medium: MediumDef::Fiber,
            tx_dbm: 3.0,
            wavelength: WavelengthDef::Nm1490,
            window_min_dbm: -27.0,
            window_max_dbm: -8.0,
            endpoint_poe_draw_w: None,
            required_bandwidth_mbps: None,
            max_segment_length_m: None,
            coax_noise_floor_dbmv: None,
            min_carrier_to_noise_db: None,
            min_snr_db: None,
            nodes,
            fixed_edges,
            available_components,
            source_node,
            target_node,
            scripted_outage: None,
            subscribers: vec![],
            on_enter_line: None,
            on_win_line: None,
            on_fail_line: None,
            api_sequence: None,
            alarm_triage: None,
            quiz: None,
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
        rebuild_live_graph(&level, &placed, None, &mut graph);

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
        rebuild_live_graph(&level, &placed, None, &mut graph);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn rebuild_live_graph_excludes_an_unresolved_full_cut_edge() {
        let level = two_choice_level();
        let mut placed = PlacedChoices::default();
        placed.0.insert((1, 2), 0);
        let outage = Outage::new(osp_sim::OutageKind::FiberCut, 1, 2);
        let mut graph = PathGraph::default();
        rebuild_live_graph(&level, &placed, Some(&outage), &mut graph);
        assert!(
            graph.edges.is_empty(),
            "a full-cut hazard on the only placed edge must sever it from the live graph"
        );
    }

    #[test]
    fn rebuild_live_graph_keeps_a_resolved_full_cut_edge_connected() {
        let level = two_choice_level();
        let mut placed = PlacedChoices::default();
        placed.0.insert((1, 2), 0);
        let mut outage = Outage::new(osp_sim::OutageKind::FiberCut, 1, 2);
        outage.resolved = true;
        let mut graph = PathGraph::default();
        rebuild_live_graph(&level, &placed, Some(&outage), &mut graph);
        assert_eq!(
            graph.edges.len(),
            1,
            "a resolved outage must not sever the edge"
        );
    }

    #[test]
    fn rebuild_live_graph_keeps_a_degrade_type_outage_edge_connected() {
        let level = two_choice_level();
        let mut placed = PlacedChoices::default();
        placed.0.insert((1, 2), 0);
        let outage = Outage::new(osp_sim::OutageKind::WaterIntrusion, 1, 2);
        let mut graph = PathGraph::default();
        rebuild_live_graph(&level, &placed, Some(&outage), &mut graph);
        assert_eq!(
            graph.edges.len(),
            1,
            "degrade-type hazards keep the edge connected; loss is added on top instead"
        );
    }

    #[test]
    fn rebuild_live_graph_full_cut_on_a_fixed_edge_is_also_severed() {
        let level = fixture(
            vec![node(0, 0.0, 0.0), node(1, 1.0, 0.0)],
            vec![LevelEdge {
                from: 0,
                to: 1,
                component: Component::Span {
                    length_km: 8.0,
                    plant: PlantType::Buried,
                },
            }],
            vec![],
            0,
            1,
        );
        let placed = PlacedChoices::default();
        let outage = Outage::new(osp_sim::OutageKind::AerialDamage, 0, 1);
        let mut graph = PathGraph::default();
        rebuild_live_graph(&level, &placed, Some(&outage), &mut graph);
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
        rebuild_live_graph(&level, &placed, None, &mut live.graph);

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
            rebuild_live_graph(&level, &placed, None, &mut live.graph);
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
            rebuild_live_graph(&level, &placed, None, &mut live.graph);
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
        world.insert_resource(ActiveOutage::default());
        world.init_resource::<Messages<crate::waifu::SpliceReaction>>();
        world.init_resource::<Messages<crate::fx::SpawnConnectSpark>>();
        world.insert_resource(crate::audio::Sfx::for_tests());
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

    #[test]
    fn splice_reaction_for_maps_quality_to_the_matching_mood() {
        assert_eq!(
            splice_reaction_for(&fusion_splice()).map(|r| r.0),
            Some(crate::waifu::Mood::Blush)
        );
        assert_eq!(
            splice_reaction_for(&mechanical_splice()).map(|r| r.0),
            Some(crate::waifu::Mood::Pout)
        );
    }

    #[test]
    fn component_icon_path_covers_every_placeable_component_and_excludes_span() {
        assert_eq!(
            component_icon_path(&fusion_splice()),
            Some("sprites/components/fusion_splice.png")
        );
        assert_eq!(
            component_icon_path(&mechanical_splice()),
            Some("sprites/components/mechanical_splice.png")
        );
        assert_eq!(
            component_icon_path(&Component::Connector {
                kind: osp_sim::ConnectorType::Upc,
                contamination_db: 0.0,
            }),
            Some("sprites/components/upc_connector.png")
        );
        assert_eq!(
            component_icon_path(&Component::Connector {
                kind: osp_sim::ConnectorType::Apc,
                contamination_db: 0.0,
            }),
            Some("sprites/components/apc_connector.png")
        );
        assert_eq!(
            component_icon_path(&Component::Splitter {
                ratio: osp_sim::component::SplitterRatio::OneByFour,
            }),
            Some("sprites/components/splitter.png")
        );
        assert_eq!(
            component_icon_path(&Component::Macrobend {
                excess_loss_db: 0.0
            }),
            Some("sprites/components/macrobend.png")
        );
        assert_eq!(
            component_icon_path(&Component::Span {
                length_km: 1.0,
                plant: osp_sim::component::PlantType::Buried,
            }),
            None,
            "Span is fixed background plant, never a pill choice, and has no icon"
        );
        // The per-medium components have Aseprite icons now (see
        // `gen_components.lua`): each must resolve to its own sprite so
        // pills never fall back to the text label.
        for (component, expected) in [
            (
                Component::Amplifier { gain_db: 5.0 },
                "sprites/components/amplifier.png",
            ),
            (
                Component::Tap { tap_loss_db: 8.0 },
                "sprites/components/tap.png",
            ),
            (
                Component::CoaxSpan { length_m: 100.0 },
                "sprites/components/coax_span.png",
            ),
            (
                Component::WirelessHop {
                    distance_m: 400.0,
                    frequency_mhz: 2400.0,
                },
                "sprites/components/wireless_hop.png",
            ),
            (
                Component::Repeater { tx_dbm: 20.0 },
                "sprites/components/repeater.png",
            ),
            (
                Component::EthernetRun {
                    length_m: 65.0,
                    category: osp_sim::component::CableCategory::Cat5e,
                },
                "sprites/components/ethernet_run.png",
            ),
            (
                Component::Switch { poe_budget_w: 0.0 },
                "sprites/components/switch.png",
            ),
        ] {
            assert_eq!(
                component_icon_path(&component),
                Some(expected),
                "per-medium component must resolve to its Aseprite icon"
            );
        }
    }

    #[test]
    fn component_value_labels_distinguish_sibling_choices() {
        // The three Unity Gain amps share one icon and differ only in
        // gain: their captions must carry the difference.
        let amp_labels: Vec<String> = [15.0, 20.0, 25.0]
            .into_iter()
            .map(|gain_db| {
                component_value_label(&Component::Amplifier { gain_db })
                    .expect("amplifier must have a value caption")
            })
            .collect();
        assert_eq!(amp_labels, vec!["+15 dB", "+20 dB", "+25 dB"]);

        let tap_labels: Vec<String> = [8.0, 11.0, 14.0]
            .into_iter()
            .map(|tap_loss_db| {
                component_value_label(&Component::Tap { tap_loss_db })
                    .expect("tap must have a value caption")
            })
            .collect();
        assert_eq!(tap_labels, vec!["-8 dB", "-11 dB", "-14 dB"]);

        assert_eq!(
            component_value_label(&Component::Splitter {
                ratio: osp_sim::component::SplitterRatio::OneByFour,
            }),
            Some("1x4".to_string())
        );
        assert_eq!(
            component_value_label(&Component::EthernetRun {
                length_m: 65.0,
                category: osp_sim::component::CableCategory::Cat6,
            }),
            Some("65 m Cat6".to_string())
        );
        // Kind is the distinction for splices/connectors (their icons
        // differ); degradation/contamination are hidden state.
        assert_eq!(component_value_label(&fusion_splice()), None);
        assert_eq!(component_value_label(&mechanical_splice()), None);
        assert_eq!(
            component_value_label(&Component::Connector {
                kind: osp_sim::ConnectorType::Apc,
                contamination_db: 0.0,
            }),
            None
        );
    }

    #[test]
    fn component_short_label_covers_every_variant_with_unique_labels() {
        let labels = [
            (fusion_splice(), "FUS"),
            (mechanical_splice(), "MEC"),
            (
                Component::Connector {
                    kind: osp_sim::ConnectorType::Upc,
                    contamination_db: 0.0,
                },
                "UPC",
            ),
            (
                Component::Connector {
                    kind: osp_sim::ConnectorType::Apc,
                    contamination_db: 0.0,
                },
                "APC",
            ),
            (
                Component::Splitter {
                    ratio: osp_sim::component::SplitterRatio::OneByFour,
                },
                "SPL",
            ),
            (
                Component::Macrobend {
                    excess_loss_db: 0.0,
                },
                "BEND",
            ),
            (
                Component::Span {
                    length_km: 1.0,
                    plant: osp_sim::component::PlantType::Buried,
                },
                "SPAN",
            ),
            (Component::Amplifier { gain_db: 5.0 }, "AMP"),
            (Component::Tap { tap_loss_db: 8.0 }, "TAP"),
            (Component::CoaxSpan { length_m: 100.0 }, "COAX"),
            (
                Component::WirelessHop {
                    distance_m: 400.0,
                    frequency_mhz: 2400.0,
                },
                "AIR",
            ),
            (Component::Repeater { tx_dbm: 20.0 }, "RPT"),
            (
                Component::EthernetRun {
                    length_m: 65.0,
                    category: osp_sim::component::CableCategory::Cat5e,
                },
                "CAT",
            ),
            (Component::Switch { poe_budget_w: 0.0 }, "SW"),
        ];
        let mut seen = std::collections::HashSet::new();
        for (component, expected) in labels {
            assert_eq!(component_short_label(&component), expected);
            assert!(
                seen.insert(expected),
                "short label {expected} must be unique across components"
            );
        }
    }

    #[test]
    fn every_component_icon_path_actually_exists_under_game_assets() {
        // `asset_server.load(icon_path)` fails silently at runtime (a
        // missing-texture magenta square, not a panic) if the file isn't
        // there -- catch that here instead of only at manual playtest.
        for (component, path) in [
            (fusion_splice(), "sprites/components/fusion_splice.png"),
            (
                mechanical_splice(),
                "sprites/components/mechanical_splice.png",
            ),
        ] {
            assert_eq!(component_icon_path(&component), Some(path));
            let on_disk = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/").to_string() + path;
            assert!(
                std::path::Path::new(&on_disk).is_file(),
                "{on_disk} referenced by component_icon_path but missing on disk"
            );
        }
        for path in [
            "sprites/components/upc_connector.png",
            "sprites/components/apc_connector.png",
            "sprites/components/splitter.png",
            "sprites/components/macrobend.png",
        ] {
            let on_disk = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/").to_string() + path;
            assert!(
                std::path::Path::new(&on_disk).is_file(),
                "{on_disk} referenced by component_icon_path but missing on disk"
            );
        }
    }

    #[test]
    fn splice_reaction_for_is_none_for_non_splice_components() {
        let splitter = Component::Splitter {
            ratio: osp_sim::component::SplitterRatio::OneByTwo,
        };
        assert!(splice_reaction_for(&splitter).is_none());
    }

    #[test]
    fn resolve_placed_component_looks_up_the_nth_choice_for_a_pair() {
        let level = two_choice_level();
        assert_eq!(
            resolve_placed_component(&level, 1, 2, 0),
            Some(fusion_splice())
        );
        assert_eq!(
            resolve_placed_component(&level, 1, 2, 1),
            Some(mechanical_splice())
        );
        assert_eq!(resolve_placed_component(&level, 1, 2, 99), None);
    }

    #[test]
    fn tapping_the_mechanical_pill_fires_a_pout_reaction_on_the_companion() {
        use bevy_ecs::system::RunSystemOnce;

        let mut world = test_world(two_choice_level());
        let companion = world
            .spawn(crate::waifu::CompanionSprite {
                companion: crate::waifu::Companion::default(),
                mood: crate::waifu::Mood::Idle,
                anim_timer: Timer::from_seconds(0.18, TimerMode::Repeating),
                frame: 0,
            })
            .id();

        // Same pointer position as `system_tapping_a_pill_selects_it_
        // without_starting_a_drag` above -- lands on the Mechanical
        // (slot 1) pill.
        world.resource_mut::<PointerWorld>().0 = Some(Vec2::new(100.0, 335.0));
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        world.run_system_once(handle_pointer_input);
        world.run_system_once(crate::waifu::react_to_splice_events);

        let sprite = world
            .get::<crate::waifu::CompanionSprite>(companion)
            .unwrap();
        assert_eq!(sprite.mood, crate::waifu::Mood::Pout);
        assert_eq!(sprite.frame, 0);
    }

    #[test]
    fn node_ring_path_maps_every_flavor() {
        assert_eq!(
            node_ring_path(NodeRingFlavor::Endpoint),
            "sprites/ui/node_ring_gold.png"
        );
        assert_eq!(
            node_ring_path(NodeRingFlavor::Tap),
            "sprites/ui/node_ring_tap.png"
        );
        assert_eq!(
            node_ring_path(NodeRingFlavor::Site),
            "sprites/ui/node_ring_site.png"
        );
        assert_eq!(
            node_ring_path(NodeRingFlavor::Standard),
            "sprites/ui/node_ring_cyan.png"
        );
    }

    #[test]
    fn node_ring_flavor_picks_endpoint_tap_site_and_standard() {
        let mut level = fixture(
            vec![
                LevelNode {
                    id: 1,
                    label: "Headend".into(),
                    grid_x: 0.0,
                    grid_y: 0.0,
                },
                LevelNode {
                    id: 2,
                    label: "Tap".into(),
                    grid_x: 1.0,
                    grid_y: 0.0,
                },
                LevelNode {
                    id: 3,
                    label: "Customer Drop".into(),
                    grid_x: 2.0,
                    grid_y: 0.0,
                },
            ],
            vec![],
            vec![],
            1,
            3,
        );
        // Coax level: endpoints gold, the "Tap"-labeled node gets the tap
        // leg, everything else standard cyan.
        level.medium = MediumDef::Coax;
        assert_eq!(
            node_ring_flavor(&level, 1, "Headend"),
            NodeRingFlavor::Endpoint
        );
        assert_eq!(
            node_ring_flavor(&level, 3, "Customer Drop"),
            NodeRingFlavor::Endpoint
        );
        assert_eq!(node_ring_flavor(&level, 2, "Tap"), NodeRingFlavor::Tap);
        // Wireless level: a non-endpoint, non-tap node gets broadcast arcs.
        level.medium = MediumDef::Wireless;
        assert_eq!(
            node_ring_flavor(&level, 2, "Ridge Repeater"),
            NodeRingFlavor::Site
        );
        assert_eq!(
            node_ring_flavor(&level, 1, "Site A"),
            NodeRingFlavor::Endpoint
        );
        // Fiber level, unknown node: standard cyan.
        level.medium = MediumDef::Fiber;
        assert_eq!(
            node_ring_flavor(&level, 9, "Splice Enclosure"),
            NodeRingFlavor::Standard
        );
    }

    #[test]
    fn pill_ring_path_is_pink_when_selected_and_cyan_otherwise() {
        assert_eq!(pill_ring_path(true), "sprites/ui/pill_ring_selected.png");
        assert_eq!(pill_ring_path(false), "sprites/ui/pill_ring_normal.png");
    }

    #[test]
    fn ring_sprite_sizes_match_their_visual_radii() {
        // 1 world unit == 1 sprite px; the 14 px margin holds the glow
        // halo and the diagonal circuit ticks.
        assert_eq!(NODE_RING_SIZE, NODE_RADIUS * 2.0 + 28.0);
        assert_eq!(PILL_RING_SIZE, PILL_RADIUS * 2.0 + 28.0);
    }

    #[test]
    fn spawn_signal_pulses_places_one_pulse_per_lit_edge() {
        use bevy::asset::AssetPlugin;
        use bevy_ecs::system::RunSystemOnce;
        use std::time::Duration;

        let level = fixture(
            vec![node(0, 0.0, 0.0), node(1, 1.0, 0.0)],
            vec![LevelEdge {
                from: 0,
                to: 1,
                component: Component::Span {
                    length_km: 8.0,
                    plant: PlantType::Buried,
                },
            }],
            vec![],
            0,
            1,
        );
        let mut live = LiveGraph {
            graph: PathGraph::default(),
            wavelength: WavelengthWrapper(level.wavelength.into()),
            tx_dbm: level.tx_dbm,
        };
        rebuild_live_graph(&level, &PlacedChoices::default(), None, &mut live.graph);
        assert_eq!(live.graph.edges.len(), 1);

        // Standalone `AssetPlugin` (no `TimePlugin`): the pulse system gets
        // a real `AssetServer` for `load()`, while `Time<()>` stays fully
        // under this test's control via `advance_by`.
        let mut app = App::new();
        app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_resource::<Time>();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.world_mut().insert_resource(PulseSpawnTimer::default());
        app.world_mut().insert_resource(live);
        app.world_mut().insert_resource(level);
        app.world_mut().spawn(BoardRoot);
        app.world_mut().run_system_once(spawn_signal_pulses);

        let world = app.world_mut();
        let pulses: Vec<(u32, u32)> = world
            .query::<&SignalPulse>()
            .iter(world)
            .map(|p| (p.from_id, p.to_id))
            .collect();
        assert_eq!(pulses, vec![(0, 1)]);
    }

    #[test]
    fn move_signal_pulses_advances_and_despawns_at_the_far_end() {
        use bevy_ecs::system::RunSystemOnce;
        use std::time::Duration;

        let mut world = World::new();
        world.init_resource::<Time>();
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        let entity = world
            .spawn((
                SignalPulse {
                    from_id: 0,
                    to_id: 1,
                    from: Vec2::new(0.0, 300.0),
                    to: Vec2::new(200.0, 300.0),
                    progress: 0.99,
                },
                Transform::default(),
            ))
            .id();
        world.run_system_once(move_signal_pulses);
        // 260 u/s over a 200 u edge crosses the last 1% in one 1 s step.
        assert!(world.get_entity(entity).is_err());
    }

    #[test]
    fn update_storm_rain_spawns_streaks_during_an_outage_and_clears_them_after() {
        use bevy::app::TaskPoolPlugin;
        use bevy::asset::AssetPlugin;
        use bevy_ecs::system::RunSystemOnce;
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_resource::<Time>();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.world_mut().spawn(BoardRoot);
        // Outage present: streaks spawn up to the per-frame batch size.
        app.world_mut().insert_resource(ActiveOutage {
            outage: Some(Outage::new(osp_sim::OutageKind::AerialDamage, 0, 1)),
            ..default()
        });
        app.world_mut().run_system_once(update_storm_rain);
        let spawned = app
            .world_mut()
            .query::<&StormStreak>()
            .iter(app.world_mut())
            .count();
        assert!(spawned > 0, "outage should spawn streaks");

        // Outage cleared: all streaks despawn.
        app.world_mut().resource_mut::<ActiveOutage>().outage = None;
        app.world_mut().run_system_once(update_storm_rain);
        let remaining = app
            .world_mut()
            .query::<&StormStreak>()
            .iter(app.world_mut())
            .count();
        assert_eq!(remaining, 0, "cleared outage should despawn streaks");
    }

    /// w1l1 geometry (from the shipped level file): three nodes in a row
    /// with two pill rings on the splice->ONT edge. Regression test for
    /// the shipped bug where the ONT and splice labels sat on top of the
    /// pill rings.
    #[test]
    fn label_layout_w1l1_avoids_pills_and_other_labels() {
        let requests = vec![
            LabelRequest {
                pos: Vec2::new(-200.0, 300.0),
                label: "OLT (Central Office)",
                prefer_above: true,
            },
            LabelRequest {
                pos: Vec2::new(0.0, 300.0),
                label: "Splice Enclosure 14+00",
                prefer_above: false,
            },
            LabelRequest {
                pos: Vec2::new(200.0, 300.0),
                label: "ONT (Customer Premise)",
                prefer_above: true,
            },
        ];
        let pills = vec![Vec2::new(100.0, 265.0), Vec2::new(100.0, 335.0)];
        let half_w = 450.0; // Matt's phone screenshot width / 2
        let placed = layout_node_labels(&requests, &pills, half_w, 0.0);
        assert_eq!(placed.len(), 3);
        for (i, placement) in placed.iter().enumerate() {
            let half = placement.plate * 0.5;
            // On screen with margin.
            assert!(
                placement.center.x - half.x >= -half_w + LABEL_EDGE_MARGIN - 0.01,
                "label {i} clips the left edge"
            );
            assert!(
                placement.center.x + half.x <= half_w - LABEL_EDGE_MARGIN + 0.01,
                "label {i} clips the right edge"
            );
            // Clear of every pill ring.
            for pill in &pills {
                assert!(
                    !rect_hits_circle(placement.center, half, *pill, LABEL_PILL_OBSTACLE_R),
                    "label {i} covers a pill ring"
                );
            }
            // Clear of every other node's ring.
            for (j, req) in requests.iter().enumerate() {
                if j != i {
                    assert!(
                        !rect_hits_circle(placement.center, half, req.pos, LABEL_NODE_OBSTACLE_R),
                        "label {i} covers node {j}'s ring"
                    );
                }
            }
            // Clear of every other label plate.
            for (j, other) in placed.iter().enumerate() {
                if j != i {
                    assert!(
                        !rects_overlap(placement.center, half, other.center, other.plate * 0.5),
                        "label {i} overlaps label {j}"
                    );
                }
            }
        }
    }

    /// c1l1 regression: node 3 ("Customer Drop") sits at world x = 400,
    /// near the right edge of a phone screen — its plate must be pulled
    /// on screen instead of clipping.
    #[test]
    fn label_layout_clamps_wide_label_onscreen() {
        let requests = vec![LabelRequest {
            pos: Vec2::new(400.0, 300.0),
            label: "Customer Drop",
            prefer_above: true,
        }];
        let placed = layout_node_labels(&requests, &[], 450.0, 0.0);
        assert_eq!(placed.len(), 1);
        let half = placed[0].plate * 0.5;
        assert!(
            placed[0].center.x + half.x <= 450.0 - LABEL_EDGE_MARGIN + 0.01,
            "plate right edge = {}",
            placed[0].center.x + half.x
        );
        assert!(
            placed[0].center.x - half.x >= -450.0 + LABEL_EDGE_MARGIN - 0.01,
            "plate left edge = {}",
            placed[0].center.x - half.x
        );
    }

    /// A plate wider than the screen centers at x = 0 rather than
    /// clipping one edge.
    #[test]
    fn label_layout_centers_oversize_plate() {
        let long_label = "W".repeat(100);
        let requests = vec![LabelRequest {
            pos: Vec2::new(100.0, 300.0),
            label: &long_label,
            prefer_above: true,
        }];
        let placed = layout_node_labels(&requests, &[], 200.0, 0.0);
        assert!(
            placed[0].center.x.abs() < 0.01,
            "oversize plate should center, got {}",
            placed[0].center.x
        );
    }

    /// Uncrowded nodes keep the historical stagger: above for even rows,
    /// below for odd.
    #[test]
    fn label_layout_prefers_stagger_when_clear() {
        let requests = vec![
            LabelRequest {
                pos: Vec2::new(-200.0, 300.0),
                label: "A",
                prefer_above: true,
            },
            LabelRequest {
                pos: Vec2::new(200.0, 300.0),
                label: "B",
                prefer_above: false,
            },
        ];
        let placed = layout_node_labels(&requests, &[], 450.0, 0.0);
        assert!(
            placed[0].center.y > 300.0,
            "even row should sit above, got {}",
            placed[0].center.y
        );
        assert!(
            placed[1].center.y < 300.0,
            "odd row should sit below, got {}",
            placed[1].center.y
        );
        // Near slots, not far ones.
        assert!((placed[0].center.y - 300.0 - LABEL_NEAR_DY).abs() < 0.01);
        assert!((300.0 - placed[1].center.y - LABEL_NEAR_DY).abs() < 0.01);
    }

    /// Deterministic: same input, same output, every time.
    #[test]
    fn label_layout_is_deterministic() {
        let requests = vec![
            LabelRequest {
                pos: Vec2::new(0.0, 300.0),
                label: "Splice Enclosure 14+00",
                prefer_above: false,
            },
            LabelRequest {
                pos: Vec2::new(200.0, 300.0),
                label: "ONT (Customer Premise)",
                prefer_above: true,
            },
        ];
        let pills = vec![Vec2::new(100.0, 265.0), Vec2::new(100.0, 335.0)];
        let a = layout_node_labels(&requests, &pills, 450.0, 0.0);
        let b = layout_node_labels(&requests, &pills, 450.0, 0.0);
        assert_eq!(a, b);
    }

    #[test]
    fn rect_hits_circle_detects_containment_and_misses() {
        let center = Vec2::new(0.0, 0.0);
        let half = Vec2::new(50.0, 20.0);
        assert!(rect_hits_circle(center, half, Vec2::new(0.0, 0.0), 10.0));
        assert!(rect_hits_circle(center, half, Vec2::new(60.0, 0.0), 15.0));
        assert!(!rect_hits_circle(center, half, Vec2::new(200.0, 0.0), 15.0));
        assert!(!rect_hits_circle(center, half, Vec2::new(0.0, 100.0), 15.0));
    }

    // -- Camera fit framing (playtest round 2: nothing bleeds off-window) --

    #[test]
    fn fit_scale_never_zooms_in() {
        // A board smaller than the window keeps the historical 1:1.
        assert_eq!(
            fit_scale(Vec2::new(400.0, 600.0), Vec2::new(720.0, 1280.0)),
            1.0
        );
        // An exact fit is still 1:1.
        assert_eq!(
            fit_scale(Vec2::new(720.0, 1280.0), Vec2::new(720.0, 1280.0)),
            1.0
        );
    }

    #[test]
    fn fit_scale_grows_with_the_tight_axis() {
        // Width-bound: 1440 world units into a 720 window.
        assert_eq!(
            fit_scale(Vec2::new(1440.0, 600.0), Vec2::new(720.0, 1280.0)),
            2.0
        );
        // Height-bound: 2560 world units into a 1280 window.
        assert_eq!(
            fit_scale(Vec2::new(400.0, 2560.0), Vec2::new(720.0, 1280.0)),
            2.0
        );
    }

    /// The playtest fit check, pinned as a test: at the narrow (~650)
    /// and wide (~1300) window widths, plus the 720 design width,
    /// every shipped level's nodes — and their furthest label
    /// extremes — sit inside the framed view. Before the fit framing,
    /// any level wider than the window (most of them) bled nodes and
    /// label fragments off the edges.
    #[test]
    fn every_shipped_level_fits_at_playtest_window_widths() {
        let windows = [
            Vec2::new(650.0, 1156.0),
            Vec2::new(720.0, 1280.0),
            Vec2::new(1300.0, 1280.0),
        ];
        let label_reach = LABEL_FAR_DY + 16.0; // far stagger + plate half
        for i in 0..crate::level::LEVEL_SOURCES.len() {
            let level = crate::level::load_level(i);
            for window in windows {
                let framing = board_framing(&level, window);
                assert!(
                    framing.scale >= 1.0,
                    "{}: fit must never zoom in (scale {})",
                    level.id,
                    framing.scale
                );
                let half_view = window * framing.scale * 0.5;
                for node in &level.nodes {
                    let p = grid_to_world(node.grid_x, node.grid_y);
                    assert!(
                        (p.x - framing.center.x).abs() <= half_view.x + 0.5,
                        "{}: node ({}, {}) outside the framed view at {window:?} \
                         (center {}, half {})",
                        level.id,
                        node.grid_x,
                        node.grid_y,
                        framing.center.x,
                        half_view.x
                    );
                    assert!(
                        p.y + label_reach <= framing.center.y + half_view.y + 0.5
                            && p.y - label_reach >= framing.center.y - half_view.y - 0.5,
                        "{}: node ({}, {}) label reach escapes the framed view \
                         at {window:?}",
                        level.id,
                        node.grid_x,
                        node.grid_y
                    );
                }
            }
        }
    }

    /// The first level at the design window keeps the historical 1:1
    /// framing — the fit must not shrink boards that already fit.
    #[test]
    fn the_first_level_keeps_its_1_to_1_framing() {
        let level = crate::level::load_level(0);
        let framing = board_framing(&level, Vec2::new(720.0, 1280.0));
        assert_eq!(framing.scale, 1.0);
    }

    /// The widest shipped content (Clara's seven-column levels) is the
    /// case that bled hardest: at the narrow playtest width it must
    /// zoom out substantially and still contain its extreme nodes.
    #[test]
    fn the_widest_levels_zoom_out_and_fit_at_the_narrow_width() {
        let mut widest: Option<(String, f32, BoardFraming)> = None;
        for i in 0..crate::level::LEVEL_SOURCES.len() {
            let level = crate::level::load_level(i);
            let framing = board_framing(&level, Vec2::new(650.0, 1156.0));
            let beats = widest.as_ref().is_none_or(|(_, s, _)| framing.scale > *s);
            if beats {
                widest = Some((level.id.clone(), framing.scale, framing));
            }
        }
        let (id, scale, _) = widest.expect("at least one level exists");
        assert!(
            scale > 1.5,
            "the widest level ({id}) should need real zoom-out at 650 px, got {scale}"
        );
    }
}

/// Tags a continuous pulse-flow quad on a placed edge. The quad's U axis
/// runs along the fiber; the pulse shader animates light traveling down
/// it. Complements the discrete `SignalPulse` sprites with smooth flow.
#[derive(Debug, Component)]
pub(crate) struct FiberFlow {
    from_id: u32,
    to_id: u32,
}

/// Sync pulse-flow quads with the live graph: one continuous light-flow
/// quad per placed edge, removed when the edge is removed. Quads are
/// `BoardRoot` children so level teardown sweeps them.
pub fn sync_fiber_flows(
    mut commands: Commands,
    live_graph: Res<LiveGraph>,
    level: Res<LevelDef>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<PulseMaterial>>>,
    flows: Query<(Entity, &FiberFlow)>,
    board_roots: Query<Entity, With<BoardRoot>>,
) {
    let Ok(board_root) = board_roots.single() else {
        return;
    };
    let mut materials = match materials {
        Some(m) => m,
        None => return,
    };
    // Remove flows whose edge left the graph.
    for (entity, flow) in &flows {
        let still_live = live_graph
            .graph
            .edges
            .iter()
            .any(|e| e.from == flow.from_id && e.to == flow.to_id);
        if !still_live {
            commands.entity(entity).despawn();
        }
    }
    // Spawn flows for new edges.
    for edge in &live_graph.graph.edges {
        if flows
            .iter()
            .any(|(_, f)| f.from_id == edge.from && f.to_id == edge.to)
        {
            continue;
        }
        let (Some(from), Some(to)) = (
            node_world_pos(&level, edge.from),
            node_world_pos(&level, edge.to),
        ) else {
            continue;
        };
        let delta = to - from;
        let len = delta.length().max(1.0);
        let angle = delta.y.atan2(delta.x);
        let mid = (from + to) / 2.0;
        commands.entity(board_root).with_children(|parent| {
            parent.spawn((
                FiberFlow {
                    from_id: edge.from,
                    to_id: edge.to,
                },
                Mesh2d(meshes.add(Rectangle::new(len, 10.0))),
                MeshMaterial2d(materials.add(PulseMaterial {
                    settings: PulseSettings {
                        color: Vec4::new(1.0, 0.85, 0.55, 1.0),
                        ..Default::default()
                    },
                })),
                Transform {
                    translation: mid.extend(0.5),
                    rotation: Quat::from_rotation_z(angle),
                    ..default()
                },
            ));
        });
    }
}
