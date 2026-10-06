//! End-to-end link-budget validation for `osp_sim`.
//!
//! Every scenario here builds a whole multi-element topology and checks the
//! received level against figures hand-computed from docs/GAME_DESIGN.md
//! ("The Optical Model"). The in-crate unit tests check single elements;
//! these tests check that the elements compose into the right answer.
//!
//! Tolerances are named constants with units. Exact sums use
//! `EXACT_DB_TOLERANCE` (float round-off only); doc figures marked "~" use
//! `DOC_APPROX_DB_TOLERANCE`.

use osp_sim::alarm::{Alarm, AlarmAck, AlarmSeverity};
use osp_sim::component::{Component, ConnectorType, PlantType, SpliceType, SplitterRatio};
use osp_sim::graph::{NodeId, PathError, PathGraph};
use osp_sim::outage::{Outage, OutageKind};
use osp_sim::wavelength::Wavelength;
use osp_sim::{LinkBudgetResult, ReceiveWindow, DEFAULT_TX_DBM};

// ---- docs/GAME_DESIGN.md figures (dB unless noted) ----

/// Float round-off budget for sums of a handful of dB terms.
const EXACT_DB_TOLERANCE: f64 = 1e-9;
/// The doc writes splitter losses as "~X"; real PLC parts vary by a couple
/// of tenths of a dB, so "~" is read as ±0.25 dB.
const DOC_APPROX_DB_TOLERANCE: f64 = 0.25;

const DOC_FUSION_DB: (f64, f64) = (0.05, 0.1);
const DOC_MECHANICAL_DB: (f64, f64) = (0.3, 0.5);
const DOC_UPC_DB: (f64, f64) = (0.3, 0.5);
const DOC_APC_DB: f64 = 0.3;
const DOC_SMF_1310_DB_PER_KM: f64 = 0.35;
/// The doc lists 0.25 dB/km at 1550 nm; the sim uses a typical-fiber 0.21,
/// so the doc figure is enforced as a ceiling, not an equality.
const DOC_SMF_1550_DB_PER_KM_MAX: f64 = 0.25;
/// Splitter rows from the doc's table, in ratio order.
const DOC_SPLITTERS: [(SplitterRatio, u32, f64); 5] = [
    (SplitterRatio::OneByTwo, 2, 3.5),
    (SplitterRatio::OneByFour, 4, 7.2),
    (SplitterRatio::OneByEight, 8, 10.5),
    (SplitterRatio::OneBySixteen, 16, 13.5),
    (SplitterRatio::OneByThirtyTwo, 32, 17.5),
];

// ---- sim's concrete per-element values (midpoints inside the doc ranges) ----

const SIM_FUSION_DB: f64 = 0.075;
const SIM_MECHANICAL_DB: f64 = 0.4;
const SIM_UPC_DB: f64 = 0.35;
const SIM_APC_DB: f64 = 0.30;
const SIM_1X32_DB: f64 = 17.7;
const SIM_1490_DB_PER_KM: f64 = 0.28;

// ---- shared topology node ids ----

const OLT: NodeId = 0;
const ONT: NodeId = 99;

fn assert_db_eq(actual_db: f64, expected_db: f64, tolerance_db: f64, what: &str) {
    assert!(
        (actual_db - expected_db).abs() <= tolerance_db,
        "{what}: got {actual_db} dB, expected {expected_db} ± {tolerance_db} dB"
    );
}

fn span(length_km: f64, plant: PlantType) -> Component {
    Component::Span { length_km, plant }
}

fn splice(kind: SpliceType) -> Component {
    Component::Splice {
        kind,
        degradation_db: 0.0,
    }
}

fn connector(kind: ConnectorType) -> Component {
    Component::Connector {
        kind,
        contamination_db: 0.0,
    }
}

fn splitter(ratio: SplitterRatio) -> Component {
    Component::Splitter { ratio }
}

