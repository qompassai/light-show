//! API console for Clara's NBI/SMx puzzle levels.
//!
//! When a level carries `LevelDef::api_sequence`, the player must issue
//! the expected API calls in order through this console before the
//! level counts as won. The console is a row of buttons (one per op in
//! `ApiSequenceDef::choices`, distractors included); clicking appends
//! to the player's sequence. A wrong pick raises an alarm — the console
//! flashes, `ApiProgress::alarms_raised` increments, and the bad pick
//! is *not* appended (the player must still produce the exact expected
//! order). The win gate lives in `playing::check_win_condition`, which
//! requires `ApiProgress::is_complete` alongside the board check.

use bevy::prelude::*;

use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY_BOLD, FONT_SIZE_ADJUST};
use crate::level::{ApiOp, LevelDef};
use crate::states::GameState;

/// The player's in-progress API call sequence for the current level.
#[derive(Resource, Debug, Default)]
pub struct ApiProgress {
    /// Ops issued so far, in click order.
    pub placed: Vec<ApiOp>,
    /// Wrong picks so far. Flavor + console feedback; never fails the
    /// level — the puzzle teaches the sequence, it doesn't punish.
    pub alarms_raised: u32,
}

impl ApiProgress {
    /// True when the player has produced exactly the expected order.
    pub fn is_complete(&self, level_id: &str, seq: &crate::level::ApiSequenceDef) -> bool {
        crate::level::verify_api_sequence(level_id, seq, &self.placed)
    }

    /// Record a click. Returns `true` when the pick was the next
    /// expected op (appended), `false` on a wrong pick (alarm raised,
    /// nothing appended). The pick is checked against the step tag
    /// for this position — the expected order itself is never held.
    pub fn push(&mut self, op: ApiOp, level_id: &str, seq: &crate::level::ApiSequenceDef) -> bool {
        let next_ok = seq.step_matches(level_id, &self.placed, op);
        if next_ok {
            self.placed.push(op);
            true
        } else {
            self.alarms_raised += 1;
            false
        }
    }

    pub fn reset(&mut self) {
        self.placed.clear();
        self.alarms_raised = 0;
    }
}

/// Marker on the console root node (for teardown).
#[derive(Component)]
struct ApiConsoleRoot;

/// Marker on each op button.
#[derive(Component)]
pub struct ApiButton(pub ApiOp);

/// Marker on the status line ("3/5 calls — next: Show ONT").
#[derive(Component)]
pub(crate) struct ApiStatusLine;

/// Marker on the alarm counter line.
#[derive(Component)]
pub(crate) struct ApiAlarmLine;

pub struct ApiConsolePlugin;

impl Plugin for ApiConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ApiProgress>()
            .add_systems(
                Update,
                handle_api_buttons.run_if(in_state(GameState::Playing)),
            )
            // Same teardown points as the board (see playing.rs): the
            // console must survive Playing -> OutageActive.
            .add_systems(OnEnter(GameState::Results), cleanup_api_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_api_console);
    }
}

/// Spawn the console when the level has an `api_sequence`. No sequence
/// means no console — pure board levels are untouched.
pub(crate) fn setup_api_console(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<ApiProgress>,
    asset_server: Res<AssetServer>,
) {
    progress.reset();
    let Some(seq) = &level.api_sequence else {
        return;
    };
    let display: Handle<Font> = asset_server.load(DISPLAY_BOLD);
    let body: Handle<Font> = asset_server.load(BODY);
    let body_medium: Handle<Font> = asset_server.load(BODY_MEDIUM);

    let api_name = if seq.api == "smx" {
        "SMx REST :18443"
    } else if seq.api == "ams" {
        "AMS NBI :8443"
    } else {
        "CMS NBI :18080"
    };
    commands
        .spawn((
            ApiConsoleRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                right: Val::Px(12.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.14, 0.95)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!("API CONSOLE — {api_name}")),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(16.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.85, 0.85)),
            ));
            root.spawn((
                ApiStatusLine,
                Text::new(status_text(
                    seq.expected_len(),
                    0,
                    seq.next_expected(&level.id, &[]),
                )),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn((
                ApiAlarmLine,
                Text::new("Alarms: 0".to_string()),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.6, 0.2)),
            ));
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                for op in &seq.choices {
                    let op = *op;
                    row.spawn((
                        ApiButton(op),
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.18, 0.30)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(op.label()),
                            TextFont {
                                font: body_medium.clone().into(),
                                font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                }
            });
        });
}

fn status_text(total: usize, done: usize, next: Option<ApiOp>) -> String {
    if done >= total {
        format!("{done}/{total} calls — sequence complete")
    } else {
        match next {
            Some(op) => format!("{done}/{total} calls — next: {}", op.label()),
            // No answer key in this build: the next call cannot be
            // named, so the hint is omitted rather than faked.
            None => format!("{done}/{total} calls"),
        }
    }
}

