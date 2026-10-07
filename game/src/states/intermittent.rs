//! Intermittent-connection console (Astra §2b/§2e composition, c1l6).
//!
//! The drop drops when the cabinet is disturbed. The console runs
//! the field method as a strict state machine:
//! reproduce (disturb the cabinet) → inspect the path's connections
//! (the loose fitting names itself) → tighten it to the card →
//! re-verify (disturb again; it must hold). Every step requires the
//! previous one — out-of-order actions are rejected, never recorded.
//! Re-verifying an unrepaired path fails deterministically: the
//! fitting is loose, disturbance drops the sync, every time.
//!
//! Winning (in `outage::check_outage_resolution`): the repair path
//! (repaired + reverified + the primary build passing on a graph
//! with the cut lifted) OR the shipped backup path — building the
//! backup edge instead stays a valid clear that forfeits the
//! Diagnosis badge (recorded in attempt telemetry).

use bevy::prelude::*;

use crate::level::{intermittent_is_well_formed, IntermittentDef, LevelDef};
use crate::states::GameState;

/// Player-side intermittent state (see module docs for the order).
#[derive(Resource, Debug, Default)]
pub struct IntermittentProgress {
    /// The drop was reproduced under disturbance.
    pub reproduced: bool,
    /// The connections were inspected (the loose one is known).
    pub inspected: bool,
    /// The loose fitting was tightened to the card.
    pub repaired: bool,
    /// The post-repair disturbance held (re-verified).
    pub reverified: bool,
    /// Times a re-verify was attempted on an unrepaired path and
    /// failed, deterministically (telemetry / Diagnosis evidence).
    pub failed_reverifies: u32,
}

impl IntermittentProgress {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Step 1: disturb the cabinet and watch the drop fall.
    pub fn reproduce(&mut self) -> bool {
        if self.reproduced {
            return false;
        }
        self.reproduced = true;
        true
    }

    /// Step 2: inspect the path's connections. Requires the drop to
    /// be reproduced first — you cannot find an intermittent by
    /// looking at a quiet plant. Returns the loose connection's
    /// label from the authored data.
    pub fn inspect(&mut self, def: &IntermittentDef) -> Option<String> {
        if !self.reproduced || self.inspected {
            return None;
        }
        if !intermittent_is_well_formed(def) {
            return None;
        }
        self.inspected = true;
        def.connections
            .iter()
            .find(|c| c.is_loose)
            .map(|c| c.label.clone())
    }

    /// Step 3: tighten the loose fitting to the card reference.
    /// Requires inspection — tightening blind is how fittings get
    /// over-torqued and drops get worse.
    pub fn repair(&mut self, def: &IntermittentDef) -> bool {
        if !self.inspected || self.repaired {
            return false;
        }
        if !intermittent_is_well_formed(def) {
            return false;
        }
        self.repaired = true;
        true
    }

    /// Step 4: disturb the cabinet again. On a repaired path the
    /// drop holds and the repair is verified. On an unrepaired path
    /// the re-verify fails deterministically (counted, state
    /// unchanged) — the loose fitting does not heal between shakes.
    pub fn reverify(&mut self) -> bool {
        if !self.reproduced || self.reverified {
            return false;
        }
        if !self.repaired {
            self.failed_reverifies = self.failed_reverifies.saturating_add(1);
            return false;
        }
        self.reverified = true;
        true
    }

    /// The repair path's gate fact.
    pub fn repair_path_complete(&self) -> bool {
        self.repaired && self.reverified
    }
}

/// Marker for the intermittent console root (despawn cleanup).
#[derive(Component)]
pub struct IntermittentConsole;

/// Marker for intermittent console buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct IntermittentButton(pub IntermittentAction);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntermittentAction {
    Reproduce,
    Inspect,
    Repair,
    Reverify,
}

/// Status text entity (single multi-line text, api-console idiom).
#[derive(Component)]
pub struct IntermittentStatusText;

/// Intermittent console plugin.
pub struct IntermittentConsolePlugin;

impl Plugin for IntermittentConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IntermittentProgress>()
            .add_systems(OnEnter(GameState::Playing), reset_intermittent_progress)
            .add_systems(
                Update,
                (handle_intermittent_clicks, refresh_intermittent_console).run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_intermittent_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_intermittent_console);
    }
}

fn reset_intermittent_progress(mut progress: ResMut<IntermittentProgress>) {
    progress.reset();
}

/// The console's status block: the method, the stage, the card.
/// Pure formatting (unit-tested).
pub fn status_text(level: &LevelDef, progress: &IntermittentProgress) -> Option<String> {
    let def = level.intermittent.as_ref()?;
    let mut out = String::from(
        "Intermittent drop — the G.fast sync blinks out whenever the cabinet is disturbed.\n",
    );
    out.push_str(&format!("Repair card: {}\n", def.repair_reference));
    out.push_str("Connections on the primary run:\n");
    for conn in &def.connections {
        let note = if progress.inspected && conn.is_loose {
            " — LOOSE (found at inspection)"
        } else if progress.inspected {
            " — sound"
        } else {
            ""
        };
        out.push_str(&format!("  • {}{}\n", conn.label, note));
    }
    let stage = if progress.reverified {
        "Re-verified: the repaired fitting held under disturbance."
    } else if progress.repaired {
        "Fitting tightened. Disturb the cabinet again to re-verify."
    } else if progress.inspected {
        "Loose fitting isolated. Tighten it to the card reference."
    } else if progress.reproduced {
        "Drop reproduced under disturbance. Inspect the connections."
    } else {
        "Disturb the cabinet to reproduce the drop."
    };
    out.push_str(stage);
    if progress.failed_reverifies > 0 {
        out.push_str(&format!(
            "\nFailed re-verifies on the unrepaired path: {}.",
            progress.failed_reverifies
        ));
    }
    Some(out)
}

