//! TIA-598 splice-tray scoring for sp1 ("True to the Color").
//!
//! A fiber's identity is the (tube, strand) color pair; its number is
//! `(tube_number - 1) * 12 + strand_number` over the 12-color TIA-598
//! sequence, so a strand color alone is only unique inside the first
//! tube (fiber 2 is blue tube / orange strand; fiber 13 is orange tube
//! / blue strand — the same two colors, transposed).
//!
//! Scoring is honest about the physics: a mis-splice never "corrupts
//! data", it sends light to the WRONG drop — the drop the fiber was
//! actually spliced to lights, and the drop that ordered that fiber
//! stays dark. [`evaluate_beat`] turns a player's splices for one beat
//! into per-drop outcomes plus a ledger that names the swapped legs.
//! Damaged strands are never forced: the designated spare is spliced
//! in their place, and splicing a damaged strand is a fault.
//!
//! Everything here is pure data-in/data-out over the authored
//! [`SpliceWorkOrdersDef`] so the tray console
//! (`crate::states::splice_console`) and the test suite drive the
//! exact same rules.

use crate::level::{SpliceBeatDef, SpliceFiberDef, SpliceWorkOrdersDef};

/// Strands inside one buffer tube (TIA-598 sequence length).
pub const FIBERS_PER_TUBE: u32 = 12;
/// Buffer tubes in the full chart.
pub const TUBE_COUNT: u32 = 12;
/// Fibers in the full chart (12 tubes x 12 strands).
pub const FIBER_COUNT: u32 = FIBERS_PER_TUBE * TUBE_COUNT;
/// A beat whose work touches at least this many distinct tubes
/// presents the whole tray (the authored "full fan-out" beat).
pub const FULL_FANOUT_TUBE_THRESHOLD: usize = 8;
/// Most splices scored for one beat: one per chart fiber.
pub const MAX_ATTEMPTS_PER_BEAT: usize = FIBER_COUNT as usize;
/// Most lines in one evaluation ledger (drops + faults + framing).
pub const MAX_LEDGER_LINES: usize = 64;

/// The fiber number for a 1-based (tube, strand) pair, or `None`
/// when either coordinate is outside 1..=12.
pub fn fiber_number(tube_number: u32, strand_number: u32) -> Option<u32> {
    if !(1..=TUBE_COUNT).contains(&tube_number) || !(1..=FIBERS_PER_TUBE).contains(&strand_number) {
        return None;
    }
    Some((tube_number - 1) * FIBERS_PER_TUBE + strand_number)
}

/// The 1-based (tube, strand) pair for a fiber number, or `None`
/// when the number is outside 1..=144.
pub fn fiber_pair(fiber: u32) -> Option<(u32, u32)> {
    if !(1..=FIBER_COUNT).contains(&fiber) {
        return None;
    }
    let zero_based = fiber - 1;
    Some((
        zero_based / FIBERS_PER_TUBE + 1,
        zero_based % FIBERS_PER_TUBE + 1,
    ))
}

/// The authored chart entry for a fiber number. The chart is stored
/// in fiber-number order; the entry is only trusted after its own
/// `fiber` field agrees with its position.
pub fn chart_entry(orders: &SpliceWorkOrdersDef, fiber: u32) -> Option<&SpliceFiberDef> {
    let index = usize::try_from(fiber.checked_sub(1)?).ok()?;
    let entry = orders.chart_144.get(index)?;
    (entry.fiber == fiber).then_some(entry)
}

/// "fiber 13 (orange tube, blue strand)" — the naming the ledger
/// uses, so a swapped pair reads as two full identities, never as
/// bare numbers.
pub fn describe_fiber(orders: &SpliceWorkOrdersDef, fiber: u32) -> String {
    match chart_entry(orders, fiber) {
        Some(entry) => format!(
            "fiber {} ({} tube, {} strand)",
            entry.fiber, entry.tube_color, entry.strand_color
        ),
        None => format!("fiber {fiber} (not on the chart)"),
    }
}

/// The fiber a drop must actually be lit by: the ordered fiber, or —
/// when that fiber is one of the beat's damaged strands — its
/// designated spare. Returns `(expected, Some(damaged))` on a
/// substitution, `(ordered, None)` otherwise.
pub fn expected_fiber(beat: &SpliceBeatDef, ordered_fiber: u32) -> (u32, Option<u32>) {
    for damage in &beat.damage_beats {
        if damage.fiber == ordered_fiber {
            return (damage.spare.fiber, Some(ordered_fiber));
        }
    }
    (ordered_fiber, None)
}

