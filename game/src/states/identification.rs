//! Cable identification console for Ondine's coax track (Astra §2b).
//!
//! When a level carries `LevelDef::identification`, the closet holds
//! several candidate homeruns and exactly one is the work order's
//! service run. The player attaches the ID remote at the customer
//! drop, tests candidates one at a time (each test reveals that
//! candidate's mapper ID), and selects the run. Selecting the service
//! run passes the Identity state, which joins the win gate in
//! `playing::check_win_condition` / `outage::check_outage_resolution`.
//!
//! The trap is authored, not simulated: one *wrong* candidate carries
//! a handwritten label naming the right suite. Testing before
//! attaching the remote reads nothing (the report's isolation
//! procedure), wrong selections are logged for the Diagnosis badge,
//! and disconnecting a run that is not the service run counts a
//! neighbor disruption (optional objective + Workmanship badge).
//!
//! Identification is a state machine over authored data — there is
//! no simulated mapper instrument (spec §2b scope honesty).

use bevy::prelude::*;

use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY_BOLD, FONT_SIZE_ADJUST};
use crate::level::{identification_is_well_formed, IdentificationDef, LevelDef, StateStatus};
use crate::states::GameState;

/// The player's identification work on the current level.
#[derive(Resource, Debug, Default)]
pub struct IdentificationProgress {
    /// The ID remote is attached at the customer drop. Testing
    /// before attaching reads nothing.
    pub remote_attached: bool,
    /// Candidate ids tested so far, in test order (deduplicated).
    pub tested: Vec<String>,
    /// The currently selected run's candidate id, if any.
    pub selected: Option<String>,
    /// Selections of a run that is not the service run.
    pub wrong_selections: u32,
    /// Disconnects of a run that is not the service run.
    pub neighbor_disruptions: u32,
    /// Candidate ids the player has disconnected, in order.
    pub disconnected: Vec<String>,
}

impl IdentificationProgress {
    /// Attach the ID remote at the customer drop. Returns false when
    /// it was already attached (a no-op, not an error).
    pub fn attach_remote(&mut self) -> bool {
        if self.remote_attached {
            return false;
        }
        self.remote_attached = true;
        true
    }

    /// Test one candidate: returns the mapper ID it reads, or `None`
    /// when no remote is attached (nothing reads) or the candidate
    /// does not exist. Repeated tests of one candidate are recorded
    /// once.
    pub fn test_candidate(
        &mut self,
        def: &IdentificationDef,
        candidate_id: &str,
    ) -> Option<String> {
        if !self.remote_attached {
            return None;
        }
        let candidate = def.candidates.iter().find(|c| c.id == candidate_id)?;
        if !self.tested.iter().any(|t| t == candidate_id) {
            self.tested.push(candidate_id.to_string());
        }
        Some(candidate.mapper_id.clone())
    }

    /// Select a run as the service run. Returns true when it is the
    /// authored service run. A wrong selection is logged (Diagnosis
    /// badge input) and becomes the current selection — Identity
    /// fails until the right run is selected instead.
    pub fn select(&mut self, def: &IdentificationDef, candidate_id: &str) -> bool {
        let Some(candidate) = def.candidates.iter().find(|c| c.id == candidate_id) else {
            return false;
        };
        self.selected = Some(candidate_id.to_string());
        if candidate.is_service_run {
            true
        } else {
            self.wrong_selections += 1;
            false
        }
    }

    /// Disconnect a candidate run. Disconnecting anything that is
    /// not the service run disrupts a neighbor and is counted;
    /// disconnecting the service run is the job itself.
    pub fn disconnect(&mut self, def: &IdentificationDef, candidate_id: &str) -> bool {
        let Some(candidate) = def.candidates.iter().find(|c| c.id == candidate_id) else {
            return false;
        };
        if !self.disconnected.iter().any(|d| d == candidate_id) {
            self.disconnected.push(candidate_id.to_string());
        }
        if candidate.is_service_run {
            false
        } else {
            self.neighbor_disruptions += 1;
            true
        }
    }

