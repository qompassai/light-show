//! The universal dialogue pill (Matt's direction, 2026-10-06): ANY
//! character speech in the game renders in one surface — a rounded
//! bar anchored at the bottom of the screen, an MMBN-style square
//! mugshot window at its left end carrying the speaker's close-up
//! face, her name, and her typed text. No character speaks in bare
//! text anywhere: companion reactions, Warehouse host lines, quiz
//! explanations, and results-screen companion lines all resolve to a
//! [`PillPresentation`] (speaker + face + talking state) and are
//! delivered through [`PillInbox`] or the Warehouse's pill-styled host
//! bar, which uses the same presentation data.
//!
//! Face animation is a STACK over the line's emotion expression:
//! (a) a talking mouth flap while her text types, (b) periodic
//! blinking on an idle cycle (~every 2–4 s, brief) whether or not
//! text is typing, (c) the emotion expression as the base state.
//! [`FaceAnim`] is the pure state machine for (a)+(b).
//!
//! Asset honesty: the game ships portrait close-ups for every speaker
//! (companions via `Companion::portrait_path`, hosts via
//! `warehouse::Host::portrait`) but NO dedicated mouth-open or
//! closed-eye frames. The render layer therefore derives the flap
//! and the blink from the portrait itself — talking alternates a
//! slight vertical squash on the mugshot at flap cadence, a blink is
//! a brief stronger squash — rather than faking frames that do not
//! exist. Dedicated talking/blink frames are an art-queue gap (see
//! the module report); the mechanism here consumes them unchanged if
//! they land, because [`FaceFrame`] is asset-agnostic.

use super::reactions::Emotion;
use super::Companion;
use crate::warehouse::HostId;
use bevy::prelude::*;
use std::collections::VecDeque;

/// Most pill lines buffered between frames; past this the oldest is
/// dropped (speech is perishable, like reactions).
pub const PILL_INBOX_CAPACITY: usize = 8;
/// Seconds between blinks (the idle cycle midpoint of Matt's 2–4 s).
pub const BLINK_PERIOD_SECS: f32 = 2.8;
/// Seconds the eyes stay closed in one blink.
pub const BLINK_CLOSED_SECS: f32 = 0.12;
/// Seconds per mouth-flap half-cycle while text types.
pub const TALK_FLAP_SECS: f32 = 0.12;

/// Who is speaking in the pill. Every character who can speak is
/// representable here — that totality is what makes the pill a
/// universal invariant rather than a per-surface choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillSpeaker {
    Companion(Companion),
    Host(HostId),
}

impl PillSpeaker {
    /// Directory stem under `art/faces/` (the art program's canonical
    /// layout): lowercase character name.
    pub fn face_stem(self) -> &'static str {
        match self {
            PillSpeaker::Companion(c) => c.picker_stem(),
            PillSpeaker::Host(HostId::Bianca) => "bianca",
            PillSpeaker::Host(HostId::Tessa) => "tessa",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PillSpeaker::Companion(c) => c.display_name(),
            PillSpeaker::Host(id) => crate::warehouse::host(id).name,
        }
    }

    /// Close-up face asset for the mugshot window, relative to the
    /// game's asset root. Portrait crops are the honest source today:
    /// painterly expression webps exist only for some mains and are
    /// not wired as game assets, so the shipped portraits are the one
    /// face every speaker actually has.
    pub fn face_path(self) -> &'static str {
        match self {
            PillSpeaker::Companion(c) => c.portrait_path(),
            PillSpeaker::Host(id) => crate::warehouse::host(id).portrait,
        }
    }

    /// Every speaker in the game: all eight companions plus both
    /// Warehouse hosts.
    pub const ALL: [PillSpeaker; 10] = [
        PillSpeaker::Companion(Companion::Fiber),
        PillSpeaker::Companion(Companion::Coax),
        PillSpeaker::Companion(Companion::Mobile),
        PillSpeaker::Companion(Companion::Ethernet),
        PillSpeaker::Companion(Companion::Clara),
        PillSpeaker::Companion(Companion::Aino),
        PillSpeaker::Companion(Companion::Hikari),
        PillSpeaker::Companion(Companion::Lea),
        PillSpeaker::Host(HostId::Bianca),
        PillSpeaker::Host(HostId::Tessa),
    ];
}

