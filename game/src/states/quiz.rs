//! Quiz UI for Léa's study track.
//!
//! When a level carries `LevelDef::quiz`, the board is unused and this UI
//! owns the level: it presents each question with four choice buttons,
//! shows Léa's explanation after each answer, and transitions to Results
//! when the question set is exhausted. The win check is
//! `level::quiz_passed` (>= `pass_pct` correct).

use bevy::prelude::*;

use crate::anim::TransitionRequest;
use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY_BOLD, FONT_SIZE_ADJUST};
use crate::level::{quiz_passed, LevelDef, QuizDef};
use crate::states::{GameState, LevelOutcome};

/// The player's in-progress quiz state for the current level.
#[derive(Resource, Debug, Default)]
pub struct QuizProgress {
    /// Index of the question currently on screen.
    pub current: usize,
    /// Player's answer per question (`None` = not yet answered).
    pub answers: Vec<Option<usize>>,
    /// True while Léa's explanation is showing (after answering).
    pub showing_explanation: bool,
    /// Whether the last answer was correct (for feedback coloring).
    pub last_correct: bool,
}

impl QuizProgress {
    /// Reset for a new quiz with `n` questions.
    pub fn reset(&mut self, n: usize) {
        self.current = 0;
        self.answers = vec![None; n];
        self.showing_explanation = false;
        self.last_correct = false;
    }

    /// Record an answer. Returns `true` if it was correct.
    /// No-op (returns `false`) when already showing the explanation —
    /// the player must advance before answering again.
    pub fn answer(&mut self, quiz: &QuizDef, choice: usize) -> bool {
        if self.showing_explanation {
            return false;
        }
        let Some(q) = quiz.questions.get(self.current) else {
            return false;
        };
        let correct = choice == q.correct_idx;
        if let Some(slot) = self.answers.get_mut(self.current) {
            *slot = Some(choice);
        }
        self.showing_explanation = true;
        self.last_correct = correct;
        correct
    }

    /// Advance to the next question. Returns `true` when the quiz is
    /// complete (all questions answered).
    pub fn advance(&mut self, quiz: &QuizDef) -> bool {
        self.showing_explanation = false;
        self.current += 1;
        self.current >= quiz.questions.len()
    }

    /// Player's chosen indices, with unanswered slots as `usize::MAX`
    /// (counts as wrong in `score_quiz`).
    pub fn answer_indices(&self) -> Vec<usize> {
        self.answers
            .iter()
            .map(|a| a.unwrap_or(usize::MAX))
            .collect()
    }
}

/// Marker on the quiz UI root (for teardown).
#[derive(Component)]
struct QuizRoot;

/// Marker on each choice button (choice index).
#[derive(Component)]
pub struct QuizChoice(pub usize);

/// Marker on the Next/Finish button.
#[derive(Component)]
struct QuizNext;

/// Marker on the question text.
#[derive(Component)]
struct QuizQuestionText;

/// Marker on the progress line ("Question 3/10").
#[derive(Component)]
struct QuizProgressLine;

/// Marker on the explanation panel.
#[derive(Component)]
struct QuizExplanation;

/// Marker on the feedback line ("Correct!" / "Not quite.").
#[derive(Component)]
struct QuizFeedback;

pub struct QuizPlugin;

impl Plugin for QuizPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuizProgress>()
            .add_systems(
                Update,
                (handle_quiz_choices, handle_quiz_next)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_quiz_ui)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_quiz_ui);
    }
}

