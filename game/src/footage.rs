//! Test-footage capture harness (debug builds only).
//!
//! Activated by `--footage <level-id>` (e.g. `--footage clara1`). Selects
//! the level by its `LevelDef::id`, drives a per-level scripted placement
//! sequence through the real game logic (`PlacedChoices` + `rebuild_live_graph`,
//! the same path the input handler uses), and exits via `AppExit` after the
//! script's frame budget. Pair with `xvfb-run` and `ffmpeg -f x11grab` to
//! capture the clip.
//!
//! `--footage-frames <N>` overrides the per-level frame budget.
//!
//! Without the flag the game builds a byte-identical plugin set and installs
//! zero footage systems: no overhead, no behavior change.

use bevy::prelude::*;

use crate::board;
use crate::level::{self, CurrentLevelIndex};
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::quiz::{
    QuizChoice, QuizChoiceLabel, QuizExplanation, QuizFeedback, QuizProgress, QuizProgressLine,
    QuizQuestionText,
};
use crate::states::GameState;

/// A single scripted action, executed at a given frame.
#[derive(Clone, Copy)]
enum FootageAction {
    /// Place a component: insert `(from, to) -> slot` into `PlacedChoices`
    /// and rebuild the live graph (the same state transition the pill-tap
    /// input handler performs).
    Place { from: u32, to: u32, slot: usize },
    /// Answer the current quiz question. `wrong` picks a wrong choice to
    /// show the correction path; otherwise the correct choice is read from
    /// the level data at runtime. Enqueued for `footage_quiz_ui_sync`.
    QuizAnswer { wrong: bool },
    /// Advance the quiz. Enqueued for `footage_quiz_ui_sync`.
    QuizNext,
}

/// Per-level footage script: timed actions plus the total frame budget.
struct FootageScript {
    /// `(frame_in_playing, action)` pairs, sorted by frame.
    actions: Vec<(u32, FootageAction)>,
    /// Frames in `Playing` before auto-exit.
    total_frames: u32,
}

/// Parsed `--footage` flags. `level_id: None` means "normal game run".
#[derive(Default)]
pub struct FootageArgs {
    pub level_id: Option<String>,
    pub frames: Option<u32>,
}

impl FootageArgs {
    /// Reads `--footage <level-id>` and `--footage-frames <N>` from the
    /// process arguments. Invalid values are reported on stderr and treated
    /// as absent; unknown flags are ignored.
    pub fn from_args() -> Self {
        let mut args = Self::default();
        let mut raw = std::env::args().skip(1);
        while let Some(flag) = raw.next() {
            match flag.as_str() {
                "--footage" => match raw.next() {
                    Some(id) if !id.is_empty() => args.level_id = Some(id),
                    _ => eprintln!("[light-show] ignoring --footage with empty id"),
                },
                "--footage-frames" => match raw.next().map(|n| n.parse::<u32>()) {
                    Some(Ok(n)) if n > 0 => args.frames = Some(n.min(100_000)),
                    Some(Ok(_)) => eprintln!("[light-show] ignoring --footage-frames 0"),
                    Some(Err(_)) | None => {
                        eprintln!("[light-show] ignoring invalid --footage-frames value")
                    }
                },
                _ => {}
            }
        }
        args
    }
}

/// Index into `LEVEL_SOURCES` for a level id (e.g. `"clara1"`, `"w1l1"`).
/// Matches against `LevelDef::id` parsed from the embedded JSON.
fn level_index_for_id(id: &str) -> Option<usize> {
    (0..level::LEVEL_SOURCES.len()).find(|&i| {
        let def: Result<level::LevelDef, _> = serde_json::from_str(level::LEVEL_SOURCES[i]);
        def.map(|d| d.id == id).unwrap_or(false)
    })
}

/// Frames in `Playing` before the first scripted placement. Covers asset
/// loading + board setup under software rendering.
const SETUP_FRAMES: u32 = 400;

