//! Font asset paths for the two-family UI type system.
//!
//! * Display — Monaspace Neon (SIL OFL 1.1): headings, titles, scores, node
//!   labels, dB readouts. Monospace, so tabular numbers align.
//! * Body — Inter (SIL OFL 1.1): briefings, dialogs, buttons, credits.
//!
//! License texts live next to the files in `game/assets/fonts/`.

/// Display face, regular: headings, node labels, readouts.
pub const DISPLAY: &str = "fonts/MonaspaceNeon-Regular.otf";
/// Display face, bold: game title, result banners, urgent callouts.
pub const DISPLAY_BOLD: &str = "fonts/MonaspaceNeon-Bold.otf";
/// Body face, regular: briefings, dialogs, flavor text, credits.
pub const BODY: &str = "fonts/Inter-Regular.ttf";
/// Body face, medium: buttons and other small UI text.
pub const BODY_MEDIUM: &str = "fonts/Inter-Medium.ttf";

/// Font-size adjustment for the 0.19 text engine (parley).
///
/// The 0.15 migration guide prescribes dividing pre-0.15 point sizes by
/// 1.2 because cosmic-text rendered the same size larger than the 0.14
/// ab_glyph stack; parley (0.19) carries the same guidance forward.
/// Every `TextFont` construction multiplies its size by this factor.
/// FLAG FOR MATT'S VISUAL REVIEW: if headings or buttons read too
/// small/large after the migration, this single constant is the knob.
pub const FONT_SIZE_ADJUST: f32 = 1.0 / 1.2;
