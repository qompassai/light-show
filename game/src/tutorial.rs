//! Tutorial sequencer: staged first-level teaching for every
//! companion, authored in `tutorial_segments.json` (the authoring
//! source) and compiled into Rust as `reactions::tutorial_spec`
//! (companion, stage) -> (bank key, [`Emotion`]) plus the segment
//! wiring in this module.
//!
//! Delivery discipline: the sequencer owns a staged queue and feeds
//! [`PillInbox`] only as capacity allows — the inbox caps at
//! [`PILL_INBOX_CAPACITY`] and drops oldest, so a whole briefing
//! block must never be dumped into it at once. Multi-line bank keys
//! emit EVERY line in order as its own pill line at the segment's
//! emotion; the sequencer deliberately does not use
//! `DialogueBank::random_line`, which returns line 1 only. A missing
//! bank key is a silent no-op (the banks' existing semantics).
//!
//! Trigger definitions (the segments file names events the game did
//! not all have; each watcher below documents its derivation):
//! * `first_component_placed` — the existing `FirstPlacement`
//!   detection (first frame a placement exists), watched here.
//! * `first_completed_path_eval` — first completed-route verdict
//!   (any of InWindow / TooLow / TooHot) per attempt, computed in
//!   the verdict block below for the Fiber/Coax/Wireless media.
//! * `OutageStart` segments — the existing scripted-outage path
//!   (`playing::check_scripted_outage` fills `ActiveOutage`): the
//!   segment fires at w4l1 (Séraphine), c1l2 (Ondine), m1l2 (Linka),
//!   and clara2 (Clara, whose segment names it
//!   `first_outage_or_audit`).
//! * Clara `first_profile_assignment` = first component placement
//!   in a provisioning level; `first_subscriber_verification` =
//!   first `is_provisioning_win`-true evaluation (observed through
//!   the public `is_win_state`, which dispatches to it).
//! * Aino `first_acknowledgment` / `first_triage_resolution` derive
//!   from `TriageProgress` (first ack / `is_complete`).
//!   `first_cascade` is DEFINED as staging at aino3 entry: that
//!   level's cascade is `alarm_triage` data with
//!   `scripted_outage: null`, so no outage event exists to attach to.
//! * Lattice `first_midlevel_failure` is DEFINED as the first
//!   completed-route `evaluate_ethernet` result carrying violations
//!   (Ethernet has no verdict block and no scripted outage).
//! * Hikari `first_dead_reading` is DEFINED as the first TooLow
//!   verdict on a completed route.
//! * Léa `first_answer_submitted` / `first_question_scored` derive
//!   from `QuizProgress::answer()` state (first recorded answer —
//!   `answer()` both records and scores, so both segments stage at
//!   that moment, in order). SUBSTITUTION, documented per the task:
//!   `first_confident_wrong_answer` names a confidence input the
//!   game does not have (`answer()` takes only a choice index), so
//!   it is mapped to the first WRONG answer.
//!
//! Scenario segments (fj/sp): on entering fj1/fj2/sp1 the survey /
//! chart beats stage as briefing-adjacent pill lines; on their
//! Results entry the completion beats stage (win vs loss). The
//! levels themselves land in a parallel pass — this wiring is by
//! level id, so it works the moment those levels exist. The fj/sp
//! emotions are authored defaults chosen here (the segments file
//! carries no fj/sp emotion mapping): survey/chart lines Neutral,
//! proof/wrap lines Determined, warnings Worried, traps Surprised.
//! `fj_partial_warning` presupposes a partial-repair signal the win
//! check (a single boolean) does not produce; it is staged on a
//! scenario LOSS at Results, the closest honest signal available.
//!
//! Seen/skip: the briefing block stages only when the companion's
//! stem is absent from `SaveData::tutorials_seen`. Results marks it
//! seen on her level-1 clear (see `states::results`). Séraphine's
//! `first_outage` is dual-attach per the segments file: it is ALSO
//! staged as the final briefing beat on w1l1, and fires live at
//! w4l1 regardless of the seen flag (it is not part of the briefing
//! block anywhere else).

