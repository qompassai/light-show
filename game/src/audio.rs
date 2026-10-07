//! Licensed background music: one track per game context, chosen by a small
//! pure music manager (`menu_track`, `playing_track`, `outage_track`,
//! `results_track`) and played through Bevy's `AudioPlayer` component. Track files
//! live under `game/assets/music/<tier>/`; the CC-BY attribution in
//! `game/assets/music/CREDITS.md` ships with the game and is REQUIRED for
//! the CC-BY tracks — do not ship a build without it. The same attribution
//! is user-visible on the in-game credits screen (`states::credits`),
//! which embeds `game/assets/music/ATTRIBUTION.txt` at compile time.
//!
//! Each state spawns its own track `OnEnter` and despawns it `OnExit`,
//! mirroring the existing per-state UI teardown convention (see
//! `board::BoardRoot`, `states::outage::OutageBanner`,
//! `states::results::ResultsRoot`). Because Bevy's state machine always
//! runs the old state's `OnExit` systems before the new state's
//! `OnEnter` systems for any transition, there is never a frame where
//! two tracks are both alive.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::level::{load_level, CurrentLevelIndex};
use crate::save::{SaveData, Settings};
use crate::states::{GameState, LevelOutcome};
use crate::waifu::{Companion, SelectedCompanion};
use bevy::audio::GlobalVolume;
use bevy::prelude::*;

pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, apply_volume_settings)
            .add_systems(Update, sync_volume_settings_on_change)
            .add_systems(OnEnter(GameState::MainMenu), play_menu_track)
            .add_systems(OnExit(GameState::MainMenu), stop_music)
            .add_systems(OnEnter(GameState::Playing), play_level_track)
            .add_systems(OnExit(GameState::Playing), stop_music)
            .add_systems(OnEnter(GameState::OutageActive), play_outage_track)
            .add_systems(OnExit(GameState::OutageActive), stop_music)
            .add_systems(OnEnter(GameState::Results), play_results_track)
            .add_systems(OnExit(GameState::Results), stop_music)
            // The credits screen keeps the menu track playing across the
            // MainMenu -> Credits hop: OnExit(MainMenu) stops it, this
            // OnEnter restarts it, OnExit(Credits) stops it on the way
            // back — the same spawn-on-enter/despawn-on-exit contract as
            // every other state above.
            .add_systems(OnEnter(GameState::Credits), play_menu_track)
            .add_systems(OnExit(GameState::Credits), stop_music)
            // Companion select plays the selected companion's theme; the
            // OnExit stops it via the shared MusicTrack marker, same as
            // every other state above.
            .add_systems(OnEnter(GameState::CompanionSelect), play_companion_theme)
            .add_systems(OnExit(GameState::CompanionSelect), stop_music)
            // In-level ambience hums under the tier music; it stops on
            // Playing exit (including the hop to OutageActive, where the
            // boss track takes over) and restarts when play resumes.
            .add_systems(OnEnter(GameState::Playing), play_ambience)
            .add_systems(OnExit(GameState::Playing), stop_ambience);
    }
}

// ---------------------------------------------------------------------------
// Persisted volume settings: the `Settings` in the save file drive the
// live audio output. `master_volume` maps to Bevy's `GlobalVolume`; the
// music and SFX sliders scale their bus constants at spawn time. All
// defaults are 1.0, so an untouched save reproduces the original fixed
// mix exactly. Before this wiring existed the sliders were stored and
// validated but never read — persisted volume was silently inert.
// ---------------------------------------------------------------------------

/// Effective music-bus level for persisted `settings`: music tracks
/// spawn at full scale, so the saved music slider is the bus level.
pub fn music_bus_volume(settings: &Settings) -> f32 {
    settings.music_volume
}

/// Effective ambience level: the fixed mix constant scaled by the saved
/// music slider (the hum layers under the music, so it follows its bus).
pub fn ambience_bus_volume(settings: &Settings) -> f32 {
    AMBIENCE_VOLUME * settings.music_volume
}

/// Effective SFX bus level for persisted `settings`: the fixed mix
/// constant scaled by the saved SFX slider.
pub fn sfx_bus_volume(settings: &Settings) -> f32 {
    SFX_VOLUME * settings.sfx_volume
}

/// Music-bus level for an optional save: no save resource means the
/// default settings, i.e. full scale.
fn music_volume_or_default(save: Option<&SaveData>) -> f32 {
    save.map_or(1.0, |s| music_bus_volume(&s.settings))
}

/// Ambience level for an optional save (see `music_volume_or_default`).
fn ambience_volume_or_default(save: Option<&SaveData>) -> f32 {
    save.map_or(AMBIENCE_VOLUME, |s| ambience_bus_volume(&s.settings))
}