/// Spawn the intermittent console (called from the `OnEnter(Playing)`
/// chain after `setup_level`). No-op without an intermittent block.
pub fn setup_intermittent_console(mut commands: Commands, level: Option<Res<LevelDef>>) {
    let Some(level) = level else {
        return;
    };
    if level.intermittent.is_none() {
        return;
    }
    let root = commands
        .spawn((
            IntermittentConsole,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(470.0),
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
            IntermittentStatusText,
            Text::new("Intermittent connection"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(root).add_child(status);
    for (action, label) in [
        (IntermittentAction::Reproduce, "Disturb the cabinet"),
        (IntermittentAction::Inspect, "Inspect the connections"),
        (IntermittentAction::Repair, "Tighten the fitting"),
        (IntermittentAction::Reverify, "Disturb again — re-verify"),
    ] {
        let button = commands
            .spawn((
                IntermittentButton(action),
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
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ))
            .id();
        commands.entity(root).add_child(button);
    }
}

fn handle_intermittent_clicks(
    level: Option<Res<LevelDef>>,
    mut progress: ResMut<IntermittentProgress>,
    buttons: Query<
        (&Interaction, &IntermittentButton),
        (
            Changed<Interaction>,
            Without<super::identification::IdentificationButton>,
            Without<super::jumper::JumperButton>,
            Without<super::survey::SurveyButton>,
            Without<super::workbench::WorkbenchButton>,
            Without<super::config_console::ConfigButton>,
            Without<super::api_console::ApiButton>,
            Without<super::triage_console::TriageButton>,
        ),
    >,
) {
    let Some(level) = level else {
        return;
    };
    let Some(def) = &level.intermittent else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.0 {
            IntermittentAction::Reproduce => {
                progress.reproduce();
            }
            IntermittentAction::Inspect => {
                progress.inspect(def);
            }
            IntermittentAction::Repair => {
                progress.repair(def);
            }
            IntermittentAction::Reverify => {
                progress.reverify();
            }
        }
    }
}

fn refresh_intermittent_console(
    level: Option<Res<LevelDef>>,
    progress: Res<IntermittentProgress>,
    mut query: Query<&mut Text, With<IntermittentStatusText>>,
) {
    let Some(level) = level else {
        return;
    };
    let Some(text) = status_text(&level, &progress) else {
        return;
    };
    for mut t in &mut query {
        if t.0 != text {
            t.0.clone_from(&text);
        }
    }
}

fn cleanup_intermittent_console(
    mut commands: Commands,
    query: Query<Entity, With<IntermittentConsole>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LEVEL_SOURCES;

    fn load(id: &str) -> LevelDef {
        LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .find(|l: &LevelDef| l.id == id)
            .unwrap()
    }

    #[test]
    fn the_method_runs_in_order_and_only_in_order() {
        let level = load("c1l6");
        let def = level
            .intermittent
            .as_ref()
            .expect("c1l6 authors intermittent");
        let mut progress = IntermittentProgress::default();
        // Out of order: everything before its prerequisite is
        // rejected and records nothing.
        assert!(progress.inspect(def).is_none());
        assert!(!progress.repair(def));
        assert!(!progress.reverify());
        assert_eq!(
            progress.failed_reverifies, 0,
            "reverify before reproduce is not an attempt"
        );
        // In order:
        assert!(progress.reproduce());
        assert!(!progress.reproduce(), "no double reproduce");
        let found = progress.inspect(def).expect("inspection names the fitting");
        assert!(
            found.contains("jumper") || found.contains("Jumper"),
            "{found}"
        );
        assert!(progress.repair(def));
        assert!(progress.reverify());
        assert!(progress.repair_path_complete());
    }

    #[test]
    fn unrepaired_reverify_fails_deterministically() {
        // Adversarial: shaking an unrepaired path never heals it —
        // the re-verify fails every time and the count is kept.
        let level = load("c1l6");
        let def = level.intermittent.as_ref().unwrap();
        let mut progress = IntermittentProgress::default();
        assert!(progress.reproduce());
        assert!(progress.inspect(def).is_some());
        for expected in 1..=3 {
            assert!(!progress.reverify());
            assert_eq!(progress.failed_reverifies, expected);
            assert!(!progress.reverified);
        }
        // The path is still completable afterward (no soft-lock).
        assert!(progress.repair(def));
        assert!(progress.reverify());
        assert!(progress.repair_path_complete());
    }

    #[test]
    fn malformed_intermittent_data_fails_closed() {
        // Adversarial: two loose connections (or none) is not a
        // diagnosable plant — inspection and repair refuse.
        let level = load("c1l6");
        let mut def = level.intermittent.as_ref().unwrap().clone();
        for conn in &mut def.connections {
            conn.is_loose = true;
        }
        let mut progress = IntermittentProgress::default();
        assert!(progress.reproduce());
        assert!(progress.inspect(&def).is_none());
        assert!(!progress.repaired);
    }

    #[test]
    fn c1l6_shipped_data_shape() {
        // The composition's data contract: one loose fitting, the
        // backup edge is a real player edge, and the identification
        // hooks remain (narrowed, not deleted).
        let level = load("c1l6");
        let def = level.intermittent.as_ref().unwrap();
        assert!(intermittent_is_well_formed(def));
        assert!(level
            .available_components
            .iter()
            .any(|c| (c.from, c.to) == def.backup_edge));
        assert!(level.identification.is_some());
        assert_eq!(def.backup_edge, (2, 3));
    }
}