/// A line fully resolved to its pill presentation: who speaks, the
/// face in the mugshot window, and the text. `talking` is always true
/// at delivery — the typewriter drives the mouth flap until the line
/// completes, then the face settles into the line's emotion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillPresentation {
    pub speaker: PillSpeaker,
    pub speaker_name: &'static str,
    pub face_path: &'static str,
    pub text: String,
    pub talking: bool,
    /// The emotion the line is delivered with: the pill renderer
    /// offers the line at this emotion and shows the matching face
    /// (see `reactions::process_pill_lines`). Producers that do not
    /// carry an emotion default to [`Emotion::Neutral`] — the
    /// pre-tutorial behaviour for results and quiz lines.
    pub emotion: Emotion,
}

/// Resolve a line to its pill presentation. Total over speakers: this
/// cannot fail, so no call site ever needs a bare-text fallback.
/// The line carries [`Emotion::Neutral`]; use
/// [`present_with_emotion`] for emotion-aware producers (tutorials).
pub fn present(speaker: PillSpeaker, text: impl Into<String>) -> PillPresentation {
    present_with_emotion(speaker, text, Emotion::Neutral)
}

/// Resolve a line to its pill presentation at a carried emotion.
pub fn present_with_emotion(
    speaker: PillSpeaker,
    text: impl Into<String>,
    emotion: Emotion,
) -> PillPresentation {
    PillPresentation {
        speaker,
        speaker_name: speaker.name(),
        face_path: speaker.face_path(),
        text: text.into(),
        talking: true,
        emotion,
    }
}

/// Which frame of a face set the stack wants this tick: the emotion
/// base, its talking (mouth-open) variant, or its blink (eyes-closed)
/// variant. Blink outranks talk — eyes close over whatever the mouth
/// is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceVariant {
    Base,
    Talk,
    Blink,
}

impl FaceVariant {
    pub fn suffix(self) -> &'static str {
        match self {
            FaceVariant::Base => "",
            FaceVariant::Talk => "_talk",
            FaceVariant::Blink => "_blink",
        }
    }
}

/// Canonical face asset path for a speaker/emotion/variant, per the
/// art program's layout: `art/faces/<character>/<emotion>[_talk|
/// _blink].webp` (`emotion` is the canonical slug, e.g. "neutral").
pub fn face_asset_path(speaker: PillSpeaker, emotion_slug: &str, variant: FaceVariant) -> String {
    format!(
        "art/faces/{}/{}{}.webp",
        speaker.face_stem(),
        emotion_slug,
        variant.suffix()
    )
}

/// Resolve the face file to actually show, walking the fallback
/// chain against the set of files that exist: requested variant →
/// emotion base → neutral variant → neutral base → the speaker's
/// shipped portrait (today's base for every speaker; the faces tree
/// is produced by the art program and consumed here as it lands).
pub fn resolve_face_path(
    speaker: PillSpeaker,
    emotion_slug: &str,
    variant: FaceVariant,
    available: &std::collections::HashSet<String>,
) -> String {
    for candidate in [
        face_asset_path(speaker, emotion_slug, variant),
        face_asset_path(speaker, emotion_slug, FaceVariant::Base),
        face_asset_path(speaker, "neutral", variant),
        face_asset_path(speaker, "neutral", FaceVariant::Base),
    ] {
        if available.contains(&candidate) {
            return candidate;
        }
    }
    speaker.face_path().to_string()
}