/// Wire `chain` as a straight path OLT → 1 → 2 → … → ONT.
fn chain_graph(chain: &[Component]) -> PathGraph {
    assert!(!chain.is_empty(), "a chain needs at least one element");
    let mut graph = PathGraph::default();
    graph.add_node(OLT, "OLT");
    let last_index = chain.len() - 1;
    let mut from = OLT;
    for (index, component) in chain.iter().enumerate() {
        let to = if index == last_index {
            ONT
        } else {
            NodeId::try_from(index + 1).expect("test chains are short")
        };
        graph.add_node(to, if to == ONT { "ONT" } else { "Plant" });
        graph.connect(from, to, component.clone());
        from = to;
    }
    graph
}

fn gpon(graph: &PathGraph, wavelength: Wavelength) -> Result<LinkBudgetResult, PathError> {
    graph.compute_link_budget(
        OLT,
        ONT,
        DEFAULT_TX_DBM,
        wavelength,
        ReceiveWindow::GPON_ONT,
    )
}

/// A realistic GPON drop: OLT patch (APC) → 12 km buried feeder → fusion →
/// 8 km conduit feeder → fusion → 1×32 PLC splitter in the FDH → fusion →
/// 1.2 km aerial drop → ONT patch (APC).
fn realistic_pon_drop() -> Vec<Component> {
    vec![
        connector(ConnectorType::Apc),
        span(12.0, PlantType::Buried),
        splice(SpliceType::Fusion),
        span(8.0, PlantType::Conduit),
        splice(SpliceType::Fusion),
        splitter(SplitterRatio::OneByThirtyTwo),
        splice(SpliceType::Fusion),
        span(1.2, PlantType::Aerial),
        connector(ConnectorType::Apc),
    ]
}

const PON_DROP_FIBER_KM: f64 = 12.0 + 8.0 + 1.2;
const PON_DROP_FUSION_COUNT: f64 = 3.0;
const PON_DROP_APC_COUNT: f64 = 2.0;

// ---- full OLT → ONT path ----

#[test]
fn realistic_pon_drop_matches_the_hand_computed_ledger_at_1490nm() {
    let graph = chain_graph(&realistic_pon_drop());
    let result = gpon(&graph, Wavelength::Nm1490).expect("drop is fully routed");

    // 2×0.30 APC + 21.2 km×0.28 + 3×0.075 fusion + 17.7 splitter = 24.461 dB.
    let expected_loss_db = PON_DROP_APC_COUNT * SIM_APC_DB
        + PON_DROP_FIBER_KM * SIM_1490_DB_PER_KM
        + PON_DROP_FUSION_COUNT * SIM_FUSION_DB
        + SIM_1X32_DB;
    assert_db_eq(expected_loss_db, 24.461, EXACT_DB_TOLERANCE, "hand ledger");
    assert_db_eq(
        result.total_loss_db,
        expected_loss_db,
        EXACT_DB_TOLERANCE,
        "loss",
    );
    assert_db_eq(result.received_dbm, -21.461, EXACT_DB_TOLERANCE, "P_rx");
    assert!(result.in_window, "−21.461 dBm sits inside −27…−8 dBm");
    // Binding edge is the −27 dBm floor: 5.539 dB of headroom.
    assert_db_eq(result.margin_db, 5.539, EXACT_DB_TOLERANCE, "margin");
    assert_eq!(result.hop_count, 9);
}

#[test]
fn realistic_pon_drop_at_1310nm_uses_the_doc_attenuation_exactly() {
    let graph = chain_graph(&realistic_pon_drop());
    let result = gpon(&graph, Wavelength::Nm1310).expect("drop is fully routed");

    // Same ledger with the doc's 0.35 dB/km: 0.6 + 7.42 + 0.225 + 17.7.
    let expected_loss_db = PON_DROP_APC_COUNT * DOC_APC_DB
        + PON_DROP_FIBER_KM * DOC_SMF_1310_DB_PER_KM
        + PON_DROP_FUSION_COUNT * SIM_FUSION_DB
        + SIM_1X32_DB;
    assert_db_eq(
        result.total_loss_db,
        expected_loss_db,
        EXACT_DB_TOLERANCE,
        "loss",
    );
    assert_db_eq(
        result.total_loss_db,
        25.945,
        EXACT_DB_TOLERANCE,
        "loss literal",
    );
}