/// Push the persisted volume settings into the live audio output: the
/// saved master volume becomes Bevy's `GlobalVolume` and the saved SFX
/// slider is cached on the `Sfx` resource, whose spawns read it.
///
/// Bevy 0.19 applies `GlobalVolume` when a sink starts, not
/// retroactively, so a change reaches already-playing music at its
/// next spawn — in this game every state transition respawns its
/// track, so that is never far away. SFX one-shots pick changes up on
/// their next play.
///
/// Registered at `Startup` by `MusicPlugin` and re-run on save changes
/// via `sync_volume_settings_on_change`, so the wiring holds if a
/// settings screen ever mutates the save at runtime. Every parameter
/// is optional: partial plugin sets and headless tests skip the pieces
/// they don't have instead of panicking.
fn apply_volume_settings(
    save: Option<Res<SaveData>>,
    mut global: Option<ResMut<GlobalVolume>>,
    mut sfx: Option<ResMut<Sfx>>,
) {
    let Some(save) = save else {
        return;
    };
    if let Some(global) = &mut global {
        global.volume = bevy::audio::Volume::Linear(save.settings.master_volume);
    }
    if let Some(sfx) = &mut sfx {
        sfx.volume_scale = save.settings.sfx_volume;
    }
}

/// `Update` companion to `apply_volume_settings`: reapplying is
/// idempotent but pointless every frame, so the body only runs when
/// the save resource actually changed.
fn sync_volume_settings_on_change(
    save: Option<Res<SaveData>>,
    global: Option<ResMut<GlobalVolume>>,
    sfx: Option<ResMut<Sfx>>,
) {
    if save.as_ref().is_some_and(|s| s.is_changed()) {
        apply_volume_settings(save, global, sfx);
    }
}

/// The same directory `AssetPlugin` loads from (see `lib::build_app`), so
/// the decode guard below validates exactly the file bevy_audio will play.
#[derive(Resource, Debug, Clone)]
pub struct AssetRootDir(pub PathBuf);

/// Marks the currently-playing music track's entity so `stop_music` can
/// find and despawn it on the way out of any state.
#[derive(Component)]
struct MusicTrack;

// ---------------------------------------------------------------------------
// Music manager: pure track selection. No Bevy types here so the mapping is
// unit-testable without an `App`. Every returned path is relative to
// `game/assets/`.
// ---------------------------------------------------------------------------

/// Title screen: Eric Skiff, "We're all under the stars" (CC-BY).
pub fn menu_track() -> &'static str {
    "music/menu/eric-skiff-16-were-all-under-the-stars.mp3"
}

/// In-level music, tiered by the level's world. Worlds with two tracks
/// rotate deterministically by level index so adjacent levels in the same
/// world don't repeat the same song back-to-back.
pub fn playing_track(level_index: usize, world: u32) -> &'static str {
    /// World 1 is Splice School (tutorial): Komiku, CC0.
    const TUTORIAL: &[&str] = &["music/tutorial/komiku-intensive-puzzle-resolution.mp3"];
    /// World 2: Eric Skiff, CC-BY.
    const EARLY: &[&str] = &[
        "music/early/eric-skiff-06-searching.mp3",
        "music/early/eric-skiff-08-ascending.mp3",
    ];
    /// World 3: Eric Skiff, CC-BY.
    const MID: &[&str] = &[
        "music/mid/eric-skiff-03-chibi-ninja.mp3",
        "music/mid/eric-skiff-02-underclocked.mp3",
    ];
    /// World 4+: Kevin MacLeod (CC-BY) / SubspaceAudio (CC0).
    const HARD: &[&str] = &[
        "music/hard/kevin-macleod-exhilarate.mp3",
        "music/hard/subspaceaudio-action2-army-approaching.ogg",
    ];

    let tier: &[&str] = match world {
        0 | 1 => TUTORIAL,
        2 => EARLY,
        3 => MID,
        _ => HARD,
    };
    // `tier` is never empty by construction above, so the remainder is a
    // valid index; no division by zero possible.
    tier[level_index % tier.len()]
}

/// Outage repair is the game's boss fight: TeknoAXE (CC-BY) alternating
/// with SubspaceAudio (CC0) each time an outage fires, so repeat outages
/// in one session don't loop the same track. `outage_count` is the
/// 0-based number of outages seen so far this run.
pub fn outage_track(outage_count: u32) -> &'static str {
    const BOSS: &[&str] = &[
        "music/boss/teknoaxe-basic-metal-5.mp3",
        "music/boss/subspaceaudio-action3-preparing-for-battle.ogg",
    ];
    BOSS[(outage_count % BOSS.len() as u32) as usize]
}

/// Results screen: Eric Skiff "We're the Resistors" (CC-BY) on a win,
/// Kevin MacLeod "In a Heartbeat" (CC-BY) on a loss.
pub fn results_track(won: bool) -> &'static str {
    if won {
        "music/victory/eric-skiff-07-were-the-resistors.mp3"
    } else {
        "music/mystery/kevin-macleod-in-a-heartbeat.mp3"
    }
}

