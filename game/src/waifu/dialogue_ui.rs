//! Companion dialogue UI: Mega Man Battle Network-style messaging with
//! the light-show neon aesthetic.
//!
//! A bottom panel holds a portrait box (face closeup with a talking
//! bounce), a neon-gold name tag, and typewriter text with electronic
//! blips. Click / Z / Enter / Space advances: the first press completes
//! the current line instantly, the second moves to the next line (or
//! closes the panel when the lines run out).
//!
//! The pure typewriter ([`Typewriter`]) is Bevy-free and unit-tested;
//! the Bevy half ([`DialogueUiPlugin`]) only renders state and routes
//! input. Lines come from [`DialogueBank`]; a missing or empty event key
//! is a silent no-op, never a crash.
//!
//! [`Expression`] is future-proofing: only `Neutral` exists today, but
//! the 11-expression sheets map onto it without touching callers.

use bevy::prelude::*;

use super::dialogue::DialogueBank;
use super::Companion;
use crate::audio::{Sfx, SfxKind};
use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY, FONT_SIZE_ADJUST};
use crate::ui::neon::{NEON_CYAN, NEON_DIM, NEON_GOLD};

// ---------------------------------------------------------------------------
// Bounds (units in names)
// ---------------------------------------------------------------------------

/// Typewriter reveal rate, characters per second.
pub const TYPEWRITER_CHARS_PER_SEC: f32 = 30.0;
/// Extra pause inserted after sentence-ending punctuation, seconds.
pub const PAUSE_AFTER_SENTENCE_SECS: f32 = 0.45;
/// Extra pause inserted after clause punctuation, seconds.
pub const PAUSE_AFTER_CLAUSE_SECS: f32 = 0.18;
/// Longest single line the typewriter will reveal, characters. Dialogue
/// data is authored, not attacker-controlled, but the bound keeps a
/// corrupt JSON from stalling the frame loop on a megabyte line.
pub const MAX_LINE_CHARS: usize = 600;
/// Portrait box size, px. Portraits are 2:3, so the box is 2:3 — no
/// distortion, no runtime size queries.
pub const PORTRAIT_BOX_W_PX: f32 = 128.0;
pub const PORTRAIT_BOX_H_PX: f32 = 192.0;
/// Dialogue panel height, px.
pub const PANEL_HEIGHT_PX: f32 = 230.0;
/// Talking bounce amplitude, fraction of portrait scale.
pub const TALK_BOUNCE_AMPLITUDE: f32 = 0.035;
/// Talking bounce angular frequency, radians per second.
pub const TALK_BOUNCE_RAD_PER_SEC: f32 = 18.0;
/// Blip cadence: one electronic blip per this many revealed characters.
pub const BLIP_EVERY_N_CHARS: usize = 2;

// ---------------------------------------------------------------------------
// Expression
// ---------------------------------------------------------------------------

/// Portrait expression. Only [`Expression::Neutral`] ships today; the
/// 11-expression sheets land later and extend this enum, with
/// [`Expression::portrait_path`] choosing the texture per expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Expression {
    #[default]
    Neutral,
}

impl Expression {
    /// Asset path for this expression's portrait of `companion`, relative
    /// to the game's asset root. Today every expression resolves to the
    /// neutral portrait; expression sheets plug in here.
    pub fn portrait_path(&self, companion: Companion) -> &'static str {
        match self {
            Expression::Neutral => companion.portrait_path(),
        }
    }
}

// ---------------------------------------------------------------------------
// Typewriter (pure logic, unit-tested)
// ---------------------------------------------------------------------------

/// Character-by-character reveal state for one dialogue line.
///
/// Contract: [`Typewriter::tick`] advances by wall-clock seconds;
/// punctuation in the just-revealed characters inserts a pause before the
/// *next* character. [`Typewriter::revealed_text`] always returns a valid
/// char boundary. Work per tick is O(characters revealed that tick).
pub struct Typewriter {
    chars: Vec<char>,
    revealed_count: usize,
    char_timer_secs: f32,
    pause_timer_secs: f32,
    completed: bool,
}

