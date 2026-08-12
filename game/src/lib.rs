//! Light Show game library. Exposes `run()` for the desktop binary
//! (`src/main.rs`) and a `#[bevy_main] fn main()` for the Android
//! native-activity entry point when this crate is built as a `cdylib` and
//! loaded via Bevy's `game-activity` glue. Both paths converge on
//! `build_app()` so there is exactly one place that configures the App.

mod board;
mod level;
mod states;
mod ui;
mod waifu;

use bevy::prelude::*;
use states::GameState;

/// Structured event logging for the on-device instrumentation tests in
/// `android/app/src/androidTest`. Compiles away to nothing unless the
/// `instrumented-test-logging` Cargo feature is enabled — retail builds
/// (cargo-apk for F-Droid, the Gradle release build type for Play Store)
/// never enable it, so this has zero footprint outside test builds.
///
/// A `NativeActivity` has no Espresso-visible view hierarchy, so those
/// tests instead drive real touch gestures via UiAutomator and assert on
/// Logcat lines this macro emits (tag "LightShow", via `android_logger`
/// initialized below). See `board::handle_pointer_input`,
/// `states::playing::setup_level`, and `docs/BUILD.md`.
#[cfg(feature = "instrumented-test-logging")]
macro_rules! test_log {
    ($($arg:tt)*) => {
        log::info!($($arg)*)
    };
}
#[cfg(not(feature = "instrumented-test-logging"))]
macro_rules! test_log {
    ($($arg:tt)*) => {};
}
pub(crate) use test_log;

/// On Android, `game-activity` loads this library and calls a function
/// literally named `main` annotated with `#[bevy_main]` — the macro
/// asserts that exact name at compile time, so this cannot be renamed.
#[cfg(target_os = "android")]
#[bevy_main]
fn main() {
    #[cfg(feature = "instrumented-test-logging")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("LightShow")
            .with_max_level(log::LevelFilter::Info),
    );
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
            resolution: (720.0_f32, 1280.0_f32).into(),
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