    /// Identity passes only on well-formed data with the service run
    /// selected — malformed authored data fails closed.
    pub fn identity_passed(&self, def: &IdentificationDef) -> bool {
        if !identification_is_well_formed(def) {
            return false;
        }
        match (
            &self.selected,
            def.candidates.iter().find(|c| c.is_service_run),
        ) {
            (Some(selected), Some(service)) => selected == &service.id,
            _ => false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The Identity state line (§2a) for this progress: status plus the
/// report's feedback wording — a wrong run names what the mapper
/// actually read against what the work order expects.
pub fn identity_state(
    def: &IdentificationDef,
    progress: &IdentificationProgress,
) -> (StateStatus, String) {
    if !identification_is_well_formed(def) {
        return (
            StateStatus::Fail,
            "Identification data is malformed — identity cannot be confirmed on this work order."
                .to_string(),
        );
    }
    let service = def
        .candidates
        .iter()
        .find(|c| c.is_service_run)
        .expect("well-formed data has exactly one service run");
    match &progress.selected {
        None => (
            StateStatus::Pending,
            format!(
                "No homerun selected — attach the remote and test the candidates for {}.",
                def.remote_id
            ),
        ),
        Some(selected) if selected == &service.id => (
            StateStatus::Pass,
            format!(
                "Mapper reads remote {} on {} — identity confirmed.",
                def.remote_id, def.expected_port
            ),
        ),
        Some(selected) => {
            let found = def
                .candidates
                .iter()
                .find(|c| &c.id == selected)
                .map(|c| c.mapper_id.clone())
                .unwrap_or_else(|| "nothing".to_string());
            (
                StateStatus::Fail,
                format!(
                    "Mapper reads remote {found} — the work order's drop is {}.",
                    def.remote_id
                ),
            )
        }
    }
}

/// Marker on the console root node (for teardown).
#[derive(Component)]
struct IdentificationConsoleRoot;

/// Marker on the multi-line status text (remote state + per-run
/// readings). One status text per console keeps the Text queries
/// disjoint from the other consoles (the B0001 lesson).
#[derive(Component)]
struct IdentificationStatusLine;

/// What an identification console button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentAction {
    AttachRemote,
    Test(usize),
    Select(usize),
    Disconnect(usize),
}

/// Marker on each console button; carries its action.
#[derive(Component)]
pub struct IdentificationButton(pub IdentAction);

pub struct IdentificationConsolePlugin;

impl Plugin for IdentificationConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IdentificationProgress>()
            .add_systems(
                Update,
                (handle_identification_buttons, sync_identification_status).run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_identification_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_identification_console);
    }
}

/// Spawn the console when the level has an `identification` block.
pub(crate) fn setup_identification_console(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<IdentificationProgress>,
    asset_server: Res<AssetServer>,
) {
    progress.reset();
    // §2e composition: on the intermittent level the identification
    // mechanic is narrowed to the connections — the intermittent
    // console owns the level, and this console does not spawn.
    if level.intermittent.is_some() {
        return;
    }
    let Some(def) = &level.identification else {
        return;
    };
    let display: Handle<Font> = asset_server.load(DISPLAY_BOLD);
    let body: Handle<Font> = asset_server.load(BODY);
    let body_medium: Handle<Font> = asset_server.load(BODY_MEDIUM);

    commands
        .spawn((
            IdentificationConsoleRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.14, 0.95)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!(
                    "CABLE ID — work order: {} on {}",
                    def.remote_id, def.expected_port
                )),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(15.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.85, 0.85)),
            ));
            root.spawn((
                IdentificationStatusLine,
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
                spawn_ident_button(
                    row,
                    &body_medium,
                    IdentAction::AttachRemote,
                    "Attach remote",
                );
                for (idx, candidate) in def.candidates.iter().enumerate() {
                    spawn_ident_button(
                        row,
                        &body_medium,
                        IdentAction::Test(idx),
                        &format!("Test {}", candidate.closet_label),
                    );
                    spawn_ident_button(
                        row,
                        &body_medium,
                        IdentAction::Select(idx),
                        &format!("Select {}", candidate.closet_label),
                    );
                    spawn_ident_button(
                        row,
                        &body_medium,
                        IdentAction::Disconnect(idx),
                        &format!("Pull {}", candidate.closet_label),
                    );
                }
            });
        });
}