#[test]
fn realistic_pon_drop_total_lies_inside_the_doc_range_envelope() {
    // Sum the doc's lowest and highest figures element by element: whatever
    // concrete values the sim picks, the total must land between them.
    let graph = chain_graph(&realistic_pon_drop());
    let result = gpon(&graph, Wavelength::Nm1310).expect("drop is fully routed");

    let fixed_db = PON_DROP_APC_COUNT * DOC_APC_DB + PON_DROP_FIBER_KM * DOC_SMF_1310_DB_PER_KM;
    let doc_1x32_db = DOC_SPLITTERS[4].2;
    let min_db = fixed_db
        + PON_DROP_FUSION_COUNT * DOC_FUSION_DB.0
        + (doc_1x32_db - DOC_APPROX_DB_TOLERANCE);
    let max_db = fixed_db
        + PON_DROP_FUSION_COUNT * DOC_FUSION_DB.1
        + (doc_1x32_db + DOC_APPROX_DB_TOLERANCE);
    assert!(
        (min_db..=max_db).contains(&result.total_loss_db),
        "{} dB outside doc envelope {min_db}…{max_db} dB",
        result.total_loss_db
    );
}

#[test]
fn per_component_ledger_rows_sum_to_the_graph_total() {
    // The OTDR-style ledger UI shows one row per element; the rows must add
    // up to exactly what the graph walk reports.
    let chain = realistic_pon_drop();
    let graph = chain_graph(&chain);
    for wavelength in [Wavelength::Nm1310, Wavelength::Nm1490, Wavelength::Nm1550] {
        let ledger_db: f64 = chain.iter().map(|c| c.loss_db(wavelength)).sum();
        let result = gpon(&graph, wavelength).expect("drop is fully routed");
        assert_db_eq(
            result.total_loss_db,
            ledger_db,
            EXACT_DB_TOLERANCE,
            "ledger sum",
        );
    }
}

// ---- splices and connectors ----

#[test]
fn fusion_and_mechanical_splices_land_in_their_doc_ranges_end_to_end() {
    let cases = [
        (SpliceType::Fusion, DOC_FUSION_DB, SIM_FUSION_DB),
        (SpliceType::Mechanical, DOC_MECHANICAL_DB, SIM_MECHANICAL_DB),
    ];
    for (kind, (doc_min_db, doc_max_db), sim_db) in cases {
        // A zero-length span on each side isolates the splice in a real path.
        let graph = chain_graph(&[
            span(0.0, PlantType::Buried),
            splice(kind),
            span(0.0, PlantType::Buried),
        ]);
        let loss_db = gpon(&graph, Wavelength::Nm1490)
            .expect("routed")
            .total_loss_db;
        assert_db_eq(loss_db, sim_db, EXACT_DB_TOLERANCE, "splice loss");
        assert!(
            (doc_min_db..=doc_max_db).contains(&loss_db),
            "{kind:?}: {loss_db} dB"
        );
    }
}

#[test]
fn swapping_ten_fusion_splices_for_mechanical_costs_the_doc_delta() {
    // Ten splices along a long-haul route: the mechanical shortcut costs
    // 10 × (0.4 − 0.075) = 3.25 dB, between the doc extremes 2.0 and 4.5.
    let route = |kind| -> PathGraph {
        let mut chain = Vec::new();
        for _ in 0..10 {
            chain.push(span(2.0, PlantType::Buried));
            chain.push(splice(kind));
        }
        chain_graph(&chain)
    };
    let fusion_db = gpon(&route(SpliceType::Fusion), Wavelength::Nm1550).expect("routed");
    let mech_db = gpon(&route(SpliceType::Mechanical), Wavelength::Nm1550).expect("routed");
    let penalty_db = mech_db.total_loss_db - fusion_db.total_loss_db;
    assert_db_eq(penalty_db, 3.25, EXACT_DB_TOLERANCE, "mechanical penalty");
    let doc_min_db = 10.0 * (DOC_MECHANICAL_DB.0 - DOC_FUSION_DB.1);
    let doc_max_db = 10.0 * (DOC_MECHANICAL_DB.1 - DOC_FUSION_DB.0);
    assert!((doc_min_db..=doc_max_db).contains(&penalty_db));
}