use crate::board::PlacedChoices;
use crate::level::LevelDef;
use crate::save::SaveData;
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::quiz::QuizProgress;
use crate::states::triage_console::TriageProgress;
use crate::states::{GameState, LevelOutcome};
use crate::waifu::dialogue::DialogueBank;
use crate::waifu::pill::{PillInbox, PillSpeaker, PILL_INBOX_CAPACITY};
use crate::waifu::reactions::{tutorial_spec, Emotion, TutorialStage};
use crate::waifu::{Companion, SelectedCompanion};
use bevy::prelude::*;
use osp_sim::Medium;
use std::collections::{HashSet, VecDeque};

/// Most staged tutorial lines buffered at once. A full briefing is
/// ~9 lines; the bound covers briefing + an in-level burst with
/// headroom, and staging past it drops the OLDEST staged line
/// (speech is perishable, like the pill inbox itself).
pub const STAGED_QUEUE_CAPACITY: usize = 32;

/// Every companion, mains and specialists (the sequencer serves
/// all eight tutorial banks; `Companion::ALL` is mains only).
pub const ALL_COMPANIONS: [Companion; 8] = [
    Companion::Fiber,
    Companion::Coax,
    Companion::Mobile,
    Companion::Ethernet,
    Companion::Clara,
    Companion::Aino,
    Companion::Hikari,
    Companion::Lea,
];

/// The level-1 (track start) level id for a companion, per
/// `tutorial_segments.json`.
pub fn level_one_id(companion: Companion) -> &'static str {
    match companion {
        Companion::Fiber => "w1l1",
        Companion::Coax => "c1l1",
        Companion::Mobile => "m1l1",
        Companion::Ethernet => "e1l1",
        Companion::Clara => "clara1",
        Companion::Aino => "aino1",
        Companion::Hikari => "hikari1",
        Companion::Lea => "lea1",
    }
}

/// The companion whose level-1 `level_id` is, if it is one.
pub fn level_one_companion(level_id: &str) -> Option<Companion> {
    ALL_COMPANIONS
        .into_iter()
        .find(|c| level_one_id(*c) == level_id)
}

/// True when `level_id` is `companion`'s own level-1. Tutorial
/// segments never fire for the wrong companion: every watcher gates
/// on this (or on a named per-companion level below).
pub fn is_own_level_one(companion: Companion, level_id: &str) -> bool {
    level_one_companion(level_id) == Some(companion)
}

/// The level a companion's `first_outage` segment fires on via the
/// scripted-outage path, where it is not her level-1 (see module
/// docs). `None` for companions whose outage segment derives from a
/// level-1 watcher instead (Lattice, Hikari, Léa) or from aino3
/// entry (Aino, handled separately).
pub fn outage_segment_level(companion: Companion) -> Option<&'static str> {
    match companion {
        Companion::Fiber => Some("w4l1"),
        Companion::Coax => Some("c1l2"),
        Companion::Mobile => Some("m1l2"),
        Companion::Clara => Some("clara2"),
        Companion::Aino | Companion::Ethernet | Companion::Hikari | Companion::Lea => None,
    }
}

/// Briefing-block stages in position order (segments file positions
/// 1, 2, 3, 4 + the final wrap beat).
pub const BRIEFING_STAGES: [TutorialStage; 5] = [
    TutorialStage::BriefingObjective,
    TutorialStage::BriefingGestures,
    TutorialStage::BriefingBudgetMath,
    TutorialStage::ScoringAndCores,
    TutorialStage::Wrap,
];