/// Build a script that answers three quiz questions (correct, except the
/// first on lea10 which is deliberately wrong to show the correction
/// path), holding each explanation ~1.4s. ~20s total at 35fps software
/// rendering.
fn quiz_script(id: &str) -> Option<FootageScript> {
    let wrong_first = id == "lea10";
    let mut actions = Vec::new();
    let mut frame = SETUP_FRAMES + 10;
    for qi in 0..3 {
        actions.push((
            frame,
            FootageAction::QuizAnswer {
                wrong: wrong_first && qi == 0,
            },
        ));
        frame += 50;
        actions.push((frame, FootageAction::QuizNext));
        frame += 30;
    }
    Some(FootageScript {
        actions,
        total_frames: frame + 60,
    })
}

/// Scripted placements per level: put down the level's signature component,
/// then hold for the level's signature beat (outage levels wait for the
/// scripted outage to fire on wall-clock time).
fn script_for_level(id: &str) -> Option<FootageScript> {
    // Lea's quiz levels have no board: drive the quiz UI instead.
    if id.starts_with("lea") {
        return quiz_script(id);
    }
    // Pure API/triage levels: no board placement, just run frame budget.
    let no_place: Option<u32> = match id {
        "clara5" => Some(1400),
        "clara7" => Some(1500),
        "aino1" => Some(900),
        "aino2" => Some(1100),
        "aino3" => Some(1300),
        "aino4" => Some(1400),
        "aino5" => Some(1400),
        "aino7" => Some(1600),
        "aino8" => Some(1800),
        "aino9" => Some(1300),
        "aino10" => Some(2000),
        "hikari2" => Some(1400),
        "hikari6" => Some(1100),
        _ => None,
    };
    if let Some(tf) = no_place {
        return Some(FootageScript {
            actions: Vec::new(),
            total_frames: tf,
        });
    }
    // (from, to, slot, total_frames): slot selects the component choice.
    let (from, to, slot, total_frames) = match id {
        // First Light: Fusion splice, link budget succeeds.
        "w1l1" => (1, 2, 0, 900),
        // Storm Season: main splice 1->3, AerialDamage outage at 20s,
        // then protection splice 2->3. (Two placements; see below.)
        "w4l1" => (1, 3, 0, 2000),
        // Unity Gain: 5dB amplifier lands in the [0,15] dBmV window.
        "c1l1" => (1, 2, 0, 900),
        // Ingress at Night: amp placed, IngressNoise outage at 15s.
        "c1l2" => (1, 2, 1, 1400),
        // Close the Link: 20dBm repeater closes the wireless link.
        "m1l1" => (1, 2, 0, 900),
        // Ride the Storm: repeater placed, interference storm at 15s.
        "m1l2" => (1, 2, 1, 1400),
        // Hundred-Meter Wall: switch regenerates past the 100m limit.
        "e1l1" => (1, 2, 0, 900),
        // Power Budget: 60W switch covers the PoE draw.
        "e1l2" => (1, 2, 0, 900),
        // First Turn-Up: 1:16 splitter for 9 subscribers.
        "clara1" => (0, 1, 2, 1100),
        // Clara outage: 1:8 splitter for 6 subs, WaterIntrusion at 25s.
        "clara2" => (0, 1, 1, 2000),
        "clara3" => (0, 1, 1, 900),
        "clara4" => (0, 1, 1, 900),
        "clara6" => (0, 1, 1, 900),
        "clara8" => (0, 1, 1, 900),
        "clara9" => (0, 1, 1, 900),
        "clara10" => (0, 1, 0, 2200),
        "aino6" => (0, 1, 0, 1600),
        "hikari1" => (1, 2, 0, 900),
        "hikari3" => (1, 2, 0, 2000),
        "hikari4" => (2, 3, 0, 900),
        "hikari5" => (1, 2, 0, 900),
        // --- Seraphine (fiber) 3-10 ---
        "f1l3" => (1, 2, 0, 900),
        "f1l4" => (1, 2, 0, 900),
        "f1l5" => (1, 3, 0, 900),
        "f1l6" => (1, 2, 1, 900),
        "f1l7" => (3, 4, 1, 900),
        "f1l8" => (1, 2, 0, 1400),
        "f1l9" => (2, 3, 0, 900),
        "f1l10" => (2, 3, 2, 1600),
        // --- Ondine (coax) 3-10 ---
        "c1l3" => (1, 2, 1, 900),
        "c1l4" => (2, 3, 1, 900),
        "c1l5" => (1, 2, 0, 1600),
        "c1l6" => (2, 3, 3, 1400),
        "c1l7" => (1, 2, 0, 900),
        "c1l8" => (1, 2, 1, 900),
        "c1l9" => (1, 2, 0, 900),
        "c1l10" => (1, 2, 0, 1400),
        "hikari7" => (1, 2, 0, 900),
        "hikari8" => (1, 2, 1, 900),
        "hikari9" => (1, 2, 0, 2200),
        "hikari10" => (1, 2, 0, 900),
        // Linka 3-10 / Lattice 3-10 (board-only, no outages)
        "m1l3" => (1, 2, 1, 900),
        "m1l4" => (1, 2, 0, 900),
        "m1l5" => (1, 2, 1, 900),
        "m1l6" => (1, 2, 0, 900),
        "m1l7" => (1, 2, 0, 900),
        "m1l8" => (1, 2, 0, 900),
        "m1l9" => (1, 2, 1, 900),
        "m1l10" => (1, 2, 0, 900),
        "e1l3" => (1, 2, 1, 900),
        "e1l4" => (1, 2, 0, 900),
        "e1l5" => (1, 2, 1, 900),
        "e1l6" => (1, 2, 0, 900),
        "e1l7" => (1, 2, 1, 900),
        "e1l8" => (1, 2, 0, 900),
        "e1l9" => (1, 2, 0, 900),
        "e1l10" => (1, 2, 2, 900),
        _ => return None,
    };
    let mut actions = vec![(SETUP_FRAMES + 10, FootageAction::Place { from, to, slot })];
    // w4l1's signature beat is the protection route: after the 20s outage
    // fires on 1->3, place the 2->3 Mechanical splice (slot 0 of its pair).
    // 1500 frames is safely past the outage at ~35fps software rendering.
    if id == "w4l1" {
        actions.push((
            1500,
            FootageAction::Place {
                from: 2,
                to: 3,
                slot: 0,
            },
        ));
    }
    Some(FootageScript {
        actions,
        total_frames,
    })
}