/// The tubes the tray presents for a beat: the tubes its work
/// touches, or every tube when the beat spans most of the chart
/// (the authored full fan-out). Sorted, deduplicated.
pub fn presented_tubes(_orders: &SpliceWorkOrdersDef, beat: &SpliceBeatDef) -> Vec<u32> {
    let mut tubes: Vec<u32> = Vec::new();
    let mut touch = |tube: u32| {
        if (1..=TUBE_COUNT).contains(&tube) && !tubes.contains(&tube) {
            tubes.push(tube);
        }
    };
    for entry in &beat.work_order {
        touch(entry.tube_number);
    }
    for damage in &beat.damage_beats {
        touch(damage.tube_number);
        touch(damage.spare.tube_number);
    }
    tubes.sort_unstable();
    if tubes.len() >= FULL_FANOUT_TUBE_THRESHOLD {
        return (1..=TUBE_COUNT).collect();
    }
    tubes
}

/// One rendered chart line: "Fiber 13: orange tube x blue strand".
pub fn chart_line(entry: &SpliceFiberDef) -> String {
    format!(
        "Fiber {}: {} tube x {} strand",
        entry.fiber, entry.tube_color, entry.strand_color
    )
}

/// The full reference chart, one line per fiber, in fiber-number
/// order — what the in-game panel shows (paged by tube).
pub fn chart_lines(orders: &SpliceWorkOrdersDef) -> Vec<String> {
    orders.chart_144.iter().map(chart_line).collect()
}

/// One tube's page of the reference chart (12 lines).
pub fn chart_page(orders: &SpliceWorkOrdersDef, tube_number: u32) -> Vec<String> {
    orders
        .chart_144
        .iter()
        .filter(|entry| entry.tube_number == tube_number)
        .map(chart_line)
        .collect()
}

/// One splice the player made: this incoming fiber joined to the
/// pigtail feeding `drop`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpliceAttempt {
    pub fiber: u32,
    pub drop: String,
}

/// How one ordered drop came out of a beat evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropStatus {
    /// Lit by its ordered fiber (or, on a damage substitution, by
    /// the designated spare — `via_spare_for` names the damaged
    /// fiber it replaced).
    Correct { via_spare_for: Option<u32> },
    /// Nothing was spliced to this drop; it stays dark.
    Dark,
    /// A different fiber was spliced to this drop: this drop lights
    /// with light that was ordered elsewhere — the wrong drop lit.
    WrongFiber { actual: u32 },
}

/// One drop's ledger row: the outcome plus its rendered line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropLine {
    pub drop: String,
    pub ordered_fiber: u32,
    pub status: DropStatus,
    pub text: String,
}

/// The scored result of one beat: per-drop rows, the extra faults
/// (forced damage, unordered splices, duplicates), and the full
/// rendered ledger. `passed` is the conjunction the win gate reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatEvaluation {
    pub passed: bool,
    pub drop_lines: Vec<DropLine>,
    pub faults: Vec<String>,
    pub ledger: Vec<String>,
}

