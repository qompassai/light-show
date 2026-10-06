//! Adversarial end-to-end tests for `osp_sim`.
//!
//! Attack surface: every public constructor and evaluation entry point fed
//! non-finite, negative, zero, and huge values; hostile topologies; and
//! malformed JSON for every serde type a level or save file can carry.
//!
//! Contract under test: nothing panics, structural faults return a typed
//! error (`PathError`, `serde_json::Error`), and numeric garbage fails
//! closed — a corrupt value may never put a link *into* its window.
//!
//! The component constructors are plain enum literals with no validating
//! constructor, so numeric garbage cannot yield a typed error today; these
//! tests check the fail-closed half instead. Five defects found here were
//! documented as `#[ignore = "BUG: …"]` tests asserting the correct
//! contract; all five are fixed and the ignores removed, so the suite now
//! asserts the fixed behavior directly.

use osp_sim::alarm::{Alarm, AlarmAck, AlarmSeverity};
use osp_sim::component::{
    CableCategory, Component, ConnectorType, PlantType, SpliceType, SplitterRatio,
};
use osp_sim::graph::{EthernetViolation, NodeId, PathError, PathGraph};
use osp_sim::outage::{Outage, OutageKind};
use osp_sim::wavelength::Wavelength;
use osp_sim::{LinkBudgetResult, ReceiveWindow, DEFAULT_TX_DBM};

const OLT: NodeId = 0;
const ONT: NodeId = 99;
/// Values no level file should ever carry.
const NON_FINITE: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
/// TIA-568 copper segment limit.
const ETHERNET_SEGMENT_M_MAX: f64 = 100.0;
const GIGABIT_MBPS: u64 = 1_000;
/// Chain depth comfortably inside any sane level (bundled levels have < 10).
const CHAIN_DEPTH_OK: u32 = 1_000;
/// Chain depth a hostile level file could carry; the recursive DFS must
/// not turn this into a stack overflow (process abort, not a panic).
const CHAIN_DEPTH_HOSTILE: u32 = 200_000;

fn span(length_km: f64) -> Component {
    Component::Span {
        length_km,
        plant: PlantType::Buried,
    }
}

/// Healthy GPON drop with a slot for one extra component in the middle:
/// OLT → 20 km → [extra] → 1×32 → ONT. Without `extra` it receives
/// 3 − (5.6 + 17.7) = −20.3 dBm, inside the GPON window.
fn drop_with(extra: Component) -> PathGraph {
    let mut graph = PathGraph::default();
    graph.connect(OLT, 1, span(20.0));
    graph.connect(1, 2, extra);
    graph.connect(
        2,
        ONT,
        Component::Splitter {
            ratio: SplitterRatio::OneByThirtyTwo,
        },
    );
    graph
}

fn gpon(graph: &PathGraph) -> Result<LinkBudgetResult, PathError> {
    graph.compute_link_budget(
        OLT,
        ONT,
        DEFAULT_TX_DBM,
        Wavelength::Nm1490,
        ReceiveWindow::GPON_ONT,
    )
}

fn assert_fails_closed(result: &LinkBudgetResult, what: &str) {
    assert!(
        !result.in_window,
        "{what}: corrupt input landed in window: {result:?}"
    );
    assert!(
        result.margin_db.is_nan() || result.margin_db < 0.0,
        "{what}: corrupt input reported positive margin: {result:?}"
    );
}

// ---- non-finite launch power and windows ----

#[test]
fn non_finite_launch_power_fails_closed_on_every_budget_entry_point() {
    let graph = drop_with(span(0.0));
    let water = Outage::new(OutageKind::WaterIntrusion, 1, 2);
    for tx_dbm in NON_FINITE {
        let plain = graph
            .compute_link_budget(
                OLT,
                ONT,
                tx_dbm,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
            )
            .expect("routing is independent of power");
        assert_fails_closed(&plain, &format!("tx={tx_dbm}"));
        let degraded = graph
            .compute_link_budget_with_outage(
                OLT,
                ONT,
                tx_dbm,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
                Some(&water),
            )
            .expect("routing is independent of power");
        assert_fails_closed(&degraded, &format!("tx={tx_dbm} + outage"));
    }
}