/// Briefing-adjacent segments staged on scenario level entry.
pub fn scenario_entry_segments(level_id: &str) -> &'static [(&'static str, Emotion)] {
    match level_id {
        "fj1" => &[
            ("fj_survey_ports", Emotion::Neutral),
            ("fj_demarc_trap", Emotion::Surprised),
            ("fj_survey_vfl", Emotion::Determined),
        ],
        "fj2" => &[
            ("fj_survey_sb", Emotion::Neutral),
            ("fj_survey_vfl", Emotion::Determined),
        ],
        "sp1" => &[
            ("sp_chart_handover", Emotion::Neutral),
            ("sp_beat2_trap", Emotion::Surprised),
            ("sp_beat3_numbers", Emotion::Determined),
            ("sp_damaged_strand", Emotion::Worried),
        ],
        _ => &[],
    }
}

/// Completion-adjacent segments staged on scenario Results entry.
pub fn scenario_completion_segments(
    level_id: &str,
    won: bool,
) -> &'static [(&'static str, Emotion)] {
    match (level_id, won) {
        ("fj1", true) => &[("fj_swap_done", Emotion::Determined)],
        ("fj2", true) => &[
            ("fj_swap_done", Emotion::Determined),
            ("fj_coda_cat3", Emotion::Neutral),
        ],
        ("fj1", false) | ("fj2", false) => &[("fj_partial_warning", Emotion::Worried)],
        ("sp1", true) => &[
            ("sp_beat1_clear", Emotion::Happy),
            ("sp_wrap", Emotion::Determined),
        ],
        _ => &[],
    }
}

/// One staged tutorial line, resolved against a bank and awaiting
/// its turn in the pill inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedTutorialLine {
    pub speaker: PillSpeaker,
    pub text: String,
    pub emotion: Emotion,
}

/// The sequencer's per-attempt state plus its staged queue. Pure
/// staging/feeding logic lives here so it is unit-testable without
/// a Bevy app; the systems below are thin adapters over it.
#[derive(Resource, Default)]
pub struct TutorialSequencer {
    queue: VecDeque<StagedTutorialLine>,
    fired: HashSet<String>,
    active_companion: Option<Companion>,
    active_level_id: String,
    outage_seen: bool,
}

impl TutorialSequencer {
    /// Begin a fresh attempt on `level_id` for `companion`: staged
    /// lines from a previous level never leak, and every segment may
    /// fire once this attempt.
    pub fn reset_for_level(&mut self, companion: Companion, level_id: &str) {
        self.queue.clear();
        self.fired.clear();
        self.active_companion = Some(companion);
        self.active_level_id = level_id.to_string();
        self.outage_seen = false;
    }

    pub fn staged_len(&self) -> usize {
        self.queue.len()
    }

    pub fn active_level_id(&self) -> &str {
        &self.active_level_id
    }

    fn push_staged(&mut self, line: StagedTutorialLine) {
        if self.queue.len() >= STAGED_QUEUE_CAPACITY {
            self.queue.pop_front();
        }
        self.queue.push_back(line);
    }

    /// Stage every line of `key` in `bank`, in bank order, at
    /// `emotion`, under the one-shot segment id `segment_id`.
    /// Returns false (silent no-op) when the segment already fired
    /// this attempt or the bank has no such key.
    pub fn stage_key(
        &mut self,
        bank: &DialogueBank,
        companion: Companion,
        segment_id: &str,
        key: &str,
        emotion: Emotion,
    ) -> bool {
        if !self.fired.insert(segment_id.to_string()) {
            return false;
        }
        let Some(lines) = bank.lines.get(key) else {
            return false;
        };
        if lines.is_empty() {
            return false;
        }
        for text in lines {
            self.push_staged(StagedTutorialLine {
                speaker: PillSpeaker::Companion(companion),
                text: text.clone(),
                emotion,
            });
        }
        true
    }

    /// Stage one tutorial stage for `companion` via `tutorial_spec`.
    pub fn stage_segment(
        &mut self,
        bank: &DialogueBank,
        companion: Companion,
        stage: TutorialStage,
    ) -> bool {
        let (key, emotion) = tutorial_spec(companion, stage);
        let segment_id = format!("{}:{stage:?}", companion.picker_stem());
        self.stage_key(bank, companion, &segment_id, key, emotion)
    }

