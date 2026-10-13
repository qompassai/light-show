//! Integration coverage for sp1's TIA-598 splice work orders: the
//! scoring engine in `light_show::splice` driven against the shipped
//! scenario data (`load_scenario("sp1")`), plus adversarial attempt
//! shapes the tray must fault instead of choking on.
//!
//! The headline acceptance: in beat 2, a deliberate transposition of
//! fiber 2 (blue tube, orange strand) and fiber 13 (orange tube,
//! blue strand) — the same two colors swapped — lights the wrong
//! drops and fails, with the ledger naming both swapped legs.

use light_show::level::{
    load_scenario, SpliceBeatDef, SpliceDamageBeatDef, SpliceFiberDef, SpliceWorkOrderEntryDef,
    SpliceWorkOrdersDef, LEVEL_SOURCES, SCENARIO_SOURCES,
};
use light_show::splice::{
    chart_entry, chart_lines, chart_page, describe_fiber, evaluate_beat, expected_fiber,
    fiber_number, fiber_pair, presented_tubes, DropStatus, SpliceAttempt, FIBERS_PER_TUBE,
    FIBER_COUNT, TUBE_COUNT,
};

fn sp1_orders() -> SpliceWorkOrdersDef {
    load_scenario("sp1")
        .expect("sp1 scenario must load")
        .splice_work_orders
        .expect("sp1 must carry splice work orders")
}

/// Every ordered entry of `beat` spliced exactly true.
fn true_attempts(beat: &SpliceBeatDef) -> Vec<SpliceAttempt> {
    beat.work_order
        .iter()
        .map(|entry| SpliceAttempt {
            fiber: expected_fiber(beat, entry.fiber).0,
            drop: entry.drop.clone(),
        })
        .collect()
}

#[test]
fn the_level_count_stays_frozen() {
    // The splice level was built inside sp1's existing slot: 80
    // track levels + 3 Field School scenarios, no 84th file.
    assert_eq!(LEVEL_SOURCES.len(), 80, "track level count moved");
    assert_eq!(SCENARIO_SOURCES.len(), 3, "scenario count moved");
}

#[test]
fn fiber_identity_is_the_tube_strand_pair() {
    // Fiber 2 and fiber 13 carry the same two colors, transposed.
    assert_eq!(fiber_pair(2), Some((1, 2)));
    assert_eq!(fiber_pair(13), Some((2, 1)));
    assert_eq!(fiber_number(1, 2), Some(2));
    assert_eq!(fiber_number(2, 1), Some(13));
    let orders = sp1_orders();
    assert_eq!(
        describe_fiber(&orders, 2),
        "fiber 2 (blue tube, orange strand)"
    );
    assert_eq!(
        describe_fiber(&orders, 13),
        "fiber 13 (orange tube, blue strand)"
    );
    // Strand color alone repeats every tube: "blue strand" names 12
    // different fibers, one per tube.
    let blue_strands: Vec<u32> = (1..=TUBE_COUNT)
        .map(|tube| fiber_number(tube, 1).expect("strand 1 exists in every tube"))
        .collect();
    assert_eq!(blue_strands.len(), 12);
    assert!(blue_strands.windows(2).all(|w| w[0] != w[1]));
}

#[test]
fn the_reference_panel_shows_all_144_pairs() {
    let orders = sp1_orders();
    let lines = chart_lines(&orders);
    assert_eq!(
        lines.len(),
        FIBER_COUNT as usize,
        "panel must show every pair"
    );
    for (index, line) in lines.iter().enumerate() {
        let fiber = u32::try_from(index + 1).expect("index fits");
        let entry = chart_entry(&orders, fiber).expect("chart entry");
        assert!(
            line.contains(&format!("Fiber {fiber}:")),
            "line {index} must be fiber {fiber}: {line}"
        );
        assert!(
            line.contains(&entry.tube_color) && line.contains(&entry.strand_color),
            "line must name the pair: {line}"
        );
        // The number on the line is the formula's number.
        assert_eq!(
            fiber_number(entry.tube_number, entry.strand_number),
            Some(entry.fiber)
        );
    }
    // Paged by tube: 12 pages of 12, and page t holds exactly tube t.
    for tube in 1..=TUBE_COUNT {
        let page = chart_page(&orders, tube);
        assert_eq!(page.len(), FIBERS_PER_TUBE as usize);
        for line in &page {
            let fiber_number_shown: u32 = line
                .trim_start_matches("Fiber ")
                .split(':')
                .next()
                .and_then(|n| n.parse().ok())
                .expect("line carries a fiber number");
            assert_eq!(fiber_pair(fiber_number_shown).map(|(t, _)| t), Some(tube));
        }
    }
    assert!(chart_page(&orders, 0).is_empty());
    assert!(chart_page(&orders, 13).is_empty());
}