#[test]
fn nan_window_bounds_reject_a_healthy_link() {
    let graph = drop_with(span(0.0));
    for window in [
        ReceiveWindow {
            min_dbm: f64::NAN,
            max_dbm: -8.0,
        },
        ReceiveWindow {
            min_dbm: -27.0,
            max_dbm: f64::NAN,
        },
        ReceiveWindow {
            min_dbm: f64::NAN,
            max_dbm: f64::NAN,
        },
    ] {
        let result = graph
            .compute_link_budget(OLT, ONT, DEFAULT_TX_DBM, Wavelength::Nm1490, window)
            .expect("routed");
        assert!(!result.in_window, "{window:?} accepted −20.3 dBm");
    }
}

#[test]
fn zero_tx_power_is_a_valid_launch_not_an_error() {
    let result = drop_with(span(0.0))
        .compute_link_budget(OLT, ONT, 0.0, Wavelength::Nm1490, ReceiveWindow::GPON_ONT)
        .expect("0 dBm = 1 mW is a legal launch");
    assert!((result.received_dbm - -23.3).abs() < 1e-9);
    assert!(result.in_window);
}

// ---- non-finite / negative / zero component parameters ----

#[test]
fn non_finite_loss_and_gain_parameters_fail_closed_end_to_end() {
    for bad in NON_FINITE {
        let components = [
            Component::Splice {
                kind: SpliceType::Fusion,
                degradation_db: bad,
            },
            Component::Connector {
                kind: ConnectorType::Apc,
                contamination_db: bad,
            },
            Component::Macrobend {
                excess_loss_db: bad,
            },
            Component::Amplifier { gain_db: bad },
            Component::Tap { tap_loss_db: bad },
            Component::Repeater { tx_dbm: bad },
        ];
        for component in components {
            let what = format!("{component:?}");
            let result = gpon(&drop_with(component)).expect("routing ignores values");
            assert_fails_closed(&result, &what);
        }
    }
}

#[test]
fn positive_infinite_lengths_fail_closed() {
    let hop = Component::WirelessHop {
        distance_m: f64::INFINITY,
        frequency_mhz: 2400.0,
    };
    for component in [
        span(f64::INFINITY),
        Component::CoaxSpan {
            length_m: f64::INFINITY,
        },
        hop,
    ] {
        let what = format!("{component:?}");
        assert_fails_closed(&gpon(&drop_with(component)).expect("routed"), &what);
    }
}

#[test]
fn huge_finite_lengths_saturate_to_a_dead_link_without_panicking() {
    for component in [span(f64::MAX), Component::CoaxSpan { length_m: f64::MAX }] {
        let what = format!("{component:?}");
        let result = gpon(&drop_with(component)).expect("routed");
        assert_fails_closed(&result, &what);
        assert!(result.received_dbm < ReceiveWindow::GPON_ONT.min_dbm);
    }
}

#[test]
fn zero_valued_parameters_are_legal_and_add_only_nominal_loss() {
    // Zero-length plant and zero hazards: only nominal splice/connector
    // loss remains (0.075 fusion, 0.30 APC).
    let mut graph = PathGraph::default();
    graph.connect(OLT, 1, span(0.0));
    graph.connect(
        1,
        2,
        Component::Splice {
            kind: SpliceType::Fusion,
            degradation_db: 0.0,
        },
    );
    graph.connect(
        2,
        3,
        Component::Connector {
            kind: ConnectorType::Apc,
            contamination_db: 0.0,
        },
    );
    graph.connect(
        3,
        4,
        Component::Macrobend {
            excess_loss_db: 0.0,
        },
    );
    graph.connect(4, 5, Component::Tap { tap_loss_db: 0.0 });
    graph.connect(5, 6, Component::CoaxSpan { length_m: 0.0 });
    graph.connect(6, ONT, Component::Amplifier { gain_db: 0.0 });
    let result = gpon(&graph).expect("routed");
    assert!((result.total_loss_db - 0.375).abs() < 1e-9, "{result:?}");
}