impl Typewriter {
    /// Build from a line, truncating at [`MAX_LINE_CHARS`]. An empty line
    /// is immediately complete.
    pub fn new(line: &str) -> Self {
        let chars: Vec<char> = line.chars().take(MAX_LINE_CHARS).collect();
        let completed = chars.is_empty();
        Self {
            chars,
            revealed_count: 0,
            char_timer_secs: 0.0,
            pause_timer_secs: 0.0,
            completed,
        }
    }

    /// Advance by `dt_secs` (clamped to [0, 1] so a hitch never
    /// fast-forwards). Returns `(new_chars, just_completed)`.
    pub fn tick(&mut self, dt_secs: f32) -> (usize, bool) {
        if self.completed {
            return (0, false);
        }
        let dt_secs = dt_secs.clamp(0.0, 1.0);
        if self.pause_timer_secs > 0.0 {
            self.pause_timer_secs = (self.pause_timer_secs - dt_secs).max(0.0);
            return (0, false);
        }
        self.char_timer_secs += dt_secs;
        let mut new_chars = 0;
        let per_char_secs = 1.0 / TYPEWRITER_CHARS_PER_SEC;
        while self.char_timer_secs >= per_char_secs
            && self.revealed_count < self.chars.len()
        {
            self.char_timer_secs -= per_char_secs;
            let c = self.chars[self.revealed_count];
            self.revealed_count += 1;
            new_chars += 1;
            match c {
                '.' | '!' | '?' => self.pause_timer_secs = PAUSE_AFTER_SENTENCE_SECS,
                ',' | ';' | ':' => self.pause_timer_secs = PAUSE_AFTER_CLAUSE_SECS,
                _ => {}
            }
            if self.pause_timer_secs > 0.0 {
                break;
            }
        }
        let was_completed = self.completed;
        self.completed = self.revealed_count >= self.chars.len();
        (new_chars, self.completed && !was_completed)
    }

    /// Reveal the whole line instantly (first advance press).
    pub fn complete(&mut self) {
        self.revealed_count = self.chars.len();
        self.char_timer_secs = 0.0;
        self.pause_timer_secs = 0.0;
        self.completed = true;
    }

    /// The currently visible prefix, always a valid char boundary.
    pub fn revealed_text(&self) -> String {
        self.chars[..self.revealed_count].iter().collect()
    }

    /// Whether every character is visible.
    pub fn is_completed(&self) -> bool {
        self.completed
    }
}

// ---------------------------------------------------------------------------
// Dialogue state
// ---------------------------------------------------------------------------

/// Drives the dialogue panel. `active == false` means the panel is hidden
/// and the systems are idle.
#[derive(Resource)]
pub struct DialogueState {
    pub active: bool,
    companion: Companion,
    expression: Expression,
    lines: Vec<String>,
    line_index: usize,
    typewriter: Typewriter,
    blip_counter: usize,
    talk_time_secs: f32,
}

impl Default for DialogueState {
    fn default() -> Self {
        Self {
            active: false,
            companion: Companion::default(),
            expression: Expression::Neutral,
            lines: Vec::new(),
            line_index: 0,
            typewriter: Typewriter::new(""),
            blip_counter: 0,
            talk_time_secs: 0.0,
        }
    }
}

/// Entity handles for the persistent (hidden/shown) dialogue UI.
#[derive(Resource)]
pub struct DialogueUiEntities {
    root: Entity,
    portrait: Entity,
    name_tag: Entity,
    text: Entity,
    advance: Entity,
}