/// Score one beat against the player's splices. Total over
/// player-shaped input: duplicates, unknown fibers, and splices to
/// unordered drops become ledger faults, never panics. The first
/// splice of a fiber and the first feed of a drop are the ones that
/// physically exist; later duplicates are reported and ignored.
pub fn evaluate_beat(
    orders: &SpliceWorkOrdersDef,
    beat: &SpliceBeatDef,
    attempts: &[SpliceAttempt],
) -> BeatEvaluation {
    let mut faults: Vec<String> = Vec::new();
    let mut kept: Vec<&SpliceAttempt> = Vec::new();
    for attempt in attempts.iter().take(MAX_ATTEMPTS_PER_BEAT) {
        if chart_entry(orders, attempt.fiber).is_none() {
            faults.push(format!(
                "{} spliced to {} — that fiber is not on the TIA-598 chart.",
                describe_fiber(orders, attempt.fiber),
                attempt.drop
            ));
            continue;
        }
        if kept.iter().any(|k| k.fiber == attempt.fiber) {
            faults.push(format!(
                "{} spliced twice — a strand has one home; the second splice is ignored.",
                describe_fiber(orders, attempt.fiber)
            ));
            continue;
        }
        if kept.iter().any(|k| k.drop == attempt.drop) {
            faults.push(format!(
                "{} fed twice — a pigtail takes one fiber; the second feed is ignored.",
                attempt.drop
            ));
            continue;
        }
        kept.push(attempt);
    }
    if attempts.len() > MAX_ATTEMPTS_PER_BEAT {
        faults.push(format!(
            "{} splices offered, more than the {} fibers on the chart — the excess is ignored.",
            attempts.len(),
            FIBER_COUNT
        ));
    }

    // A damaged strand the player tried to splice was forced,
    // whatever it fed — even when the tray rejected the splice
    // itself (duplicate fiber, fed pigtail): the attempt is the
    // violation the ledger teaches against. One fault per damaged
    // fiber, however many times it was tried.
    let mut forced_reported: Vec<u32> = Vec::new();
    for attempt in attempts.iter().take(MAX_ATTEMPTS_PER_BEAT) {
        if forced_reported.contains(&attempt.fiber) {
            continue;
        }
        if let Some(damage) = beat.damage_beats.iter().find(|d| d.fiber == attempt.fiber) {
            forced_reported.push(attempt.fiber);
            faults.push(format!(
                "{} is damaged ({}) and was spliced anyway — damaged strands are excluded, never forced. The work order designates {} in its place.",
                describe_fiber(orders, attempt.fiber),
                damage.condition,
                describe_fiber(orders, damage.spare.fiber)
            ));
        }
    }

    let mut drop_lines: Vec<DropLine> = Vec::new();
    for entry in &beat.work_order {
        let (expected, via_spare_for) = expected_fiber(beat, entry.fiber);
        let fed_by = kept.iter().find(|k| k.drop == entry.drop).map(|k| k.fiber);
        let (status, text) = match fed_by {
            Some(actual) if actual == expected => (
                DropStatus::Correct { via_spare_for },
                match via_spare_for {
                    Some(damaged) => format!(
                        "{} lit by {} — the designated spare, spliced in place of damaged {}. True.",
                        entry.drop,
                        describe_fiber(orders, actual),
                        describe_fiber(orders, damaged)
                    ),
                    None => format!(
                        "{} lit by {}. True.",
                        entry.drop,
                        describe_fiber(orders, actual)
                    ),
                },
            ),
            Some(actual) => (
                DropStatus::WrongFiber { actual },
                format!(
                    "{} lights from {} — WRONG DROP: that light was ordered for {}, and {} was ordered {}.",
                    entry.drop,
                    describe_fiber(orders, actual),
                    ordered_drop_of(beat, actual)
                        .unwrap_or_else(|| "no drop on this work order".to_string()),
                    entry.drop,
                    describe_fiber(orders, expected)
                ),
            ),
            None => (
                DropStatus::Dark,
                match kept.iter().find(|k| k.fiber == expected) {
                    Some(stray) => format!(
                        "{} dark — its ordered {} was spliced to {} instead.",
                        entry.drop,
                        describe_fiber(orders, expected),
                        stray.drop
                    ),
                    None => format!(
                        "{} dark — its ordered {} was never spliced.",
                        entry.drop,
                        describe_fiber(orders, expected)
                    ),
                },
            ),
        };
        drop_lines.push(DropLine {
            drop: entry.drop.clone(),
            ordered_fiber: entry.fiber,
            status,
            text,
        });
    }

    // Name transposed pairs explicitly: two ordered fibers each
    // spliced to the other's drop are swapped legs, and the ledger
    // says so in one line naming both.
    let mut swap_lines: Vec<String> = Vec::new();
    for (i, a) in beat.work_order.iter().enumerate() {
        for b in beat.work_order.iter().skip(i + 1) {
            let (expected_a, _) = expected_fiber(beat, a.fiber);
            let (expected_b, _) = expected_fiber(beat, b.fiber);
            let a_feeds_b = kept
                .iter()
                .any(|k| k.fiber == expected_a && k.drop == b.drop);
            let b_feeds_a = kept
                .iter()
                .any(|k| k.fiber == expected_b && k.drop == a.drop);
            if a_feeds_b && b_feeds_a {
                swap_lines.push(format!(
                    "Swapped legs: {} and {} are transposed between {} and {} — each lights the other's drop.",
                    describe_fiber(orders, expected_a),
                    describe_fiber(orders, expected_b),
                    a.drop,
                    b.drop
                ));
            }
        }
    }

    // Splices feeding drops the work order never named.
    for attempt in &kept {
        if !beat.work_order.iter().any(|e| e.drop == attempt.drop) {
            faults.push(format!(
                "{} spliced to {} — no such drop on this work order; {} lights with no order behind it.",
                describe_fiber(orders, attempt.fiber),
                attempt.drop,
                attempt.drop
            ));
        }
    }

    let passed = drop_lines
        .iter()
        .all(|line| matches!(line.status, DropStatus::Correct { .. }))
        && faults.is_empty()
        && swap_lines.is_empty();

    let mut ledger: Vec<String> = Vec::new();
    ledger.push(format!(
        "Beat {} — {}: {}",
        beat.beat,
        beat.name,
        if passed {
            "every drop lit true."
        } else {
            "not true — the tray reads:"
        }
    ));
    ledger.extend(drop_lines.iter().map(|line| line.text.clone()));
    ledger.extend(swap_lines.iter().cloned());
    ledger.extend(faults.iter().cloned());
    if passed && !beat.damage_beats.is_empty() {
        let excluded: Vec<String> = beat
            .damage_beats
            .iter()
            .map(|d| describe_fiber(orders, d.fiber))
            .collect();
        ledger.push(format!(
            "Damaged strands excluded and reported, never forced: {}.",
            excluded.join("; ")
        ));
    }
    ledger.truncate(MAX_LEDGER_LINES);

    BeatEvaluation {
        passed,
        drop_lines,
        faults,
        ledger,
    }
}