#[test]
fn the_ramp_presents_one_tube_then_three_then_the_whole_tray() {
    let orders = sp1_orders();
    assert_eq!(presented_tubes(&orders, &orders.beats[0]), vec![1]);
    assert_eq!(presented_tubes(&orders, &orders.beats[1]), vec![1, 2, 3]);
    assert_eq!(
        presented_tubes(&orders, &orders.beats[2]),
        (1..=TUBE_COUNT).collect::<Vec<u32>>()
    );
}

#[test]
fn beat_1_single_tube_matching_passes_when_true() {
    let orders = sp1_orders();
    let beat = &orders.beats[0];
    let evaluation = evaluate_beat(&orders, beat, &true_attempts(beat));
    assert!(
        evaluation.passed,
        "ledger:\n{}",
        evaluation.ledger.join("\n")
    );
    assert!(evaluation
        .drop_lines
        .iter()
        .all(|line| matches!(line.status, DropStatus::Correct { .. })));
}

#[test]
fn the_fiber_2_fiber_13_transposition_lights_the_wrong_drops_and_fails() {
    let orders = sp1_orders();
    let beat = &orders.beats[1];
    // The acceptance error: fiber 2 and fiber 13 spliced to each
    // other's drops, everything else true.
    let mut attempts = true_attempts(beat);
    for attempt in &mut attempts {
        attempt.fiber = match attempt.fiber {
            2 => 13,
            13 => 2,
            other => other,
        };
    }
    let evaluation = evaluate_beat(&orders, beat, &attempts);
    assert!(!evaluation.passed, "a transposition must fail the beat");
    let line_for = |drop: &str| {
        evaluation
            .drop_lines
            .iter()
            .find(|line| line.drop == drop)
            .unwrap_or_else(|| panic!("{drop} must have a ledger row"))
    };
    // Each transposed drop lights — from the wrong fiber. Light went
    // somewhere real; it just went to the wrong premises.
    assert_eq!(
        line_for("Drop 1").status,
        DropStatus::WrongFiber { actual: 13 },
        "Drop 1 must be lit by fiber 13"
    );
    assert_eq!(
        line_for("Drop 3").status,
        DropStatus::WrongFiber { actual: 2 },
        "Drop 3 must be lit by fiber 2"
    );
    // Every untouched drop is still correct.
    for line in &evaluation.drop_lines {
        if line.drop != "Drop 1" && line.drop != "Drop 3" {
            assert!(
                matches!(line.status, DropStatus::Correct { .. }),
                "{} must stay correct: {}",
                line.drop,
                line.text
            );
        }
    }
    let ledger = evaluation.ledger.join("\n");
    assert!(
        ledger.contains("Swapped legs")
            && ledger.contains("fiber 2 (blue tube, orange strand)")
            && ledger.contains("fiber 13 (orange tube, blue strand)"),
        "the ledger must name the swapped legs:\n{ledger}"
    );
    // Honest scoring vocabulary: a mis-splice is a wrong drop lit,
    // never corrupted data.
    assert!(
        !ledger.to_lowercase().contains("corrupt"),
        "the ledger must not call a mis-splice corruption:\n{ledger}"
    );
}

#[test]
fn a_single_wrong_splice_leaves_its_ordered_drop_dark() {
    let orders = sp1_orders();
    let beat = &orders.beats[1];
    // Fiber 26 (ordered for Drop 5) spliced to Drop 6 instead;
    // fiber 30 (Drop 6's own) never spliced.
    let mut attempts = true_attempts(beat);
    attempts.retain(|a| a.fiber != 30);
    for attempt in &mut attempts {
        if attempt.fiber == 26 {
            attempt.drop = "Drop 6".to_string();
        }
    }
    let evaluation = evaluate_beat(&orders, beat, &attempts);
    assert!(!evaluation.passed);
    let line_for = |drop: &str| {
        evaluation
            .drop_lines
            .iter()
            .find(|line| line.drop == drop)
            .expect("ledger row")
    };
    assert_eq!(
        line_for("Drop 6").status,
        DropStatus::WrongFiber { actual: 26 },
        "Drop 6 lights from the wrong fiber"
    );
    assert_eq!(
        line_for("Drop 5").status,
        DropStatus::Dark,
        "Drop 5's fiber went elsewhere, so Drop 5 stays dark"
    );
    assert!(
        line_for("Drop 5").text.contains("Drop 6"),
        "the dark row must say where its light went: {}",
        line_for("Drop 5").text
    );
}

#[test]
fn beat_3_numbers_only_passes_with_the_damaged_strands_excluded() {
    let orders = sp1_orders();
    let beat = &orders.beats[2];
    assert_eq!(
        beat.damage_beats.len(),
        2,
        "beat 3 authors two damaged strands"
    );
    let evaluation = evaluate_beat(&orders, beat, &true_attempts(beat));
    assert!(
        evaluation.passed,
        "ledger:\n{}",
        evaluation.ledger.join("\n")
    );
    let ledger = evaluation.ledger.join("\n");
    assert!(
        ledger.contains("fiber 55") && ledger.contains("fiber 118"),
        "the pass ledger must report the exclusions:\n{ledger}"
    );
}