#[test]
fn nan_lengths_and_frequencies_fail_closed() {
    let nan_hop_distance = Component::WirelessHop {
        distance_m: f64::NAN,
        frequency_mhz: 2400.0,
    };
    let nan_hop_frequency = Component::WirelessHop {
        distance_m: 100.0,
        frequency_mhz: f64::NAN,
    };
    for component in [
        span(f64::NAN),
        Component::CoaxSpan { length_m: f64::NAN },
        nan_hop_distance,
        nan_hop_frequency,
    ] {
        let what = format!("{component:?}");
        assert_fails_closed(&gpon(&drop_with(component)).expect("routed"), &what);
    }
}

#[test]
fn negative_passive_losses_never_add_power() {
    // Baseline drop is −20.3 dBm; a passive element must never raise it.
    const BASELINE_RX_DBM: f64 = -20.3;
    let components = [
        Component::Splice {
            kind: SpliceType::Mechanical,
            degradation_db: -10.0,
        },
        Component::Connector {
            kind: ConnectorType::Upc,
            contamination_db: -10.0,
        },
        Component::Macrobend {
            excess_loss_db: -10.0,
        },
        Component::Tap { tap_loss_db: -10.0 },
    ];
    for component in components {
        let what = format!("{component:?}");
        let result = gpon(&drop_with(component)).expect("routed");
        assert!(
            result.received_dbm <= BASELINE_RX_DBM,
            "{what} added power: {result:?}"
        );
    }
}

// ---- hostile topologies ----

#[test]
fn empty_topology_is_disconnected_on_every_entry_point() {
    let empty = PathGraph::default();
    let cut = Outage::new(OutageKind::FiberCut, OLT, ONT);
    assert_eq!(gpon(&empty), Err(PathError::Disconnected));
    let with_outage = empty.compute_link_budget_with_outage(
        OLT,
        ONT,
        DEFAULT_TX_DBM,
        Wavelength::Nm1490,
        ReceiveWindow::GPON_ONT,
        Some(&cut),
    );
    assert_eq!(with_outage, Err(PathError::Disconnected));
    let ethernet = empty.evaluate_ethernet(OLT, ONT, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS);
    assert_eq!(ethernet, Err(PathError::Disconnected));
}

#[test]
fn single_node_graph_with_a_self_loop_cannot_reach_another_node() {
    let mut graph = PathGraph::default();
    graph.add_node(OLT, "OLT");
    graph.connect(OLT, OLT, span(1.0));
    assert_eq!(gpon(&graph), Err(PathError::Disconnected));
    // Source == target is a zero-hop link even with the loop present.
    let zero_hop = graph
        .compute_link_budget(
            OLT,
            OLT,
            DEFAULT_TX_DBM,
            Wavelength::Nm1490,
            ReceiveWindow::GPON_ONT,
        )
        .expect("zero-hop link");
    assert_eq!(zero_hop.hop_count, 0);
}

#[test]
fn ont_on_an_island_is_disconnected_from_the_olt() {
    // OLT feeds its own cluster; the ONT sits on a separate island whose
    // only link to the OLT side points the wrong way.
    let mut graph = PathGraph::default();
    graph.connect(OLT, 1, span(1.0));
    graph.connect(1, 2, span(1.0));
    graph.connect(50, ONT, span(1.0));
    graph.connect(ONT, 1, span(1.0)); // ONT → OLT side only
    assert_eq!(gpon(&graph), Err(PathError::Disconnected));
    let ethernet = graph.evaluate_ethernet(OLT, ONT, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS);
    assert_eq!(ethernet, Err(PathError::Disconnected));
}

#[test]
fn extreme_node_ids_route_without_overflow() {
    let mut graph = PathGraph::default();
    graph.connect(u32::MAX, 0, span(1.0));
    graph.connect(0, u32::MAX - 1, span(1.0));
    let result = graph
        .compute_link_budget(
            u32::MAX,
            u32::MAX - 1,
            DEFAULT_TX_DBM,
            Wavelength::Nm1310,
            ReceiveWindow::GPON_ONT,
        )
        .expect("ids are opaque labels");
    assert_eq!(result.hop_count, 2);
    assert!((result.total_loss_db - 0.7).abs() < 1e-9);
}