/// The drop a fiber was ordered for in this beat, if any.
fn ordered_drop_of(beat: &SpliceBeatDef, fiber: u32) -> Option<String> {
    beat.work_order
        .iter()
        .find(|e| e.fiber == fiber)
        .map(|e| e.drop.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::load_scenario;

    fn sp1_orders() -> SpliceWorkOrdersDef {
        load_scenario("sp1")
            .expect("sp1 scenario must load")
            .splice_work_orders
            .expect("sp1 must carry splice work orders")
    }

    #[test]
    fn fiber_number_follows_the_tia598_formula() {
        assert_eq!(fiber_number(1, 1), Some(1));
        assert_eq!(fiber_number(1, 2), Some(2));
        assert_eq!(fiber_number(2, 1), Some(13));
        assert_eq!(fiber_number(5, 7), Some(55));
        assert_eq!(fiber_number(12, 12), Some(144));
        // Fiber 2 and fiber 13 carry the same two colors transposed:
        // (blue, orange) vs (orange, blue) — different pairs, and the
        // formula keeps them apart.
        assert_ne!(fiber_number(1, 2), fiber_number(2, 1));
    }

    #[test]
    fn fiber_number_rejects_out_of_range_coordinates() {
        assert_eq!(fiber_number(0, 1), None);
        assert_eq!(fiber_number(1, 0), None);
        assert_eq!(fiber_number(13, 1), None);
        assert_eq!(fiber_number(1, 13), None);
        assert_eq!(fiber_number(u32::MAX, u32::MAX), None);
    }

    #[test]
    fn fiber_pair_inverts_fiber_number_across_the_chart() {
        for fiber in 1..=FIBER_COUNT {
            let (tube, strand) = fiber_pair(fiber).expect("in-range fiber has a pair");
            assert_eq!(fiber_number(tube, strand), Some(fiber));
        }
        assert_eq!(fiber_pair(2), Some((1, 2)));
        assert_eq!(fiber_pair(13), Some((2, 1)));
        assert_eq!(fiber_pair(0), None);
        assert_eq!(fiber_pair(145), None);
    }

    #[test]
    fn chart_lookup_agrees_with_the_formula_on_real_data() {
        let orders = sp1_orders();
        assert_eq!(orders.chart_144.len(), FIBER_COUNT as usize);
        for entry in &orders.chart_144 {
            assert_eq!(
                fiber_number(entry.tube_number, entry.strand_number),
                Some(entry.fiber),
                "chart entry must satisfy the formula: {entry:?}"
            );
            assert_eq!(
                chart_entry(&orders, entry.fiber).map(|e| e.fiber),
                Some(entry.fiber)
            );
        }
        assert_eq!(chart_entry(&orders, 0), None);
        assert_eq!(chart_entry(&orders, 145), None);
    }
}