    /// Stage the briefing block in position order. Suppressed
    /// wholesale when the tutorial is marked seen. Séraphine's
    /// dual-attach `first_outage` stages as the final briefing beat
    /// (it also fires live at w4l1 — see module docs). Returns the
    /// number of lines staged.
    pub fn stage_briefing(
        &mut self,
        bank: &DialogueBank,
        companion: Companion,
        seen: bool,
    ) -> usize {
        if seen {
            return 0;
        }
        let before = self.queue.len();
        for stage in BRIEFING_STAGES {
            self.stage_segment(bank, companion, stage);
        }
        if companion == Companion::Fiber {
            self.stage_segment(bank, companion, TutorialStage::FirstOutage);
        }
        self.queue.len() - before
    }

    /// Stage a scenario segment list (fj/sp). Returns lines staged.
    pub fn stage_scenario(
        &mut self,
        bank: &DialogueBank,
        segments: &[(&'static str, Emotion)],
        phase: &str,
    ) -> usize {
        let before = self.queue.len();
        for (key, emotion) in segments {
            let segment_id = format!("scenario:{phase}:{key}");
            self.stage_key(bank, Companion::Fiber, &segment_id, key, *emotion);
        }
        self.queue.len() - before
    }

    /// Move staged lines into the pill inbox, oldest first, only as
    /// inbox capacity allows. Returns the number moved.
    pub fn feed(&mut self, inbox: &mut PillInbox) -> usize {
        let mut moved = 0;
        while inbox.len() < PILL_INBOX_CAPACITY {
            let Some(line) = self.queue.pop_front() else {
                break;
            };
            inbox.push_line_with_emotion(line.speaker, line.text, line.emotion);
            moved += 1;
        }
        moved
    }

    /// Peek at the staged queue (tests and diagnostics).
    pub fn staged(&self) -> impl Iterator<Item = &StagedTutorialLine> {
        self.queue.iter()
    }
}

// ---------------------------------------------------------------------------
// Plugin + systems
// ---------------------------------------------------------------------------

pub struct TutorialPlugin;

impl Plugin for TutorialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TutorialSequencer>()
            // Registered after PlayingPlugin in `build_app`, so this
            // OnEnter chain runs after `setup_level` inserted the
            // LevelDef (the same ordering the console setups rely on).
            .add_systems(OnEnter(GameState::Playing), stage_on_level_entry)
            .add_systems(OnEnter(GameState::Results), stage_scenario_completion)
            .add_systems(
                Update,
                watch_tutorial_events.run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            // Feeding runs in every state: briefing feeds during
            // Playing, scenario completion lines during Results.
            .add_systems(Update, feed_tutorial_lines);
    }
}

/// Level entry: reset the sequencer, stage the briefing block on a
/// companion's level-1 (unless seen), the aino3 cascade segment, and
/// scenario entry segments on fj1/fj2/sp1.
fn stage_on_level_entry(
    level: Option<Res<LevelDef>>,
    selected: Res<SelectedCompanion>,
    bank: Res<DialogueBank>,
    save: Option<Res<SaveData>>,
    mut seq: ResMut<TutorialSequencer>,
) {
    let Some(level) = level else {
        return;
    };
    let companion = selected.0;
    seq.reset_for_level(companion, &level.id);
    if !scenario_entry_segments(&level.id).is_empty() {
        // Scenario levels are Séraphine-hosted; under any other
        // companion the fj/sp keys are absent and staging no-ops.
        let segments = scenario_entry_segments(&level.id);
        seq.stage_scenario(&bank, segments, "entry");
        return;
    }
    // Aino's first_cascade definition: staged at aino3 entry (its
    // cascade is alarm data; there is no outage event — see docs).
    if level.id == "aino3" && companion == Companion::Aino {
        seq.stage_segment(&bank, companion, TutorialStage::FirstOutage);
        return;
    }
    if is_own_level_one(companion, &level.id) {
        let seen = save
            .as_deref()
            .is_some_and(|s| s.is_tutorial_seen(companion.picker_stem()));
        seq.stage_briefing(&bank, companion, seen);
    }
}

/// Results entry for a scenario level: stage its completion beats.
fn stage_scenario_completion(
    outcome: Res<LevelOutcome>,
    bank: Res<DialogueBank>,
    mut seq: ResMut<TutorialSequencer>,
) {
    let level_id = seq.active_level_id().to_string();
    let segments = scenario_completion_segments(&level_id, outcome.won);
    if segments.is_empty() {
        return;
    }
    seq.stage_scenario(&bank, segments, "completion");
}

/// Feed staged lines into the pill inbox as capacity allows.
fn feed_tutorial_lines(mut seq: ResMut<TutorialSequencer>, inbox: Option<ResMut<PillInbox>>) {
    let Some(mut inbox) = inbox else {
        return;
    };
    if seq.staged_len() > 0 {
        seq.feed(&mut inbox);
    }
}

/// The completed-route verdict, mirroring the reaction watcher's
/// verdict block (Fiber/Coax/Wireless media only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteVerdict {
    InWindow,
    TooLow,
    TooHot,
}