#[test]
fn parallel_edges_resolve_deterministically_to_the_first_inserted() {
    // DFS takes the first routed edge, not the cheapest: a lossy
    // mechanical splice inserted first wins over a later fusion splice.
    let mut graph = PathGraph::default();
    graph.connect(
        OLT,
        ONT,
        Component::Splice {
            kind: SpliceType::Mechanical,
            degradation_db: 0.0,
        },
    );
    graph.connect(
        OLT,
        ONT,
        Component::Splice {
            kind: SpliceType::Fusion,
            degradation_db: 0.0,
        },
    );
    for _ in 0..3 {
        let result = gpon(&graph).expect("routed");
        assert!((result.total_loss_db - 0.4).abs() < 1e-9, "{result:?}");
    }
}

#[test]
fn thousand_hop_chain_routes_and_sums_exactly() {
    let mut graph = PathGraph::default();
    for hop in 0..CHAIN_DEPTH_OK {
        graph.connect(hop, hop + 1, span(0.01));
    }
    let result = graph
        .compute_link_budget(
            0,
            CHAIN_DEPTH_OK,
            DEFAULT_TX_DBM,
            Wavelength::Nm1310,
            ReceiveWindow::GPON_ONT,
        )
        .expect("chain is connected");
    assert_eq!(result.hop_count, 1_000);
    // 10 km of 1310 fiber in 1 000 pieces: 3.5 dB, float drift < 1e-9.
    assert!((result.total_loss_db - 3.5).abs() < 1e-9, "{result:?}");
}

#[test]
fn hostile_deep_chain_does_not_overflow_the_stack() {
    let mut graph = PathGraph::default();
    for hop in 0..CHAIN_DEPTH_HOSTILE {
        graph.connect(hop, hop + 1, span(0.0));
    }
    let outcome = graph.compute_link_budget(
        0,
        CHAIN_DEPTH_HOSTILE,
        DEFAULT_TX_DBM,
        Wavelength::Nm1490,
        ReceiveWindow::GPON_ONT,
    );
    // Either a typed refusal or a correct result is acceptable; an abort is not.
    if let Ok(result) = outcome {
        assert_eq!(result.hop_count, 200_000);
    }
}

// ---- Ethernet evaluator ----

#[test]
fn ethernet_nan_inputs_never_pass() {
    let mut over_long = PathGraph::default();
    over_long.connect(
        OLT,
        ONT,
        Component::EthernetRun {
            length_m: 130.0,
            category: CableCategory::Cat6,
        },
    );
    let nan_limit = over_long
        .evaluate_ethernet(OLT, ONT, f64::NAN, 0.0, GIGABIT_MBPS)
        .expect("routed");
    assert!(!nan_limit.passes(), "NaN segment limit passed a 130 m run");

    let mut powered = PathGraph::default();
    powered.connect(
        OLT,
        1,
        Component::EthernetRun {
            length_m: 10.0,
            category: CableCategory::Cat6,
        },
    );
    powered.connect(1, ONT, Component::Switch { poe_budget_w: 15.0 });
    let nan_draw = powered
        .evaluate_ethernet(OLT, ONT, ETHERNET_SEGMENT_M_MAX, f64::NAN, GIGABIT_MBPS)
        .expect("routed");
    assert!(!nan_draw.passes(), "NaN PoE draw passed");

    let mut nan_switch = PathGraph::default();
    nan_switch.connect(
        OLT,
        ONT,
        Component::Switch {
            poe_budget_w: f64::NAN,
        },
    );
    let nan_budget = nan_switch
        .evaluate_ethernet(OLT, ONT, ETHERNET_SEGMENT_M_MAX, 25.0, GIGABIT_MBPS)
        .expect("routed");
    assert!(
        !nan_budget.passes(),
        "NaN switch budget covered a 25 W draw"
    );

    let mut nan_run = PathGraph::default();
    nan_run.connect(
        OLT,
        ONT,
        Component::EthernetRun {
            length_m: f64::NAN,
            category: CableCategory::Cat6,
        },
    );
    let nan_length = nan_run
        .evaluate_ethernet(OLT, ONT, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS)
        .expect("routed");
    assert!(
        nan_length
            .violations
            .iter()
            .any(|v| matches!(v, EthernetViolation::NonFiniteInput { .. })),
        "NaN run length was not flagged: {nan_length:?}"
    );
    assert!(!nan_length.passes(), "NaN run length passed");
}