/// Mutable state for one footage run.
#[derive(Resource)]
struct FootageRun {
    /// Frames spent in `GameState::Playing` so far.
    frames_in_playing: u32,
    script: FootageScript,
    /// Next action index to execute.
    next_action: usize,
    /// Whether we've entered `Playing` yet. The driver requests the
    /// transition exactly once; afterwards it leaves state alone so the
    /// win -> Results flow (which re-runs `setup_level` on re-enter) is
    /// not fought.
    entered: bool,
    /// Frames spent in `GameState::Results` (win screen hold before exit).
    frames_in_results: u32,
    /// Pending quiz UI refresh, applied by `footage_quiz_ui_sync`.
    quiz_ui_pending: Option<QuizUiUpdate>,
}

/// A quiz UI refresh for the exclusive `footage_quiz_ui_sync` system.
#[derive(Clone, Copy)]
enum QuizUiUpdate {
    /// Answer the current question (`wrong` shows the correction path).
    Answer { wrong: bool },
    /// Advance to the next question.
    Next,
}

/// Installs the footage driver. Call only for a real footage run
/// (`args.level_id.is_some()` and the id resolves); normal runs must not
/// call this. Also seeds `CurrentLevelIndex` so `setup_level` loads the
/// requested level on `OnEnter(Playing)`.
pub fn add_footage_systems(app: &mut App, level_id: &str, frames_override: Option<u32>) -> bool {
    let Some(index) = level_index_for_id(level_id) else {
        eprintln!("[light-show] --footage: unknown level id {level_id:?}");
        return false;
    };
    let Some(mut script) = script_for_level(level_id) else {
        eprintln!("[light-show] --footage: no script for level id {level_id:?}");
        return false;
    };
    if let Some(n) = frames_override {
        script.total_frames = n;
    }
    eprintln!(
        "[light-show] footage: level {level_id} (index {index}), {} frames",
        script.total_frames
    );
    app.insert_resource(CurrentLevelIndex(index));
    app.insert_resource(FootageRun {
        frames_in_playing: 0,
        script,
        next_action: 0,
        entered: false,
        frames_in_results: 0,
        quiz_ui_pending: None,
    })
    // The driver is resource-only (no component queries), so it cannot
    // trip Bevy's B0001. Quiz Text updates happen in `footage_quiz_ui_sync`,
    // an exclusive system (&mut World) which bypasses the static check.
    // (The &mut Node query was dropped: it conflicts with the outage
    // banner systems' &mut Node.)
    .add_systems(Update, (footage_driver, footage_quiz_ui_sync).chain());
    true
}