fn completed_route_verdict(
    level: &LevelDef,
    live: &LiveGraph,
    active_outage: Option<&ActiveOutage>,
) -> Option<RouteVerdict> {
    let (_dbm, hops, frontier) =
        live.graph
            .frontier_budget(level.source_node, live.tx_dbm, live.wavelength.0);
    if hops == 0 || frontier != level.target_node {
        return None;
    }
    let window = level.receive_window();
    let outage = active_outage.and_then(|active| active.outage.as_ref());
    live.graph
        .compute_link_budget_with_outage(
            level.source_node,
            level.target_node,
            live.tx_dbm,
            live.wavelength.0,
            window,
            outage,
        )
        .map(|result| {
            if result.in_window {
                RouteVerdict::InWindow
            } else if result.received_dbm < window.min_dbm {
                RouteVerdict::TooLow
            } else {
                RouteVerdict::TooHot
            }
        })
        .ok()
}

/// In-level tutorial watchers. Each derivation is documented in the
/// module header; every segment is gated on the selected companion
/// owning the level context it was authored for.
#[allow(clippy::too_many_arguments)]
fn watch_tutorial_events(
    level: Option<Res<LevelDef>>,
    live: Option<Res<LiveGraph>>,
    placed: Option<Res<PlacedChoices>>,
    triage: Option<Res<TriageProgress>>,
    quiz_progress: Option<Res<QuizProgress>>,
    active_outage: Option<Res<ActiveOutage>>,
    selected: Res<SelectedCompanion>,
    bank: Res<DialogueBank>,
    mut seq: ResMut<TutorialSequencer>,
) {
    let (Some(level), Some(live)) = (level, live) else {
        return;
    };
    let companion = selected.0;
    if seq.active_companion != Some(companion) || seq.active_level_id != level.id {
        return;
    }
    let own_level_one = is_own_level_one(companion, &level.id);

    // Scripted-outage segments (w4l1 / c1l2 / m1l2 / clara2): the
    // existing scripted-outage path fills ActiveOutage; the first
    // frame it is present on the named level fires the segment.
    if let Some(active) = active_outage.as_deref() {
        if active.outage.is_some() && !seq.outage_seen {
            seq.outage_seen = true;
            if outage_segment_level(companion) == Some(level.id.as_str()) {
                seq.stage_segment(&bank, companion, TutorialStage::FirstOutage);
            }
        }
    }

    if !own_level_one {
        return;
    }

    // first_component_placed family (board companions + Hikari),
    // and Clara's first_profile_assignment (first placement in a
    // provisioning level). Both are "first placement exists".
    if let Some(placed) = placed.as_deref() {
        if !placed.0.is_empty() {
            match companion {
                Companion::Fiber
                | Companion::Coax
                | Companion::Mobile
                | Companion::Ethernet
                | Companion::Hikari => {
                    seq.stage_segment(&bank, companion, TutorialStage::FirstPlacement);
                }
                Companion::Clara if !level.subscribers.is_empty() => {
                    seq.stage_segment(&bank, companion, TutorialStage::FirstPlacement);
                }
                _ => {}
            }
        }
    }

    match companion {
        Companion::Fiber | Companion::Coax | Companion::Mobile | Companion::Hikari => {
            watch_db_verdicts(
                &level,
                &live,
                active_outage.as_deref(),
                companion,
                &bank,
                &mut seq,
            );
        }
        Companion::Ethernet => watch_ethernet_failure(&level, &live, &bank, &mut seq),
        Companion::Clara => watch_clara_verification(&level, &live, &bank, &mut seq),
        Companion::Aino => watch_aino_triage(&level, triage.as_deref(), &bank, &mut seq),
        Companion::Lea => watch_lea_quiz(&level, quiz_progress.as_deref(), &bank, &mut seq),
    }
}