#[test]
fn ethernet_infinite_inputs_fail_closed() {
    let mut graph = PathGraph::default();
    graph.connect(
        OLT,
        1,
        Component::EthernetRun {
            length_m: f64::INFINITY,
            category: CableCategory::Cat6,
        },
    );
    graph.connect(1, ONT, Component::Switch { poe_budget_w: 15.0 });
    let eval = graph
        .evaluate_ethernet(
            OLT,
            ONT,
            ETHERNET_SEGMENT_M_MAX,
            f64::INFINITY,
            GIGABIT_MBPS,
        )
        .expect("routed");
    assert!(!eval.passes());
    assert_eq!(eval.violations.len(), 2, "{:?}", eval.violations);
}

// ---- outage timers and alarm clocks ----

#[test]
fn outage_survives_zero_subnormal_and_maximal_ticks() {
    let mut water = Outage::new(OutageKind::WaterIntrusion, 1, 2);
    water.tick(0.0);
    water.tick(-0.0);
    assert_eq!(water.elapsed_seconds, 0.0);
    // 10 000 subnormal ticks: bounded, monotonic, still nowhere near expiry.
    for _ in 0..10_000 {
        water.tick(f64::MIN_POSITIVE);
    }
    assert!(water.elapsed_seconds > 0.0 && !water.is_expired());
    // Two f64::MAX ticks overflow elapsed to +inf: must clamp, not panic.
    water.tick(f64::MAX);
    water.tick(f64::MAX);
    assert_eq!(water.time_remaining(), 0.0);
    assert!(water.is_expired());
    assert_eq!(water.accumulated_extra_loss_db(), 15.0);
    let mut alarm = Alarm::new(1, water, 0.0);
    alarm.sync_from_outage();
    assert_eq!(alarm.severity, AlarmSeverity::Major);
}

#[test]
fn alarm_clocks_survive_non_finite_timestamps() {
    for raised_s in NON_FINITE {
        for acked_s in NON_FINITE {
            let mut alarm = Alarm::new(9, Outage::new(OutageKind::Macrobend, 1, 2), raised_s);
            alarm.acknowledge(0, acked_s);
            let response_s = alarm.response_time_secs().expect("acknowledged");
            // f64::max drops a NaN operand, so the clamp yields ≥ 0, never NaN.
            assert!(response_s >= 0.0, "{raised_s}/{acked_s}: {response_s}");
            assert_eq!(alarm.ack, AlarmAck::Acknowledged { companion_idx: 0 });
        }
    }
}

#[test]
fn deserialized_negative_elapsed_cannot_rescue_a_dim_link() {
    // A dim link (−27.86 dBm, under the −27 floor) plus a forged
    // water-intrusion outage at elapsed −50 s (= −5 dB "loss").
    let forged: Outage = serde_json::from_str(
        r#"{"kind":"WaterIntrusion","edge_from":1,"edge_to":2,
            "elapsed_seconds":-50.0,"resolved":false}"#,
    )
    .expect("well-formed JSON");
    let graph = drop_with(span(27.0)); // +7.56 dB on top of the −20.3 baseline
    let result = graph
        .compute_link_budget_with_outage(
            OLT,
            ONT,
            DEFAULT_TX_DBM,
            Wavelength::Nm1490,
            ReceiveWindow::GPON_ONT,
            Some(&forged),
        )
        .expect("routed");
    assert!(
        !result.in_window,
        "forged outage rescued a dead link: {result:?}"
    );
}

// ---- malformed JSON into serde types ----

const VALID_GRAPH_JSON: &str = r#"{
    "nodes": [{"id": 0, "label": "OLT"}, {"id": 99, "label": "ONT"}],
    "edges": [
        {"from": 0, "to": 1, "component": {"Span": {"length_km": 20.0, "plant": "Buried"}}},
        {"from": 1, "to": 99, "component": {"Splitter": {"ratio": "OneByThirtyTwo"}}}
    ]
}"#;

#[test]
fn valid_graph_json_parses_and_evaluates_as_the_control_case() {
    let graph: PathGraph = serde_json::from_str(VALID_GRAPH_JSON).expect("control JSON is valid");
    let result = gpon(&graph).expect("routed");
    assert!((result.received_dbm - -20.3).abs() < 1e-9, "{result:?}");
}

