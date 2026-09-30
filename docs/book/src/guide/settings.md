# Settings

## In plain terms

The game has no settings screen. There is no volume slider, no
mute button, no graphics quality toggle, no difficulty selector. The
things most games let you configure are fixed here — this chapter lists
exactly what is fixed and where in the code it is fixed, so you know
what you're getting.

## There is no options menu in the source

A search of the `game` crate for settings/options handling turns up
nothing player-facing: no settings state, no options UI module, no
persisted configuration file. The five game states are `MainMenu`,
`Playing`, `OutageActive`, `Results`, and `Credits` — no `Settings`
among them. So everything below is a *fixed* behavior, not an option.

## Fixed behaviors (the "settings" that aren't)

### Window: 720 × 1280, portrait

The desktop window is created in `game/src/lib.rs::build_app` with a
hardcoded resolution of **720 × 1280** — a portrait, phone-shaped
window, since the game is designed Android-first (the in-game UI,
including the 720-wide outage banner, is laid out for a phone screen).
There is no fullscreen toggle and no resolution picker in the code.

### Audio: always on, state-driven music, no volume control

Music is managed by `game/src/audio.rs` (`MusicPlugin`). Each game
state spawns its own track on enter and despawns it on exit — menu
gets one chiptune loop, playing levels get a track tiered by world
(tutorial: Komiku CC0; worlds 2–3: Eric Skiff; world 4+: Kevin MacLeod /
SubspaceAudio), outages get the "boss fight" track (TeknoAXE
alternating with SubspaceAudio each outage so repeats don't loop the
same song), and results get a victory or defeat jingle depending on
the outcome. There is **no volume slider, no mute toggle, no track
skip** — the manager is a set of pure track-selection functions, and
nothing in the game changes playback settings. If you need quiet, use
your OS mixer.

### Companion: the one choice the menu offers

The closest thing to a setting is the companion picker on the main
menu: four buttons, one per companion, defaulting to Séraphine (fiber).
The selection swaps the sprite and the dialogue bank (see
`waifu::respawn_on_companion_change`). It is cosmetic-plus-flavor —
it never changes the physics or the level.

### Levels: bundled and embedded

Level JSON is embedded at compile time (`include_str!` in
`game/src/level.rs`), so there is no level editor, no custom-level
loading, and no level-select screen. The build ships exactly the
levels listed in `LEVEL_SOURCES` — currently two:
`world1_level1.json` and `world4_level1_outage.json`.

### Physics constants: real numbers, not tunable

Transmit power, the receive window, per-component loss figures, and
wavelength attenuation are level data and simulation constants (see
[Playing the Game](playing.md)), not user options. There is no
difficulty setting; difficulty comes from the level definitions.

## What this means in practice

If you were hoping to rebind controls, adjust audio, or tweak
graphics: none of that exists yet. The game's philosophy is
phone-first, tap-to-play, fixed presentation — the variability lives
in the levels and the physics, not in a settings menu.