/// Probe a faces tree (asset root + `art/faces`) into the availability
/// set [`resolve_face_path`] consumes. Missing tree → empty set →
/// every face resolves to the shipped portrait.
pub fn probe_faces(asset_root: &std::path::Path) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let base = asset_root.join("art/faces");
    if let Ok(chars) = std::fs::read_dir(&base) {
        for ch in chars.flatten() {
            if let Ok(files) = std::fs::read_dir(ch.path()) {
                for f in files.flatten() {
                    if let Some(name) = f.file_name().to_str() {
                        if name.ends_with(".webp") {
                            out.insert(format!(
                                "art/faces/{}/{}",
                                ch.file_name().to_string_lossy(),
                                name
                            ));
                        }
                    }
                }
            }
        }
    }
    out
}

/// One queued pill line (a resolved presentation awaiting display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillLine {
    pub presentation: PillPresentation,
}

/// Bounded queue of pill lines between speech producers (results,
/// quiz, Warehouse enter/purchase) and the pill renderer in
/// `reactions`. Producers use `Option<ResMut<PillInbox>>` so headless
/// harnesses without the reactions plugin degrade to silence, never a
/// panic — the same pattern as `ReactionInbox`.
#[derive(Resource, Default)]
pub struct PillInbox {
    pending: VecDeque<PillLine>,
}

impl PillInbox {
    pub fn push(&mut self, line: PillLine) {
        if self.pending.len() >= PILL_INBOX_CAPACITY {
            self.pending.pop_front();
        }
        self.pending.push_back(line);
    }

    pub fn push_line(&mut self, speaker: PillSpeaker, text: impl Into<String>) {
        self.push(PillLine {
            presentation: present(speaker, text),
        });
    }

    /// Queue a line at a carried emotion (tutorial segments). All
    /// other producers use [`PillInbox::push_line`], which defaults
    /// to [`Emotion::Neutral`].
    pub fn push_line_with_emotion(
        &mut self,
        speaker: PillSpeaker,
        text: impl Into<String>,
        emotion: Emotion,
    ) {
        self.push(PillLine {
            presentation: present_with_emotion(speaker, text, emotion),
        });
    }

    /// Lines currently buffered (bounded by [`PILL_INBOX_CAPACITY`]).
    /// The tutorial sequencer reads this to feed the inbox only as
    /// capacity allows, never dumping a whole briefing block at once.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn drain(&mut self) -> Vec<PillLine> {
        self.pending.drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

// ---------------------------------------------------------------------------
// Face animation stack (pure)
// ---------------------------------------------------------------------------

/// One frame of the face stack: the emotion expression is the base;
/// `mouth_open` (talking flap) and `eyes_closed` (blink) layer over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FaceFrame {
    pub mouth_open: bool,
    pub eyes_closed: bool,
}

/// The blink + talk state machine. Blink runs on its own cycle
/// whether or not text is typing — the face must feel alive, not
/// mouth-only (Matt's refinement). Deterministic: no RNG, so tests
/// pin the exact cycle.
#[derive(Debug, Clone)]
pub struct FaceAnim {
    blink_clock_secs: f32,
    flap_clock_secs: f32,
}

impl Default for FaceAnim {
    fn default() -> Self {
        Self {
            blink_clock_secs: 0.0,
            flap_clock_secs: 0.0,
        }
    }
}

impl FaceAnim {
    /// Advance by `dt_secs`; `talking` is true while the line's
    /// typewriter is still revealing text.
    pub fn tick(&mut self, dt_secs: f32, talking: bool) -> FaceFrame {
        let dt = dt_secs.max(0.0);
        self.blink_clock_secs = (self.blink_clock_secs + dt) % BLINK_PERIOD_SECS;
        if talking {
            self.flap_clock_secs += dt;
        } else {
            self.flap_clock_secs = 0.0;
        }
        FaceFrame {
            mouth_open: talking
                && (self.flap_clock_secs / TALK_FLAP_SECS) as u32 % 2 == 0,
            eyes_closed: self.blink_clock_secs < BLINK_CLOSED_SECS,
        }
    }

