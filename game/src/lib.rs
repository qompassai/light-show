//! Light Show game library. Exposes `run()` for the desktop binary
//! (`src/main.rs`) and a `#[bevy_main] fn main()` for the Android
//! native-activity entry point when this crate is built as a `cdylib` and
//! loaded via Bevy's `game-activity` glue. Both paths converge on
//! `build_app()` so there is exactly one place that configures the App.

mod level;
mod states;
mod ui;
mod waifu;

use bevy::prelude::*;
use states::GameState;

/// On Android, `game-activity` loads this library and calls a function
/// literally named `main` annotated with `#[bevy_main]` — the macro
/// asserts that exact name at compile time, so this cannot be renamed.
#[cfg(target_os = "android")]
#[bevy_main]
fn main() {
    build_app().run();
}

/// Desktop entry point, called from `src/main.rs`.
pub fn run() {
    build_app().run();
}

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Light Show".into(),
            resolution: (720.0, 1280.0).into(),
            ..default()
        }),
        ..default()
    }))
    .init_state::<GameState>()
    .add_plugins((
        states::menu::MenuPlugin,
        states::playing::PlayingPlugin,
        states::outage::OutagePlugin,
        states::results::ResultsPlugin,
        waifu::SeraphinePlugin,
        ui::LedgerUiPlugin,
    ));
    app
}