#[test]
fn splitter_ratios_outside_the_catalog_are_rejected() {
    // The catalog stops at 1×32; 1×64 and non-power-of-two ratios do not exist.
    for raw in [
        "\"OneBySixtyFour\"",
        "\"OneByThree\"",
        "\"OneByOne\"",
        "\"onebytwo\"",
        "64",
        "32",
        "null",
        "\"\"",
    ] {
        let parsed: Result<SplitterRatio, _> = serde_json::from_str(raw);
        assert!(parsed.is_err(), "{raw} parsed as {parsed:?}");
    }
}

#[test]
fn malformed_graph_json_returns_typed_errors() {
    let cases = [
        (
            "unknown component",
            VALID_GRAPH_JSON.replace("\"Splitter\"", "\"Laser\""),
        ),
        (
            "1x64 splitter",
            VALID_GRAPH_JSON.replace("OneByThirtyTwo", "OneBySixtyFour"),
        ),
        (
            "unknown plant",
            VALID_GRAPH_JSON.replace("\"Buried\"", "\"Orbital\""),
        ),
        (
            "string length",
            VALID_GRAPH_JSON.replace("20.0", "\"20.0\""),
        ),
        ("NaN token", VALID_GRAPH_JSON.replace("20.0", "NaN")),
        (
            "Infinity token",
            VALID_GRAPH_JSON.replace("20.0", "Infinity"),
        ),
        (
            "overflow literal",
            VALID_GRAPH_JSON.replace("20.0", "1e999"),
        ),
        (
            "negative node id",
            VALID_GRAPH_JSON.replace("\"to\": 99", "\"to\": -1"),
        ),
        (
            "node id > u32",
            VALID_GRAPH_JSON.replace("\"to\": 99", "\"to\": 4294967296"),
        ),
        (
            "missing plant",
            VALID_GRAPH_JSON.replace(", \"plant\": \"Buried\"", ""),
        ),
        (
            "truncated",
            VALID_GRAPH_JSON[..VALID_GRAPH_JSON.len() / 2].to_string(),
        ),
        ("empty", String::new()),
        ("deep nesting", "[".repeat(100_000)),
        ("binary junk", "\u{0}\u{fffd}\u{1b}[31m".to_string()),
    ];
    for (what, raw) in cases {
        let parsed: Result<PathGraph, serde_json::Error> = serde_json::from_str(&raw);
        assert!(parsed.is_err(), "{what}: accepted malformed graph");
    }
}

#[test]
fn malformed_outage_and_alarm_json_return_typed_errors() {
    let outages = [
        r#"{"kind":"Tornado","edge_from":1,"edge_to":2,"elapsed_seconds":0.0,"resolved":false}"#,
        r#"{"kind":"FiberCut","edge_from":1,"edge_to":2,"elapsed_seconds":"soon","resolved":false}"#,
        r#"{"kind":"FiberCut","edge_from":1,"edge_to":2,"elapsed_seconds":0.0,"resolved":"no"}"#,
        r#"{"kind":"FiberCut","edge_from":1,"edge_to":2}"#,
        r#"{"kind":"FiberCut","edge_from":-1,"edge_to":2,"elapsed_seconds":0.0,"resolved":false}"#,
    ];
    for raw in outages {
        let parsed: Result<Outage, serde_json::Error> = serde_json::from_str(raw);
        assert!(parsed.is_err(), "accepted outage {raw}");
    }
    for raw in ["\"Catastrophic\"", "\"critical\"", "0", "null"] {
        let parsed: Result<AlarmSeverity, serde_json::Error> = serde_json::from_str(raw);
        assert!(parsed.is_err(), "accepted severity {raw}");
    }
    // companion_idx is a u8: 256 and −1 must not wrap into a valid roster slot.
    for raw in [
        r#"{"Acknowledged":{"companion_idx":256}}"#,
        r#"{"Acknowledged":{"companion_idx":-1}}"#,
        r#"{"Acknowledged":{}}"#,
    ] {
        let parsed: Result<AlarmAck, serde_json::Error> = serde_json::from_str(raw);
        assert!(parsed.is_err(), "accepted ack {raw}");
    }
}
