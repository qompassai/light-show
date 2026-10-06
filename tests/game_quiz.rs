//! Integration coverage for Léa's quiz track (`game/assets/levels/lea*.json`).
//!
//! Each of Léa's 10 levels carries an inline `quiz` def with real NEC/WA-law
//! questions. This file proves, per level:
//! - the level parses and has a non-empty question set (validation),
//! - every question has exactly 4 choices with a valid `correct_idx` and
//!   a non-empty explanation (Léa's teaching moment),
//! - a perfect playthrough passes `quiz_passed` (completable),
//! - an all-wrong playthrough fails (the gate is real),
//! - the 70% threshold is enforced at the boundary (adversarial).

use light_show::level::{load_level, quiz_passed, score_quiz, LEVEL_SOURCES};

/// Léa's track indices in the 80-level registry.
const LEA_LEVELS: [(usize, &str, usize); 10] = [
    (70, "lea1", 10),  // Tutorial: NEC 90/100/110
    (71, "lea2", 10),  // Grounding: NEC 250
    (72, "lea3", 10),  // Wiring methods: NEC 300-398
    (73, "lea4", 10),  // Hazloc: NEC 500-516
    (74, "lea5", 10),  // Special conditions: NEC 705-780
    (75, "lea6", 12),  // Comms systems boss: NEC 800-830
    (76, "lea7", 10),  // Theory workshop
    (77, "lea8", 10),  // WA law: RCW 19.28
    (78, "lea9", 10),  // WA admin: WAC 296-46B
    (79, "lea10", 20), // Mock exam: pretest A+B
];

#[test]
fn lea_track_has_ten_quiz_levels_at_70_79() {
    assert_eq!(LEVEL_SOURCES.len(), 80, "80-level registry expected");
    for (idx, expected_id, expected_qs) in LEA_LEVELS {
        let level = load_level(idx);
        assert_eq!(level.id, expected_id, "level {idx} id mismatch");
        let quiz = level
            .quiz
            .as_ref()
            .expect(&format!("{expected_id} must have quiz"));
        assert_eq!(
            quiz.questions.len(),
            expected_qs,
            "{expected_id}: expected {expected_qs} questions"
        );
    }
}

#[test]
fn lea_questions_are_well_formed() {
    for (idx, expected_id, _) in LEA_LEVELS {
        let level = load_level(idx);
        let quiz = level.quiz.as_ref().unwrap();
        for q in &quiz.questions {
            assert_eq!(
                q.choices.len(),
                4,
                "{} {}: must have 4 choices",
                expected_id,
                q.id
            );
            assert!(
                q.correct_idx < 4,
                "{} {}: correct_idx {} out of range",
                expected_id,
                q.id,
                q.correct_idx
            );
            assert!(
                !q.prompt.is_empty(),
                "{} {}: prompt must not be empty",
                expected_id,
                q.id
            );
            assert!(
                !q.explanation.is_empty(),
                "{} {}: explanation must not be empty (Léa's teaching moment)",
                expected_id,
                q.id
            );
            // Choices must be distinct (no duplicate answers to guess between).
            let mut seen = std::collections::HashSet::new();
            for c in &q.choices {
                assert!(
                    seen.insert(c.as_str()),
                    "{} {}: duplicate choice",
                    expected_id,
                    q.id
                );
            }
        }
    }
}

#[test]
fn lea_perfect_playthrough_passes_every_level() {
    for (idx, expected_id, _) in LEA_LEVELS {
        let level = load_level(idx);
        let quiz = level.quiz.as_ref().unwrap();
        // Answer every question correctly.
        let answers: Vec<usize> = quiz.questions.iter().map(|q| q.correct_idx).collect();
        let (correct, total) = score_quiz(&quiz.questions, &answers);
        assert_eq!(
            correct, total,
            "{expected_id}: perfect playthrough must score 100%"
        );
        assert!(
            quiz_passed(quiz, &answers),
            "{expected_id}: perfect playthrough must pass"
        );
    }
}

#[test]
fn lea_all_wrong_playthrough_fails_every_level() {
    for (idx, expected_id, _) in LEA_LEVELS {
        let level = load_level(idx);
        let quiz = level.quiz.as_ref().unwrap();
        // Answer every question wrong (pick the next index cyclically).
        let answers: Vec<usize> = quiz
            .questions
            .iter()
            .map(|q| (q.correct_idx + 1) % 4)
            .collect();
        let (correct, _) = score_quiz(&quiz.questions, &answers);
        assert_eq!(correct, 0, "{expected_id}: all-wrong must score 0");
        assert!(
            !quiz_passed(quiz, &answers),
            "{expected_id}: all-wrong playthrough must fail"
        );
    }
}

#[test]
fn lea_pass_threshold_is_70_percent() {
    // Boundary: 7/10 = 70% passes, 6/10 = 60% fails.
    let level = load_level(70); // lea1, 10 questions
    let quiz = level.quiz.as_ref().unwrap();
    assert_eq!(quiz.questions.len(), 10);
    assert!((quiz.pass_pct - 0.70).abs() < f32::EPSILON);

    let mut answers: Vec<usize> = quiz.questions.iter().map(|q| q.correct_idx).collect();
    // Get exactly 7 right.
    for i in 7..10 {
        answers[i] = (quiz.questions[i].correct_idx + 1) % 4;
    }
    let (correct, _) = score_quiz(&quiz.questions, &answers);
    assert_eq!(correct, 7);
    assert!(quiz_passed(quiz, &answers), "7/10 = 70% must pass");

    // Get exactly 6 right.
    answers[6] = (quiz.questions[6].correct_idx + 1) % 4;
    let (correct, _) = score_quiz(&quiz.questions, &answers);
    assert_eq!(correct, 6);
    assert!(!quiz_passed(quiz, &answers), "6/10 = 60% must fail");
}

#[test]
fn lea_quiz_levels_never_win_via_board() {
    // Quiz levels must not accidentally satisfy a board win check —
    // the quiz UI owns the win condition.
    use light_show::level::LevelDef;
    for (idx, expected_id, _) in LEA_LEVELS {
        let level: LevelDef = load_level(idx);
        assert!(level.quiz.is_some(), "{expected_id}: must be a quiz level");
        // Empty board + quiz guard => is_win_state is false.
        let graph = osp_sim::PathGraph {
            nodes: vec![],
            edges: vec![],
        };
        assert!(
            !level.is_win_state(&graph, level.tx_dbm, osp_sim::Wavelength::Nm1490),
            "{expected_id}: quiz level must never win via board"
        );
    }
}
