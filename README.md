# Light Show

A puzzle game about real Outside Plant (OSP) fiber-optic engineering, built
in Rust with [Bevy](https://bevyengine.org/), for Android (Google Play +
F-Droid), with an anime-styled AI companion who reacts to every splice you
make.

Route light from the OLT (Point A) to the customer's ONT (Point B). Hit the
target receive-power window. Survive outages. All the loss figures are
real: fusion vs. mechanical splice loss, UPC vs. APC connectors, PON
splitter ratios, wavelength-dependent fiber attenuation.

## Quick start

```sh
cargo run -p light-show
```

See [`docs/BUILD.md`](docs/BUILD.md) for Android build instructions
(Google Play `.aab` and F-Droid-reproducible `cargo-apk` paths) and
[`docs/GAME_DESIGN.md`](docs/GAME_DESIGN.md) for the full design doc,
including the link-budget model and level progression.

## Project layout

```
crates/osp_sim/    Engine-agnostic fiber-optic link-budget simulation core
game/               Bevy application: states, UI, level loading, Séraphine
game/assets/levels/ Level definitions (JSON)
game/assets/dialogue/ Séraphine's dialogue bank (JSON, localizable)
docs/               Design, art direction, build, and F-Droid docs
fastlane/           Shared Play Store / F-Droid store listing metadata
tools/              Placeholder art generator
```

## Status

Early scaffold: core simulation (`osp_sim`) is fully implemented and
tested; the Bevy front-end has working state machine, level loading, win
condition checking, a live dB ledger UI, and Séraphine's dialogue/animation
skeleton. Interactive drag-to-route input, final hand-drawn art, and full
outage-repair UI are the next milestones — see open issues.

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE) and
[`docs/CREDITS.md`](docs/CREDITS.md) for asset attribution.

## Store compliance

No ads, no in-app purchases, no tracking, no network permission requested —
the same build ships unmodified to both Google Play and F-Droid. See
[`docs/FDROID.md`](docs/FDROID.md) for the anti-features checklist.
