//! Alarm triage console for Aino's NOC track levels.
//!
//! When a level carries `LevelDef::alarm_triage`, the player must
//! acknowledge the listed alarms through this console in exactly the
//! expected priority order before the level counts as won. The console
//! is a column of alarm buttons (one per alarm in
//! `AlarmTriageDef::alarms`); clicking appends to the player's ack
//! order. A wrong pick bumps the wrong-pick counter and plays the alarm
//! SFX but is *not* appended -- the player must still produce the exact
//! expected order. The win gate lives in `playing::check_win_condition`
//! (and the outage variant), which requires `TriageProgress::is_complete`
//! alongside the board and API-sequence checks.

use bevy::prelude::*;

use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY_BOLD, FONT_SIZE_ADJUST};
use crate::level::{AlarmSeverityDef, LevelDef};
use crate::states::GameState;

/// The player's in-progress alarm ack order for the current level.
#[derive(Resource, Debug, Default)]
pub struct TriageProgress {
    /// Alarm ids acked so far, in click order.
    pub acked: Vec<u8>,
    /// Wrong picks so far. Flavor + console feedback; never fails the
    /// level -- triage teaches priority, it does not punish.
    pub wrong_picks: u32,
}

impl TriageProgress {
    /// True when the player has acked exactly the expected order.
    pub fn is_complete(&self, expected: &[u8]) -> bool {
        crate::level::verify_triage_order(expected, &self.acked)
    }

    /// Record a click. Returns `true` when the pick was the next
    /// expected alarm id (appended), `false` on a wrong pick
    /// (wrong-pick counter bumped, nothing appended).
    pub fn push(&mut self, alarm_id: u8, expected: &[u8]) -> bool {
        let next_ok = expected.get(self.acked.len()) == Some(&alarm_id);
        if next_ok {
            self.acked.push(alarm_id);
            true
        } else {
            self.wrong_picks += 1;
            false
        }
    }

    pub fn reset(&mut self) {
        self.acked.clear();
        self.wrong_picks = 0;
    }
}

/// Marker on the console root node (for teardown).
#[derive(Component)]
struct TriageConsoleRoot;

/// Marker on each alarm button; carries the alarm id.
#[derive(Component)]
pub(crate) struct TriageButton(pub(crate) u8);

/// Marker on the status line ("2/5 acked").
#[derive(Component)]
struct TriageStatusLine;

/// Marker on the wrong-pick counter line.
#[derive(Component)]
struct TriageWrongLine;

pub struct TriageConsolePlugin;

impl Plugin for TriageConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TriageProgress>()
            .add_systems(
                Update,
                handle_triage_buttons.run_if(in_state(GameState::Playing)),
            )
            // Same teardown points as the board (see playing.rs).
            .add_systems(OnEnter(GameState::Results), cleanup_triage_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_triage_console);
    }
}