/// Executes the scripted placements and exits when the frame budget is
/// spent. Places components by writing `PlacedChoices` + rebuilding the
/// live graph directly — the same state transition the input handler
/// performs on a pill tap, without the fragility of synthetic cursor
/// coordinates under software rendering.
fn footage_driver(
    mut footage: ResMut<FootageRun>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
    level: Option<Res<level::LevelDef>>,
    placed: Option<ResMut<board::PlacedChoices>>,
    live: Option<ResMut<LiveGraph>>,
    active_outage: Option<Res<ActiveOutage>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !footage.entered {
        if *state.get() != GameState::Playing {
            next_state.set(GameState::Playing);
            return;
        }
        footage.entered = true;
    }
    // Win -> Results: hold the win screen briefly, then exit.
    if *state.get() == GameState::Results {
        footage.frames_in_results += 1;
        if footage.frames_in_results >= 180 {
            eprintln!("[light-show] footage: win screen held, exiting");
            exit.write(AppExit::Success);
        }
        return;
    }
    if *state.get() != GameState::Playing {
        return;
    }
    footage.frames_in_playing += 1;
    let frame = footage.frames_in_playing;

    let (Some(level), Some(mut placed), Some(mut live), Some(active_outage)) =
        (level.as_ref(), placed, live, active_outage.as_ref())
    else {
        return;
    };
    while footage.next_action < footage.script.actions.len()
        && footage.script.actions[footage.next_action].0 <= frame
    {
        let (_, action) = footage.script.actions[footage.next_action];
        footage.next_action += 1;
        match action {
            FootageAction::Place { from, to, slot } => {
                placed.0.insert((from, to), slot);
                board::rebuild_live_graph(
                    level,
                    &placed,
                    active_outage.outage.as_ref(),
                    &mut live.graph,
                );
                eprintln!("[light-show] footage: placed {from}->{to} slot {slot} at frame {frame}");
            }
            FootageAction::QuizAnswer { wrong } => {
                footage.quiz_ui_pending = Some(QuizUiUpdate::Answer { wrong });
                eprintln!("[light-show] footage: quiz answer at frame {frame}");
            }
            FootageAction::QuizNext => {
                footage.quiz_ui_pending = Some(QuizUiUpdate::Next);
                eprintln!("[light-show] footage: quiz next at frame {frame}");
            }
        }
    }

    if frame >= footage.script.total_frames {
        eprintln!("[light-show] footage: done after {frame} frames in Playing");
        exit.write(AppExit::Success);
    }
}

/// Applies a pending quiz UI refresh. Exclusive system (`&mut World`):
/// the `&mut Text` queries would trip Bevy's B0001 static conflict check
/// as normal system params. Runs chained directly after `footage_driver`,
/// so the refresh lands in the same frame the action was enqueued.
/// Mirrors what `handle_quiz_choices` / `handle_quiz_next` do on a real
/// button press (minus the Next-button visibility toggle, whose `&mut Node`
/// query conflicts with the outage banner systems).
fn footage_quiz_ui_sync(world: &mut World) {
    let pending = {
        let mut footage = match world.get_resource_mut::<FootageRun>() {
            Some(f) => f,
            None => return,
        };
        match footage.quiz_ui_pending.take() {
            Some(p) => p,
            None => return,
        }
    };
    match pending {
        QuizUiUpdate::Answer { wrong } => quiz_answer_ui(world, wrong),
        QuizUiUpdate::Next => quiz_next_ui(world),
    }
}