// ---------------------------------------------------------------------------
// Validates that an audio file can be decoded by rodio (the same decoder
// bevy_audio uses). This is a curative guard against bevy_audio 0.19.1's
// `Decodable::decoder()` which calls `.unwrap()` on the decode result —
// a corrupt or undecodable file would panic a Compute Task Pool thread.
// By validating here on the main thread, we log a clear error and skip
// the track instead of crashing audio.
//
// `path` is relative to `asset_root`, the AssetServer's own root, so the
// guard reads the installed file rather than a compile-time source path.
fn audio_file_decodable(asset_root: &Path, path: &str) -> bool {
    let full_path = asset_root.join(path);
    let bytes = match std::fs::read(&full_path) {
        Ok(b) => b,
        Err(e) => {
            bevy::log::error!("Music track not found: {} ({})", full_path.display(), e);
            return false;
        }
    };
    if bytes.is_empty() {
        bevy::log::error!("Music track is empty: {}", full_path.display());
        return false;
    }
    // Mirror bevy_audio 0.19.1's Decodable::decoder() exactly.
    match rodio::Decoder::builder()
        .with_byte_len(bytes.len() as u64)
        .with_data(Cursor::new(bytes))
        .build()
    {
        Ok(_) => true,
        Err(e) => {
            bevy::log::error!(
                "Music track failed to decode: {} ({:?})",
                full_path.display(),
                e
            );
            false
        }
    }
}

// Bevy systems: spawn the selected track on state enter, despawn on exit.
// ---------------------------------------------------------------------------

fn spawn_track(
    commands: &mut Commands,
    asset_server: &AssetServer,
    asset_root: &AssetRootDir,
    path: &str,
    looping: bool,
    volume: f32,
) {
    // Curative guard: validate decodability on the main thread before
    // handing the asset to bevy_audio. If the file is corrupt, we log
    // and skip instead of panicking a worker thread in `decoder().unwrap()`.
    if !audio_file_decodable(&asset_root.0, path) {
        return;
    }
    commands.spawn((
        MusicTrack,
        AudioPlayer::<AudioSource>(asset_server.load(path.to_string())),
        PlaybackSettings {
            mode: if looping {
                bevy::audio::PlaybackMode::Loop
            } else {
                bevy::audio::PlaybackMode::Despawn
            },
            volume: bevy::audio::Volume::Linear(volume),
            ..default()
        },
    ));
}

fn play_menu_track(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_root: Res<AssetRootDir>,
    save: Option<Res<SaveData>>,
) {
    spawn_track(
        &mut commands,
        &asset_server,
        &asset_root,
        menu_track(),
        true,
        music_volume_or_default(save.as_deref()),
    );
}

fn play_level_track(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_root: Res<AssetRootDir>,
    level_index: Res<CurrentLevelIndex>,
    save: Option<Res<SaveData>>,
) {
    // `load_level` parses the compile-time-embedded level JSON (the same
    // call `playing::setup_level` makes on this same state enter); the
    // world number is what tiers the music.
    let world = load_level(level_index.0).world;
    spawn_track(
        &mut commands,
        &asset_server,
        &asset_root,
        playing_track(level_index.0, world),
        true,
        music_volume_or_default(save.as_deref()),
    );
}

fn play_outage_track(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_root: Res<AssetRootDir>,
    mut outage_count: Local<u32>,
    save: Option<Res<SaveData>>,
) {
    spawn_track(
        &mut commands,
        &asset_server,
        &asset_root,
        outage_track(*outage_count),
        true,
        music_volume_or_default(save.as_deref()),
    );
    *outage_count += 1;
}

/// Picks the win/loss track based on the same `LevelOutcome` resource
/// `results::show_results` reads to pick its banner and dialogue key.
fn play_results_track(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_root: Res<AssetRootDir>,
    outcome: Res<LevelOutcome>,
    save: Option<Res<SaveData>>,
) {
    spawn_track(
        &mut commands,
        &asset_server,
        &asset_root,
        results_track(outcome.won),
        false,
        music_volume_or_default(save.as_deref()),
    );
}

fn stop_music(mut commands: Commands, tracks: Query<Entity, With<MusicTrack>>) {
    for entity in &tracks {
        commands.entity(entity).despawn();
    }
}

// ---------------------------------------------------------------------------
// Per-companion audio: theme loops for the companion-select screen and
// ambient hums layered under in-level music. All files are generated
// from scratch by `tools/gen_audio_pack.py` (chiptune themes via the
// NES-2A03 engine in `tools/gen_chiptune_music.py`) — no licensed
// samples, so no attribution burden.
// ---------------------------------------------------------------------------

/// Marks the currently-playing ambience hum so `stop_ambience` can find
/// and despawn it. Separate from `MusicTrack` so the hum layers under
/// the tier music instead of replacing it.
#[derive(Component)]
struct AmbienceTrack;

/// Ambience bus level: the hum sits well under the music, felt more
/// than heard.
pub const AMBIENCE_VOLUME: f32 = 0.30;

/// Per-companion theme loop for the companion-select screen, relative
/// to `game/assets/`. Pure and unit-tested like the music manager.
pub fn companion_theme(companion: Companion) -> &'static str {
    match companion {
        Companion::Fiber => "music/themes/seraphine.ogg",
        Companion::Coax => "music/themes/ondine.ogg",
        Companion::Mobile => "music/themes/linka.ogg",
        Companion::Ethernet => "music/themes/lattice.ogg",
        Companion::Clara => "music/themes/clara.ogg",
        Companion::Aino => "music/themes/aino.ogg",
        Companion::Hikari => "music/themes/hikari.ogg",
        Companion::Lea => "music/themes/lea.ogg",
    }
}