#[test]
fn upc_and_apc_patch_pairs_match_doc_values() {
    let pair = |kind| chain_graph(&[connector(kind), connector(kind)]);
    let upc = gpon(&pair(ConnectorType::Upc), Wavelength::Nm1490).expect("routed");
    let apc = gpon(&pair(ConnectorType::Apc), Wavelength::Nm1490).expect("routed");

    assert_db_eq(
        upc.total_loss_db,
        2.0 * SIM_UPC_DB,
        EXACT_DB_TOLERANCE,
        "UPC pair",
    );
    assert!((2.0 * DOC_UPC_DB.0..=2.0 * DOC_UPC_DB.1).contains(&upc.total_loss_db));
    assert_db_eq(
        apc.total_loss_db,
        2.0 * DOC_APC_DB,
        EXACT_DB_TOLERANCE,
        "APC pair",
    );
    // APC: no worse insertion loss and 10 dB better return loss.
    assert!(apc.total_loss_db <= upc.total_loss_db);
    assert_db_eq(
        ConnectorType::Upc.typical_return_loss_db() - ConnectorType::Apc.typical_return_loss_db(),
        10.0,
        EXACT_DB_TOLERANCE,
        "return-loss gap",
    );
}

#[test]
fn dirty_connector_hazard_inside_doc_range_pushes_a_marginal_drop_out() {
    // The doc's dirty-connector hazard adds +2…+5 dB. On the 5.539 dB-margin
    // drop, +5 dB still passes and the worst real-world dirt (+6) fails.
    let with_dirt = |contamination_db| {
        let mut chain = realistic_pon_drop();
        let last = chain.len() - 1;
        chain[last] = Component::Connector {
            kind: ConnectorType::Apc,
            contamination_db,
        };
        gpon(&chain_graph(&chain), Wavelength::Nm1490).expect("routed")
    };
    let clean = with_dirt(0.0);
    for dirt_db in [2.0, 5.0] {
        let dirty = with_dirt(dirt_db);
        assert_db_eq(
            dirty.received_dbm,
            clean.received_dbm - dirt_db,
            EXACT_DB_TOLERANCE,
            "rx",
        );
        assert!(
            dirty.in_window,
            "+{dirt_db} dB still inside 5.539 dB margin"
        );
    }
    assert!(!with_dirt(6.0).in_window);
}

// ---- PON split ratios ----

#[test]
fn every_splitter_ratio_matches_the_doc_table_through_a_feeder() {
    for (ratio, branches, doc_db) in DOC_SPLITTERS {
        assert_eq!(ratio.branch_count(), branches);
        let graph = chain_graph(&[span(1.0, PlantType::Buried), splitter(ratio)]);
        let result = gpon(&graph, Wavelength::Nm1310).expect("routed");
        let splitter_db = result.total_loss_db - DOC_SMF_1310_DB_PER_KM;
        assert_db_eq(
            splitter_db,
            doc_db,
            DOC_APPROX_DB_TOLERANCE,
            &format!("{ratio:?}"),
        );
    }
}

#[test]
fn each_doubling_of_split_ratio_costs_three_to_four_db() {
    // Ideal power division is 10·log10(2) ≈ 3.01 dB per doubling; excess
    // loss keeps every step at or under 4.0 dB (doc table: 3.7 3.3 3.0 4.0).
    const STEP_DB_MIN: f64 = 3.0;
    const STEP_DB_MAX: f64 = 4.0;
    for pair in DOC_SPLITTERS.windows(2) {
        let (smaller, larger) = (pair[0].0, pair[1].0);
        let step_db = larger.insertion_loss_db() - smaller.insertion_loss_db();
        assert!(
            (STEP_DB_MIN - EXACT_DB_TOLERANCE..=STEP_DB_MAX + EXACT_DB_TOLERANCE)
                .contains(&step_db),
            "{smaller:?}→{larger:?}: {step_db} dB"
        );
    }
}

