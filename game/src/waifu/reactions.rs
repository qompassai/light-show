//! Companion reaction dialogue: lines fire ONLY as reactions to game
//! events — never because a screen opened (spec:
//! `~/workspace/light-show-ux/cores-and-reactions-design.md`, Part 2).
//!
//! Flow: gameplay systems push a [`ReactionTrigger`] into
//! [`ReactionInbox`]; [`process_reactions`] resolves it against the
//! selected companion's [`DialogueBank`] (trigger → line + canonical
//! [`Emotion`] → sheet [`Mood`]), offers it to the [`BubbleQueue`], and
//! on acceptance shows the speech bubble above the companion avatar and
//! swaps her sprite to the matching mood for the line's read time.
//!
//! Queue discipline (spec rule 5): depth 1. A new trigger replaces a
//! bubble that has been up longer than [`STALE_SECS`]; otherwise it is
//! dropped, never queued — a busy sequence can never backlog into spam.
//!
//! Completion timing (playtest round 2, tightened in round 3):
//! level-ending systems fire the completion reaction and begin the
//! [`ResultsGate`] hold instead of requesting `Results` directly;
//! [`drive_results_gate`] issues the transition only once the hold's
//! minimum has elapsed AND the pill has fully finished its run — the
//! current line typed out and read, the queue and both inboxes
//! drained (see [`pill_run_complete`]). A long completion line is
//! therefore never flash-covered by the results screen, while a win
//! with no line at all still gets exactly the minimum beat.
//!
//! Presentation (Matt's universal pill rule): the bubble described
//! in the original spec IS the bottom pill — a rounded bar anchored
//! to the bottom of the screen with an MMBN-style square mugshot
//! window at its left end (speaker close-up), name + typed text, a
//! talking mouth flap while the text types, periodic blinking on an
//! idle cycle, and the line's emotion as the base state (see
//! `super::pill`). Warehouse host lines render through the same pill
//! presentation data (`states::warehouse` pill bar + [`PillInbox`]
//! lines here); no character speaks in bare text anywhere.

use super::dialogue::DialogueBank;
use super::pill::{FaceAnim, PillInbox, PillLine, PillSpeaker};
use super::dialogue_ui::Typewriter;
use super::{trigger_mood_pop, Companion, CompanionSprite, Mood, SelectedCompanion};
use crate::anim::{ResultsGate, TransitionRequest};
use crate::audio::{Sfx, SfxKind};
use crate::board::PlacedChoices;
use crate::fonts::{BODY, DISPLAY};
use crate::level::LevelDef;
use crate::salvage::SalvageTracker;
use crate::states::outage::ActiveOutage;
use crate::states::playing::{LevelClock, LiveGraph};
use crate::states::GameState;
use crate::ui::neon::{NEON_CYAN, NEON_GOLD};
use bevy::prelude::*;
use osp_sim::Medium;
use std::collections::{HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Bounds (units in names)
// ---------------------------------------------------------------------------

/// A bubble older than this (seconds) is stale: a new trigger replaces
/// it instead of waiting for its read time to finish.
pub const STALE_SECS: f32 = 2.0;
/// Base seconds a bubble stays up before its per-character read time.
pub const READ_BASE_SECS: f32 = 1.4;
/// Additional read seconds per character of line text.
pub const READ_PER_CHAR_SECS: f32 = 0.045;
/// Shortest bubble lifetime (seconds) — also the floor that keeps the
/// completion reaction visible through the results hold.
pub const READ_MIN_SECS: f32 = 1.6;
/// Longest bubble lifetime (seconds): a reaction is a glance, not a
/// cutscene.
pub const READ_MAX_SECS: f32 = 6.0;
/// Most triggers buffered between frames; past this the oldest is
/// dropped (reactions are perishable by design).
pub const INBOX_CAPACITY: usize = 8;
/// Seconds without a placement change before the single idle nudge.
pub const IDLE_NUDGE_AFTER_SECS: f64 = 45.0;
/// Mugshot window size, px: the square portrait inset at the pill's
/// left end (MMBN framing per Matt's reference screenshots).
const MUGSHOT_PX: f32 = 96.0;
/// Pill side inset, px: the bar spans the window minus this margin on
/// each side, anchored flush to the bottom edge.
const PILL_INSET_PX: f32 = 12.0;

// ---------------------------------------------------------------------------
// Emotion (canonical set) → sheet Mood
// ---------------------------------------------------------------------------

/// The canonical emotion set (Lumen templates v1). Each reaction line
/// carries one; [`Emotion::mood`] resolves it to the sprite sheet's
/// rows. The sheets ship six rows (`Idle, Blush, Wink, Pout, Celebrate,
/// Alarmed`), so emotions without a dedicated row fall back to the
/// nearest expression the sheet does have — `Sad` reads through `Pout`,
/// `Determined`/`Smug` through `Wink`, `Worried` through `Alarmed` —
/// and `Neutral` (the `Idle` row) is the floor. This is the spec's
/// rule-4 fallback, implemented once at the mapping layer so no caller
/// ever blocks on a missing expression asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emotion {
    Neutral,
    Happy,
    Angry,
    Sad,
    Surprised,
    Playful,
    Annoyed,
    Worried,
    Embarrassed,
    Determined,
    Smug,
}

impl Emotion {
    pub const ALL: [Emotion; 11] = [
        Emotion::Neutral,
        Emotion::Happy,
        Emotion::Angry,
        Emotion::Sad,
        Emotion::Surprised,
        Emotion::Playful,
        Emotion::Annoyed,
        Emotion::Worried,
        Emotion::Embarrassed,
        Emotion::Determined,
        Emotion::Smug,
    ];

    /// The sprite-sheet mood this emotion displays as.
    pub fn mood(self) -> Mood {
        match self {
            Emotion::Neutral => Mood::Idle,
            Emotion::Happy => Mood::Celebrate,
            Emotion::Angry | Emotion::Annoyed | Emotion::Sad => Mood::Pout,
            Emotion::Surprised | Emotion::Worried => Mood::Alarmed,
            Emotion::Playful | Emotion::Smug | Emotion::Determined => Mood::Wink,
            Emotion::Embarrassed => Mood::Blush,
        }
    }
}

// ---------------------------------------------------------------------------
// Triggers
// ---------------------------------------------------------------------------

/// A game event a companion can react to. Fired by gameplay systems
/// into [`ReactionInbox`]; the full v1 table from the design spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionTrigger {
    /// First component placed this attempt.
    FirstPlacement,
    /// The connected chain first reaches the target this attempt.
    SegmentComplete,
    /// A placed component fried (see `crate::salvage`) — the salvage
    /// moment; the line acknowledges the core.
    TooHot,
    /// A completed route evaluated TOO LOW (re-arms when the verdict
    /// changes).
    TooLow,
    /// Level completed (win), fired in-level before the results hold.
    LevelComplete,
    /// Level failed, fired in-level before the results hold.
    LevelFailed,
    /// ~45 s without a placement; at most once per attempt.
    IdleNudge,
    /// A scripted outage just fired.
    OutageStart,
}

