//! Multi-point survey + historical diagnosis console (Astra §2d).
//!
//! The survey itself is not a console interaction: per-point readings
//! are derived from the live graph by `LevelDef::survey_point_reading`
//! (N pure evaluator calls), rendered here and in the §2a Coverage
//! states. What the player *does* here is the diagnosis pick on
//! levels that author one (the W2 anchor): name what moved between
//! the historical survey and today. The pick never gates the board —
//! the hot-swap still wins or loses on physics (§3 guardrail: history
//! informs, never requires screen time) — it is scored for the
//! Diagnosis badge, and the authored trap pick fires Linka's
//! `diagnosis_wrong` reaction.

use bevy::prelude::*;

use crate::level::{DiagnosisDef, LevelDef};
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::GameState;
use crate::waifu::reactions::{ReactionInbox, ReactionTrigger};

/// Player-side survey state: the diagnosis pick. Readings are never
/// stored — they are recomputed from the live graph every frame.
#[derive(Resource, Debug, Default)]
pub struct SurveyProgress {
    /// The committed diagnosis option index, once picked.
    pub diagnosis_pick: Option<usize>,
}

impl SurveyProgress {
    pub fn reset(&mut self) {
        self.diagnosis_pick = None;
    }

    /// Commit a diagnosis pick. Rejected (returns false) when no
    /// diagnosis is authored or the index is out of range — a pick
    /// can never be recorded against malformed data. Re-picking
    /// replaces the pick (the tech can change their mind before the
    /// level ends; the badge scores the final pick).
    pub fn pick_diagnosis(&mut self, def: &DiagnosisDef, index: usize) -> bool {
        if index >= def.options.len() {
            return false;
        }
        self.diagnosis_pick = Some(index);
        true
    }

    /// The pick names the authored cause.
    pub fn diagnosis_correct(&self, def: &DiagnosisDef) -> bool {
        self.diagnosis_pick == Some(def.correct)
    }

    /// The pick fell for the authored trap (e.g. "wrong channel").
    pub fn trap_picked(&self, def: &DiagnosisDef) -> bool {
        def.trap_option.is_some() && self.diagnosis_pick == def.trap_option
    }
}

/// Marker for the survey console root (despawn cleanup).
#[derive(Component)]
pub struct SurveyConsole;

/// Marker for survey console buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SurveyButton(pub SurveyAction);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SurveyAction {
    PickDiagnosis(usize),
}

/// Status text entity (single multi-line text, api-console idiom).
#[derive(Component)]
pub struct SurveyStatusText;

/// Survey + diagnosis console plugin.
pub struct SurveyConsolePlugin;

impl Plugin for SurveyConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SurveyProgress>()
            .add_systems(OnEnter(GameState::Playing), reset_survey_progress)
            .add_systems(
                Update,
                (handle_survey_clicks, refresh_survey_console).run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_survey_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_survey_console);
    }
}

fn reset_survey_progress(mut progress: ResMut<SurveyProgress>) {
    progress.reset();
}

/// The console's status block: per-point readings against their
/// requirements and the historical survey, then the diagnosis state.
/// Pure formatting over live data (unit-tested).
pub fn status_text(
    level: &LevelDef,
    graph: &osp_sim::PathGraph,
    tx_dbm: f64,
    outage: Option<&osp_sim::Outage>,
    progress: &SurveyProgress,
) -> Option<String> {
    let survey = level.survey.as_ref()?;
    let mut out = String::from("Site survey — measured now vs the last survey:\n");
    for point in &survey.points {
        let reading = level.survey_point_reading(graph, tx_dbm, outage, point);
        let now = match reading {
            Some((rssi, snr)) => format!("{:.1} dBm / SNR {:.1}", rssi, snr),
            None => "no path".to_string(),
        };
        let verdict = match reading {
            Some((rssi, snr)) => {
                let snr_req = point
                    .required_snr_db
                    .unwrap_or_else(|| level.min_snr_db.unwrap_or(10.0));
                if rssi >= point.required_rssi_dbm && snr >= snr_req {
                    "PASS"
                } else {
                    "FAIL"
                }
            }
            None => "FAIL",
        };
        out.push_str(&format!(
            "{}: {} (needs {:.1} dBm) — was {:.1} dBm … {}\n",
            point.label, now, point.required_rssi_dbm, point.historical_rssi_dbm, verdict
        ));
    }
    if let Some(diagnosis) = &survey.diagnosis {
        out.push_str("Diagnosis — what changed since the last survey?\n");
        for (i, option) in diagnosis.options.iter().enumerate() {
            let mark = if progress.diagnosis_pick == Some(i) {
                " [picked]"
            } else {
                ""
            };
            out.push_str(&format!("{}. {}{}\n", i + 1, option, mark));
        }
        match progress.diagnosis_pick {
            None => out.push_str("No diagnosis committed yet."),
            Some(pick) if Some(pick) == diagnosis.trap_option => {
                out.push_str("Diagnosis committed — Linka is shaking her head.");
            }
            Some(_) => out.push_str("Diagnosis committed."),
        }
    }
    Some(out)
}