/// Fiber/Coax/Wireless verdict segments, plus Hikari's dead reading.
fn watch_db_verdicts(
    level: &LevelDef,
    live: &LiveGraph,
    active_outage: Option<&ActiveOutage>,
    companion: Companion,
    bank: &DialogueBank,
    seq: &mut TutorialSequencer,
) {
    if level.medium() == Medium::Ethernet || !level.subscribers.is_empty() || level.quiz.is_some() {
        return;
    }
    let Some(verdict) = completed_route_verdict(level, live, active_outage) else {
        return;
    };
    // first_completed_path_eval: any completed-route verdict.
    seq.stage_segment(bank, companion, TutorialStage::FirstVerdict);
    // Hikari's first_dead_reading: the first TooLow verdict.
    if companion == Companion::Hikari && verdict == RouteVerdict::TooLow {
        seq.stage_segment(bank, companion, TutorialStage::FirstOutage);
    }
}

/// Lattice's first_midlevel_failure (definition in module docs).
fn watch_ethernet_failure(
    level: &LevelDef,
    live: &LiveGraph,
    bank: &DialogueBank,
    seq: &mut TutorialSequencer,
) {
    // Same Ethernet parameters `level::is_win_state` uses (the
    // evaluator entry itself is public; its parameter defaults are
    // replicated here from the level's public fields).
    let eval = live.graph.evaluate_ethernet(
        level.source_node,
        level.target_node,
        level.max_segment_length_m.unwrap_or(100.0),
        level.endpoint_poe_draw_w.unwrap_or(0.0),
        level.required_bandwidth_mbps.unwrap_or(0),
    );
    if let Ok(eval) = eval {
        if !eval.violations.is_empty() {
            seq.stage_segment(bank, Companion::Ethernet, TutorialStage::FirstOutage);
        }
    }
}

/// Clara's first_subscriber_verification (definition in module docs).
fn watch_clara_verification(
    level: &LevelDef,
    live: &LiveGraph,
    bank: &DialogueBank,
    seq: &mut TutorialSequencer,
) {
    if level.subscribers.is_empty() {
        return;
    }
    if level.is_win_state(&live.graph, live.tx_dbm, live.wavelength.0) {
        seq.stage_segment(bank, Companion::Clara, TutorialStage::FirstVerdict);
    }
}

/// Aino's ack / triage-resolution segments from `TriageProgress`.
fn watch_aino_triage(
    level: &LevelDef,
    triage: Option<&TriageProgress>,
    bank: &DialogueBank,
    seq: &mut TutorialSequencer,
) {
    let (Some(triage), Some(def)) = (triage, level.alarm_triage.as_ref()) else {
        return;
    };
    if !triage.acked.is_empty() {
        seq.stage_segment(bank, Companion::Aino, TutorialStage::FirstPlacement);
    }
    if triage.is_complete(&def.expected_order) {
        seq.stage_segment(bank, Companion::Aino, TutorialStage::FirstVerdict);
    }
}