impl ReactionTrigger {
    pub const ALL: [ReactionTrigger; 8] = [
        ReactionTrigger::FirstPlacement,
        ReactionTrigger::SegmentComplete,
        ReactionTrigger::TooHot,
        ReactionTrigger::TooLow,
        ReactionTrigger::LevelComplete,
        ReactionTrigger::LevelFailed,
        ReactionTrigger::IdleNudge,
        ReactionTrigger::OutageStart,
    ];
}

/// The trigger table: each trigger's dialogue-bank key and the emotion
/// its line is delivered with. One table for every companion — the
/// per-companion variation lives in the bank text, not the wiring.
pub fn trigger_spec(trigger: ReactionTrigger) -> (&'static str, Emotion) {
    match trigger {
        ReactionTrigger::FirstPlacement => ("first_placement", Emotion::Happy),
        ReactionTrigger::SegmentComplete => ("segment_complete", Emotion::Determined),
        ReactionTrigger::TooHot => ("too_hot", Emotion::Surprised),
        ReactionTrigger::TooLow => ("too_low", Emotion::Worried),
        ReactionTrigger::LevelComplete => ("level_complete", Emotion::Happy),
        ReactionTrigger::LevelFailed => ("level_failed", Emotion::Sad),
        ReactionTrigger::IdleNudge => ("idle_nudge", Emotion::Playful),
        ReactionTrigger::OutageStart => ("outage_start", Emotion::Surprised),
    }
}

/// One stage of a companion's first-level tutorial, as authored in
/// `tutorial_segments.json` (the authoring source; this table is its
/// compiled form — see [`tutorial_spec`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialStage {
    BriefingObjective,
    BriefingGestures,
    BriefingBudgetMath,
    FirstPlacement,
    FirstVerdict,
    FirstOutage,
    ScoringAndCores,
    Wrap,
}

impl TutorialStage {
    pub const ALL: [TutorialStage; 8] = [
        TutorialStage::BriefingObjective,
        TutorialStage::BriefingGestures,
        TutorialStage::BriefingBudgetMath,
        TutorialStage::FirstPlacement,
        TutorialStage::FirstVerdict,
        TutorialStage::FirstOutage,
        TutorialStage::ScoringAndCores,
        TutorialStage::Wrap,
    ];

    /// The dialogue-bank key every companion carries for this stage.
    pub fn key(self) -> &'static str {
        match self {
            TutorialStage::BriefingObjective => "tutorial_briefing_objective",
            TutorialStage::BriefingGestures => "tutorial_briefing_gestures",
            TutorialStage::BriefingBudgetMath => "tutorial_briefing_math",
            TutorialStage::FirstPlacement => "tutorial_first_placement",
            TutorialStage::FirstVerdict => "tutorial_first_verdict",
            TutorialStage::FirstOutage => "tutorial_first_outage",
            TutorialStage::ScoringAndCores => "tutorial_scoring_cores",
            TutorialStage::Wrap => "tutorial_wrap",
        }
    }
}

/// The tutorial table, transcribed from `tutorial_segments.json`:
/// `(companion, stage) -> (bank key, Emotion)`. Unlike
/// [`trigger_spec`], the emotion is per companion — the segments file
/// voices the same stage differently per girl (e.g. `scoring_cores`
/// is `Playful` for six companions, `Smug` for Lattice, `Neutral`
/// for Clara) — so no single per-stage row can express it. The key
/// is uniform per stage (see [`TutorialStage::key`]).
pub fn tutorial_spec(companion: Companion, stage: TutorialStage) -> (&'static str, Emotion) {
    use Companion as C;
    use Emotion as E;
    let emotion = match stage {
        TutorialStage::BriefingObjective => match companion {
            C::Fiber | C::Hikari => E::Happy,
            C::Coax | C::Mobile => E::Determined,
            C::Ethernet | C::Clara | C::Aino | C::Lea => E::Neutral,
        },
        TutorialStage::BriefingGestures => E::Neutral,
        TutorialStage::BriefingBudgetMath => match companion {
            C::Mobile => E::Neutral,
            _ => E::Determined,
        },
        TutorialStage::FirstPlacement => E::Happy,
        TutorialStage::FirstVerdict => match companion {
            C::Coax | C::Aino | C::Lea => E::Neutral,
            _ => E::Determined,
        },
        TutorialStage::FirstOutage => match companion {
            C::Fiber | C::Mobile | C::Aino | C::Lea => E::Surprised,
            C::Coax | C::Clara | C::Hikari => E::Worried,
            C::Ethernet => E::Annoyed,
        },
        TutorialStage::ScoringAndCores => match companion {
            C::Ethernet => E::Smug,
            C::Clara => E::Neutral,
            _ => E::Playful,
        },
        TutorialStage::Wrap => match companion {
            C::Fiber | C::Ethernet | C::Clara | C::Lea => E::Determined,
            C::Coax => E::Smug,
            C::Mobile | C::Aino | C::Hikari => E::Happy,
        },
    };
    (stage.key(), emotion)
}

/// A trigger fully resolved against a companion's bank: the line to
/// show, the emotion it carries, and the sheet mood that displays it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReaction {
    pub line: String,
    pub emotion: Emotion,
    pub mood: Mood,
}

/// Resolve `trigger` for the companion owning `bank`. `None` when the
/// bank has no line for the trigger (the specialist banks are partial)
/// — a missing line is a silent no-op, never a crash and never a
/// substitute line in the wrong voice.
pub fn resolve_reaction(bank: &DialogueBank, trigger: ReactionTrigger) -> Option<ResolvedReaction> {
    let (key, emotion) = trigger_spec(trigger);
    let line = bank.random_line(key)?;
    Some(ResolvedReaction {
        line: line.to_string(),
        emotion,
        mood: emotion.mood(),
    })
}

// ---------------------------------------------------------------------------
// Bubble queue (pure, unit-tested)
// ---------------------------------------------------------------------------

/// How long a line stays up: base time plus per-character read time,
/// clamped to [READ_MIN_SECS, READ_MAX_SECS].
pub fn read_time_secs(line: &str) -> f32 {
    let scaled = READ_BASE_SECS + READ_PER_CHAR_SECS * line.chars().count() as f32;
    scaled.clamp(READ_MIN_SECS, READ_MAX_SECS)
}

/// The bubble currently on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveBubble {
    pub line: String,
    pub emotion: Emotion,
    /// Seconds since the bubble appeared.
    pub age_secs: f32,
    /// Seconds it stays up (see [`read_time_secs`]).
    pub read_secs: f32,
}

/// What happened when a resolved reaction was offered to the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferOutcome {
    /// The bubble was free; the line is up.
    Shown,
    /// The previous bubble was stale (> [`STALE_SECS`]); replaced.
    ReplacedStale,
    /// A fresh bubble is mid-read; the line was dropped, not queued.
    DroppedBusy,
}

/// Depth-1 bubble slot with the spec's staleness policy.
#[derive(Debug, Default)]
pub struct BubbleQueue {
    active: Option<ActiveBubble>,
}