/// Spawn the survey console (called from the `OnEnter(Playing)`
/// chain after `setup_level`). No-op without a survey block.
pub fn setup_survey_console(mut commands: Commands, level: Option<Res<LevelDef>>) {
    let Some(level) = level else {
        return;
    };
    let Some(survey) = &level.survey else {
        return;
    };
    let root = commands
        .spawn((
            SurveyConsole,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(300.0),
                width: Val::Px(400.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.92)),
        ))
        .id();
    let status = commands
        .spawn((
            SurveyStatusText,
            Text::new("Site survey"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(root).add_child(status);
    if let Some(diagnosis) = &survey.diagnosis {
        for index in 0..diagnosis.options.len() {
            let label = format!("Diagnose: {}", diagnosis.options[index]);
            let button = commands
                .spawn((
                    SurveyButton(SurveyAction::PickDiagnosis(index)),
                    Button,
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.16, 0.2, 0.32, 1.0)),
                ))
                .with_child((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ))
                .id();
            commands.entity(root).add_child(button);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_survey_clicks(
    level: Option<Res<LevelDef>>,
    mut progress: ResMut<SurveyProgress>,
    buttons: Query<
        (&Interaction, &SurveyButton),
        (
            Changed<Interaction>,
            Without<super::identification::IdentificationButton>,
            Without<super::jumper::JumperButton>,
            Without<super::api_console::ApiButton>,
            Without<super::triage_console::TriageButton>,
        ),
    >,
    mut reaction_inbox: Option<ResMut<ReactionInbox>>,
) {
    let Some(level) = level else {
        return;
    };
    let Some(diagnosis) = level.survey.as_ref().and_then(|s| s.diagnosis.as_ref()) else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let SurveyAction::PickDiagnosis(index) = button.0;
        let was_unpicked = progress.diagnosis_pick.is_none();
        if progress.pick_diagnosis(diagnosis, index)
            && was_unpicked
            && !progress.diagnosis_correct(diagnosis)
        {
            if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
                inbox.push(ReactionTrigger::DiagnosisWrong);
            }
        }
    }
}

fn refresh_survey_console(
    level: Option<Res<LevelDef>>,
    live: Res<LiveGraph>,
    active_outage: Res<ActiveOutage>,
    progress: Res<SurveyProgress>,
    mut query: Query<&mut Text, With<SurveyStatusText>>,
) {
    let Some(level) = level else {
        return;
    };
    let Some(text) = status_text(
        &level,
        &live.graph,
        live.tx_dbm,
        active_outage.outage.as_ref(),
        &progress,
    ) else {
        return;
    };
    for mut t in &mut query {
        if t.0 != text {
            t.0.clone_from(&text);
        }
    }
}

fn cleanup_survey_console(mut commands: Commands, query: Query<Entity, With<SurveyConsole>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{DiagnosisDef, SurveyDef, SurveyPointDef};

    fn diagnosis() -> DiagnosisDef {
        DiagnosisDef {
            options: vec![
                "New obstruction".into(),
                "Interference — the noise floor rose".into(),
                "AP fault".into(),
                "Wrong channel".into(),
            ],
            correct: 1,
            trap_option: Some(3),
        }
    }

    #[test]
    fn diagnosis_pick_scores_correct_and_trap() {
        let def = diagnosis();
        let mut progress = SurveyProgress::default();
        assert!(progress.pick_diagnosis(&def, 1));
        assert!(progress.diagnosis_correct(&def));
        assert!(!progress.trap_picked(&def));
        // Re-picking replaces the pick.
        assert!(progress.pick_diagnosis(&def, 3));
        assert!(progress.trap_picked(&def));
        assert!(!progress.diagnosis_correct(&def));
    }

    #[test]
    fn diagnosis_pick_rejects_out_of_range_indices() {
        // Adversarial: a pick can never be recorded against
        // malformed data.
        let def = diagnosis();
        let mut progress = SurveyProgress::default();
        assert!(!progress.pick_diagnosis(&def, 4));
        assert!(!progress.pick_diagnosis(&def, usize::MAX));
        assert_eq!(progress.diagnosis_pick, None);
    }

    #[test]
    fn shipped_survey_data_is_consistent() {
        // The shipped survey levels: m1l9's gate data, m1l2's
        // diagnosis, m1l8's display point.
        let load = |id: &str| -> LevelDef {
            crate::level::LEVEL_SOURCES
                .iter()
                .map(|s| serde_json::from_str(s).unwrap())
                .find(|l: &LevelDef| l.id == id)
                .unwrap()
        };
        let m1l9 = load("m1l9");
        let survey = m1l9.survey.as_ref().expect("m1l9 authors a survey");
        assert!(survey.required_for_win, "m1l9's survey gates the win");
        assert_eq!(survey.points.len(), 2);
        let m1l2 = load("m1l2");
        let d = m1l2
            .survey
            .as_ref()
            .and_then(|s| s.diagnosis.as_ref())
            .expect("m1l2 authors a diagnosis");
        assert!(d.correct < d.options.len());
        assert_eq!(d.trap_option, Some(3));
        let m1l8 = load("m1l8");
        assert!(m1l8.survey.is_some(), "m1l8 authors survey points");
    }

    #[test]
    fn m1l9_winner_passes_the_survey_gate_and_losers_fail() {
        // The anchor's whole point: the shipped winning build passes
        // every point; the weaker repeater fails points by name.
        let level: LevelDef = crate::level::LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .find(|l: &LevelDef| l.id == "m1l9")
            .unwrap();
        let survey: &SurveyDef = level.survey.as_ref().unwrap();
        let graph_for = |tx: f64| {
            let choice = level
                .available_components
                .iter()
                .find(|c| matches!(c.component, osp_sim::Component::Repeater { tx_dbm } if tx_dbm == tx))
                .unwrap();
            let mut graph = osp_sim::PathGraph::default();
            for node in &level.nodes {
                graph.add_node(node.id, node.label.clone());
            }
            for edge in &level.fixed_edges {
                graph.connect(edge.from, edge.to, edge.component.clone());
            }
            graph.connect(choice.from, choice.to, choice.component.clone());
            graph
        };
        let winner = graph_for(20.0);
        assert!(level.survey_points_pass(&winner, level.tx_dbm, None));
        assert!(crate::astra::survey_gate_pass(
            &level,
            &winner,
            level.tx_dbm,
            None
        ));
        let loser = graph_for(15.0);
        assert!(!level.survey_points_pass(&loser, level.tx_dbm, None));
        assert!(!crate::astra::survey_gate_pass(
            &level,
            &loser,
            level.tx_dbm,
            None
        ));
        // The failing points are named in the status text.
        let text = status_text(
            &level,
            &loser,
            level.tx_dbm,
            None,
            &SurveyProgress::default(),
        )
        .expect("survey text");
        assert!(text.contains("Front Desk"), "{text}");
        assert!(text.contains("Stockroom"), "{text}");
        assert!(text.contains("FAIL"), "{text}");
        let _ = survey;
    }

    #[test]
    fn empty_required_survey_fails_closed() {
        // Adversarial: a required survey with no points can never
        // pass (fail-closed, like an unresolvable point).
        let mut level: LevelDef = crate::level::LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .find(|l: &LevelDef| l.id == "m1l9")
            .unwrap();
        level.survey = Some(SurveyDef {
            points: vec![],
            diagnosis: None,
            required_for_win: true,
        });
        let graph = osp_sim::PathGraph::default();
        assert!(!crate::astra::survey_gate_pass(
            &level,
            &graph,
            level.tx_dbm,
            None
        ));
        let _ = SurveyPointDef {
            id: "x".into(),
            label: "x".into(),
            node: 0,
            extra_loss_db: 0.0,
            required_rssi_dbm: 0.0,
            required_snr_db: None,
            historical_rssi_dbm: 0.0,
            historical_snr_db: None,
        };
    }
}
