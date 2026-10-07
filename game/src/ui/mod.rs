//! Ledger UI: the running dB budget readout styled like an OTDR trace.
//! Shows each placed component's contribution and the live received-power
//! number so players learn to read a loss budget the way a real OSP tech
//! reads an OTDR printout. Ethernet levels get the constraint checklist
//! instead (see `LevelDef::signal_ledger`).

use crate::level::LevelDef;
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::GameState;
use crate::waifu::Cores;
use bevy::prelude::*;

pub mod neon;

pub struct LedgerUiPlugin;

impl Plugin for LedgerUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_ledger_text
                .run_if(in_state(GameState::Playing).or_else(in_state(GameState::OutageActive))),
        );
    }
}

#[derive(Component)]
pub struct LedgerText;

fn update_ledger_text(
    live: Res<LiveGraph>,
    level: Res<LevelDef>,
    active_outage: Res<ActiveOutage>,
    cores: Res<Cores>,
    mut query: Query<&mut Text, With<LedgerText>>,
) {
    let outage_suffix = active_outage
        .outage
        .as_ref()
        .filter(|o| !o.resolved)
        .map(|o| format!("  |  OUTAGE: {:.0}s left", o.time_remaining()))
        .unwrap_or_default();

    let signal = level.signal_ledger(
        &live.graph,
        live.tx_dbm,
        live.wavelength.0,
        active_outage.outage.as_ref(),
    );
    for mut text in &mut query {
        text.0 = format!("{signal}{outage_suffix}  |  Cores: {}", cores.0);
    }
}

/// Shared button styling: every menu-style button carries a
/// [`ButtonPalette`] and gets its face and border colors from
/// [`apply_button_palette`] as its `Interaction` changes, so hover and
/// press read the same way on every screen instead of each screen
/// shipping flat, feedback-free rectangles (the playtest complaint).
pub struct ButtonStylePlugin;

impl Plugin for ButtonStylePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_button_palette);
    }
}

/// Border width shared by every styled button.
pub const BUTTON_BORDER: UiRect = UiRect::all(Val::Px(2.0));

/// Corner radius shared by every styled button.
pub const BUTTON_RADIUS_PX: f32 = 8.0;

/// The face colors of one styled button across its interaction states,
/// plus its border color. Derived from a single face color so each
/// screen keeps its identity (gold Start, slate Warehouse) while the
/// state feedback becomes uniform.
#[derive(Component, Clone, Copy)]
pub struct ButtonPalette {
    pub normal: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub border: Color,
    /// Last interaction state the palette wrote colors for. The system
    /// writes only on a state change, so other writers of
    /// `BackgroundColor` (the results entrance fade) are never fought
    /// frame to frame.
    applied: Interaction,
}

impl ButtonPalette {
    pub fn from_face(face: Color) -> Self {
        Self {
            normal: face,
            hovered: face.lighter(0.15),
            pressed: face.darker(0.2),
            border: face.lighter(0.3),
            applied: Interaction::None,
        }
    }

    /// The main menu's gold Start face.
    pub fn gold() -> Self {
        Self::from_face(neon::NEON_GOLD)
    }

    /// The menu's dark slate face (Warehouse / Credits).
    pub fn slate() -> Self {
        Self::from_face(Color::srgb(0.13, 0.15, 0.22))
    }

    /// The lighter slate used by Back buttons (credits, companion select).
    pub fn back() -> Self {
        Self::from_face(Color::srgb(0.2, 0.2, 0.28))
    }
}

/// Corner radius shared by every styled button, as a `Node` field
/// value (Bevy 0.19 carries radius on `Node`, not as a component).
pub const BUTTON_RADIUS: BorderRadius = BorderRadius::all(Val::Px(BUTTON_RADIUS_PX));

/// The components that style a `Button`: the palette itself and its
/// border color. Add [`BUTTON_BORDER`] and [`BUTTON_RADIUS`] to the
/// button's `Node` alongside.
pub fn styled_button(palette: ButtonPalette) -> (ButtonPalette, BorderColor) {
    (palette, BorderColor::all(palette.border))
}

fn apply_button_palette(
    mut query: Query<(
        &Interaction,
        &mut ButtonPalette,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (interaction, mut palette, mut bg, mut border) in &mut query {
        if *interaction == palette.applied {
            continue;
        }
        palette.applied = *interaction;
        bg.0 = match interaction {
            Interaction::Pressed => palette.pressed,
            Interaction::Hovered => palette.hovered,
            Interaction::None => palette.normal,
        };
        *border = BorderColor::all(palette.border);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel_sum(color: Color) -> f32 {
        let s = color.to_srgba();
        s.red + s.green + s.blue
    }

    #[test]
    fn palette_derives_distinct_states_from_one_face() {
        let face = Color::srgb(0.13, 0.15, 0.22);
        let palette = ButtonPalette::from_face(face);
        assert_eq!(palette.normal, face);
        assert!(channel_sum(palette.hovered) > channel_sum(palette.normal));
        assert!(channel_sum(palette.pressed) < channel_sum(palette.normal));
        assert!(channel_sum(palette.border) > channel_sum(palette.normal));
    }

    #[test]
    fn presets_keep_their_faces() {
        assert_eq!(ButtonPalette::gold().normal, neon::NEON_GOLD);
        assert_eq!(
            ButtonPalette::slate().normal,
            Color::srgb(0.13, 0.15, 0.22)
        );
        assert_eq!(ButtonPalette::back().normal, Color::srgb(0.2, 0.2, 0.28));
    }
}