/// Answer the current question; show feedback + explanation.
fn quiz_answer_ui(world: &mut World, wrong: bool) {
    struct AnswerData {
        correct: bool,
        correct_idx: usize,
        correct_choice: String,
        explanation: String,
        article_ref: Option<String>,
    }
    let data = {
        // Clone the quiz out first: `Ref<LevelDef>` and `Mut<QuizProgress>`
        // cannot be held simultaneously.
        let quiz = match world.get_resource::<level::LevelDef>() {
            Some(l) => match &l.quiz {
                Some(q) => q.clone(),
                None => return,
            },
            None => return,
        };
        let mut progress = match world.get_resource_mut::<QuizProgress>() {
            Some(p) => p,
            None => return,
        };
        let current = progress.current;
        let first = match quiz.questions.get(current) {
            Some(q) => q,
            None => return,
        };
        let choice_count = first.choices.len().max(1);
        let choice = if wrong {
            (first.correct_idx + 1) % choice_count
        } else {
            first.correct_idx
        };
        let correct = progress.answer(&quiz, choice);
        // `answer` leaves `current` on the answered question.
        let q = &quiz.questions[progress.current];
        AnswerData {
            correct,
            correct_idx: q.correct_idx,
            correct_choice: q.choices[q.correct_idx].clone(),
            explanation: q.explanation.clone(),
            article_ref: q.article_ref.clone(),
        }
    };

    let feedback = if data.correct {
        "\u{2713} Correct!".to_string()
    } else {
        format!(
            "\u{2717} Not quite \u{2014} the answer was {}. {}",
            (b'A' + data.correct_idx as u8) as char,
            data.correct_choice
        )
    };
    let article = data
        .article_ref
        .as_deref()
        .map(|a| format!(" [{a}]"))
        .unwrap_or_default();
    let explanation = format!("L\u{e9}a:{article} {}", data.explanation);

    let mut fb =
        world.query_filtered::<&mut Text, (With<QuizFeedback>, Without<QuizQuestionText>)>();
    for mut text in fb.iter_mut(world) {
        **text = feedback.clone();
    }
    let mut ex =
        world.query_filtered::<&mut Text, (With<QuizExplanation>, Without<QuizQuestionText>)>();
    for mut text in ex.iter_mut(world) {
        **text = explanation.clone();
    }
}

/// Advance to the next question; refresh prompt / progress / labels.
fn quiz_next_ui(world: &mut World) {
    struct NextData {
        idx: usize,
        total: usize,
        prompt: String,
        choices: Vec<String>,
    }
    let data = {
        let quiz = match world.get_resource::<level::LevelDef>() {
            Some(l) => match &l.quiz {
                Some(q) => q.clone(),
                None => return,
            },
            None => return,
        };
        let mut progress = match world.get_resource_mut::<QuizProgress>() {
            Some(p) => p,
            None => return,
        };
        if progress.advance(&quiz) {
            // Quiz complete: not expected in the 3-question scripts.
            return;
        }
        let idx = progress.current;
        let q = match quiz.questions.get(idx) {
            Some(q) => q,
            None => return,
        };
        NextData {
            idx,
            total: quiz.questions.len(),
            prompt: q.prompt.clone(),
            choices: q.choices.to_vec(),
        }
    };

    let mut qt = world.query_filtered::<&mut Text, With<QuizQuestionText>>();
    for mut text in qt.iter_mut(world) {
        **text = data.prompt.clone();
    }
    let progress_line = format!("Question {}/{}", data.idx + 1, data.total);
    let mut pl = world.query_filtered::<&mut Text, With<QuizProgressLine>>();
    for mut text in pl.iter_mut(world) {
        **text = progress_line.clone();
    }
    let mut fb =
        world.query_filtered::<&mut Text, (With<QuizFeedback>, Without<QuizQuestionText>)>();
    for mut text in fb.iter_mut(world) {
        **text = String::new();
    }
    let mut ex =
        world.query_filtered::<&mut Text, (With<QuizExplanation>, Without<QuizQuestionText>)>();
    for mut text in ex.iter_mut(world) {
        **text = String::new();
    }
    // Refresh choice button labels.
    let mut cl = world.query::<(&QuizChoice, &Children)>();
    let pairs: Vec<(usize, Entity)> = cl
        .iter(world)
        .flat_map(|(choice, children)| {
            children
                .iter()
                .map(|child| (choice.0, child))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut tx = world.query_filtered::<&mut Text, With<QuizChoiceLabel>>();
    for (choice_idx, child) in pairs {
        if let Some(label) = data.choices.get(choice_idx) {
            if let Ok(mut text) = tx.get_mut(world, child) {
                **text = format!("{}. {}", (b'A' + choice_idx as u8) as char, label);
            }
        }
    }
}