#[test]
fn cascaded_1x4_then_1x8_costs_at_least_a_single_1x32() {
    // Same 32-way fan-out, but two PLC stages pay excess loss twice:
    // 7.3 + 10.6 = 17.9 dB vs 17.7 dB for one 1×32.
    let cascade = chain_graph(&[
        splitter(SplitterRatio::OneByFour),
        splitter(SplitterRatio::OneByEight),
    ]);
    let single = chain_graph(&[splitter(SplitterRatio::OneByThirtyTwo)]);
    let cascade_db = gpon(&cascade, Wavelength::Nm1490)
        .expect("routed")
        .total_loss_db;
    let single_db = gpon(&single, Wavelength::Nm1490)
        .expect("routed")
        .total_loss_db;
    assert_db_eq(cascade_db, 17.9, EXACT_DB_TOLERANCE, "cascade");
    assert!(cascade_db >= single_db);
}

#[test]
fn branching_pon_tree_delivers_a_distinct_budget_to_each_ont() {
    // OLT → 10 km feeder → FDH (1×4) → four drops of different length.
    const FDH_IN: NodeId = 1;
    const FDH_OUT: NodeId = 2;
    const FEEDER_KM: f64 = 10.0;
    let drops_km = [0.5, 1.0, 2.0, 4.0];
    let mut graph = PathGraph::default();
    graph.add_node(OLT, "OLT");
    graph.connect(OLT, FDH_IN, span(FEEDER_KM, PlantType::Buried));
    graph.connect(FDH_IN, FDH_OUT, splitter(SplitterRatio::OneByFour));
    for (ont, drop_km) in (10u32..).zip(drops_km) {
        graph.add_node(ont, "ONT");
        graph.connect(FDH_OUT, ont, span(drop_km, PlantType::Aerial));
    }
    for (ont, drop_km) in (10u32..).zip(drops_km) {
        let result = graph
            .compute_link_budget(
                OLT,
                ont,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
            )
            .expect("every ONT hangs off the FDH");
        let expected_db = (FEEDER_KM + drop_km) * SIM_1490_DB_PER_KM + 7.3;
        assert_db_eq(
            result.total_loss_db,
            expected_db,
            EXACT_DB_TOLERANCE,
            "ONT loss",
        );
        assert_eq!(result.hop_count, 3);
    }
}

// ---- wavelength-dependent attenuation ----

#[test]
fn longer_wavelengths_lose_less_over_a_40km_route() {
    const ROUTE_KM: f64 = 40.0;
    let graph = chain_graph(&[span(ROUTE_KM, PlantType::Buried)]);
    let loss = |wavelength| gpon(&graph, wavelength).expect("routed").total_loss_db;
    let (o_band_db, gpon_db, c_band_db) = (
        loss(Wavelength::Nm1310),
        loss(Wavelength::Nm1490),
        loss(Wavelength::Nm1550),
    );

    assert!(o_band_db > gpon_db && gpon_db > c_band_db);
    assert_db_eq(
        o_band_db,
        ROUTE_KM * DOC_SMF_1310_DB_PER_KM,
        EXACT_DB_TOLERANCE,
        "1310",
    );
    assert_db_eq(o_band_db, 14.0, EXACT_DB_TOLERANCE, "1310 literal");
    assert_db_eq(gpon_db, 11.2, EXACT_DB_TOLERANCE, "1490 literal");
    assert_db_eq(c_band_db, 8.4, EXACT_DB_TOLERANCE, "1550 literal");
    assert!(c_band_db <= ROUTE_KM * DOC_SMF_1550_DB_PER_KM_MAX);
}

#[test]
fn wavelength_choice_alone_decides_whether_a_long_link_closes() {
    // 60 km into a 1×8: "wavelength choice matters" per the doc's span row.
    let graph = chain_graph(&[
        span(60.0, PlantType::Buried),
        splitter(SplitterRatio::OneByEight),
    ]);
    let o_band = gpon(&graph, Wavelength::Nm1310).expect("routed");
    let c_band = gpon(&graph, Wavelength::Nm1550).expect("routed");
    // 1310: 3 − (21.0 + 10.6) = −28.6 dBm, under the −27 floor.
    assert_db_eq(o_band.received_dbm, -28.6, EXACT_DB_TOLERANCE, "1310 rx");
    assert!(!o_band.in_window);
    // 1550: 3 − (12.6 + 10.6) = −20.2 dBm, comfortably inside.
    assert_db_eq(c_band.received_dbm, -20.2, EXACT_DB_TOLERANCE, "1550 rx");
    assert!(c_band.in_window);
}