/// Per-companion ambient hum looped quietly under in-level music,
/// relative to `game/assets/`. Pure and unit-tested.
pub fn companion_ambience(companion: Companion) -> &'static str {
    match companion {
        Companion::Fiber => "music/ambience/seraphine_hum.ogg",
        Companion::Coax => "music/ambience/ondine_hum.ogg",
        Companion::Mobile => "music/ambience/linka_hum.ogg",
        Companion::Ethernet => "music/ambience/lattice_hum.ogg",
        Companion::Clara => "music/ambience/clara_hum.ogg",
        Companion::Aino => "music/ambience/aino_hum.ogg",
        Companion::Hikari => "music/ambience/hikari_hum.ogg",
        Companion::Lea => "music/ambience/lea_hum.ogg",
    }
}

/// Dialogue blip playback speed per companion — the Mega Man Battle
/// Network trick of pitch-shifting one blip sample per character
/// instead of shipping eight near-identical files. 1.0 is the raw
/// `dialogue.wav`.
pub fn dialogue_blip_speed(companion: Companion) -> f32 {
    match companion {
        Companion::Fiber => 1.00,
        Companion::Coax => 1.12,
        Companion::Mobile => 0.90,
        Companion::Ethernet => 1.06,
        Companion::Clara => 1.18,
        Companion::Aino => 0.85,
        Companion::Hikari => 1.25,
        Companion::Lea => 1.10,
    }
}

fn play_companion_theme(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_root: Res<AssetRootDir>,
    selected: Res<SelectedCompanion>,
    save: Option<Res<SaveData>>,
) {
    spawn_track(
        &mut commands,
        &asset_server,
        &asset_root,
        companion_theme(selected.0),
        true,
        music_volume_or_default(save.as_deref()),
    );
}

fn play_ambience(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_root: Res<AssetRootDir>,
    selected: Res<SelectedCompanion>,
    save: Option<Res<SaveData>>,
) {
    let path = companion_ambience(selected.0);
    // Same curative guard as `spawn_track`: validate on the main thread
    // before handing the asset to bevy_audio.
    if !audio_file_decodable(&asset_root.0, path) {
        return;
    }
    commands.spawn((
        AmbienceTrack,
        AudioPlayer::<AudioSource>(asset_server.load(path.to_string())),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Loop,
            volume: bevy::audio::Volume::Linear(ambience_volume_or_default(save.as_deref())),
            ..default()
        },
    ));
}

fn stop_ambience(mut commands: Commands, tracks: Query<Entity, With<AmbienceTrack>>) {
    for entity in &tracks {
        commands.entity(entity).despawn();
    }
}

// ---------------------------------------------------------------------------
// SFX: one-shot chiptune sound effects for game events. The seventeen WAVs
// under `game/assets/sfx/` are synthesized from scratch (numpy envelopes
// over square/sine oscillators, see `tools/gen_audio_pack.py`) — no
// licensed samples, so no attribution burden unlike the music tracks.
//
// Contract: `SfxPlugin` preloads every handle once via `FromWorld` (the
// same pattern as `waifu::CompanionAtlasLayout`), so playback sites pay
// no load cost and a missing file fails fast at startup instead of
// surfacing as silence mid-game. `Sfx::play` spawns a `Despawn`-mode
// `AudioPlayer` + `PlaybackMode::Despawn`: the entity cleans itself up when the sample ends, no
// bookkeeping, safe to call from any event system.
// ---------------------------------------------------------------------------

/// SFX bus level relative to full scale. Music tracks play at 1.0; the
/// chiptune blips sit underneath at less than half so rapid-fire UI
/// clicks never drown the track.
pub const SFX_VOLUME: f32 = 0.45;

/// One-shot sound-effect kinds. Each maps to a file under
/// `game/assets/sfx/` via `sfx_path` (pure, unit-tested like the music
/// manager above).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxKind {
    /// Generic UI button press.
    Click,
    /// Component pill picked up from the tray.
    Pick,
    /// Splice snapped into place on the board.
    Place,
    /// Outage klaxon when a hazard fires.
    Alarm,
    /// Repair countdown tick (final 5 seconds, one per whole second).
    Tick,
    /// Level-clear sting.
    Win,
    /// Level-fail sting.
    Lose,
    /// Companion mood/dialogue reaction blip.
    Dialogue,
    /// Track-complete fanfare (bigger than the per-level Win sting).
    Fanfare,
    /// Button hover tick.
    Hover,
    /// Invalid-action buzz.
    Error,
    /// Link-connection zap sweep.
    Zap,
    /// Escalated klaxon (faster than Alarm).
    AlarmUrgent,
    /// Gentle non-critical warning chime.
    AlarmSoft,
    /// Panel-open whoosh.
    MenuOpen,
    /// Panel-close whoosh.
    MenuClose,
    /// Tab-switch click-slide.
    TabSwitch,
}

