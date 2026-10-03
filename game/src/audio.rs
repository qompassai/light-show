//! Licensed background music: one track per game context, chosen by a small
//! pure music manager (`menu_track`, `playing_track`, `outage_track`,
//! `results_track`) and played through Bevy's `AudioBundle`. Track files
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

use crate::level::{load_level, CurrentLevelIndex};
use crate::states::{GameState, LevelOutcome};
use bevy::prelude::*;

pub struct MusicPlugin;

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), play_menu_track)
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
            .add_systems(OnExit(GameState::Credits), stop_music);
    }
}

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
// Bevy systems: spawn the selected track on state enter, despawn on exit.
// ---------------------------------------------------------------------------

fn spawn_track(commands: &mut Commands, asset_server: &AssetServer, path: &str, looping: bool) {
    commands.spawn((
        MusicTrack,
        AudioBundle {
            source: asset_server.load(path.to_string()),
            settings: PlaybackSettings {
                mode: if looping {
                    bevy::audio::PlaybackMode::Loop
                } else {
                    bevy::audio::PlaybackMode::Despawn
                },
                ..default()
            },
        },
    ));
}

fn play_menu_track(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_track(&mut commands, &asset_server, menu_track(), true);
}

fn play_level_track(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    level_index: Res<CurrentLevelIndex>,
) {
    // `load_level` parses the compile-time-embedded level JSON (the same
    // call `playing::setup_level` makes on this same state enter); the
    // world number is what tiers the music.
    let world = load_level(level_index.0).world;
    spawn_track(
        &mut commands,
        &asset_server,
        playing_track(level_index.0, world),
        true,
    );
}

fn play_outage_track(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut outage_count: Local<u32>,
) {
    spawn_track(
        &mut commands,
        &asset_server,
        outage_track(*outage_count),
        true,
    );
    *outage_count += 1;
}

/// Picks the win/loss track based on the same `LevelOutcome` resource
/// `results::show_results` reads to pick its banner and dialogue key.
fn play_results_track(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    outcome: Res<LevelOutcome>,
) {
    spawn_track(
        &mut commands,
        &asset_server,
        results_track(outcome.won),
        false,
    );
}

fn stop_music(mut commands: Commands, tracks: Query<Entity, With<MusicTrack>>) {
    for entity in &tracks {
        commands.entity(entity).despawn();
    }
}


// ---------------------------------------------------------------------------
// SFX: one-shot chiptune sound effects for game events. The eight WAVs
// under `game/assets/sfx/` are synthesized from scratch (square waves,
// see `tools/gen_sfx.py` provenance in the file header there) — no
// licensed samples, so no attribution burden unlike the music tracks.
//
// Contract: `SfxPlugin` preloads every handle once via `FromWorld` (the
// same pattern as `waifu::CompanionAtlasLayout`), so playback sites pay
// no load cost and a missing file fails fast at startup instead of
// surfacing as silence mid-game. `Sfx::play` spawns a `Despawn`-mode
// `AudioBundle`: the entity cleans itself up when the sample ends, no
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
    }
}

/// All eight SFX kinds in one place so the on-disk test can't drift
/// out of sync with the enum when a kind is added. Test-only: the
/// loader above names each handle explicitly.
#[cfg(test)]
const ALL_SFX_KINDS: [SfxKind; 8] = [
    SfxKind::Click,
    SfxKind::Pick,
    SfxKind::Place,
    SfxKind::Alarm,
    SfxKind::Tick,
    SfxKind::Win,
    SfxKind::Lose,
    SfxKind::Dialogue,
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
        }
    }

    /// Spawn a one-shot playback of `kind` at [`SFX_VOLUME`].
    /// `PlaybackMode::Despawn` removes the entity when the sample ends.
    pub fn play(&self, commands: &mut Commands, kind: SfxKind) {
        commands.spawn(AudioBundle {
            source: self.handle(kind).clone(),
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: bevy::audio::Volume::new(SFX_VOLUME),
                ..default()
            },
        });
    }

    /// Test-world constructor with invalid handles. `play` still spawns
    /// the `AudioBundle`, but no audio system runs under
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

    /// Every path the music manager can return must actually exist under
    /// `game/assets/` — a typo here would otherwise only surface as
    /// silence at manual playtest, never a build error
    /// (`AssetServer::load` takes a path, not a compile-time checked
    /// handle). Covers every tier including both rotation slots.
    #[test]
    fn every_referenced_track_exists_on_disk() {
        let mut paths = vec![menu_track(), results_track(true), results_track(false)];
        for world in 0..=5 {
            for level_index in 0..2 {
                paths.push(playing_track(level_index, world));
            }
        }
        for outage_count in 0..2 {
            paths.push(outage_track(outage_count));
        }
        for path in paths {
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
            8,
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

    /// `Sfx::play` spawns exactly one audio entity per call (no leaks,
    /// no bookkeeping for callers to get wrong). The source handle is the
    /// queryable component — `AudioBundle` itself is a bundle, not a
    /// component, so it can't appear in a `Query`.
    #[test]
    fn play_spawns_one_audio_entity() {
        use bevy_ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(Sfx::for_tests());
        world.run_system_once(|mut commands: Commands, sfx: Res<Sfx>| {
            sfx.play(&mut commands, SfxKind::Click);
        });
        world.run_system_once(|query: Query<Entity, With<Handle<AudioSource>>>| {
            assert_eq!(query.iter().count(), 1);
        });
    }
}