// ---- receive window ----

#[test]
fn healthy_too_hot_and_too_dim_receives_classify_correctly() {
    let window = ReceiveWindow::GPON_ONT;
    let healthy = gpon(&chain_graph(&realistic_pon_drop()), Wavelength::Nm1490).expect("routed");
    // Patch-cord-only link: 3 − 0.6 = 2.4 dBm saturates the ONT (> −8).
    let too_hot = gpon(
        &chain_graph(&[connector(ConnectorType::Apc), connector(ConnectorType::Apc)]),
        Wavelength::Nm1490,
    )
    .expect("routed");
    // Two cascaded 1×32s over 20 km: 3 − (35.4 + 5.6) = −38 dBm (< −27).
    let too_dim = gpon(
        &chain_graph(&[
            span(20.0, PlantType::Buried),
            splitter(SplitterRatio::OneByThirtyTwo),
            splitter(SplitterRatio::OneByThirtyTwo),
        ]),
        Wavelength::Nm1490,
    )
    .expect("routed");

    assert!(healthy.in_window && healthy.margin_db > 0.0);
    assert!(!too_hot.in_window);
    assert_db_eq(too_hot.received_dbm, 2.4, EXACT_DB_TOLERANCE, "hot rx");
    assert_db_eq(
        too_hot.margin_db,
        window.max_dbm - 2.4,
        EXACT_DB_TOLERANCE,
        "hot",
    );
    assert!(!too_dim.in_window);
    assert_db_eq(too_dim.received_dbm, -38.0, EXACT_DB_TOLERANCE, "dim rx");
    assert_db_eq(too_dim.margin_db, -11.0, EXACT_DB_TOLERANCE, "dim margin");
}

#[test]
fn receive_level_exactly_on_the_gpon_floor_still_passes() {
    // 3 dBm launch, 30 dB of loss: 1×32 (17.7) + 12.3 dB of 1310 fiber
    // lands exactly on −27 dBm — inclusive edge, zero margin.
    let fiber_km = (30.0 - SIM_1X32_DB) / DOC_SMF_1310_DB_PER_KM;
    let graph = chain_graph(&[
        span(fiber_km, PlantType::Buried),
        splitter(SplitterRatio::OneByThirtyTwo),
    ]);
    let result = gpon(&graph, Wavelength::Nm1310).expect("routed");
    assert_db_eq(result.received_dbm, -27.0, EXACT_DB_TOLERANCE, "floor");
    assert_db_eq(result.margin_db, 0.0, EXACT_DB_TOLERANCE, "zero margin");
}

// ---- outage + alarm, end to end ----

/// Mirror of the game's `board::rebuild_live_graph` contract: an unresolved
/// full-cut outage removes its edge; everything else stays connected.
fn live_graph(plant: &PathGraph, outage: &Outage) -> PathGraph {
    let mut live = plant.clone();
    if !outage.resolved && outage.kind.is_full_cut() {
        live.edges
            .retain(|edge| !(edge.from == outage.edge_from && edge.to == outage.edge_to));
    }
    live
}

/// Storm Season shape: a 5 km aerial working path and a 6.5 km buried
/// protection path, both ending on a fusion splice into the ONT.
fn protected_route() -> PathGraph {
    let mut graph = PathGraph::default();
    for (id, label) in [(OLT, "OLT"), (1, "Aerial"), (2, "Buried"), (ONT, "ONT")] {
        graph.add_node(id, label);
    }
    graph.connect(OLT, 1, span(5.0, PlantType::Aerial));
    graph.connect(OLT, 2, span(6.5, PlantType::Buried));
    graph.connect(1, ONT, splitter(SplitterRatio::OneBySixteen));
    graph.connect(2, ONT, splitter(SplitterRatio::OneBySixteen));
    graph
}