/// Maps a kind to its asset path, relative to `game/assets/`. Pure so
/// the on-disk existence test below covers every kind without an `App`.
pub fn sfx_path(kind: SfxKind) -> &'static str {
    match kind {
        SfxKind::Click => "sfx/click.wav",
        SfxKind::Pick => "sfx/pick.wav",
        SfxKind::Place => "sfx/place.wav",
        SfxKind::Alarm => "sfx/alarm.wav",
        SfxKind::Tick => "sfx/tick.wav",
        SfxKind::Win => "sfx/win.wav",
        SfxKind::Lose => "sfx/lose.wav",
        SfxKind::Dialogue => "sfx/dialogue.wav",
        SfxKind::Fanfare => "sfx/fanfare.wav",
        SfxKind::Hover => "sfx/hover.wav",
        SfxKind::Error => "sfx/error.wav",
        SfxKind::Zap => "sfx/zap.wav",
        SfxKind::AlarmUrgent => "sfx/alarm_urgent.wav",
        SfxKind::AlarmSoft => "sfx/alarm_soft.wav",
        SfxKind::MenuOpen => "sfx/menu_open.wav",
        SfxKind::MenuClose => "sfx/menu_close.wav",
        SfxKind::TabSwitch => "sfx/tab_switch.wav",
    }
}

/// All seventeen SFX kinds in one place so the on-disk test can't drift
/// out of sync with the enum when a kind is added. Test-only: the
/// loader above names each handle explicitly.
#[cfg(test)]
const ALL_SFX_KINDS: [SfxKind; 17] = [
    SfxKind::Click,
    SfxKind::Pick,
    SfxKind::Place,
    SfxKind::Alarm,
    SfxKind::Tick,
    SfxKind::Win,
    SfxKind::Lose,
    SfxKind::Dialogue,
    SfxKind::Fanfare,
    SfxKind::Hover,
    SfxKind::Error,
    SfxKind::Zap,
    SfxKind::AlarmUrgent,
    SfxKind::AlarmSoft,
    SfxKind::MenuOpen,
    SfxKind::MenuClose,
    SfxKind::TabSwitch,
];

/// Preloaded one-shot SFX handles. See the section header for the
/// lifecycle contract.
#[derive(Resource)]
pub struct Sfx {
    click: Handle<AudioSource>,
    pick: Handle<AudioSource>,
    place: Handle<AudioSource>,
    alarm: Handle<AudioSource>,
    tick: Handle<AudioSource>,
    win: Handle<AudioSource>,
    lose: Handle<AudioSource>,
    dialogue: Handle<AudioSource>,
    fanfare: Handle<AudioSource>,
    hover: Handle<AudioSource>,
    error: Handle<AudioSource>,
    zap: Handle<AudioSource>,
    alarm_urgent: Handle<AudioSource>,
    alarm_soft: Handle<AudioSource>,
    menu_open: Handle<AudioSource>,
    menu_close: Handle<AudioSource>,
    tab_switch: Handle<AudioSource>,
    /// Saved SFX slider (0.0..=1.0), seeded from the save file at
    /// startup by `apply_volume_settings` and refreshed whenever the
    /// save changes; scales [`SFX_VOLUME`] on every spawn. The 1.0
    /// default is the unscaled original mix.
    volume_scale: f32,
}

impl FromWorld for Sfx {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        Self {
            click: server.load(sfx_path(SfxKind::Click)),
            pick: server.load(sfx_path(SfxKind::Pick)),
            place: server.load(sfx_path(SfxKind::Place)),
            alarm: server.load(sfx_path(SfxKind::Alarm)),
            tick: server.load(sfx_path(SfxKind::Tick)),
            win: server.load(sfx_path(SfxKind::Win)),
            lose: server.load(sfx_path(SfxKind::Lose)),
            dialogue: server.load(sfx_path(SfxKind::Dialogue)),
            fanfare: server.load(sfx_path(SfxKind::Fanfare)),
            hover: server.load(sfx_path(SfxKind::Hover)),
            error: server.load(sfx_path(SfxKind::Error)),
            zap: server.load(sfx_path(SfxKind::Zap)),
            alarm_urgent: server.load(sfx_path(SfxKind::AlarmUrgent)),
            alarm_soft: server.load(sfx_path(SfxKind::AlarmSoft)),
            menu_open: server.load(sfx_path(SfxKind::MenuOpen)),
            menu_close: server.load(sfx_path(SfxKind::MenuClose)),
            tab_switch: server.load(sfx_path(SfxKind::TabSwitch)),
            volume_scale: 1.0,
        }
    }
}