impl BubbleQueue {
    /// Offer a line. See [`OfferOutcome`]; a dropped line is gone —
    /// reactions are perishable and never replayed later.
    pub fn offer(&mut self, line: String, emotion: Emotion) -> OfferOutcome {
        match &self.active {
            None => {
                let read_secs = read_time_secs(&line);
                self.active = Some(ActiveBubble {
                    line,
                    emotion,
                    age_secs: 0.0,
                    read_secs,
                });
                OfferOutcome::Shown
            }
            Some(active) if active.age_secs > STALE_SECS => {
                let read_secs = read_time_secs(&line);
                self.active = Some(ActiveBubble {
                    line,
                    emotion,
                    age_secs: 0.0,
                    read_secs,
                });
                OfferOutcome::ReplacedStale
            }
            Some(_) => OfferOutcome::DroppedBusy,
        }
    }

    /// Advance the active bubble; returns `true` on the tick it
    /// expires (reached its read time).
    pub fn tick(&mut self, dt_secs: f32) -> bool {
        let Some(active) = &mut self.active else {
            return false;
        };
        active.age_secs += dt_secs.max(0.0);
        if active.age_secs >= active.read_secs {
            self.active = None;
            true
        } else {
            false
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active(&self) -> Option<&ActiveBubble> {
        self.active.as_ref()
    }

    pub fn clear(&mut self) {
        self.active = None;
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Bounded trigger queue between gameplay systems and the bubble.
/// Producers `push` (via `Option<ResMut<ReactionInbox>>` so headless
/// harnesses without this plugin degrade to no reactions, never a
/// panic); [`process_reactions`] drains it each frame.
#[derive(Resource, Default)]
pub struct ReactionInbox {
    pending: VecDeque<ReactionTrigger>,
}

impl ReactionInbox {
    /// Buffer a trigger. Beyond [`INBOX_CAPACITY`] the oldest pending
    /// trigger is dropped — a burst of events keeps only its freshest
    /// reactions.
    pub fn push(&mut self, trigger: ReactionTrigger) {
        if self.pending.len() >= INBOX_CAPACITY {
            self.pending.pop_front();
        }
        self.pending.push_back(trigger);
    }

    /// Take every pending trigger, oldest first.
    pub fn drain(&mut self) -> Vec<ReactionTrigger> {
        self.pending.drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

/// Live bubble state: the queue plus the render-side bookkeeping
/// (typewriter progress and the mood the bubble applied, so expiry can
/// restore the sprite only if nothing else claimed it meanwhile).
#[derive(Resource, Default)]
pub struct BubbleState {
    pub queue: BubbleQueue,
    typewriter: Option<Typewriter>,
    applied_mood: Option<Mood>,
    /// Blink/talk stack for the mugshot (see `super::pill::FaceAnim`).
    face: FaceAnim,
    /// Face asset currently loaded in the mugshot window.
    face_path: Option<String>,
    /// Speaker whose face the mugshot shows.
    face_speaker: Option<PillSpeaker>,
    /// Canonical emotion slug of the current line (faces layout key).
    face_emotion: &'static str,
    /// Probed `art/faces` availability (empty until probed / when the
    /// tree does not exist yet — portraits serve as faces then).
    face_available: std::collections::HashSet<String>,
    face_probed: bool,
}

/// Entity handles for the persistent bubble UI (spawned once, hidden
/// until a reaction shows it).
#[derive(Resource)]
struct BubbleUi {
    root: Entity,
    face: Entity,
    name: Entity,
    body: Entity,
    advance: Entity,
}

/// Per-attempt watcher state for [`watch_level_events`]: which
/// once-per-attempt triggers have fired and the last completed-route
/// verdict (so TOO LOW re-arms when the verdict changes).
#[derive(Resource, Default)]
struct AttemptState {
    placed_seen: usize,
    first_placement_fired: bool,
    segment_fired: bool,
    idle_fired: bool,
    last_activity_secs: f64,
    last_verdict: Option<RouteVerdict>,
}

/// The completed-route verdict the watcher last observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteVerdict {
    InWindow,
    TooLow,
    TooHot,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Installs the reaction system: inbox, bubble UI, the level-event
/// watcher, and the results-gate driver.
pub struct ReactionsPlugin;

impl Plugin for ReactionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReactionInbox>()
            .init_resource::<PillInbox>()
            .init_resource::<BubbleState>()
            .init_resource::<AttemptState>()
            .init_resource::<SalvageTracker>()
            .init_resource::<crate::salvage::PendingHaul>()
            .add_systems(Startup, spawn_bubble_ui)
            .add_systems(OnEnter(GameState::Playing), reset_attempt)
            .add_systems(OnEnter(GameState::MainMenu), hide_bubble)
            .add_systems(
                Update,
                (watch_level_events, process_reactions, drive_results_gate)
                    .run_if(in_state(GameState::Playing).or_else(in_state(GameState::OutageActive))),
            )
            // The pill is universal: queued pill lines (results,
            // Warehouse, quiz) must render in their own states too.
            .add_systems(Update, (process_pill_lines, tick_bubble));
    }
}

// ---------------------------------------------------------------------------
// Bubble UI construction
// ---------------------------------------------------------------------------

/// Spawn the speech bubble once: a rounded box with the speaker's name
/// and the line text, a tail glyph under it pointing down at the
/// companion avatar, anchored bottom-center above her. Hidden until
/// the first reaction.
fn spawn_bubble_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let display: Handle<Font> = asset_server.load(DISPLAY);
    let body_font: Handle<Font> = asset_server.load(BODY);
    let placeholder: Handle<Image> =
        asset_server.load(Companion::default().portrait_path());

    // The pill: a rounded bar anchored to the bottom of the screen.
    let root = commands
        .spawn((
            Visibility::Hidden,
            ZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(PILL_INSET_PX),
                right: Val::Px(PILL_INSET_PX),
                bottom: Val::Px(PILL_INSET_PX),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(18.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.1, 0.18, 0.96)),
            BorderColor::all(NEON_CYAN),
        ))
        .id();

    // MMBN-style mugshot window: square portrait inset at the left end.
    let face = commands
        .spawn((
            Node {
                width: Val::Px(MUGSHOT_PX),
                height: Val::Px(MUGSHOT_PX),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BorderColor::all(NEON_GOLD),
            ImageNode::new(placeholder),
        ))
        .id();
    commands.entity(root).add_child(face);

    let column = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .id();
    commands.entity(root).add_child(column);

    let name = commands
        .spawn((
            Text::new(""),
            TextFont {
                font: display.into(),
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(NEON_GOLD),
        ))
        .id();
    commands.entity(column).add_child(name);

    let body = commands
        .spawn((
            Text::new(""),
            TextFont {
                font: body_font.clone().into(),
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(column).add_child(body);

    // Advance indicator (MMBN): shown once the line has typed out.
    let advance = commands
        .spawn((
            Text::new("▼"),
            TextFont {
                font: body_font.into(),
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(NEON_GOLD),
            Visibility::Hidden,
            Node {
                align_self: AlignSelf::FlexEnd,
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(advance);

    commands.insert_resource(BubbleUi {
        root,
        face,
        name,
        body,
        advance,
    });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Fresh attempt: clear the watcher, the salvage ledger, any pending
/// results hold, the inbox, and the bubble. Runs on every
/// `OnEnter(Playing)` — which is exactly "a level is starting or
/// restarting", since outage resolution never routes back here.
fn reset_attempt(
    mut attempt: ResMut<AttemptState>,
    mut salvage: ResMut<SalvageTracker>,
    mut inbox: ResMut<ReactionInbox>,
    gate: Option<ResMut<ResultsGate>>,
    mut bubble: ResMut<BubbleState>,
    mut commands: Commands,
    ui: Option<Res<BubbleUi>>,
) {
    *attempt = AttemptState::default();
    salvage.reset();
    inbox.clear();
    if let Some(mut gate) = gate {
        gate.clear();
    }
    bubble.queue.clear();
    bubble.typewriter = None;
    bubble.applied_mood = None;
    bubble.face = FaceAnim::default();
    bubble.face_path = None;
    bubble.face_speaker = None;
    if let Some(ui) = ui {
        commands.entity(ui.root).insert(Visibility::Hidden);
    }
}

/// Results covers the board; the bubble's moment is over. (The line
/// itself also appears on the results screen from the results pool.)
fn hide_bubble(
    mut commands: Commands,
    mut bubble: ResMut<BubbleState>,
    ui: Option<Res<BubbleUi>>,
) {
    bubble.queue.clear();
    bubble.typewriter = None;
    bubble.applied_mood = None;
    if let Some(ui) = ui {
        commands.entity(ui.root).insert(Visibility::Hidden);
    }
}

/// Watch the level for reaction-worthy events: placements, the chain
/// first reaching the target, components frying (salvage), a completed
/// route landing TOO LOW, and the once-per-attempt idle nudge.
///
/// Detection rides on `LiveGraph` change ticks — the graph is rebuilt
/// by the board on every placement change and by the outage flow — so
/// no board code had to learn about reactions.
#[allow(clippy::too_many_arguments)]
fn watch_level_events(
    level: Res<LevelDef>,
    live: Res<LiveGraph>,
    placed: Res<PlacedChoices>,
    clock: Res<LevelClock>,
    active_outage: Option<Res<ActiveOutage>>,
    mut attempt: ResMut<AttemptState>,
    mut salvage: ResMut<SalvageTracker>,
    mut inbox: ResMut<ReactionInbox>,
) {
    // Idle nudge: once per attempt, after IDLE_NUDGE_AFTER_SECS with
    // no placement change. Checked every frame (cheap); the activity
    // clock updates below whenever placements change.
    if !attempt.idle_fired
        && clock.elapsed_seconds - attempt.last_activity_secs >= IDLE_NUDGE_AFTER_SECS
    {
        attempt.idle_fired = true;
        inbox.push(ReactionTrigger::IdleNudge);
    }

    if !live.is_changed() && !placed.is_changed() {
        return;
    }

    let placed_count = placed.0.len();
    if placed_count != attempt.placed_seen {
        attempt.placed_seen = placed_count;
        attempt.last_activity_secs = clock.elapsed_seconds;
        if placed_count > 0 && !attempt.first_placement_fired {
            attempt.first_placement_fired = true;
            inbox.push(ReactionTrigger::FirstPlacement);
        }
    }

    let (_level_dbm, hops, frontier) =
        live.graph
            .frontier_budget(level.source_node, live.tx_dbm, live.wavelength.0);
    let route_connected = hops > 0 && frontier == level.target_node;
    if route_connected && !attempt.segment_fired {
        attempt.segment_fired = true;
        inbox.push(ReactionTrigger::SegmentComplete);
    }

    // Signal-level verdicts and salvage live on the dB media only.
    // Ethernet has no level to overdrive, provisioning fans out to
    // subscriber windows instead of one route verdict, and quiz levels
    // have no board economy at all.
    if level.medium() == Medium::Ethernet || !level.subscribers.is_empty() || level.quiz.is_some() {
        return;
    }
    let window = level.receive_window();

    let placed_edges: HashSet<(u32, u32)> = placed.0.keys().copied().collect();
    let detected = crate::salvage::detect_fried(
        &live.graph,
        &placed_edges,
        level.source_node,
        level.target_node,
        live.tx_dbm,
        live.wavelength.0,
        window,
    );
    if !salvage.record(&detected).is_empty() {
        inbox.push(ReactionTrigger::TooHot);
    }

    if route_connected {
        let outage = active_outage
            .as_deref()
            .and_then(|active| active.outage.as_ref());
        let verdict = live
            .graph
            .compute_link_budget_with_outage(
                level.source_node,
                level.target_node,
                live.tx_dbm,
                live.wavelength.0,
                window,
                outage,
            )
            .map(|result| {
                if result.in_window {
                    RouteVerdict::InWindow
                } else if result.received_dbm < window.min_dbm {
                    RouteVerdict::TooLow
                } else {
                    RouteVerdict::TooHot
                }
            })
            .ok();
        if verdict == Some(RouteVerdict::TooLow) && attempt.last_verdict != verdict {
            inbox.push(ReactionTrigger::TooLow);
        }
        attempt.last_verdict = verdict;
    } else {
        attempt.last_verdict = None;
    }
}

/// Drain the inbox into the bubble: resolve each trigger against the
/// selected companion's bank, offer it to the depth-1 queue, and on
/// acceptance swap the companion's mood (with the mood pop), show the
/// bubble, and blip.
#[allow(clippy::too_many_arguments)]
fn process_reactions(
    mut commands: Commands,
    mut inbox: ResMut<ReactionInbox>,
    mut bubble: ResMut<BubbleState>,
    bank: Res<DialogueBank>,
    selected: Res<SelectedCompanion>,
    sfx: Res<Sfx>,
    asset_server: Res<AssetServer>,
    ui: Option<Res<BubbleUi>>,
    mut texts: Query<&mut Text>,
    mut images: Query<&mut ImageNode>,
    mut companions: Query<(Entity, &mut CompanionSprite)>,
) {
    if inbox.is_empty() {
        return;
    }
    for trigger in inbox.drain() {
        let Some(resolved) = resolve_reaction(&bank, trigger) else {
            continue;
        };
        match bubble.queue.offer(resolved.line.clone(), resolved.emotion) {
            OfferOutcome::Shown | OfferOutcome::ReplacedStale => {
                bubble.typewriter = Some(Typewriter::new(&resolved.line));
                bubble.applied_mood = Some(resolved.mood);
                bubble.face_emotion = emotion_slug(resolved.emotion);
                bubble.face = FaceAnim::default();
                if let Some(ui) = &ui {
                    show_pill_face(
                        &mut bubble,
                        &asset_server,
                        ui,
                        &mut images,
                        PillSpeaker::Companion(selected.0),
                    );
                    commands.entity(ui.root).insert(Visibility::Visible);
                    if let Ok(mut name) = texts.get_mut(ui.name) {
                        name.0 = selected.0.display_name().to_string();
                    }
                    if let Ok(mut body) = texts.get_mut(ui.body) {
                        body.0 = String::new();
                    }
                    commands.entity(ui.advance).insert(Visibility::Hidden);
                }
                for (entity, mut sprite) in &mut companions {
                    sprite.mood = resolved.mood;
                    sprite.frame = 0;
                    trigger_mood_pop(&mut commands, entity);
                }
                sfx.play(&mut commands, SfxKind::Dialogue);
            }
            OfferOutcome::DroppedBusy => {}
        }
    }
}

/// Load `speaker`'s close-up into the mugshot window (only when it
/// changes — portrait loads are not free).
fn show_pill_face(
    bubble: &mut BubbleState,
    asset_server: &AssetServer,
    ui: &BubbleUi,
    images: &mut Query<&mut ImageNode>,
    speaker: PillSpeaker,
) {
    bubble.face_speaker = Some(speaker);
    let path = super::pill::resolve_face_path(
        speaker,
        bubble.face_emotion,
        super::pill::FaceVariant::Base,
        &bubble.face_available,
    );
    if bubble.face_path.as_deref() == Some(path.as_str()) {
        return;
    }
    bubble.face_path = Some(path.clone());
    if let Ok(mut image) = images.get_mut(ui.face) {
        image.image = asset_server.load(path);
    }
}

/// Canonical slug for an emotion (the faces-layout file stem).
fn emotion_slug(emotion: Emotion) -> &'static str {
    match emotion {
        Emotion::Neutral => "neutral",
        Emotion::Happy => "happy",
        Emotion::Angry => "angry",
        Emotion::Sad => "sad",
        Emotion::Surprised => "surprised",
        Emotion::Playful => "fun",
        Emotion::Annoyed => "annoyed",
        Emotion::Worried => "worried",
        Emotion::Embarrassed => "embarrassed",
        Emotion::Determined => "determined",
        Emotion::Smug => "smug",
    }
}

/// Drain queued pill lines (results / Warehouse / quiz producers)
/// into the same pill the reactions use: same depth-1 queue, same
/// staleness discipline, same mugshot + typewriter + face stack.
#[allow(clippy::too_many_arguments)]
fn process_pill_lines(
    mut commands: Commands,
    mut inbox: ResMut<PillInbox>,
    mut bubble: ResMut<BubbleState>,
    asset_server: Res<AssetServer>,
    sfx: Option<Res<Sfx>>,
    ui: Option<Res<BubbleUi>>,
    mut texts: Query<&mut Text>,
    mut images: Query<&mut ImageNode>,
) {
    if inbox.is_empty() {
        return;
    }
    for PillLine { presentation } in inbox.drain() {
        // The line is offered at the emotion it carries on its
        // presentation (tutorial segments carry their authored
        // emotion; results/quiz producers default to Neutral — see
        // `pill::present`), and the mugshot shows that emotion's face.
        match bubble
            .queue
            .offer(presentation.text.clone(), presentation.emotion)
        {
            OfferOutcome::Shown | OfferOutcome::ReplacedStale => {
                bubble.typewriter = Some(Typewriter::new(&presentation.text));
                bubble.applied_mood = None;
                bubble.face_emotion = emotion_slug(presentation.emotion);
                bubble.face = FaceAnim::default();
                if let Some(ui) = &ui {
                    show_pill_face(
                        &mut bubble,
                        &asset_server,
                        ui,
                        &mut images,
                        presentation.speaker,
                    );
                    commands.entity(ui.root).insert(Visibility::Visible);
                    if let Ok(mut name) = texts.get_mut(ui.name) {
                        name.0 = presentation.speaker_name.to_string();
                    }
                    if let Ok(mut body) = texts.get_mut(ui.body) {
                        body.0 = String::new();
                    }
                    commands.entity(ui.advance).insert(Visibility::Hidden);
                }
                if let Some(sfx) = &sfx {
                    sfx.play(&mut commands, SfxKind::Dialogue);
                }
            }
            OfferOutcome::DroppedBusy => {}
        }
    }
}

/// Advance the bubble: typewriter reveal while it is up; on expiry,
/// hide it and return the companion's sprite to Idle — but only if her
/// mood is still the one this bubble applied (an outage alarm or a
/// splice reaction may have claimed the sprite meanwhile; their owners
/// manage their own moods).
fn tick_bubble(
    mut commands: Commands,
    time: Res<Time>,
    mut bubble: ResMut<BubbleState>,
    ui: Option<Res<BubbleUi>>,
    asset_root: Option<Res<crate::audio::AssetRootDir>>,
    asset_server: Res<AssetServer>,
    mut texts: Query<&mut Text>,
    mut faces: Query<&mut UiTransform, With<ImageNode>>,
    mut face_images: Query<&mut ImageNode>,
    mut companions: Query<&mut CompanionSprite>,
) {
    if !bubble.face_probed {
        bubble.face_probed = true;
        if let Some(root) = &asset_root {
            bubble.face_available = super::pill::probe_faces(&root.0);
        }
    }
    if !bubble.queue.is_active() {
        return;
    }
    let dt = time.delta_secs();
    let mut typing = false;
    if let Some(typewriter) = &mut bubble.typewriter {
        typewriter.tick(dt);
        typing = !typewriter.is_completed();
        if let Some(ui) = &ui {
            if let Ok(mut body) = texts.get_mut(ui.body) {
                body.0 = typewriter.revealed_text();
            }
            commands.entity(ui.advance).insert(if typing {
                Visibility::Hidden
            } else {
                Visibility::Visible
            });
        }
    }
    // Face stack: blink on its idle cycle + mouth flap while typing,
    // layered over the emotion base. Where the art program's faces
    // tree provides the variant frame (`_blink` / `_talk`), the
    // mugshot swaps to it; where it does not (the whole tree is still
    // being produced), the derived squash stands in — same timing,
    // honest source.
    let frame = bubble.face.tick(dt, typing);
    if let Some(ui) = &ui {
        let variant = if frame.eyes_closed {
            super::pill::FaceVariant::Blink
        } else if frame.mouth_open {
            super::pill::FaceVariant::Talk
        } else {
            super::pill::FaceVariant::Base
        };
        let mut swapped = false;
        if let Some(speaker) = bubble.face_speaker {
            let path = super::pill::resolve_face_path(
                speaker,
                bubble.face_emotion,
                variant,
                &bubble.face_available,
            );
            if path.contains("art/faces/") && bubble.face_path.as_deref() != Some(path.as_str()) {
                bubble.face_path = Some(path.clone());
                if let Ok(mut image) = face_images.get_mut(ui.face) {
                    image.image = asset_server.load(path);
                }
                swapped = true;
            }
        }
        if let Ok(mut transform) = faces.get_mut(ui.face) {
            transform.scale = Vec2::new(1.0, if swapped { 1.0 } else { FaceAnim::scale_y(frame) });
        }
    }
    if bubble.queue.tick(dt) {
        bubble.typewriter = None;
        if let Some(ui) = &ui {
            commands.entity(ui.root).insert(Visibility::Hidden);
        }
        if let Some(mood) = bubble.applied_mood.take() {
            for mut sprite in &mut companions {
                if sprite.mood == mood {
                    sprite.mood = Mood::Idle;
                    sprite.frame = 0;
                }
            }
        }
    }
}

/// Whether the pill has FULLY finished its current run: no line is
/// up in the queue, the typewriter (if one survives its line) has
/// typed out, and no reaction or pill line is still waiting to be
/// offered. A harness without the pill state at all counts as
/// complete — the gate must never stall on a surface that does not
/// exist (headless degrade, the same pattern as the gate resource
/// itself). The driver and the pill's own tick systems are installed
/// together by [`ReactionsPlugin`], so wherever the driver runs the
/// queue is guaranteed to be draining.
fn pill_run_complete(
    bubble: Option<&BubbleState>,
    reaction_inbox: Option<&ReactionInbox>,
    pill_inbox: Option<&PillInbox>,
) -> bool {
    let Some(bubble) = bubble else {
        return true;
    };
    if bubble.queue.is_active() {
        return false;
    }
    if let Some(typewriter) = &bubble.typewriter {
        if !typewriter.is_completed() {
            return false;
        }
    }
    if let Some(inbox) = reaction_inbox {
        if !inbox.is_empty() {
            return false;
        }
    }
    if let Some(inbox) = pill_inbox {
        if !inbox.is_empty() {
            return false;
        }
    }
    true
}

/// Tick the completion hold; once its minimum has elapsed AND the
/// pill signals its run is complete (see [`pill_run_complete`]),
/// release the hold and request the Results transition the
/// level-ending system deferred. This is the only system that turns
/// a held completion into a screen change, so no reaction can be
/// flash-covered by a state change. When no reaction was queued the
/// minimum is the sole condition — no phantom wait.
fn drive_results_gate(
    time: Res<Time>,
    gate: Option<ResMut<ResultsGate>>,
    bubble: Option<Res<BubbleState>>,
    reaction_inbox: Option<Res<ReactionInbox>>,
    pill_inbox: Option<Res<PillInbox>>,
    mut request: ResMut<TransitionRequest>,
) {
    let Some(mut gate) = gate else {
        return;
    };
    gate.tick(time.delta_secs());
    if !gate.minimum_met() || request.0.is_some() {
        return;
    }
    let pill_done = pill_run_complete(
        bubble.as_deref(),
        reaction_inbox.as_deref(),
        pill_inbox.as_deref(),
    );
    if pill_done && gate.release() {
        request.0 = Some(GameState::Results);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waifu::sprite;

    // ---- emotion mapping ----

    #[test]
    fn every_canonical_emotion_maps_to_a_real_sheet_row() {
        for emotion in Emotion::ALL {
            let row = emotion.mood().sheet_row();
            assert!(
                row < sprite::MOOD_ROWS as usize,
                "{emotion:?} maps to row {row}, sheet has {}",
                sprite::MOOD_ROWS
            );
        }
        assert_eq!(Emotion::Neutral.mood(), Mood::Idle);
        assert_eq!(Emotion::Happy.mood(), Mood::Celebrate);
        assert_eq!(Emotion::Worried.mood(), Mood::Alarmed);
        assert_eq!(Emotion::Embarrassed.mood(), Mood::Blush);
    }

    // ---- trigger → line → emotion resolution ----

    #[test]
    fn every_trigger_resolves_for_every_main_companion() {
        for companion in Companion::ALL {
            let bank = DialogueBank::load_default(companion);
            for trigger in ReactionTrigger::ALL {
                let resolved = resolve_reaction(&bank, trigger);
                assert!(
                    resolved.is_some(),
                    "{companion:?} has no line for {trigger:?}"
                );
                let resolved = resolved.expect("checked above");
                assert!(!resolved.line.is_empty());
                let (_key, emotion) = trigger_spec(trigger);
                assert_eq!(resolved.emotion, emotion);
                assert_eq!(resolved.mood, emotion.mood());
            }
        }
    }

    #[test]
    fn specialist_banks_resolve_what_they_have_and_skip_the_rest() {
        // Specialists ship partial banks (greeting + level_win only):
        // reaction triggers resolve to None — a documented coverage
        // gap, and it must degrade to silence, never a panic or a
        // borrowed line in another companion's voice.
        for companion in [
            Companion::Clara,
            Companion::Aino,
            Companion::Hikari,
            Companion::Lea,
        ] {
            let bank = DialogueBank::load_default(companion);
            for trigger in ReactionTrigger::ALL {
                assert!(
                    resolve_reaction(&bank, trigger).is_none(),
                    "{companion:?} unexpectedly resolves {trigger:?}"
                );
            }
        }
    }

    // ---- tutorial spec table ----

    #[test]
    fn tutorial_spec_keys_resolve_in_every_companions_bank() {
        // Every (companion, stage) the compiled table references must
        // resolve in that companion's bank — a missing key would be a
        // silent no-op at runtime, so the table is pinned here.
        for companion in [
            Companion::Fiber,
            Companion::Coax,
            Companion::Mobile,
            Companion::Ethernet,
            Companion::Clara,
            Companion::Aino,
            Companion::Hikari,
            Companion::Lea,
        ] {
            let bank = DialogueBank::load_default(companion);
            for stage in TutorialStage::ALL {
                let (key, _emotion) = tutorial_spec(companion, stage);
                assert_eq!(key, stage.key());
                assert!(
                    bank.lines.get(key).is_some_and(|l| !l.is_empty()),
                    "{companion:?} bank is missing tutorial key '{key}'"
                );
            }
        }
    }

    #[test]
    fn tutorial_spec_emotions_vary_per_companion_as_authored() {
        // Spot-pin the segments file's per-companion variance: the
        // cases a single global row could not express.
        assert_eq!(
            tutorial_spec(Companion::Ethernet, TutorialStage::ScoringAndCores).1,
            Emotion::Smug
        );
        assert_eq!(
            tutorial_spec(Companion::Clara, TutorialStage::ScoringAndCores).1,
            Emotion::Neutral
        );
        assert_eq!(
            tutorial_spec(Companion::Fiber, TutorialStage::ScoringAndCores).1,
            Emotion::Playful
        );
        assert_eq!(
            tutorial_spec(Companion::Coax, TutorialStage::Wrap).1,
            Emotion::Smug
        );
        assert_eq!(
            tutorial_spec(Companion::Mobile, TutorialStage::BriefingBudgetMath).1,
            Emotion::Neutral
        );
        assert_eq!(
            tutorial_spec(Companion::Ethernet, TutorialStage::FirstOutage).1,
            Emotion::Annoyed
        );
    }

    // ---- bubble queue discipline ----

    #[test]
    fn queue_shows_first_offer_and_drops_while_fresh() {
        let mut queue = BubbleQueue::default();
        assert_eq!(
            queue.offer("first".into(), Emotion::Happy),
            OfferOutcome::Shown
        );
        queue.tick(1.0); // younger than STALE_SECS
        assert_eq!(
            queue.offer("second".into(), Emotion::Sad),
            OfferOutcome::DroppedBusy
        );
        assert_eq!(queue.active().expect("bubble up").line, "first");
    }

    #[test]
    fn queue_replaces_a_stale_bubble() {
        let mut queue = BubbleQueue::default();
        queue.offer("a much longer stale line".into(), Emotion::Happy);
        queue.tick(STALE_SECS + 0.1);
        assert_eq!(
            queue.offer("new".into(), Emotion::Worried),
            OfferOutcome::ReplacedStale
        );
        let active = queue.active().expect("bubble up");
        assert_eq!(active.line, "new");
        assert_eq!(active.age_secs, 0.0);
    }

    #[test]
    fn queue_expires_at_its_read_time() {
        let mut queue = BubbleQueue::default();
        queue.offer("hi".into(), Emotion::Neutral);
        let read = queue.active().expect("bubble up").read_secs;
        assert!(!queue.tick(read - 0.01));
        assert!(queue.is_active());
        assert!(queue.tick(0.02), "crossing the read time must expire");
        assert!(!queue.is_active());
    }

    #[test]
    fn read_time_is_clamped_to_bounds() {
        assert_eq!(read_time_secs(""), READ_MIN_SECS);
        assert_eq!(read_time_secs(&"x".repeat(10_000)), READ_MAX_SECS);
        let mid = read_time_secs(&"x".repeat(40));
        assert!(mid > READ_MIN_SECS && mid < READ_MAX_SECS);
    }

    #[test]
    fn inbox_is_bounded_and_drops_oldest() {
        let mut inbox = ReactionInbox::default();
        for _ in 0..INBOX_CAPACITY + 4 {
            inbox.push(ReactionTrigger::TooHot);
        }
        inbox.push(ReactionTrigger::LevelComplete);
        let drained = inbox.drain();
        assert_eq!(drained.len(), INBOX_CAPACITY);
        assert_eq!(
            drained.last(),
            Some(&ReactionTrigger::LevelComplete),
            "freshest triggers survive the cap"
        );
        assert!(inbox.is_empty());
    }

    // ---- no dialogue without an event ----

    #[test]
    fn quiet_queue_never_shows_anything() {
        // Five simulated minutes of ticks with no offer: the bubble
        // must stay down the entire time. Dialogue exists only as a
        // reaction to an event.
        let mut queue = BubbleQueue::default();
        for _ in 0..600 {
            assert!(!queue.tick(0.5));
            assert!(!queue.is_active());
        }
    }

    /// App with the companion + reaction plugins and a loaded level,
    /// ready to enter `Playing`.
    fn reaction_test_app() -> App {
        use crate::waifu::SeraphinePlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), bevy::state::app::StatesPlugin));
        app.init_asset::<bevy::image::Image>();
        app.init_asset::<bevy::image::TextureAtlasLayout>();
        app.init_asset::<bevy::text::Font>();
        app.init_state::<GameState>();
        app.init_resource::<crate::anim::TransitionRequest>();
        app.insert_resource(crate::audio::Sfx::for_tests());
        app.insert_resource(crate::level::load_level(0));
        app.init_resource::<LiveGraph>();
        app.init_resource::<PlacedChoices>();
        app.init_resource::<LevelClock>();
        app.add_plugins(SeraphinePlugin);
        app.add_plugins(ReactionsPlugin);
        app
    }

    fn enter_playing(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::Playing
        );
    }

    // ---- completion gate: the pill finishes before Results ----

    /// `reaction_test_app` plus the results gate and a fixed 0.1 s
    /// frame step, so gate/pill timing assertions are deterministic.
    fn gate_test_app() -> App {
        let mut app = reaction_test_app();
        app.init_resource::<ResultsGate>();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
        app
    }

    fn enter_outage(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::OutageActive);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            *app.world().resource::<State<GameState>>().get(),
            GameState::OutageActive
        );
    }

    /// Begin the completion hold and offer `text` to the pill — the
    /// two halves of what a level-ending system does on a win.
    fn begin_completion_with_line(app: &mut App, text: &str) {
        app.world_mut().resource_mut::<ResultsGate>().begin();
        app.world_mut()
            .resource_mut::<PillInbox>()
            .push_line(PillSpeaker::Companion(Companion::Fiber), text);
    }

    fn run_frames(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    fn results_requested(app: &App) -> bool {
        app.world().resource::<TransitionRequest>().0.is_some()
    }

    #[test]
    fn pill_run_complete_truth_table() {
        // No pill state at all: complete (headless degrade).
        assert!(pill_run_complete(None, None, None));
        let idle = BubbleState::default();
        assert!(
            pill_run_complete(Some(&idle), None, None),
            "an idle pill is complete"
        );
        // A line up in the queue blocks.
        let mut showing = BubbleState::default();
        let _ = showing.queue.offer("hello".to_string(), Emotion::Neutral);
        assert!(
            !pill_run_complete(Some(&showing), None, None),
            "a line up in the queue blocks"
        );
        // A typewriter that has not typed out blocks; completed frees.
        let mut typing = BubbleState::default();
        typing.typewriter = Some(Typewriter::new("hello"));
        assert!(
            !pill_run_complete(Some(&typing), None, None),
            "unfinished typing blocks"
        );
        typing.typewriter.as_mut().expect("set above").complete();
        assert!(
            pill_run_complete(Some(&typing), None, None),
            "typed-out text is complete"
        );
        // Work still waiting in either inbox blocks.
        let mut reactions = ReactionInbox::default();
        reactions.push(ReactionTrigger::LevelComplete);
        assert!(
            !pill_run_complete(Some(&idle), Some(&reactions), None),
            "a pending reaction blocks"
        );
        let mut pill_lines = PillInbox::default();
        pill_lines.push_line(PillSpeaker::Companion(Companion::Fiber), "queued");
        assert!(
            !pill_run_complete(Some(&idle), None, Some(&pill_lines)),
            "a pending pill line blocks"
        );
    }

    #[test]
    fn gate_driver_without_pill_state_releases_at_the_minimum() {
        use bevy::ecs::system::RunSystemOnce;
        // Headless shape: gate + request + clock, but no BubbleState
        // and no inboxes — the minimum is the sole condition.
        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<ResultsGate>();
        world.init_resource::<TransitionRequest>();
        world.resource_mut::<ResultsGate>().begin();
        for _ in 0..17 {
            world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(100));
            world
                .run_system_once(drive_results_gate)
                .expect("driver runs headless");
        }
        assert!(
            world.resource::<TransitionRequest>().0.is_none(),
            "released before the minimum"
        );
        world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(300));
        world
            .run_system_once(drive_results_gate)
            .expect("driver runs headless");
        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Results)
        );
    }

    #[test]
    fn gate_driver_without_a_gate_is_a_no_op() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<TransitionRequest>();
        world
            .run_system_once(drive_results_gate)
            .expect("driver tolerates a missing gate");
        assert!(world.resource::<TransitionRequest>().0.is_none());
    }

    #[test]
    fn gate_waits_for_a_long_completion_line_to_finish() {
        let mut app = gate_test_app();
        enter_playing(&mut app);
        // 100 chars: read time 1.4 + 4.5 = 5.9 s, typing ~3.3 s.
        let line = "a".repeat(100);
        begin_completion_with_line(&mut app, &line);
        run_frames(&mut app, 18); // the old fixed 1.8 s hold point
        assert!(
            !results_requested(&app),
            "results fired at the 1.8 s minimum while the line still runs"
        );
        run_frames(&mut app, 37); // 5.5 s: read time not yet elapsed
        assert!(
            !results_requested(&app),
            "results fired before the read time elapsed"
        );
        run_frames(&mut app, 10); // 6.5 s: the line's full run is done
        assert!(
            results_requested(&app),
            "results never fired after the line finished"
        );
    }

    #[test]
    fn gate_releases_a_short_line_at_the_minimum_not_instantly() {
        let mut app = gate_test_app();
        enter_playing(&mut app);
        begin_completion_with_line(&mut app, "ok"); // read clamps to 1.6 s
        run_frames(&mut app, 17); // 1.7 s: line done, minimum not met
        assert!(
            !results_requested(&app),
            "short line flashed past before the minimum hold"
        );
        run_frames(&mut app, 3); // 2.0 s
        assert!(
            results_requested(&app),
            "gate never released after the minimum"
        );
    }

    #[test]
    fn gate_without_a_line_releases_at_the_minimum_only() {
        let mut app = gate_test_app();
        enter_playing(&mut app);
        app.world_mut().resource_mut::<ResultsGate>().begin();
        run_frames(&mut app, 17);
        assert!(!results_requested(&app), "released before the minimum");
        run_frames(&mut app, 3);
        assert!(
            results_requested(&app),
            "no-line win stalled past the minimum (phantom wait)"
        );
    }

    #[test]
    fn gate_holds_during_outage_until_the_line_finishes() {
        // Outage resolutions begin the same gate from OutageActive;
        // the one shared driver must gate there too. (Quiz
        // completions begin it from Playing — the same driver and
        // state arm as the Playing scenarios above.)
        let mut app = gate_test_app();
        enter_outage(&mut app);
        let line = "a".repeat(100);
        begin_completion_with_line(&mut app, &line);
        run_frames(&mut app, 18);
        assert!(
            !results_requested(&app),
            "outage results covered the line at 1.8 s"
        );
        run_frames(&mut app, 37);
        assert!(!results_requested(&app));
        run_frames(&mut app, 10);
        assert!(results_requested(&app), "outage gate never released");
    }

    #[test]
    fn advancing_the_line_keeps_results_on_the_read_clock() {
        let mut app = gate_test_app();
        enter_playing(&mut app);
        let line = "a".repeat(100); // read time 5.9 s
        begin_completion_with_line(&mut app, &line);
        // 1.0 s in: still typing.
        run_frames(&mut app, 10);
        // The advance interaction completes the typewriter instantly
        // — the same `Typewriter::complete` the dialogue advance path
        // calls. The gate keys on completion state, so after this the
        // line's READ clock is what remains.
        let mut bubble = app.world_mut().resource_mut::<BubbleState>();
        let typewriter = bubble.typewriter.as_mut().expect("the line is typing");
        assert!(!typewriter.is_completed());
        typewriter.complete();
        drop(bubble);
        run_frames(&mut app, 10); // 2.0 s: past minimum, typing done
        assert!(
            !results_requested(&app),
            "advance skipped the read time: results followed the click, not the clock"
        );
        run_frames(&mut app, 45); // 6.5 s
        assert!(
            results_requested(&app),
            "gate never released after the read time"
        );
    }

    #[test]
    fn a_second_line_extends_the_gate_until_it_finishes() {
        let mut app = gate_test_app();
        enter_playing(&mut app);
        // Line A: 40 chars, read time 3.2 s.
        begin_completion_with_line(&mut app, &"a".repeat(40));
        // 2.1 s in: A is stale (> 2.0 s) but still up.
        run_frames(&mut app, 21);
        // Line B replaces A under the queue's staleness rule and runs
        // its own full 5.9 s (until ~8.0 s).
        app.world_mut()
            .resource_mut::<PillInbox>()
            .push_line(PillSpeaker::Companion(Companion::Fiber), "b".repeat(100));
        run_frames(&mut app, 14); // 3.5 s: past A's original expiry
        assert!(
            !results_requested(&app),
            "gate released on the first line's clock while the second still runs"
        );
        run_frames(&mut app, 40); // 7.5 s: B still reading
        assert!(!results_requested(&app));
        run_frames(&mut app, 11); // 8.6 s: B done
        assert!(
            results_requested(&app),
            "gate never released after the second line"
        );
    }

    #[test]
    fn entering_a_level_shows_no_dialogue_until_an_event_fires() {
        let mut app = reaction_test_app();
        enter_playing(&mut app);
        // Settle frames in Playing with an empty inbox: no bubble, and
        // the companion stays in her Idle mood.
        for _ in 0..10 {
            app.update();
        }
        assert!(
            !app.world().resource::<BubbleState>().queue.is_active(),
            "a bubble appeared without an event"
        );
        let world = app.world_mut();
        let mut query = world.query::<&CompanionSprite>();
        let sprite = query.single(world).expect("one companion sprite");
        assert_eq!(sprite.mood, Mood::Idle);
    }

    #[test]
    fn a_queued_pill_line_shows_the_pill_with_the_speakers_face() {
        let mut app = reaction_test_app();
        enter_playing(&mut app);
        app.world_mut()
            .resource_mut::<PillInbox>()
            .push_line(
                PillSpeaker::Host(crate::warehouse::HostId::Tessa),
                "Howdy.",
            );
        app.update();
        let bubble = &app.world().resource::<BubbleState>().queue;
        let active = bubble.active().expect("pill line must be up");
        assert_eq!(active.line, "Howdy.");
        let pres = crate::waifu::pill::present(PillSpeaker::Host(crate::warehouse::HostId::Tessa), "x");
        assert_eq!(pres.face_path, "art/companions/tessa_portrait.jpg");
        assert_eq!(pres.speaker_name, "Tessa");
    }

    #[test]
    fn a_pill_line_carries_its_emotion_to_the_render_path() {
        let mut app = reaction_test_app();
        enter_playing(&mut app);
        app.world_mut()
            .resource_mut::<PillInbox>()
            .push_line_with_emotion(
                PillSpeaker::Companion(Companion::Fiber),
                "Determined line.",
                Emotion::Determined,
            );
        app.update();
        let bubble = app.world().resource::<BubbleState>();
        let active = bubble.queue.active().expect("pill line must be up");
        assert_eq!(active.emotion, Emotion::Determined);
        assert_eq!(bubble.face_emotion, "determined");
    }

    #[test]
    fn a_default_pill_line_renders_neutral() {
        // Existing producers (results/quiz) keep their pre-tutorial
        // behaviour: no carried emotion means Neutral on the render
        // path, face included.
        let mut app = reaction_test_app();
        enter_playing(&mut app);
        app.world_mut()
            .resource_mut::<PillInbox>()
            .push_line(PillSpeaker::Companion(Companion::Fiber), "Plain line.");
        app.update();
        let bubble = app.world().resource::<BubbleState>();
        let active = bubble.queue.active().expect("pill line must be up");
        assert_eq!(active.emotion, Emotion::Neutral);
        assert_eq!(bubble.face_emotion, "neutral");
    }

    #[test]
    fn a_pushed_trigger_shows_the_bubble_and_swaps_the_mood() {
        let mut app = reaction_test_app();
        enter_playing(&mut app);
        app.world_mut()
            .resource_mut::<ReactionInbox>()
            .push(ReactionTrigger::LevelComplete);
        app.update();
        let bubble = &app.world().resource::<BubbleState>().queue;
        let active = bubble.active().expect("completion bubble must be up");
        assert_eq!(active.emotion, Emotion::Happy);
        let world = app.world_mut();
        let mut query = world.query::<&CompanionSprite>();
        let sprite = query.single(world).expect("one companion sprite");
        assert_eq!(sprite.mood, Mood::Celebrate);
    }
}