#[test]
fn forcing_a_damaged_strand_fails_and_names_the_designated_spare() {
    let orders = sp1_orders();
    let beat = &orders.beats[2];
    let mut attempts = true_attempts(beat);
    attempts.push(SpliceAttempt {
        fiber: 118,
        drop: "Drop K".to_string(),
    });
    let evaluation = evaluate_beat(&orders, beat, &attempts);
    assert!(!evaluation.passed);
    let faults = evaluation.faults.join("\n");
    assert!(
        faults.contains("fiber 118") && faults.contains("fiber 120"),
        "the fault must name the damaged strand and its spare:\n{faults}"
    );
}

/// A beat where the damaged fiber IS the ordered one: the spare is
/// the only true splice. Built from the shipped def types because
/// the authored beats keep their damage on un-ordered tray fibers.
fn substitution_beat() -> SpliceBeatDef {
    let fiber = |n: u32, tube: u32, strand: u32, tcol: &str, scol: &str| SpliceFiberDef {
        fiber: n,
        tube_number: tube,
        tube_color: tcol.to_string(),
        strand_number: strand,
        strand_color: scol.to_string(),
    };
    SpliceBeatDef {
        beat: 9,
        name: "Substitution drill".to_string(),
        teaching: String::new(),
        presented: String::new(),
        work_order: vec![SpliceWorkOrderEntryDef {
            fiber: 55,
            tube_number: 5,
            tube_color: "slate".to_string(),
            strand_number: 7,
            strand_color: "red".to_string(),
            drop: "Drop S".to_string(),
        }],
        damage_beats: vec![SpliceDamageBeatDef {
            fiber: 55,
            tube_number: 5,
            tube_color: "slate".to_string(),
            strand_number: 7,
            strand_color: "red".to_string(),
            condition: "crushed at the tray edge".to_string(),
            action: "Exclude it. Splice the designated spare instead: fiber 60.".to_string(),
            spare: fiber(60, 5, 12, "slate", "aqua"),
        }],
        win_rule: String::new(),
    }
}

#[test]
fn the_designated_spare_lights_the_drop_for_a_damaged_ordered_fiber() {
    let orders = sp1_orders();
    let beat = substitution_beat();
    // Splicing the spare is the true play, and the row says so.
    let evaluation = evaluate_beat(
        &orders,
        &beat,
        &[SpliceAttempt {
            fiber: 60,
            drop: "Drop S".to_string(),
        }],
    );
    assert!(
        evaluation.passed,
        "ledger:\n{}",
        evaluation.ledger.join("\n")
    );
    assert_eq!(
        evaluation.drop_lines[0].status,
        DropStatus::Correct {
            via_spare_for: Some(55)
        }
    );
    // Splicing the damaged fiber itself is forcing it, not repairing.
    let forced = evaluate_beat(
        &orders,
        &beat,
        &[SpliceAttempt {
            fiber: 55,
            drop: "Drop S".to_string(),
        }],
    );
    assert!(!forced.passed);
    assert!(
        forced.faults.iter().any(|f| f.contains("never forced")),
        "forcing must be faulted: {:?}",
        forced.faults
    );
    // The spare spliced to the wrong place lights nothing true.
    let stray = evaluate_beat(
        &orders,
        &beat,
        &[SpliceAttempt {
            fiber: 60,
            drop: "Drop Elsewhere".to_string(),
        }],
    );
    assert!(!stray.passed);
    assert_eq!(stray.drop_lines[0].status, DropStatus::Dark);
}

#[test]
fn hostile_attempt_shapes_are_faults_never_panics() {
    let orders = sp1_orders();
    let beat = &orders.beats[0];
    // Nothing spliced: every drop dark, no pass.
    let empty = evaluate_beat(&orders, beat, &[]);
    assert!(!empty.passed);
    assert!(empty
        .drop_lines
        .iter()
        .all(|line| line.status == DropStatus::Dark));
    // Duplicates, unknown fibers, unordered drops: all faulted,
    // none fatal, and the true splices underneath still score.
    let mut attempts = true_attempts(beat);
    attempts.push(SpliceAttempt {
        fiber: 1,
        drop: "B".to_string(),
    });
    attempts.push(SpliceAttempt {
        fiber: 999,
        drop: "A".to_string(),
    });
    attempts.push(SpliceAttempt {
        fiber: 0,
        drop: "A".to_string(),
    });
    attempts.push(SpliceAttempt {
        fiber: 30,
        drop: "Nowhere".to_string(),
    });
    let evaluation = evaluate_beat(&orders, beat, &attempts);
    assert!(!evaluation.passed, "faults must hold the pass back");
    assert!(
        evaluation.faults.len() >= 4,
        "every hostile shape is its own fault: {:?}",
        evaluation.faults
    );
    assert!(
        evaluation
            .drop_lines
            .iter()
            .all(|line| matches!(line.status, DropStatus::Correct { .. })),
        "the true splices under the noise still score correct"
    );
    assert!(
        evaluation.ledger.len() <= light_show::splice::MAX_LEDGER_LINES,
        "the ledger stays bounded"
    );
}