// UI marker components.
#[derive(Component)]
struct DialoguePortrait;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// What to play when opening the dialogue panel.
pub struct DialogueRequest<'a> {
    /// Line source; the event key is looked up in its map.
    pub bank: &'a DialogueBank,
    /// Who is speaking (portrait + name tag).
    pub companion: Companion,
    /// Key into [`DialogueBank::lines`]; every line under it plays.
    pub event_key: &'a str,
    /// Portrait expression (future-proof; only neutral today).
    pub expression: Expression,
}

/// Open the dialogue panel, playing the requested lines.
///
/// Returns `true` when the panel opened. Returns `false` (silent no-op)
/// when the key is missing or has no lines — dialogue data must never
/// crash the game. Call [`close_dialogue`] first if a panel is already
/// open; opening twice without closing replaces the lines but leaks the
/// old panel's state.
pub fn start_dialogue(
    commands: &mut Commands,
    asset_server: &AssetServer,
    ui: &DialogueUiEntities,
    state: &mut DialogueState,
    request: DialogueRequest<'_>,
) -> bool {
    let Some(lines) = request.bank.lines.get(request.event_key) else {
        return false;
    };
    if lines.is_empty() {
        return false;
    }
    state.active = true;
    state.companion = request.companion;
    state.expression = request.expression;
    state.lines = lines.clone();
    state.line_index = 0;
    state.typewriter = Typewriter::new(&state.lines[0]);
    state.blip_counter = 0;
    state.talk_time_secs = 0.0;
    // Swap the portrait texture and name tag for this companion.
    let portrait: Handle<Image> =
        asset_server.load(request.expression.portrait_path(request.companion));
    commands
        .entity(ui.portrait)
        .insert(ImageNode::new(portrait));
    commands
        .entity(ui.name_tag)
        .insert(Text::new(request.companion.display_name()));
    commands.entity(ui.root).insert(Visibility::Visible);
    true
}

/// Hide the dialogue panel and idle the systems.
pub fn close_dialogue(
    commands: &mut Commands,
    ui: &DialogueUiEntities,
    state: &mut DialogueState,
) {
    state.active = false;
    commands.entity(ui.root).insert(Visibility::Hidden);
}

// ---------------------------------------------------------------------------
// UI construction (spawned once, hidden until used)
// ---------------------------------------------------------------------------