#[test]
fn severing_the_only_span_drops_the_link_and_raises_a_critical_alarm() {
    let plant = chain_graph(&[
        span(10.0, PlantType::Buried),
        splitter(SplitterRatio::OneBySixteen),
    ]);
    let cut = Outage::new(OutageKind::FiberCut, OLT, 1);
    let mut alarm = Alarm::new(1, cut, 0.0);

    assert_eq!(
        gpon(&live_graph(&plant, &alarm.outage), Wavelength::Nm1490),
        Err(PathError::Disconnected)
    );
    assert_eq!(alarm.severity, AlarmSeverity::Critical);
    assert_eq!((alarm.outage.edge_from, alarm.outage.edge_to), (OLT, 1));
    assert_eq!(alarm.ack, AlarmAck::New);

    // Dispatch, burn some timer, repair: link back, alarm resolves/clears.
    alarm.acknowledge(0, 4.0);
    alarm.outage.tick(30.0);
    alarm.outage.resolved = true;
    alarm.sync_from_outage();
    let restored = gpon(&live_graph(&plant, &alarm.outage), Wavelength::Nm1490).expect("spliced");
    // 10 km × 0.28 + 13.7 = 16.5 dB → −13.5 dBm.
    assert_db_eq(
        restored.received_dbm,
        -13.5,
        EXACT_DB_TOLERANCE,
        "restored rx",
    );
    assert!(restored.in_window);
    assert_eq!(alarm.ack, AlarmAck::Resolved);
    assert_eq!(alarm.response_time_secs(), Some(4.0));
    alarm.clear();
    assert_eq!(alarm.ack, AlarmAck::Cleared);
}

#[test]
fn aerial_cut_fails_over_to_the_protection_path_with_its_own_budget() {
    let plant = protected_route();
    let working = gpon(&plant, Wavelength::Nm1490).expect("working path up");
    // Working path wins DFS (inserted first): 5 × 0.28 + 13.7 = 15.1 dB.
    assert_db_eq(working.total_loss_db, 15.1, EXACT_DB_TOLERANCE, "working");

    let storm = Outage::new(OutageKind::AerialDamage, OLT, 1);
    let alarm = Alarm::new(7, storm, 120.0);
    assert_eq!(alarm.severity, AlarmSeverity::Critical);

    let protected = gpon(&live_graph(&plant, &alarm.outage), Wavelength::Nm1490)
        .expect("protection path survives the storm");
    // 6.5 × 0.28 + 13.7 = 15.52 dB.
    assert_db_eq(
        protected.total_loss_db,
        15.52,
        EXACT_DB_TOLERANCE,
        "protection",
    );
    assert!(protected.in_window);
}

#[test]
fn water_intrusion_walks_a_healthy_drop_out_of_window_as_the_alarm_escalates() {
    // Drop has 5.539 dB margin; water adds 1 dB per 10 s, so the link
    // fails after 55.39 s while the alarm goes Warning → Minor → Major.
    let plant = chain_graph(&realistic_pon_drop());
    let mut alarm = Alarm::new(3, Outage::new(OutageKind::WaterIntrusion, 4, 5), 0.0);
    let at = |alarm: &Alarm| {
        plant
            .compute_link_budget_with_outage(
                OLT,
                ONT,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
                Some(&alarm.outage),
            )
            .expect("degrade hazards keep the edge connected")
    };

    // (elapsed s, severity, extra dB, still in window)
    let timeline = [
        (0.0, AlarmSeverity::Warning, 0.0, true),
        (36.0, AlarmSeverity::Minor, 3.6, true),
        (55.0, AlarmSeverity::Minor, 5.5, true),
        (60.0, AlarmSeverity::Minor, 6.0, false),
        (78.0, AlarmSeverity::Major, 7.8, false),
        (500.0, AlarmSeverity::Major, 15.0, false),
    ];
    for (elapsed_s, severity, extra_db, in_window) in timeline {
        alarm.outage.tick(elapsed_s - alarm.outage.elapsed_seconds);
        alarm.sync_from_outage();
        let result = at(&alarm);
        assert_eq!(alarm.severity, severity, "t={elapsed_s}s");
        assert_db_eq(
            result.received_dbm,
            -21.461 - extra_db,
            EXACT_DB_TOLERANCE,
            "rx",
        );
        assert_eq!(result.in_window, in_window, "t={elapsed_s}s");
    }
    assert!(
        alarm.outage.is_expired(),
        "500 s is past the 120 s complaint timer"
    );
}
