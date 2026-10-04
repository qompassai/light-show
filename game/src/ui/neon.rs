//! Neon-circuit text: the menu title-card style (bright core + a soft
//! two-ring halo that reads as luminous glow around the glyphs).
//! Shared by the main menu and the companion-select screen so both read
//! as the same title card.
//!
//! Why two rings with alpha falloff instead of one solid ring: a single
//! full-alpha ring reads as a thick blocky outline next to the keeper
//! artwork's smooth painted lettering. Eight copies at a 1px inner radius
//! (~55% alpha) plus eight more at 2px (~25% alpha) blend into a halo
//! that the eye reads as light, not geometry.

use bevy::color::Srgba;
use bevy::prelude::*;

use crate::fonts::FONT_SIZE_ADJUST;

/// Neon-circuit palette, sampled from the keeper title artwork (angular
/// cyan-to-gold letterforms, circuit-bracket frame, night-city dark).
pub const NEON_CYAN: Color = Color::srgb(0.435, 0.949, 1.0); // #6ff2ff
pub const NEON_GOLD: Color = Color::srgb(1.0, 0.82, 0.4); // #ffd166
pub const NEON_DIM: Color = Color::srgb(0.55, 0.62, 0.72); // dim slate-cyan
pub const NEON_INK: Color = Color::srgb(0.04, 0.055, 0.1); // night-city dark

/// Eight-way glow compass (cardinals + diagonals), one copy per entry
/// per ring. Eight copies at a 1px radius form a continuous ring; four
/// diagonals alone leave gaps at the cardinals and read as an outline.
pub const NEON_GLOW_UNIT: [(f32, f32); 8] = [
    (1.0, 0.0),
    (1.0, 1.0),
    (0.0, 1.0),
    (-1.0, 1.0),
    (-1.0, 0.0),
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
];

/// The full contract for one neon text run: wording, font, bright core
/// color, halo color, halo spread and falloff, and layout. `marker` is
/// cloned onto every spawned copy (core + each halo layer), so sync
/// systems keep them identical; use `()` for static text.
pub struct NeonText<'a, M: Bundle + Clone> {
    pub marker: M,
    pub value: &'a str,
    pub font: Handle<Font>,
    pub font_size: f32,
    pub core: Color,
    pub glow: Color,
    /// Inner halo ring radius in px (8 copies). Keep small (1.0): the
    /// ring should read as luminous glow, not as an outline.
    pub glow_px: f32,
    /// Alpha multiplier for the inner ring (0..1).
    pub glow_inner_alpha: f32,
    /// Alpha multiplier for the outer ring at 2x `glow_px` (0..1).
    /// Must not exceed the inner alpha: the falloff is what sells
    /// "glow" instead of "outline".
    pub glow_outer_alpha: f32,
    pub width: Val,
    pub justify: Justify,
}

/// Spawns a neon text run inside `parent`: a relatively-positioned
/// wrapper holding the bright core in normal flow, plus sixteen glow
/// copies (eight per ring) absolutely positioned behind it. `spec.width`
/// constrains the text node (`Val::Auto` for none); `spec.justify` must
/// match across copies or wrapped text will not line up.
pub fn spawn_neon_text<M: Bundle + Clone>(parent: &mut ChildSpawnerCommands, spec: NeonText<'_, M>) {
    debug_assert!(
        spec.glow_outer_alpha <= spec.glow_inner_alpha,
        "neon glow must fall off: outer alpha ({}), inner alpha ({})",
        spec.glow_outer_alpha,
        spec.glow_inner_alpha,
    );
    // Parley (Bevy 0.19's text engine) renders the same point size
    // larger than the 0.14 ab_glyph stack did; the 0.15 migration guide
    // prescribes dividing by 1.2 for identical rendering. Flagged for
    // Matt's visual review (see FONT_SIZE_ADJUST).
    let font_size = FontSize::Px(spec.font_size * FONT_SIZE_ADJUST);
    let style_for = |color: Color| {
        (
            TextFont {
                font: spec.font.clone().into(),
                font_size,
                ..default()
            },
            TextColor(color),
        )
    };
    parent
        .spawn(Node {
            position_type: PositionType::Relative,
            ..default()
        })
        .with_children(|run| {
            for (ring_scale, alpha) in [(1.0, spec.glow_inner_alpha), (2.0, spec.glow_outer_alpha)] {
                let halo = with_scaled_alpha(spec.glow, alpha);
                let offset = spec.glow_px * ring_scale;
                let (halo_font, halo_color) = style_for(halo);
                for (ux, uy) in NEON_GLOW_UNIT {
                    run.spawn((
                        spec.marker.clone(),
                        Text::new(spec.value),
                        halo_font.clone(),
                        halo_color,
                        TextLayout { justify: spec.justify, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(ux * offset),
                            top: Val::Px(uy * offset),
                            width: spec.width,
                            ..default()
                        },
                    ));
                }
            }
            let (core_font, core_color) = style_for(spec.core);
            run.spawn((
                spec.marker,
                Text::new(spec.value),
                core_font,
                core_color,
                TextLayout { justify: spec.justify, ..default() },
                Node {
                    width: spec.width,
                    ..default()
                },
            ));
        });
}

/// Returns `color` with its alpha multiplied by `factor` (clamped).
/// The caller's glow color may already carry alpha (e.g. a shadow tone),
/// so this multiplies rather than replaces.
fn with_scaled_alpha(color: Color, factor: f32) -> Color {
    let mut srgba = Srgba::from(color);
    srgba.alpha = (srgba.alpha * factor).clamp(0.0, 1.0);
    Color::Srgba(srgba)
}