fn spawn_ident_button(
    row: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    action: IdentAction,
    label: &str,
) {
    row.spawn((
        IdentificationButton(action),
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

/// The console's status block: remote state, then one line per
/// candidate — closet label, handwritten label when taped on, the
/// mapper reading once tested, and the selection marker.
pub fn status_text(def: &IdentificationDef, progress: &IdentificationProgress) -> String {
    let mut out = if progress.remote_attached {
        "Remote attached at the drop.".to_string()
    } else {
        "No remote attached — tests read nothing.".to_string()
    };
    for candidate in &def.candidates {
        let mut line = format!("\n{} ({})", candidate.closet_label, candidate.id);
        if let Some(hand) = &candidate.handwritten_label {
            line.push_str(&format!(" — handwritten: \"{hand}\""));
        }
        if progress.tested.iter().any(|t| t == &candidate.id) {
            line.push_str(&format!(" — mapper: {}", candidate.mapper_id));
        }
        if progress.selected.as_deref() == Some(candidate.id.as_str()) {
            line.push_str(" — SELECTED");
        }
        if progress.disconnected.iter().any(|d| d == &candidate.id) {
            line.push_str(" — pulled");
        }
        out.push_str(&line);
    }
    out.push_str(&format!(
        "\nWrong picks: {} · Neighbor disruptions: {}",
        progress.wrong_selections, progress.neighbor_disruptions
    ));
    out
}

fn handle_identification_buttons(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<IdentificationProgress>,
    buttons: Query<
        (&Interaction, &IdentificationButton),
        (
            Changed<Interaction>,
            Without<crate::states::api_console::ApiButton>,
            Without<crate::states::triage_console::TriageButton>,
            Without<crate::states::quiz::QuizChoice>,
        ),
    >,
    sfx: Res<crate::audio::Sfx>,
    mut reaction_inbox: Option<ResMut<crate::waifu::reactions::ReactionInbox>>,
) {
    let Some(def) = &level.identification else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let mut ok = true;
        match button.0 {
            IdentAction::AttachRemote => {
                progress.attach_remote();
            }
            IdentAction::Test(idx) => {
                if let Some(candidate) = def.candidates.get(idx) {
                    let id = candidate.id.clone();
                    ok = progress.test_candidate(def, &id).is_some();
                }
            }
            IdentAction::Select(idx) => {
                if let Some(candidate) = def.candidates.get(idx) {
                    let id = candidate.id.clone();
                    ok = progress.select(def, &id);
                    if !ok {
                        if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
                            inbox.push(
                                crate::waifu::reactions::ReactionTrigger::IdentificationWrong,
                            );
                        }
                    }
                }
            }
            IdentAction::Disconnect(idx) => {
                if let Some(candidate) = def.candidates.get(idx) {
                    let id = candidate.id.clone();
                    progress.disconnect(def, &id);
                }
            }
        }
        if ok {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
        } else {
            sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
        }
    }
}

fn sync_identification_status(
    level: Res<LevelDef>,
    progress: Res<IdentificationProgress>,
    mut status: Query<&mut Text, With<IdentificationStatusLine>>,
) {
    let Some(def) = &level.identification else {
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

fn cleanup_identification_console(
    mut commands: Commands,
    roots: Query<Entity, With<IdentificationConsoleRoot>>,
    mut progress: ResMut<IdentificationProgress>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    progress.reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::IdentificationCandidate;

    fn def() -> IdentificationDef {
        IdentificationDef {
            remote_id: "ID-7".into(),
            expected_port: "Port 3".into(),
            candidates: vec![
                IdentificationCandidate {
                    id: "run-a".into(),
                    closet_label: "Port 1".into(),
                    handwritten_label: None,
                    is_service_run: false,
                    mapper_id: "ID-2".into(),
                },
                IdentificationCandidate {
                    id: "run-b".into(),
                    closet_label: "Port 2".into(),
                    handwritten_label: Some("Suite 204 drop".into()),
                    is_service_run: false,
                    mapper_id: "ID-5".into(),
                },
                IdentificationCandidate {
                    id: "run-c".into(),
                    closet_label: "Port 3".into(),
                    handwritten_label: None,
                    is_service_run: true,
                    mapper_id: "ID-7".into(),
                },
                IdentificationCandidate {
                    id: "run-d".into(),
                    closet_label: "Port 4".into(),
                    handwritten_label: None,
                    is_service_run: false,
                    mapper_id: "ID-9".into(),
                },
            ],
        }
    }

    #[test]
    fn testing_before_attaching_the_remote_reads_nothing() {
        // Adversarial (spec Phase 2): the isolation procedure comes
        // first — no remote, no readings, nothing recorded.
        let mut p = IdentificationProgress::default();
        let d = def();
        assert_eq!(p.test_candidate(&d, "run-c"), None);
        assert!(p.tested.is_empty());
        assert!(p.attach_remote());
        assert!(!p.attach_remote(), "re-attaching is a no-op");
        assert_eq!(p.test_candidate(&d, "run-c"), Some("ID-7".to_string()));
        assert_eq!(p.tested, vec!["run-c".to_string()]);
        // Re-testing records once.
        assert_eq!(p.test_candidate(&d, "run-c"), Some("ID-7".to_string()));
        assert_eq!(p.tested.len(), 1);
        // Unknown candidates read nothing.
        assert_eq!(p.test_candidate(&d, "run-zzz"), None);
    }

    #[test]
    fn the_handwritten_label_candidate_is_the_trap() {
        let mut p = IdentificationProgress::default();
        let d = def();
        p.attach_remote();
        // run-b wears the handwritten "Suite 204 drop" label — and
        // is not the service run. Selecting it is logged and fails
        // Identity with the mapper-vs-work-order feedback string.
        assert!(!p.select(&d, "run-b"));
        assert_eq!(p.wrong_selections, 1);
        assert!(!p.identity_passed(&d));
        let (status, feedback) = identity_state(&d, &p);
        assert_eq!(status, StateStatus::Fail);
        assert_eq!(
            feedback,
            "Mapper reads remote ID-5 — the work order's drop is ID-7."
        );
        // Recovering is always possible: test and select the real run.
        assert_eq!(p.test_candidate(&d, "run-c"), Some("ID-7".to_string()));
        assert!(p.select(&d, "run-c"));
        assert!(p.identity_passed(&d));
        let (status, _) = identity_state(&d, &p);
        assert_eq!(status, StateStatus::Pass);
        // The wrong pick stays on the record (Diagnosis badge input).
        assert_eq!(p.wrong_selections, 1);
    }

    #[test]
    fn neighbor_disruptions_count_only_non_service_disconnects() {
        let mut p = IdentificationProgress::default();
        let d = def();
        assert!(
            p.disconnect(&d, "run-a"),
            "pulling a neighbor's run disrupts them"
        );
        assert_eq!(p.neighbor_disruptions, 1);
        assert!(
            !p.disconnect(&d, "run-c"),
            "pulling the service run is the job"
        );
        assert_eq!(p.neighbor_disruptions, 1);
        assert!(!p.disconnect(&d, "run-zzz"));
        assert_eq!(p.neighbor_disruptions, 1);
    }

    #[test]
    fn malformed_identification_data_fails_closed() {
        // Adversarial: two service runs (or a label on the service
        // run) can never yield an Identity pass.
        let mut d = def();
        d.candidates[0].is_service_run = true;
        let mut p = IdentificationProgress::default();
        p.attach_remote();
        assert!(p.select(&d, "run-a"));
        assert!(!p.identity_passed(&d));
        let (status, _) = identity_state(&d, &p);
        assert_eq!(status, StateStatus::Fail);

        let mut d = def();
        d.candidates[2].handwritten_label = Some("Suite 204 drop".into());
        d.candidates[1].handwritten_label = None;
        assert!(!crate::level::identification_is_well_formed(&d));
    }

    #[test]
    fn reset_clears_the_attempt() {
        let mut p = IdentificationProgress::default();
        let d = def();
        p.attach_remote();
        p.test_candidate(&d, "run-b");
        p.select(&d, "run-b");
        p.disconnect(&d, "run-a");
        p.reset();
        assert!(!p.remote_attached);
        assert!(p.tested.is_empty());
        assert!(p.selected.is_none());
        assert_eq!(p.wrong_selections, 0);
        assert_eq!(p.neighbor_disruptions, 0);
    }
}