impl Sfx {
    fn handle(&self, kind: SfxKind) -> &Handle<AudioSource> {
        match kind {
            SfxKind::Click => &self.click,
            SfxKind::Pick => &self.pick,
            SfxKind::Place => &self.place,
            SfxKind::Alarm => &self.alarm,
            SfxKind::Tick => &self.tick,
            SfxKind::Win => &self.win,
            SfxKind::Lose => &self.lose,
            SfxKind::Dialogue => &self.dialogue,
            SfxKind::Fanfare => &self.fanfare,
            SfxKind::Hover => &self.hover,
            SfxKind::Error => &self.error,
            SfxKind::Zap => &self.zap,
            SfxKind::AlarmUrgent => &self.alarm_urgent,
            SfxKind::AlarmSoft => &self.alarm_soft,
            SfxKind::MenuOpen => &self.menu_open,
            SfxKind::MenuClose => &self.menu_close,
            SfxKind::TabSwitch => &self.tab_switch,
        }
    }

    /// The level one-shots spawn at: the fixed mix constant scaled by
    /// the saved SFX slider cached on this resource.
    fn effective_volume(&self) -> f32 {
        SFX_VOLUME * self.volume_scale
    }

    /// Spawn a one-shot playback of `kind` at the SFX bus level
    /// ([`SFX_VOLUME`] scaled by the saved SFX slider).
    /// `PlaybackMode::Despawn` removes the entity when the sample ends.
    pub fn play(&self, commands: &mut Commands, kind: SfxKind) {
        self.play_with_speed(commands, kind, 1.0);
    }

    /// Spawn a one-shot playback of `kind` at the SFX bus level with a
    /// playback-speed multiplier (pitch-shifts the sample; the
    /// per-companion dialogue blip uses this instead of shipping eight
    /// near-identical files).
    pub fn play_with_speed(&self, commands: &mut Commands, kind: SfxKind, speed: f32) {
        commands.spawn((
            AudioPlayer(self.handle(kind).clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: bevy::audio::Volume::Linear(self.effective_volume()),
                speed,
                ..default()
            },
        ));
    }

    /// Test-world constructor with invalid handles. `play` still spawns
    /// the `AudioPlayer`, but no audio system runs under
    /// `run_system_once`, so the handles are never resolved.
    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self {
            click: Handle::default(),
            pick: Handle::default(),
            place: Handle::default(),
            alarm: Handle::default(),
            tick: Handle::default(),
            win: Handle::default(),
            lose: Handle::default(),
            dialogue: Handle::default(),
            fanfare: Handle::default(),
            hover: Handle::default(),
            error: Handle::default(),
            zap: Handle::default(),
            alarm_urgent: Handle::default(),
            alarm_soft: Handle::default(),
            menu_open: Handle::default(),
            menu_close: Handle::default(),
            tab_switch: Handle::default(),
            volume_scale: 1.0,
        }
    }
}

/// Installs the preloaded [`Sfx`] resource. Add alongside `MusicPlugin`;
/// every event system below assumes the resource exists.
pub struct SfxPlugin;