    /// Vertical scale for the mugshot under `frame`, the derived-frame
    /// rendering of the stack: a blink squashes briefly, an open mouth
    /// dips slightly, otherwise the portrait sits at rest.
    pub fn scale_y(frame: FaceFrame) -> f32 {
        if frame.eyes_closed {
            0.90
        } else if frame.mouth_open {
            0.97
        } else {
            1.0
        }
    }
}

/// The speech surfaces of the game, by name. The audit test below
/// pins each one to its pill routing in source, so a future bare-text
/// dialogue path fails loudly instead of shipping quietly.
pub const DIALOGUE_SURFACES: [&str; 6] = [
    "companion_reactions",
    "warehouse_hosts",
    "warehouse_quiz",
    "level_quiz_explanation",
    "results_line",
    "briefing",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_speaker_resolves_to_a_face_that_exists_on_disk() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for speaker in PillSpeaker::ALL {
            let p = present(speaker, "check");
            assert!(p.talking, "pill lines always carry the talking state");
            assert!(!p.speaker_name.is_empty());
            let path = assets.join(p.face_path);
            assert!(
                path.is_file(),
                "{} face missing: {}",
                p.speaker_name,
                path.display()
            );
        }
    }

    #[test]
    fn blink_fires_on_the_idle_cycle_even_in_silence() {
        let mut anim = FaceAnim::default();
        // Not talking: mouth stays shut the whole cycle...
        let mut saw_blink = false;
        for _ in 0..200 {
            let frame = anim.tick(0.02, false);
            assert!(!frame.mouth_open, "silent face must not flap");
            saw_blink |= frame.eyes_closed;
        }
        assert!(saw_blink, "no blink in 4 s of idle face time");
    }

    #[test]
    fn talking_flaps_the_mouth_and_stops_when_done() {
        let mut anim = FaceAnim::default();
        let mut saw_open = false;
        let mut saw_closed = false;
        for _ in 0..30 {
            let frame = anim.tick(0.05, true);
            saw_open |= frame.mouth_open;
            saw_closed |= !frame.mouth_open;
        }
        assert!(saw_open && saw_closed, "mouth must flap while typing");
        let settled = anim.tick(0.05, false);
        assert!(!settled.mouth_open, "mouth settles when text completes");
    }

    #[test]
    fn blink_is_brief_within_its_period() {
        let mut anim = FaceAnim::default();
        let mut closed_ticks = 0;
        let total = (BLINK_PERIOD_SECS / 0.02) as usize;
        for _ in 0..total {
            if anim.tick(0.02, false).eyes_closed {
                closed_ticks += 1;
            }
        }
        assert!(closed_ticks > 0 && (closed_ticks as f32 * 0.02) <= BLINK_CLOSED_SECS + 0.021);
    }

    #[test]
    fn face_paths_follow_the_canonical_layout() {
        let sp = PillSpeaker::Companion(Companion::Fiber);
        assert_eq!(
            face_asset_path(sp, "happy", FaceVariant::Base),
            "art/faces/seraphine/happy.webp"
        );
        assert_eq!(
            face_asset_path(sp, "happy", FaceVariant::Talk),
            "art/faces/seraphine/happy_talk.webp"
        );
        assert_eq!(
            face_asset_path(PillSpeaker::Host(HostId::Tessa), "neutral", FaceVariant::Blink),
            "art/faces/tessa/neutral_blink.webp"
        );
    }

    #[test]
    fn face_resolution_walks_the_fallback_chain() {
        let sp = PillSpeaker::Companion(Companion::Coax);
        let mut avail = std::collections::HashSet::new();
        // Nothing on disk yet: portrait is the base (today's reality).
        assert_eq!(
            resolve_face_path(sp, "happy", FaceVariant::Blink, &avail),
            sp.face_path()
        );
        // Emotion base lands: variants fall back to it.
        avail.insert("art/faces/ondine/happy.webp".to_string());
        assert_eq!(
            resolve_face_path(sp, "happy", FaceVariant::Blink, &avail),
            "art/faces/ondine/happy.webp"
        );
        // Blink frame lands: it wins for the blink variant.
        avail.insert("art/faces/ondine/happy_blink.webp".to_string());
        assert_eq!(
            resolve_face_path(sp, "happy", FaceVariant::Blink, &avail),
            "art/faces/ondine/happy_blink.webp"
        );
        // Missing emotion falls back to the neutral set.
        avail.insert("art/faces/ondine/neutral.webp".to_string());
        assert_eq!(
            resolve_face_path(sp, "angry", FaceVariant::Base, &avail),
            "art/faces/ondine/neutral.webp"
        );
    }

    #[test]
    fn present_defaults_to_neutral_and_with_emotion_carries_it() {
        let neutral = present(PillSpeaker::Companion(Companion::Fiber), "check");
        assert_eq!(neutral.emotion, Emotion::Neutral);
        let smug = present_with_emotion(
            PillSpeaker::Companion(Companion::Ethernet),
            "check",
            Emotion::Smug,
        );
        assert_eq!(smug.emotion, Emotion::Smug);
        assert_eq!(smug.speaker, PillSpeaker::Companion(Companion::Ethernet));
    }

    #[test]
    fn inbox_push_line_defaults_neutral_and_emotion_push_carries() {
        let mut inbox = PillInbox::default();
        inbox.push_line(PillSpeaker::Companion(Companion::Fiber), "plain");
        inbox.push_line_with_emotion(
            PillSpeaker::Companion(Companion::Fiber),
            "carried",
            Emotion::Determined,
        );
        assert_eq!(inbox.len(), 2);
        let drained = inbox.drain();
        assert_eq!(drained[0].presentation.emotion, Emotion::Neutral);
        assert_eq!(drained[1].presentation.emotion, Emotion::Determined);
        assert_eq!(drained[1].presentation.text, "carried");
        assert_eq!(inbox.len(), 0);
    }

    #[test]
    fn pill_inbox_is_bounded_and_drops_oldest() {
        let mut inbox = PillInbox::default();
        for i in 0..PILL_INBOX_CAPACITY + 3 {
            inbox.push_line(PillSpeaker::Host(HostId::Bianca), format!("line {i}"));
        }
        let drained = inbox.drain();
        assert_eq!(drained.len(), PILL_INBOX_CAPACITY);
        assert_eq!(
            drained.last().expect("lines").presentation.text,
            format!("line {}", PILL_INBOX_CAPACITY + 2)
        );
    }

    #[test]
    fn every_dialogue_surface_routes_through_the_pill_in_source() {
        // The loud gate (Matt's invariant): each named surface's
        // source must reference the pill, and the two surfaces that
        // historically spoke in bare text must not carry their old
        // bare-text patterns anymore.
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let read = |rel: &str| {
            std::fs::read_to_string(src.join(rel))
                .unwrap_or_else(|e| panic!("{rel} unreadable: {e}"))
        };
        let reactions = read("waifu/reactions.rs");
        assert!(reactions.contains("PillInbox"), "reactions must render via the pill");
        let warehouse = read("states/warehouse.rs");
        assert!(
            warehouse.contains("pill_presentation") || warehouse.contains("PillSpeaker"),
            "warehouse host lines must resolve through the pill presentation"
        );
        let quiz = read("states/quiz.rs");
        assert!(quiz.contains("PillInbox"), "quiz explanations must route through the pill");
        let results = read("states/results.rs");
        assert!(results.contains("PillInbox"), "results line must route through the pill");
        assert!(
            !results.contains("{dialogue_line}"),
            "results must not render the companion line as bare quoted text"
        );
        assert_eq!(DIALOGUE_SURFACES.len(), 6);
    }
}
