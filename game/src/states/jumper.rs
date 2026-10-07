//! Defective-jumper console for the C3 anchor (Astra §2c, c1l5).
//!
//! One fixed plant piece is a degraded jumper: while it is in the
//! live graph, the Service (CNR) state is measured against a floor
//! raised by the authored `floor_penalty_dbmv`, so a board state
//! exists that passes Continuity and Carrier Level yet fails Service
//! permanently — no amp choice repairs it, because gain raises the
//! carrier and the penalty sits on the floor it is measured against
//! (proven by the exhaustive sweep test below). The fix is
//! substitution: this console swaps in the known-good spare from the
//! kit. The board and the evaluators never learn any of it — the
//! penalty lives in `LevelDef::coax_noise_floor_with_defect` and the
//! win gate in `crate::astra`.
//!
//! The console also carries the *compare beat*: projecting the CNR
//! with and without the defect before swapping is the evidence-led
//! diagnosis the Diagnosis badge rewards (with the watcher-observed
//! service failure in `crate::astra`).

use bevy::prelude::*;

use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY_BOLD, FONT_SIZE_ADJUST};
use crate::level::{DefectiveEdgeDef, LevelDef};
use crate::states::GameState;

/// The player's defective-jumper work on the current level.
#[derive(Resource, Debug, Default)]
pub struct JumperProgress {
    /// The known-good spare has been swapped in; the defect is out
    /// of the plant and the Service state runs on the clean floor.
    pub swapped: bool,
    /// The player ran the compare beat before swapping.
    pub compared: bool,
    /// Swap actions taken (a second swap is a no-op but is counted
    /// when attempted through the console — fiddling is telemetry).
    pub swap_attempts: u32,
}

impl JumperProgress {
    /// Run the compare beat: project the service state with and
    /// without the defect. Purely observational — always allowed.
    pub fn compare(&mut self) -> bool {
        if self.compared {
            return false;
        }
        self.compared = true;
        true
    }

    /// Swap in the known-good spare. Returns true the first time;
    /// later attempts are counted and change nothing.
    pub fn swap_in_spare(&mut self) -> bool {
        self.swap_attempts += 1;
        if self.swapped {
            return false;
        }
        self.swapped = true;
        true
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Marker on the console root node (for teardown).
#[derive(Component)]
struct JumperConsoleRoot;

/// Marker on the console's single multi-line status text.
#[derive(Component)]
struct JumperStatusLine;

/// What a jumper console button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumperAction {
    Compare,
    SwapSpare,
}

/// Marker on each console button; carries its action.
#[derive(Component)]
pub struct JumperButton(pub JumperAction);

pub struct JumperConsolePlugin;

impl Plugin for JumperConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JumperProgress>()
            .add_systems(
                Update,
                (handle_jumper_buttons, sync_jumper_status).run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_jumper_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_jumper_console);
    }
}

/// Spawn the console when the level carries a `defective_edge`.
pub(crate) fn setup_jumper_console(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<JumperProgress>,
    asset_server: Res<AssetServer>,
) {
    progress.reset();
    let Some(def) = &level.defective_edge else {
        return;
    };
    let display: Handle<Font> = asset_server.load(DISPLAY_BOLD);
    let body: Handle<Font> = asset_server.load(BODY);
    let body_medium: Handle<Font> = asset_server.load(BODY_MEDIUM);

    commands
        .spawn((
            JumperConsoleRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                right: Val::Px(12.0),
                width: Val::Px(340.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.14, 0.95)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("PLANT INSPECTION — JUMPER BAY"),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(15.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.65, 0.0)),
            ));
            root.spawn((
                JumperStatusLine,
                Text::new(status_text(def, &progress)),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(13.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                spawn_jumper_button(row, &body_medium, JumperAction::Compare, "Compare CNR");
                spawn_jumper_button(
                    row,
                    &body_medium,
                    JumperAction::SwapSpare,
                    &format!("Swap spare: {}", def.spare_label),
                );
            });
        });
}