impl Plugin for SfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sfx>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every path the music manager can return, every tier and rotation slot.
    fn every_music_path() -> Vec<&'static str> {
        let mut paths = vec![menu_track(), results_track(true), results_track(false)];
        for world in 0..=5 {
            for level_index in 0..2 {
                paths.push(playing_track(level_index, world));
            }
        }
        for outage_count in 0..2 {
            paths.push(outage_track(outage_count));
        }
        paths
    }

    /// bevy_audio's `Decodable::decoder()` unwraps the decode result on a
    /// task-pool thread, so any shipped file the compiled-in decoders can't
    /// read panics the game the first time it plays. `audio_file_decodable`
    /// mirrors that decoder exactly (rodio's features unify across the
    /// graph), so this proves every SFX and track will decode in-game.
    /// Regression: the SFX are WAV and Bevy's `wav` feature was missing, so
    /// the first button click panicked with `IoError("end of stream")`.
    #[test]
    fn every_shipped_audio_file_decodes() {
        let asset_root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"));
        let sfx_paths = ALL_SFX_KINDS.map(sfx_path);
        for path in sfx_paths.iter().copied().chain(every_music_path()) {
            assert!(
                audio_file_decodable(asset_root, path),
                "{path} does not decode with the compiled-in rodio decoders"
            );
        }
    }

    /// Adversarial: the guard must reject, not panic on, bytes that no
    /// decoder accepts and on a path that does not exist.
    #[test]
    fn decode_guard_rejects_garbage_and_missing_files() {
        let src_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        assert!(!audio_file_decodable(src_dir, "audio.rs"));
        assert!(!audio_file_decodable(src_dir, "no-such-track.mp3"));
    }

    /// Every path the music manager can return must actually exist under
    /// `game/assets/` — a typo here would otherwise only surface as
    /// silence at manual playtest, never a build error
    /// (`AssetServer::load` takes a path, not a compile-time checked
    /// handle). Covers every tier including both rotation slots.
    #[test]
    fn every_referenced_track_exists_on_disk() {
        for path in every_music_path() {
            let on_disk = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/").to_string() + path;
            assert!(
                std::path::Path::new(&on_disk).is_file(),
                "{on_disk} referenced by audio::MusicPlugin but missing on disk"
            );
        }
    }

    #[test]
    fn world_1_plays_the_tutorial_track() {
        assert_eq!(
            playing_track(0, 1),
            "music/tutorial/komiku-intensive-puzzle-resolution.mp3"
        );
        // World 0 is treated as world 1 (defensive: no level should ever
        // have world 0, but if one does it still gets music, not a panic).
        assert_eq!(
            playing_track(0, 0),
            "music/tutorial/komiku-intensive-puzzle-resolution.mp3"
        );
    }

    #[test]
    fn tier_tracks_rotate_by_level_index() {
        // World 2 (early): two tracks, alternating by level index.
        assert_eq!(
            playing_track(0, 2),
            "music/early/eric-skiff-06-searching.mp3"
        );
        assert_eq!(
            playing_track(1, 2),
            "music/early/eric-skiff-08-ascending.mp3"
        );
        assert_eq!(
            playing_track(2, 2),
            "music/early/eric-skiff-06-searching.mp3"
        );
        // World 4+ (hard), including far-future worlds.
        assert_eq!(
            playing_track(0, 4),
            "music/hard/kevin-macleod-exhilarate.mp3"
        );
        assert_eq!(
            playing_track(1, 9),
            "music/hard/subspaceaudio-action2-army-approaching.ogg"
        );
    }

    #[test]
    fn outage_tracks_alternate_each_outage() {
        assert_eq!(outage_track(0), "music/boss/teknoaxe-basic-metal-5.mp3");
        assert_eq!(
            outage_track(1),
            "music/boss/subspaceaudio-action3-preparing-for-battle.ogg"
        );
        assert_eq!(outage_track(2), "music/boss/teknoaxe-basic-metal-5.mp3");
    }

    #[test]
    fn results_track_matches_the_level_outcome() {
        assert_eq!(
            results_track(true),
            "music/victory/eric-skiff-07-were-the-resistors.mp3"
        );
        assert_eq!(
            results_track(false),
            "music/mystery/kevin-macleod-in-a-heartbeat.mp3"
        );
    }

    /// Every SFX path `sfx_path` can return must exist under
    /// `game/assets/` — same rationale as the music track test: a typo
    /// would otherwise surface as silence at playtest, never a build
    /// error. Covers all eight kinds via `ALL_SFX_KINDS` so adding a
    /// kind without a file fails here.
    #[test]
    fn every_referenced_sfx_exists_on_disk() {
        assert_eq!(
            ALL_SFX_KINDS.len(),
            17,
            "SfxKind gained a variant; extend ALL_SFX_KINDS"
        );
        let mut seen = std::collections::HashSet::new();
        for kind in ALL_SFX_KINDS {
            let path = sfx_path(kind);
            assert!(seen.insert(path), "duplicate sfx path {path}");
            let on_disk = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/").to_string() + path;
            assert!(
                std::path::Path::new(&on_disk).is_file(),
                "{on_disk} referenced by audio::Sfx but missing on disk"
            );
        }
    }

    /// SFX must sit under the music, never over it: a blip at full scale
    /// layered over rapid UI clicks would drown the licensed tracks.
    #[test]
    fn sfx_volume_is_below_music_level() {
        assert!(SFX_VOLUME > 0.0, "SFX_VOLUME must be audible");
        assert!(SFX_VOLUME < 1.0, "SFX_VOLUME must sit below the music bus");
    }

    /// Every companion theme and ambience hum must exist on disk and
    /// decode with the compiled-in rodio decoders — same rationale as
    /// the SFX/track tests: a missing loop would otherwise surface as
    /// silence at playtest, never a build error.
    #[test]
    fn companion_audio_exists_and_decodes() {
        let asset_root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"));
        let companions = [
            Companion::Fiber,
            Companion::Coax,
            Companion::Mobile,
            Companion::Ethernet,
            Companion::Clara,
            Companion::Aino,
            Companion::Hikari,
            Companion::Lea,
        ];
        let mut seen = std::collections::HashSet::new();
        for companion in companions {
            for path in [companion_theme(companion), companion_ambience(companion)] {
                assert!(seen.insert(path), "duplicate companion audio path {path}");
                let on_disk = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/").to_string() + path;
                assert!(
                    std::path::Path::new(&on_disk).is_file(),
                    "{on_disk} referenced by audio but missing on disk"
                );
                assert!(
                    audio_file_decodable(asset_root, path),
                    "{path} does not decode with the compiled-in rodio decoders"
                );
            }
        }
    }

    /// Blip speeds must stay in a sane audible band: too slow sounds
    /// broken, too fast aliases into a click.
    #[test]
    fn dialogue_blip_speeds_are_sane() {
        let companions = [
            Companion::Fiber,
            Companion::Coax,
            Companion::Mobile,
            Companion::Ethernet,
            Companion::Clara,
            Companion::Aino,
            Companion::Hikari,
            Companion::Lea,
        ];
        for companion in companions {
            let speed = dialogue_blip_speed(companion);
            assert!(
                (0.7..=1.5).contains(&speed),
                "{companion:?} blip speed {speed} out of the audible band"
            );
        }
    }

    /// Ambience must sit under the music, never over it.
    #[test]
    fn ambience_volume_is_quiet() {
        assert!(AMBIENCE_VOLUME > 0.0, "AMBIENCE_VOLUME must be audible");
        assert!(
            AMBIENCE_VOLUME < SFX_VOLUME,
            "AMBIENCE_VOLUME must sit below the SFX bus"
        );
    }

    /// `Sfx::play` spawns exactly one audio entity per call (no leaks,
    /// no bookkeeping for callers to get wrong). `AudioPlayer` is the
    /// queryable component marking a spawned playback entity.
    #[test]
    fn play_spawns_one_audio_entity() {
        use bevy_ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(Sfx::for_tests());
        world.run_system_once(|mut commands: Commands, sfx: Res<Sfx>| {
            sfx.play(&mut commands, SfxKind::Click);
        });
        world.run_system_once(|query: Query<Entity, With<AudioPlayer<AudioSource>>>| {
            assert_eq!(query.iter().count(), 1);
        });
    }

    /// The persisted sliders scale their buses, and the default
    /// settings (all 1.0) reproduce the original fixed mix exactly —
    /// wiring the save through must not change an untouched profile.
    #[test]
    fn saved_settings_scale_the_audio_buses() {
        let defaults = Settings::default();
        assert_eq!(music_bus_volume(&defaults), 1.0);
        assert_eq!(ambience_bus_volume(&defaults), AMBIENCE_VOLUME);
        assert_eq!(sfx_bus_volume(&defaults), SFX_VOLUME);

        let settings = Settings {
            master_volume: 0.5,
            music_volume: 0.5,
            sfx_volume: 0.25,
        };
        assert_eq!(music_bus_volume(&settings), 0.5);
        assert!((ambience_bus_volume(&settings) - AMBIENCE_VOLUME * 0.5).abs() < f32::EPSILON);
        assert!((sfx_bus_volume(&settings) - SFX_VOLUME * 0.25).abs() < f32::EPSILON);
        // No save resource: the buses fall back to the default mix.
        assert_eq!(music_volume_or_default(None), 1.0);
        assert_eq!(ambience_volume_or_default(None), AMBIENCE_VOLUME);
    }

    /// Startup wiring: the saved master volume lands in `GlobalVolume`
    /// and the SFX slider on the `Sfx` resource, and a later save
    /// change re-applies both without a relaunch.
    #[test]
    fn saved_volumes_apply_at_startup_and_on_change() {
        fn linear_volume(app: &App) -> f32 {
            match app.world().resource::<GlobalVolume>().volume {
                bevy::audio::Volume::Linear(v) => v,
                other => panic!("expected a linear global volume, got {other:?}"),
            }
        }

        let mut app = App::new();
        let mut save = SaveData::default();
        save.settings.master_volume = 0.4;
        save.settings.sfx_volume = 0.5;
        app.insert_resource(save);
        app.insert_resource(GlobalVolume::default());
        app.insert_resource(Sfx::for_tests());
        app.add_systems(Startup, apply_volume_settings);
        app.add_systems(Update, sync_volume_settings_on_change);
        app.update();
        assert!((linear_volume(&app) - 0.4).abs() < f32::EPSILON);
        assert!((app.world().resource::<Sfx>().volume_scale - 0.5).abs() < f32::EPSILON);

        // Simulate a settings edit landing in the save resource: the
        // change-detected sync must follow it.
        {
            let mut save = app.world_mut().resource_mut::<SaveData>();
            save.settings.master_volume = 0.9;
            save.settings.sfx_volume = 1.0;
        }
        app.update();
        assert!((linear_volume(&app) - 0.9).abs() < f32::EPSILON);
        assert!((app.world().resource::<Sfx>().volume_scale - 1.0).abs() < f32::EPSILON);
    }

    /// Adversarial: with no save, no `GlobalVolume`, and no `Sfx`
    /// resource installed, the apply system degrades to a no-op
    /// instead of panicking on a missing resource.
    #[test]
    fn apply_volume_settings_tolerates_missing_resources() {
        use bevy_ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.run_system_once(apply_volume_settings).unwrap();
    }

    /// `Sfx::play` spawns at the fixed mix constant scaled by the
    /// saved SFX slider cached on the resource.
    #[test]
    fn play_uses_the_saved_sfx_scale() {
        use bevy_ecs::system::RunSystemOnce;
        let mut world = World::new();
        let mut sfx = Sfx::for_tests();
        sfx.volume_scale = 0.5;
        world.insert_resource(sfx);
        world
            .run_system_once(|mut commands: Commands, sfx: Res<Sfx>| {
                sfx.play(&mut commands, SfxKind::Click);
            })
            .unwrap();
        world
            .run_system_once(
                |query: Query<&PlaybackSettings, With<AudioPlayer<AudioSource>>>| {
                    let settings = query.single().expect("one playback spawned");
                    match settings.volume {
                        bevy::audio::Volume::Linear(v) => {
                            assert!((v - SFX_VOLUME * 0.5).abs() < f32::EPSILON)
                        }
                        other => panic!("expected a linear volume, got {other:?}"),
                    }
                },
            )
            .unwrap();
    }
}