/// Spawn the console when the level has an `alarm_triage`. No triage
/// means no console -- pure board levels are untouched.
pub(crate) fn setup_triage_console(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<TriageProgress>,
    asset_server: Res<AssetServer>,
) {
    progress.reset();
    let Some(triage) = &level.alarm_triage else {
        return;
    };
    let display: Handle<Font> = asset_server.load(DISPLAY_BOLD);
    let body: Handle<Font> = asset_server.load(BODY);
    let body_medium: Handle<Font> = asset_server.load(BODY_MEDIUM);

    commands
        .spawn((
            TriageConsoleRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                right: Val::Px(12.0),
                width: Val::Px(320.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.14, 0.95)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("NOC ALARM BOARD"),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(16.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.65, 0.0)),
            ));
            root.spawn((
                TriageStatusLine,
                Text::new(triage_status_text(triage.expected_order.len(), 0)),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn((
                TriageWrongLine,
                Text::new("Wrong picks: 0".to_string()),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.6, 0.2)),
            ));
            for alarm in &triage.alarms {
                let alarm_id = alarm.id;
                let (label, border) = severity_style(alarm.severity);
                root.spawn((
                    TriageButton(alarm_id),
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.18, 0.30)),
                    BorderColor::all(border),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(format!("[{label}] {}", alarm.summary)),
                        TextFont {
                            font: body_medium.clone().into(),
                            font_size: FontSize::Px(13.0 * FONT_SIZE_ADJUST),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
}

/// Severity tag + border color for an alarm button.
fn severity_style(severity: AlarmSeverityDef) -> (&'static str, Color) {
    match severity {
        AlarmSeverityDef::Critical => ("CRIT", Color::srgb(1.0, 0.2, 0.2)),
        AlarmSeverityDef::Major => ("MAJR", Color::srgb(1.0, 0.55, 0.1)),
        AlarmSeverityDef::Minor => ("MINR", Color::srgb(1.0, 0.85, 0.2)),
        AlarmSeverityDef::Warning => ("WARN", Color::srgb(0.5, 0.6, 0.8)),
    }
}

fn triage_status_text(total: usize, done: usize) -> String {
    if done >= total {
        format!("{done}/{total} acked -- board clear")
    } else {
        format!("{done}/{total} acked -- ack highest priority first")
    }
}

/// Click handling: append correct picks, count wrong ones, refresh the
/// status/wrong-pick lines, and dim acked buttons.
fn handle_triage_buttons(
    mut commands: Commands,
    level: Option<Res<LevelDef>>,
    mut progress: ResMut<TriageProgress>,
    buttons: Query<(&Interaction, &TriageButton), Changed<Interaction>>,
    mut status: Query<&mut Text, (With<TriageStatusLine>, Without<TriageWrongLine>)>,
    mut wrongs: Query<&mut Text, With<TriageWrongLine>>,
    sfx: Res<crate::audio::Sfx>,
) {
    // LevelDef isn't inserted until a level starts; the console only
    // exists when a triage level is active.
    let Some(level) = level else {
        return;
    };
    let Some(triage) = &level.alarm_triage else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // Ignore clicks on already-acked alarms: re-acking is a no-op,
        // not a wrong pick.
        if progress.acked.contains(&button.0) {
            continue;
        }
        let ok = progress.push(button.0, &triage.expected_order);
        if ok {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
        } else {
            sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
        }
        let done = progress.acked.len();
        for mut text in &mut status {
            **text = triage_status_text(triage.expected_order.len(), done);
        }
        for mut text in &mut wrongs {
            **text = format!("Wrong picks: {}", progress.wrong_picks);
        }
    }
}

fn cleanup_triage_console(
    mut commands: Commands,
    roots: Query<Entity, With<TriageConsoleRoot>>,
    mut progress: ResMut<TriageProgress>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    progress.reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::verify_triage_order;

    fn expected() -> Vec<u8> {
        vec![3, 1, 2]
    }

    #[test]
    fn triage_push_appends_in_order() {
        let mut p = TriageProgress::default();
        assert!(p.push(3, &expected()));
        assert!(p.push(1, &expected()));
        assert!(!p.is_complete(&expected()));
        assert!(p.push(2, &expected()));
        assert!(p.is_complete(&expected()));
    }

    #[test]
    fn triage_wrong_pick_not_appended() {
        let mut p = TriageProgress::default();
        assert!(!p.push(1, &expected()));
        assert_eq!(p.wrong_picks, 1);
        assert!(p.acked.is_empty());
        // Correct first pick still works after a wrong one.
        assert!(p.push(3, &expected()));
        assert_eq!(p.acked, vec![3]);
    }

    #[test]
    fn verify_triage_order_cases() {
        let exp = expected();
        assert!(verify_triage_order(&exp, &exp));
        assert!(!verify_triage_order(&exp, &[]));
        assert!(!verify_triage_order(&[], &[]));
        assert!(!verify_triage_order(&[], &[3]));
        let swapped = vec![1, 3, 2];
        assert!(!verify_triage_order(&exp, &swapped));
        let short = vec![3, 1];
        assert!(!verify_triage_order(&exp, &short));
    }

    #[test]
    fn triage_reset_clears_state() {
        let mut p = TriageProgress::default();
        p.push(3, &expected());
        p.push(9, &expected());
        p.reset();
        assert!(p.acked.is_empty());
        assert_eq!(p.wrong_picks, 0);
    }
}