fn spawn_dialogue_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let display: Handle<Font> = asset_server.load(DISPLAY);
    let body: Handle<Font> = asset_server.load(BODY);
    let body_medium: Handle<Font> = asset_server.load(BODY_MEDIUM);
    // Placeholder portrait until `start_dialogue` swaps in the real one.
    let placeholder: Handle<Image> = asset_server.load(
        Expression::Neutral.portrait_path(Companion::default()),
    );

    let root = commands
        .spawn((
            Visibility::Hidden,
            ZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(PANEL_HEIGHT_PX),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                border: UiRect::top(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.1, 0.18, 0.95)),
            BorderColor::all(NEON_CYAN),
        ))
        .id();

    let portrait = commands
        .spawn((
            DialoguePortrait,
            Node {
                width: Val::Px(PORTRAIT_BOX_W_PX),
                height: Val::Px(PORTRAIT_BOX_H_PX),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(NEON_CYAN),
            ImageNode::new(placeholder),
        ))
        .id();
    commands.entity(root).add_child(portrait);

    let column = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .id();
    commands.entity(root).add_child(column);

    // Name tag (companion display name, set at open time).
    let name_tag = commands
        .spawn((
            Text::new(""),
            TextFont {
                font: display.clone().into(),
                font_size: FontSize::Px(20.0 * FONT_SIZE_ADJUST),
                ..default()
            },
            TextColor(NEON_GOLD),
        ))
        .id();
    commands.entity(column).add_child(name_tag);

    let body_text = commands
        .spawn((
            Text::new(""),
            TextFont {
                font: body.clone().into(),
                font_size: FontSize::Px(17.0 * FONT_SIZE_ADJUST),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(column).add_child(body_text);

    // Advance indicator: pulsing "▼" when the line is complete.
    let advance = commands
        .spawn((
            Text::new("▼"),
            TextFont {
                font: body_medium.into(),
                font_size: FontSize::Px(18.0 * FONT_SIZE_ADJUST),
                ..default()
            },
            TextColor(NEON_DIM),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(10.0),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(advance);

    commands.insert_resource(DialogueUiEntities {
        root,
        portrait,
        name_tag,
        text: body_text,
        advance,
    });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Advance the typewriter, play blips, bounce the portrait while talking,
/// and pulse the advance indicator when the line is done.
#[allow(clippy::too_many_arguments)]
fn tick_dialogue(
    mut commands: Commands,
    time: Res<Time>,
    sfx: Res<Sfx>,
    mut state: ResMut<DialogueState>,
    ui: Res<DialogueUiEntities>,
    mut texts: Query<&mut Text>,
    mut portraits: Query<&mut Transform, With<DialoguePortrait>>,
    mut advance_parts: Query<(&mut TextColor, &mut Visibility)>,
) {
    if !state.active {
        return;
    }
    let Ok(mut body) = texts.get_mut(ui.text) else {
        return;
    };
    let (new_chars, _) = state.typewriter.tick(time.delta_secs());
    if new_chars > 0 {
        state.blip_counter += new_chars;
        if state.blip_counter >= BLIP_EVERY_N_CHARS {
            state.blip_counter %= BLIP_EVERY_N_CHARS;
            sfx.play(&mut commands, SfxKind::Dialogue);
        }
    }
    body.0 = state.typewriter.revealed_text();

    // MMBN-style talking bounce while text flows; settle when done.
    let talking = !state.typewriter.is_completed();
    state.talk_time_secs += time.delta_secs();
    if let Ok(mut transform) = portraits.single_mut() {
        let scale = if talking {
            1.0 + TALK_BOUNCE_AMPLITUDE
                * (state.talk_time_secs * TALK_BOUNCE_RAD_PER_SEC).sin()
        } else {
            1.0
        };
        transform.scale = Vec3::splat(scale);
    }

    // Advance indicator: visible + pulsing only when the line is done.
    if let Ok((mut color, mut visibility)) = advance_parts.get_mut(ui.advance) {
        if state.typewriter.is_completed() {
            *visibility = Visibility::Visible;
            let pulse = 0.55 + 0.45 * (state.talk_time_secs * 6.0).sin();
            color.0 = NEON_GOLD.with_alpha(pulse);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Click / Z / Enter / Space: first press completes the line instantly,
/// second press advances to the next line (or closes at the end).
fn advance_dialogue(
    mut commands: Commands,
    ui: Res<DialogueUiEntities>,
    mut state: ResMut<DialogueState>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !state.active {
        return;
    }
    let pressed = mouse.just_pressed(MouseButton::Left)
        || keyboard.any_just_pressed([KeyCode::KeyZ, KeyCode::Enter, KeyCode::Space]);
    if !pressed {
        return;
    }
    if !state.typewriter.is_completed() {
        state.typewriter.complete();
    } else if state.line_index + 1 < state.lines.len() {
        state.line_index += 1;
        let next = state.lines[state.line_index].clone();
        state.typewriter = Typewriter::new(&next);
        state.blip_counter = 0;
    } else {
        close_dialogue(&mut commands, &ui, &mut state);
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Installs the dialogue UI: hidden panel at startup, typewriter and
/// advance systems while a dialogue is active.
pub struct DialogueUiPlugin;

impl Plugin for DialogueUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueState>()
            .add_systems(Startup, spawn_dialogue_ui)
            .add_systems(Update, (tick_dialogue, advance_dialogue));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Tick a typewriter `n` times with a fixed `dt`. Using exact tick
    /// counts (not accumulated durations) keeps f32 rounding from
    /// stealing the final character.
    fn tick_n(tw: &mut Typewriter, n: usize, dt: f32) {
        for _ in 0..n {
            tw.tick(dt);
        }
    }

    /// One typewriter step: exactly enough time for one character.
    const STEP: f32 = 1.0 / TYPEWRITER_CHARS_PER_SEC;

    #[test]
    fn reveals_at_configured_rate() {
        let mut tw = Typewriter::new(&"a".repeat(100));
        tick_n(&mut tw, 30, STEP);
        // 30 chars/sec, no punctuation: exactly 30 after thirty steps.
        assert_eq!(tw.revealed_text().len(), 30);
        assert!(!tw.is_completed());
    }

    #[test]
    fn sentence_punctuation_inserts_a_pause() {
        let mut tw = Typewriter::new("Hi. Yo");
        // "Hi." is 3 chars; the period then holds for 0.45s.
        tick_n(&mut tw, 3, STEP);
        assert_eq!(tw.revealed_text(), "Hi.");
        // 0.2s of pause time: still held.
        tick_n(&mut tw, 6, STEP);
        assert_eq!(
            tw.revealed_text(),
            "Hi.",
            "period must hold the next char through the pause"
        );
        // Run out the clock: the rest flows.
        tick_n(&mut tw, 60, STEP);
        assert_eq!(tw.revealed_text(), "Hi. Yo");
        assert!(tw.is_completed());
    }

    #[test]
    fn clause_punctuation_pauses_shorter_than_sentence() {
        let mut comma = Typewriter::new("a,b");
        tick_n(&mut comma, 2, STEP);
        assert_eq!(comma.revealed_text(), "a,");
        // Clause pause is 0.18s; sentence pause is 0.45s. After 0.3s the
        // comma line must have advanced but an equivalent period line
        // must still be held.
        let mut period = Typewriter::new("a.b");
        tick_n(&mut period, 2, STEP);
        tick_n(&mut comma, 9, STEP);
        tick_n(&mut period, 9, STEP);
        assert_eq!(comma.revealed_text(), "a,b");
        assert_eq!(period.revealed_text(), "a.");
    }

    #[test]
    fn complete_reveals_everything_instantly() {
        let mut tw = Typewriter::new("Hello, world!");
        tw.tick(0.01);
        tw.complete();
        assert_eq!(tw.revealed_text(), "Hello, world!");
        assert!(tw.is_completed());
        // Further ticks are no-ops.
        assert_eq!(tw.tick(1.0), (0, false));
    }

    #[test]
    fn empty_line_is_immediately_complete() {
        let tw = Typewriter::new("");
        assert!(tw.is_completed());
        assert_eq!(tw.revealed_text(), "");
    }

    #[test]
    fn overlong_line_is_truncated_at_the_bound() {
        let mut tw = Typewriter::new(&"x".repeat(MAX_LINE_CHARS + 50));
        tw.complete();
        assert_eq!(tw.revealed_text().len(), MAX_LINE_CHARS);
    }

    #[test]
    fn just_completed_fires_exactly_once() {
        let mut tw = Typewriter::new("ab");
        let (_, first) = tw.tick(1.0);
        assert!(first, "completing tick must report it");
        let (_, second) = tw.tick(1.0);
        assert!(!second, "completed typewriter stays quiet");
    }

    #[test]
    fn unicode_chars_reveal_by_char_not_byte() {
        let mut tw = Typewriter::new("Séraphine ♥");
        tw.complete();
        // Must not panic on a multi-byte boundary and must round-trip.
        assert_eq!(tw.revealed_text(), "Séraphine ♥");
    }

    #[test]
    fn neutral_expression_uses_the_companion_portrait() {
        // Mature portraits are the default where they exist.
        assert_eq!(
            Expression::Neutral.portrait_path(Companion::Clara),
            "art/companions/clara_portrait_mature.jpg"
        );
        assert_eq!(
            Expression::Neutral.portrait_path(Companion::Fiber),
            "art/companions/seraphine_portrait.jpg"
        );
    }
}
