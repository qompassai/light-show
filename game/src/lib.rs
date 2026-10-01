//! Light Show game library. Exposes `run()` for the desktop binary
//! (`src/main.rs`) and a `#[bevy_main] fn main()` for the Android
//! native-activity entry point when this crate is built as a `cdylib` and
//! loaded via Bevy's `game-activity` glue. Both paths converge on
//! `build_app()` so there is exactly one place that configures the App.

mod audio;
#[cfg(debug_assertions)]
mod bench;
mod board;
mod level;
#[cfg(test)]
mod playthrough;
mod states;
mod ui;
mod waifu;

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::{Render, RenderApp};
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

/// Logs the wgpu backend and adapter the renderer actually initialized,
/// exactly once, on the first render frame. Reads `RenderAdapterInfo` from
/// the render sub-app's world (populated by `RenderPlugin::finish`, so it is
/// only present after the async adapter request completes); a missing
/// resource is logged as unknown rather than panicking. Uses `eprintln!`
/// because the desktop feature set does not include `bevy_log`.
fn log_render_backend_once(
    adapter_info: Option<Res<RenderAdapterInfo>>,
    mut already_logged: Local<bool>,
) {
    if *already_logged {
        return;
    }
    *already_logged = true;
    match adapter_info {
        Some(info) => {
            let wgpu_info = &info.0;
            eprintln!(
                "[light-show] render backend: {} | adapter: {} | vendor: {:#06x} device: {:#06x} | driver: {} ({})",
                wgpu_info.backend.to_str(),
                wgpu_info.name,
                wgpu_info.vendor,
                wgpu_info.device,
                wgpu_info.driver,
                wgpu_info.driver_info,
            );
        }
        None => eprintln!("[light-show] render backend: unknown (no adapter info)"),
    }
}

fn build_app() -> App {
    let mut app = App::new();
    #[cfg(debug_assertions)]
    let bench_args = bench::BenchArgs::from_args();
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Light Show".into(),
            resolution: (720.0_f32, 1280.0_f32).into(),
            ..default()
        }),
        ..default()
    });
    // `--bench-backend` swaps in a backend-forcing `RenderPlugin`; without
    // the flag the builder is returned untouched (byte-identical plugins).
    #[cfg(debug_assertions)]
    let plugins = bench::maybe_force_backend(plugins, &bench_args);
    app.add_plugins(plugins)
        .init_state::<GameState>()
        .add_plugins((
            states::menu::MenuPlugin,
            states::companion_select::CompanionSelectPlugin,
            states::credits::CreditsPlugin,
            states::playing::PlayingPlugin,
            states::outage::OutagePlugin,
            states::results::ResultsPlugin,
            waifu::SeraphinePlugin,
            ui::LedgerUiPlugin,
            audio::MusicPlugin,
        ));
    // Bench driver (synthetic input + frame timing + auto-exit). Not
    // installed for normal runs: zero overhead when the flag is absent.
    #[cfg(debug_assertions)]
    if let Some(frames) = bench_args.frames {
        bench::add_bench_systems(&mut app, frames);
    }
    // The render sub-app only exists once `RenderPlugin` has built; without
    // it there is no adapter to log. The system one-shots itself via a
    // `Local<bool>` because the `Render` schedule runs every frame.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(Render, log_render_backend_once);
    }
    app
}