/// Léa's answer segments from `QuizProgress` (substitution for the
/// confidence trigger is documented in the module header).
fn watch_lea_quiz(
    level: &LevelDef,
    progress: Option<&QuizProgress>,
    bank: &DialogueBank,
    seq: &mut TutorialSequencer,
) {
    let (Some(progress), Some(quiz)) = (progress, level.quiz.as_ref()) else {
        return;
    };
    let answered: Vec<(usize, usize)> = progress
        .answers
        .iter()
        .enumerate()
        .filter_map(|(i, a)| a.map(|choice| (i, choice)))
        .collect();
    if answered.is_empty() {
        return;
    }
    // answer() records and scores in one call: both segments stage
    // at the first recorded answer, in segment order.
    seq.stage_segment(bank, Companion::Lea, TutorialStage::FirstPlacement);
    seq.stage_segment(bank, Companion::Lea, TutorialStage::FirstVerdict);
    // SUBSTITUTION: first_confident_wrong_answer -> first wrong
    // answer (no confidence input exists in the quiz flow).
    let any_wrong = answered.iter().any(|(i, choice)| {
        quiz.questions
            .get(*i)
            .is_some_and(|q| q.correct_idx != *choice)
    });
    if any_wrong {
        seq.stage_segment(bank, Companion::Lea, TutorialStage::FirstOutage);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn bank(companion: Companion) -> DialogueBank {
        DialogueBank::load_default(companion)
    }

    #[test]
    fn briefing_stages_in_position_order_with_all_lines() {
        // Multi-line keys emit EVERY line in bank order (not
        // random_line's first line only), segments in position order.
        let b = bank(Companion::Coax);
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Coax, "c1l1");
        let staged = seq.stage_briefing(&b, Companion::Coax, false);
        let mut expected: Vec<&str> = Vec::new();
        for stage in BRIEFING_STAGES {
            let (key, _) = tutorial_spec(Companion::Coax, stage);
            expected.extend(b.lines[key].iter().map(|s| s.as_str()));
        }
        assert_eq!(staged, expected.len());
        let texts: Vec<&str> = seq.staged().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, expected);
        // Emotions ride along per segment.
        let objective_emotion = seq.staged().next().expect("objective line").emotion;
        assert_eq!(objective_emotion, Emotion::Determined);
    }

    #[test]
    fn seraphine_briefing_ends_with_the_dual_attach_outage_beat() {
        let b = bank(Companion::Fiber);
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Fiber, "w1l1");
        seq.stage_briefing(&b, Companion::Fiber, false);
        let last = seq.staged().last().expect("briefing lines");
        let (key, _) = tutorial_spec(Companion::Fiber, TutorialStage::FirstOutage);
        assert_eq!(last.text, b.lines[key][0]);
        assert_eq!(last.emotion, Emotion::Surprised);
    }

    #[test]
    fn seen_flag_suppresses_the_briefing_block() {
        let b = bank(Companion::Fiber);
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Fiber, "w1l1");
        assert_eq!(seq.stage_briefing(&b, Companion::Fiber, true), 0);
        assert_eq!(seq.staged_len(), 0);
    }

    #[test]
    fn feed_respects_pill_inbox_capacity() {
        let b = bank(Companion::Fiber);
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Fiber, "w1l1");
        seq.stage_briefing(&b, Companion::Fiber, false);
        let total = seq.staged_len();
        assert!(total > 1);
        let mut inbox = PillInbox::default();
        // Fill the inbox to one slot short of capacity.
        for _ in 0..PILL_INBOX_CAPACITY - 1 {
            inbox.push_line(PillSpeaker::Companion(Companion::Fiber), "filler");
        }
        assert_eq!(seq.feed(&mut inbox), 1, "only the free slot may be fed");
        assert_eq!(inbox.len(), PILL_INBOX_CAPACITY);
        assert_eq!(seq.staged_len(), total - 1);
        // Drain and feed again: the next staged line moves.
        let _ = inbox.drain();
        assert_eq!(seq.feed(&mut inbox), total - 1);
        assert_eq!(seq.staged_len(), 0);
    }

    #[test]
    fn fed_lines_carry_the_segments_emotion() {
        let b = bank(Companion::Ethernet);
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Ethernet, "e1l1");
        assert!(seq.stage_segment(&b, Companion::Ethernet, TutorialStage::ScoringAndCores));
        let mut inbox = PillInbox::default();
        assert_eq!(seq.feed(&mut inbox), 1);
        let drained = inbox.drain();
        assert_eq!(drained[0].presentation.emotion, Emotion::Smug);
        assert_eq!(
            drained[0].presentation.speaker,
            PillSpeaker::Companion(Companion::Ethernet)
        );
    }

    #[test]
    fn a_segment_fires_once_per_attempt_and_reset_rearms_it() {
        let b = bank(Companion::Fiber);
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Fiber, "w1l1");
        assert!(seq.stage_segment(&b, Companion::Fiber, TutorialStage::FirstPlacement));
        let len = seq.staged_len();
        assert!(!seq.stage_segment(&b, Companion::Fiber, TutorialStage::FirstPlacement));
        assert_eq!(seq.staged_len(), len);
        seq.reset_for_level(Companion::Fiber, "w1l1");
        assert!(seq.stage_segment(&b, Companion::Fiber, TutorialStage::FirstPlacement));
    }

    #[test]
    fn missing_bank_key_is_a_silent_no_op() {
        // Adversarial: an empty bank must degrade to silence, never
        // a panic or a substitute line.
        let empty = DialogueBank {
            lines: std::collections::HashMap::new(),
        };
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Fiber, "w1l1");
        assert!(!seq.stage_segment(&empty, Companion::Fiber, TutorialStage::FirstVerdict));
        assert_eq!(seq.stage_briefing(&empty, Companion::Fiber, false), 0);
        assert_eq!(seq.staged_len(), 0);
    }

    #[test]
    fn segments_never_match_the_wrong_companion() {
        // Adversarial: level/companion pairing is exact for all 8x8
        // combinations — a segment context only belongs to its owner.
        for owner in ALL_COMPANIONS {
            for other in ALL_COMPANIONS {
                assert_eq!(
                    is_own_level_one(other, level_one_id(owner)),
                    owner == other,
                    "{other:?} vs {}",
                    level_one_id(owner)
                );
            }
        }
        assert_eq!(level_one_companion("not_a_level"), None);
        assert_eq!(outage_segment_level(Companion::Fiber), Some("w4l1"));
        assert_eq!(outage_segment_level(Companion::Coax), Some("c1l2"));
        assert_eq!(outage_segment_level(Companion::Mobile), Some("m1l2"));
        assert_eq!(outage_segment_level(Companion::Clara), Some("clara2"));
    }

    #[test]
    fn scenario_tables_reference_keys_seraphine_has() {
        let b = bank(Companion::Fiber);
        for level_id in ["fj1", "fj2", "sp1"] {
            for (key, _) in scenario_entry_segments(level_id) {
                assert!(b.lines.contains_key(*key), "entry {level_id} -> {key}");
            }
            for won in [true, false] {
                for (key, _) in scenario_completion_segments(level_id, won) {
                    assert!(b.lines.contains_key(*key), "completion {level_id} -> {key}");
                }
            }
        }
        assert!(scenario_entry_segments("w1l1").is_empty());
        assert!(scenario_completion_segments("w1l1", true).is_empty());
    }

    #[test]
    fn scenario_staging_emits_the_entry_beats_in_order() {
        let b = bank(Companion::Fiber);
        let mut seq = TutorialSequencer::default();
        seq.reset_for_level(Companion::Fiber, "sp1");
        let staged = seq.stage_scenario(&b, scenario_entry_segments("sp1"), "entry");
        assert_eq!(staged, scenario_entry_segments("sp1").len());
        let first = seq.staged().next().expect("chart handover");
        assert_eq!(first.text, b.lines["sp_chart_handover"][0]);
    }
}