/// Click handling: append correct picks, raise alarms on wrong ones,
/// and refresh the status/alarm lines.
fn handle_api_buttons(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<ApiProgress>,
    buttons: Query<
        (&Interaction, &ApiButton),
        (
            Changed<Interaction>,
            Without<crate::states::quiz::QuizChoice>,
            Without<crate::states::triage_console::TriageButton>,
        ),
    >,
    mut status: Query<&mut Text, (With<ApiStatusLine>, Without<ApiAlarmLine>)>,
    mut alarms: Query<&mut Text, With<ApiAlarmLine>>,
    sfx: Res<crate::audio::Sfx>,
) {
    let Some(seq) = &level.api_sequence else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let ok = progress.push(button.0, &level.id, seq);
        if ok {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
        } else {
            sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
        }
        let done = progress.placed.len();
        let next = seq.next_expected(&level.id, &progress.placed);
        for mut text in &mut status {
            **text = status_text(seq.expected_len(), done, next);
        }
        for mut text in &mut alarms {
            **text = format!("Alarms: {}", progress.alarms_raised);
        }
    }
}

fn cleanup_api_console(
    mut commands: Commands,
    roots: Query<Entity, With<ApiConsoleRoot>>,
    mut progress: ResMut<ApiProgress>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    progress.reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::verify_api_sequence;

    const LEVEL_ID: &str = "console-test";

    fn expected() -> Vec<ApiOp> {
        vec![
            ApiOp::Login,
            ApiOp::ShowOnt,
            ApiOp::CreateService,
            ApiOp::VerifyService,
        ]
    }

    /// The def under test: `expected()` as keyed tags under the
    /// resolved key, built the way the level-data migration builds
    /// them (full-sequence tag + one step tag per prefix).
    fn def() -> crate::level::ApiSequenceDef {
        let exp = expected();
        crate::level::ApiSequenceDef {
            api: "nbi".to_string(),
            expected_tag: crate::answer_verify::test_tag_for(
                "api_sequence",
                LEVEL_ID,
                &crate::level::serialize_api_ops(&exp),
            ),
            expected_step_tags: (1..=exp.len())
                .map(|n| {
                    crate::answer_verify::test_tag_for(
                        "api_sequence",
                        LEVEL_ID,
                        &crate::level::serialize_api_ops(&exp[..n]),
                    )
                })
                .collect(),
            choices: exp,
        }
    }

    fn empty_def() -> crate::level::ApiSequenceDef {
        crate::level::ApiSequenceDef {
            api: "nbi".to_string(),
            expected_tag: String::new(),
            expected_step_tags: Vec::new(),
            choices: Vec::new(),
        }
    }

    #[test]
    fn push_appends_correct_picks_in_order() {
        let mut p = ApiProgress::default();
        let def = def();
        for op in &expected() {
            assert!(p.push(*op, LEVEL_ID, &def));
        }
        assert!(p.is_complete(LEVEL_ID, &def));
        assert_eq!(p.alarms_raised, 0);
    }

    #[test]
    fn push_rejects_wrong_pick_and_raises_alarm() {
        let mut p = ApiProgress::default();
        let def = def();
        assert!(p.push(ApiOp::Login, LEVEL_ID, &def));
        // Skipping ShowOnt straight to CreateService is wrong.
        assert!(!p.push(ApiOp::CreateService, LEVEL_ID, &def));
        assert_eq!(p.alarms_raised, 1);
        // The bad pick was not appended — the player must still
        // produce the exact expected order.
        assert_eq!(p.placed, vec![ApiOp::Login]);
        assert!(!p.is_complete(LEVEL_ID, &def));
    }

    #[test]
    fn push_rejects_extra_calls_past_the_end() {
        let mut p = ApiProgress::default();
        let def = def();
        for op in &expected() {
            assert!(p.push(*op, LEVEL_ID, &def));
        }
        assert!(p.is_complete(LEVEL_ID, &def));
        // One more click after completion is a wrong pick.
        assert!(!p.push(ApiOp::Logout, LEVEL_ID, &def));
        assert_eq!(p.alarms_raised, 1);
        assert!(p.is_complete(LEVEL_ID, &def));
    }

    #[test]
    fn verify_api_sequence_cases() {
        let exp = expected();
        let def = def();
        assert!(verify_api_sequence(LEVEL_ID, &def, &exp));
        assert!(!verify_api_sequence(LEVEL_ID, &def, &[]));
        assert!(!verify_api_sequence(LEVEL_ID, &empty_def(), &[]));
        assert!(!verify_api_sequence(
            LEVEL_ID,
            &empty_def(),
            &[ApiOp::Login]
        ));
        let mut swapped = exp.clone();
        swapped.swap(1, 2);
        assert!(!verify_api_sequence(LEVEL_ID, &def, &swapped));
        let mut short = exp.clone();
        short.pop();
        assert!(!verify_api_sequence(LEVEL_ID, &def, &short));
    }

    #[test]
    fn reset_clears_progress_and_alarms() {
        let mut p = ApiProgress::default();
        let def = def();
        p.push(ApiOp::Login, LEVEL_ID, &def);
        p.push(ApiOp::RebootOnt, LEVEL_ID, &def);
        p.reset();
        assert!(p.placed.is_empty());
        assert_eq!(p.alarms_raised, 0);
    }
}
