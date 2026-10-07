//! Light Show game library. Exposes `run()` for the desktop binary
//! (`src/main.rs`) and a `#[bevy_main] fn main()` for the Android
//! native-activity entry point when this crate is built as a `cdylib` and
//! loaded via Bevy's `game-activity` glue. Both paths converge on
//! `build_app()` so there is exactly one place that configures the App.

mod anim;
mod asset_root;
pub mod audio;
#[cfg(debug_assertions)]
mod bench;
mod board;
pub mod cheat_codes;
mod fonts;
mod salvage;
#[cfg(debug_assertions)]
mod footage;
mod fx;
pub mod level;
#[cfg(test)]
mod playthrough;
pub mod save;
pub mod shaders;
mod states;
mod ui;
pub mod waifu;
pub mod warehouse;

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
    // Android loads assets from the APK; this root only feeds the music
    // decode guard and is the same compile-time path it always used.
    build_app(concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into()).run();
}

/// Desktop entry point, called from `src/main.rs`. Exits with status 1 and
/// the list of searched directories when no assets directory exists, rather
/// than opening a window with no fonts, sprites, or music.
pub fn run() {
    let search_env = asset_root::AssetSearchEnv::from_process();
    let asset_root = match asset_root::resolve(&search_env, asset_root::probe_dir) {
        Ok(dir) => dir,
        Err(tried) => {
            eprintln!("[light-show] error: no assets directory found. Searched:");
            for path in &tried {
                eprintln!("[light-show]   {}", path.display());
            }
            eprintln!("[light-show] Install assets/ at one of these, or set BEVY_ASSET_ROOT.");
            std::process::exit(1);
        }
    };
    eprintln!("[light-show] assets: {}", asset_root.display());
    build_app(asset_root).run();
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

/// `asset_root` is the absolute directory assets load from; on desktop it
/// replaces Bevy's exe-relative default so an installed binary finds them.
fn build_app(asset_root: std::path::PathBuf) -> App {
    let mut app = App::new();
    #[cfg(debug_assertions)]
    let bench_args = bench::BenchArgs::from_args();
    #[cfg(debug_assertions)]
    let footage_args = footage::FootageArgs::from_args();
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Light Show".into(),
            resolution: (720, 1280).into(),
            ..default()
        }),
        ..default()
    });
    // An absolute `file_path` replaces Bevy's base path when joined.
    #[cfg(not(target_os = "android"))]
    let plugins = plugins.set(AssetPlugin {
        file_path: asset_root
            .to_str()
            .expect("asset_root::resolve only returns UTF-8 paths")
            .to_owned(),
        ..default()
    });
    app.insert_resource(audio::AssetRootDir(asset_root));
    // `--bench-backend` swaps in a backend-forcing `RenderPlugin`; without
    // the flag the builder is returned untouched (byte-identical plugins).
    #[cfg(debug_assertions)]
    let plugins = bench::maybe_force_backend(plugins, &bench_args);
    app.add_plugins(plugins)
        .init_state::<GameState>()
        .add_plugins(states::menu::MenuPlugin)
        .add_plugins(states::companion_select::CompanionSelectPlugin)
        .add_plugins(states::credits::CreditsPlugin)
        // Also owns the `warehouse::Loadout` resource that outage.rs and
        // results.rs read.
        .add_plugins(states::warehouse::WarehousePlugin)
        .add_plugins(states::playing::PlayingPlugin);
    // The console plugins are installed unconditionally, footage mode
    // included. Footage used to skip them for 28 Clara/Aino/Hikari levels
    // over Bevy B0001 query conflicts; 1eeb4b7 fixed those conflicts with
    // disjoint Text-query filters, and every skipped level is now pinned
    // loading under the full plugin set by
    // `playthrough::footage_skipped_levels_load_with_full_ui_plugins`
    // (one console family at a time in
    // `playthrough::production_plugin_set_no_b0001_on_level_load`). The
    // footage driver is unaffected: it writes `PlacedChoices` and quiz
    // state directly and never sets `Interaction`, which is all the
    // consoles' button handlers read.
    app.add_plugins(states::api_console::ApiConsolePlugin);
    app.add_plugins(states::quiz::QuizPlugin);
    app.add_plugins(states::triage_console::TriageConsolePlugin);
    app.add_plugins(states::outage::OutagePlugin)
        .add_plugins(states::results::ResultsPlugin)
        .add_plugins(save::SavePlugin)
        .add_plugins(shaders::ShaderPlugin)
        .add_plugins(waifu::SeraphinePlugin)
        .add_plugins(waifu::dialogue_ui::DialogueUiPlugin)
        .add_plugins(waifu::reactions::ReactionsPlugin)
        .add_plugins(fx::FxPlugin)
        .add_plugins(ui::LedgerUiPlugin)
        .add_plugins(ui::ButtonStylePlugin)
        .add_plugins(anim::AnimPlugin)
        .add_plugins(audio::MusicPlugin)
        .add_plugins(audio::SfxPlugin);
    // Bench driver (synthetic input + frame timing + auto-exit). Not
    // installed for normal runs: zero overhead when the flag is absent.
    #[cfg(debug_assertions)]
    if let Some(frames) = bench_args.frames {
        bench::add_bench_systems(&mut app, frames);
    }
    // Footage driver (level select + scripted taps + auto-exit). Not
    // installed for normal runs: zero overhead when the flag is absent.
    #[cfg(debug_assertions)]
    if let Some(ref level_id) = footage_args.level_id {
        footage::add_footage_systems(&mut app, level_id, footage_args.frames);
    }
    // The render sub-app only exists once `RenderPlugin` has built; without
    // it there is no adapter to log. The system one-shots itself via a
    // `Local<bool>` because the `Render` schedule runs every frame.
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(Render, log_render_backend_once);
    }
    app
}