fn spawn_jumper_button(
    row: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    action: JumperAction,
    label: &str,
) {
    row.spawn((
        JumperButton(action),
        Button,
        Node {
            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.12, 0.18, 0.30)),
    ))
    .with_children(|btn| {
        btn.spawn((
            Text::new(label.to_string()),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

/// The console's status block: what the evidence says, and what has
/// been done about it.
pub fn status_text(def: &DefectiveEdgeDef, progress: &JumperProgress) -> String {
    let state = if progress.swapped {
        format!(
            "Spare in place ({}) — the degraded jumper is out of the plant.",
            def.spare_label
        )
    } else {
        format!(
            "Supplied jumper on the {}->{} slot reads degraded (+{:.0} dBmV on the floor). Continuity and level can pass while service fails — gain cannot fix a floor problem.",
            def.from, def.to, def.floor_penalty_dbmv
        )
    };
    let compare = if progress.compared {
        "Compare beat run: service was projected with and without the defect."
    } else {
        "Compare beat not run yet."
    };
    format!("{state}\n{compare}")
}

fn handle_jumper_buttons(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<JumperProgress>,
    buttons: Query<
        (&Interaction, &JumperButton),
        (
            Changed<Interaction>,
            Without<crate::states::api_console::ApiButton>,
            Without<crate::states::triage_console::TriageButton>,
            Without<crate::states::quiz::QuizChoice>,
            Without<crate::states::identification::IdentificationButton>,
        ),
    >,
    sfx: Res<crate::audio::Sfx>,
) {
    if level.defective_edge.is_none() {
        return;
    }
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let ok = match button.0 {
            JumperAction::Compare => progress.compare(),
            JumperAction::SwapSpare => progress.swap_in_spare(),
        };
        if ok {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
        } else {
            sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
        }
    }
}

fn sync_jumper_status(
    level: Res<LevelDef>,
    progress: Res<JumperProgress>,
    mut status: Query<&mut Text, With<JumperStatusLine>>,
) {
    let Some(def) = &level.defective_edge else {
        return;
    };
    if !progress.is_changed() {
        return;
    }
    let text = status_text(def, &progress);
    for mut line in &mut status {
        **line = text.clone();
    }
}

fn cleanup_jumper_console(
    mut commands: Commands,
    roots: Query<Entity, With<JumperConsoleRoot>>,
    mut progress: ResMut<JumperProgress>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    progress.reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{load_level, MechanicStates, StateId, StateStatus};
    use osp_sim::{Component, Outage, PathGraph, Wavelength};

    /// Build c1l5's live graph with one placed amp choice, mirroring
    /// the level-test helper (fixed plant + the placed slot).
    fn graph_with_amp(level: &LevelDef, choice: &crate::level::ComponentChoice) -> PathGraph {
        let mut graph = PathGraph::default();
        for node in &level.nodes {
            graph.add_node(node.id, node.label.clone());
        }
        for edge in &level.fixed_edges {
            graph.connect(edge.from, edge.to, edge.component.clone());
        }
        graph.connect(choice.from, choice.to, choice.component.clone());
        graph
    }

    fn service_status(
        level: &LevelDef,
        graph: &PathGraph,
        outage: Option<&Outage>,
        defect_present: bool,
    ) -> StateStatus {
        let mechanics = MechanicStates {
            service_defect_present: defect_present,
            ..MechanicStates::default()
        };
        let lines = level.verification_states(
            graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            outage,
            &mechanics,
        );
        lines
            .iter()
            .find(|l| l.id == StateId::ServiceCnr)
            .expect("coax levels carry a Service state")
            .status
    }

    /// THE Phase 3 hard gate: with the defect in place, no available
    /// amp at any point of the ingress accrual timeline ever passes
    /// Service — and the same sweep after substitution passes, with
    /// the pinned clean-path arithmetic (CNR 28.5 at full accrual)
    /// untouched.
    #[test]
    fn defect_in_place_service_never_passes_the_exhaustive_sweep() {
        let level = load_level(14); // c1l5
        assert!(
            level.defective_edge.is_some(),
            "c1l5 must carry the defective edge"
        );
        for choice in &level.available_components {
            let graph = graph_with_amp(&level, choice);
            for tick_secs in (0..=600).step_by(30) {
                let mut storm = Outage::new(
                    osp_sim::OutageKind::IngressNoise,
                    level.scripted_outage.as_ref().unwrap().edge_from,
                    level.scripted_outage.as_ref().unwrap().edge_to,
                );
                storm.tick(tick_secs as f64);
                assert_ne!(
                    service_status(&level, &graph, Some(&storm), true),
                    StateStatus::Pass,
                    "amp {choice:?} at {tick_secs}s accrual must never pass Service with the defect in place"
                );
            }
        }

        // After substitution the winner passes — at full accrual too.
        let winner = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 5.0))
            .expect("c1l5 must offer a 5 dB amp");
        let graph = graph_with_amp(&level, winner);
        let mut storm = Outage::new(
            osp_sim::OutageKind::IngressNoise,
            level.scripted_outage.as_ref().unwrap().edge_from,
            level.scripted_outage.as_ref().unwrap().edge_to,
        );
        storm.tick(600.0);
        assert_eq!(
            service_status(&level, &graph, Some(&storm), false),
            StateStatus::Pass,
            "the substituted path must pass Service at full accrual"
        );
        // The pinned arithmetic itself: clean floor, CNR 28.5.
        let eval = graph
            .evaluate_coax(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                level.receive_window(),
                level.coax_noise_floor_with_defect(Some(&storm), &MechanicStates::default()),
                25.0,
            )
            .expect("c1l5 path must resolve");
        assert!(
            (eval.carrier_to_noise_db - 28.5).abs() < 0.01,
            "clean-path CNR at full accrual must stay 28.5, got {}",
            eval.carrier_to_noise_db
        );
    }

    #[test]
    fn the_trap_state_exists_with_the_defect_in_place() {
        // Continuity Pass + Carrier Level Pass + Service Fail, at
        // rest, on the shipped winner — the C3 readout itself.
        let level = load_level(14);
        let winner = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 5.0))
            .expect("c1l5 must offer a 5 dB amp");
        let graph = graph_with_amp(&level, winner);
        let mechanics = MechanicStates {
            service_defect_present: true,
            ..MechanicStates::default()
        };
        let lines = level.verification_states(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
            &mechanics,
        );
        let status = |id: StateId| lines.iter().find(|l| l.id == id).unwrap().status;
        assert_eq!(status(StateId::Continuity), StateStatus::Pass);
        assert_eq!(status(StateId::CarrierLevel), StateStatus::Pass);
        assert_eq!(status(StateId::ServiceCnr), StateStatus::Fail);
    }

    #[test]
    fn swap_and_compare_are_recorded_once() {
        let mut p = JumperProgress::default();
        assert!(p.compare());
        assert!(!p.compare(), "the compare beat records once");
        assert!(p.swap_in_spare());
        assert!(p.swapped);
        assert!(!p.swap_in_spare(), "a second swap changes nothing");
        assert_eq!(p.swap_attempts, 2);
        p.reset();
        assert!(!p.swapped && !p.compared && p.swap_attempts == 0);
    }
}