/// Spawn the quiz UI when the level has a `quiz` def. No quiz means no
/// UI — pure board levels are untouched.
pub(crate) fn setup_quiz_ui(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<QuizProgress>,
    asset_server: Res<AssetServer>,
) {
    let Some(quiz) = &level.quiz else {
        return;
    };
    progress.reset(quiz.questions.len());

    let display: Handle<Font> = asset_server.load(DISPLAY_BOLD);
    let body: Handle<Font> = asset_server.load(BODY);
    let body_medium: Handle<Font> = asset_server.load(BODY_MEDIUM);

    let total = quiz.questions.len();
    commands
        .spawn((
            QuizRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                right: Val::Px(24.0),
                top: Val::Px(80.0),
                bottom: Val::Px(24.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.05, 0.10, 0.96)),
        ))
        .with_children(|root| {
            // Header: Léa's study hall + progress
            root.spawn((
                Text::new("LÉA'S STUDY HALL"),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(20.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.45, 1.0)),
            ));
            root.spawn((
                QuizProgressLine,
                Text::new(format!("Question 1/{total}")),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
            ));
            // Question text
            root.spawn((
                QuizQuestionText,
                Text::new(question_text(quiz, 0)),
                TextFont {
                    font: body_medium.clone().into(),
                    font_size: FontSize::Px(18.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            // Choice buttons
            for (i, choice) in quiz.questions[0].choices.iter().enumerate() {
                root.spawn((
                    QuizChoice(i),
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.14, 0.12, 0.22)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(format!("{}. {}", (b'A' + i as u8) as char, choice)),
                        TextFont {
                            font: body.clone().into(),
                            font_size: FontSize::Px(15.0 * FONT_SIZE_ADJUST),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
            // Feedback line (hidden until answered)
            root.spawn((
                QuizFeedback,
                Text::new(""),
                TextFont {
                    font: body_medium.clone().into(),
                    font_size: FontSize::Px(16.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            // Explanation panel (hidden until answered)
            root.spawn((
                QuizExplanation,
                Text::new(""),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.85, 0.9)),
            ));
            // Next button (hidden until answered)
            root.spawn((
                QuizNext,
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.25, 0.15, 0.45)),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("Next →"),
                    TextFont {
                        font: body_medium.clone().into(),
                        font_size: FontSize::Px(15.0 * FONT_SIZE_ADJUST),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn question_text(quiz: &QuizDef, idx: usize) -> String {
    quiz.questions
        .get(idx)
        .map(|q| q.prompt.clone())
        .unwrap_or_default()
}

/// Handle choice button clicks: record the answer, show feedback +
/// explanation, reveal the Next button.
#[allow(clippy::too_many_arguments)]
fn handle_quiz_choices(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<QuizProgress>,
    buttons: Query<
        (&Interaction, &QuizChoice),
        (
            Changed<Interaction>,
            Without<crate::states::triage_console::TriageButton>,
            Without<crate::states::api_console::ApiButton>,
        ),
    >,
    mut feedback: Query<
        &mut Text,
        (
            With<QuizFeedback>,
            Without<QuizExplanation>,
            Without<QuizQuestionText>,
            Without<QuizProgressLine>,
        ),
    >,
    mut explanation: Query<
        &mut Text,
        (
            With<QuizExplanation>,
            Without<QuizFeedback>,
            Without<QuizQuestionText>,
            Without<QuizProgressLine>,
        ),
    >,
    mut next_btn: Query<&mut Node, With<QuizNext>>,
    sfx: Res<crate::audio::Sfx>,
) {
    let Some(quiz) = &level.quiz else {
        return;
    };
    for (interaction, choice) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let correct = progress.answer(quiz, choice.0);
        if correct {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
        } else {
            sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
        }
        let q = &quiz.questions[progress.current];
        for mut text in &mut feedback {
            **text = if correct {
                "✓ Correct!".to_string()
            } else {
                format!(
                    "✗ Not quite — the answer was {}. {}",
                    (b'A' + q.correct_idx as u8) as char,
                    q.choices[q.correct_idx]
                )
            };
        }
        for mut text in &mut explanation {
            let article = q
                .article_ref
                .as_deref()
                .map(|a| format!(" [{a}]"))
                .unwrap_or_default();
            **text = format!("Léa:{article} {}", q.explanation);
        }
        for mut node in &mut next_btn {
            node.display = Display::Flex;
        }
    }
}

/// Handle the Next button: advance or finish the quiz.
#[allow(clippy::too_many_arguments)]
fn handle_quiz_next(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<QuizProgress>,
    buttons: Query<&Interaction, (With<QuizNext>, Changed<Interaction>)>,
    mut q_text_q: Query<&mut Text, With<QuizQuestionText>>,
    mut progress_line: Query<&mut Text, (With<QuizProgressLine>, Without<QuizQuestionText>)>,
    mut feedback: Query<
        &mut Text,
        (
            With<QuizFeedback>,
            Without<QuizQuestionText>,
            Without<QuizProgressLine>,
        ),
    >,
    mut explanation: Query<
        &mut Text,
        (
            With<QuizExplanation>,
            Without<QuizQuestionText>,
            Without<QuizProgressLine>,
            Without<QuizFeedback>,
        ),
    >,
    mut next_btn: Query<&mut Node, With<QuizNext>>,
    mut choice_labels: Query<(&QuizChoice, &Children)>,
    mut texts: Query<&mut Text, Without<QuizQuestionText>>,
    mut outcome: ResMut<LevelOutcome>,
    mut request: ResMut<TransitionRequest>,
    next_state: Res<NextState<GameState>>,
    sfx: Res<crate::audio::Sfx>,
) {
    let Some(quiz) = &level.quiz else {
        return;
    };
    for interaction in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let done = progress.advance(quiz);
        if done {
            // Quiz complete — score it.
            let answers = progress.answer_indices();
            let won = quiz_passed(quiz, &answers);
            if won {
                sfx.play(&mut commands, crate::audio::SfxKind::Win);
            } else {
                sfx.play(&mut commands, crate::audio::SfxKind::Lose);
            }
            outcome.won = won;
            let transition_pending =
                request.0.is_some() || matches!(*next_state, NextState::Pending(_));
            if !transition_pending {
                request.0 = Some(GameState::Results);
            }
            return;
        }
        // Show the next question.
        let idx = progress.current;
        let total = quiz.questions.len();
        for mut text in &mut q_text_q {
            **text = question_text(quiz, idx);
        }
        for mut text in &mut progress_line {
            **text = format!("Question {}/{}", idx + 1, total);
        }
        for mut text in &mut feedback {
            **text = String::new();
        }
        for mut text in &mut explanation {
            **text = String::new();
        }
        for mut node in &mut next_btn {
            node.display = Display::None;
        }
        // Refresh choice button labels.
        if let Some(q) = quiz.questions.get(idx) {
            for (choice, children) in &mut choice_labels {
                if let Some(label) = q.choices.get(choice.0) {
                    for child in children.iter() {
                        if let Ok(mut text) = texts.get_mut(child) {
                            // Skip non-choice texts by checking the marker
                            // isn't present — choice labels are the only
                            // Text under QuizChoice buttons.
                            **text = format!("{}. {}", (b'A' + choice.0 as u8) as char, label);
                        }
                    }
                }
            }
        }
    }
}

fn cleanup_quiz_ui(
    mut commands: Commands,
    roots: Query<Entity, With<QuizRoot>>,
    mut progress: ResMut<QuizProgress>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    *progress = QuizProgress::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{QuizDef, QuizDomain, QuizQuestion};

    fn sample_quiz() -> QuizDef {
        QuizDef {
            questions: vec![
                QuizQuestion {
                    id: "t-001".to_string(),
                    prompt: "What is 2+2?".to_string(),
                    choices: [
                        "3".to_string(),
                        "4".to_string(),
                        "5".to_string(),
                        "6".to_string(),
                    ],
                    correct_idx: 1,
                    explanation: "Basic arithmetic.".to_string(),
                    article_ref: None,
                    domain: QuizDomain::Theory,
                },
                QuizQuestion {
                    id: "t-002".to_string(),
                    prompt: "What is 3+3?".to_string(),
                    choices: [
                        "5".to_string(),
                        "6".to_string(),
                        "7".to_string(),
                        "8".to_string(),
                    ],
                    correct_idx: 1,
                    explanation: "Basic arithmetic.".to_string(),
                    article_ref: None,
                    domain: QuizDomain::Theory,
                },
            ],
            pass_pct: 0.7,
            time_limit_s: None,
        }
    }

    #[test]
    fn answer_records_and_detects_correctness() {
        let quiz = sample_quiz();
        let mut p = QuizProgress::default();
        p.reset(2);
        assert!(p.answer(&quiz, 1)); // correct
        assert!(p.last_correct);
        assert!(p.showing_explanation);
        // Answering again while showing explanation is a no-op.
        assert!(!p.answer(&quiz, 0));
        assert_eq!(p.answers[0], Some(1));
    }

    #[test]
    fn answer_marks_wrong() {
        let quiz = sample_quiz();
        let mut p = QuizProgress::default();
        p.reset(2);
        assert!(!p.answer(&quiz, 0)); // wrong
        assert!(!p.last_correct);
        assert_eq!(p.answers[0], Some(0));
    }

    #[test]
    fn advance_moves_through_questions() {
        let quiz = sample_quiz();
        let mut p = QuizProgress::default();
        p.reset(2);
        p.answer(&quiz, 1);
        assert!(!p.advance(&quiz)); // more questions remain
        assert_eq!(p.current, 1);
        assert!(!p.showing_explanation);
        p.answer(&quiz, 1);
        assert!(p.advance(&quiz)); // done
    }

    #[test]
    fn quiz_passed_uses_threshold() {
        let quiz = sample_quiz();
        // 2/2 = 100% >= 70% → pass
        assert!(quiz_passed(&quiz, &[1, 1]));
        // 1/2 = 50% < 70% → fail
        assert!(!quiz_passed(&quiz, &[1, 0]));
        // 0/2 → fail
        assert!(!quiz_passed(&quiz, &[0, 0]));
    }

    #[test]
    fn empty_quiz_never_passes() {
        let quiz = QuizDef {
            questions: vec![],
            pass_pct: 0.7,
            time_limit_s: None,
        };
        assert!(!quiz_passed(&quiz, &[]));
    }

    #[test]
    fn score_quiz_counts_correct() {
        let quiz = sample_quiz();
        assert_eq!(crate::level::score_quiz(&quiz.questions, &[1, 1]), (2, 2));
        assert_eq!(crate::level::score_quiz(&quiz.questions, &[1, 0]), (1, 2));
        assert_eq!(crate::level::score_quiz(&quiz.questions, &[0, 0]), (0, 2));
        // Short answer list: missing counts as wrong, no panic.
        assert_eq!(crate::level::score_quiz(&quiz.questions, &[1]), (1, 2));
    }
}
