# Light Show

![Light Show companions](assets/art/companions/app_icon_group.jpg)

A puzzle game about real Outside Plant (OSP) fiber-optic engineering, built
in Rust with [Bevy](https://bevyengine.org/), for Android (Google Play +
F-Droid), with a squad of anime-styled AI companions — one per access
technology — who react to every splice you make.

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

## Meet the companions

Pick a companion from the menu before you start splicing — each one is tied
to a real access technology, has her own dialogue bank, and reacts to your
splices, outages, and level results in her own voice.

| | | | |
|---|---|---|---|
| ![Séraphine](assets/art/companions/seraphine_animated.gif)<br>**Séraphine** — Fiber | ![Ondine](assets/art/companions/ondine_animated.gif)<br>**Ondine** — Coax | ![Linka](assets/art/companions/linka_animated.gif)<br>**Linka** — Mobile | ![Lattice](assets/art/companions/lattice_animated.gif)<br>**Lattice** — Ethernet |

Full character briefs, palettes, and mood-sheet specs live in
[`docs/ART_STYLE.md`](docs/ART_STYLE.md).

## Project layout

```
crates/osp_sim/       Engine-agnostic fiber-optic link-budget simulation core
game/                  Bevy application: states, UI, level loading, companions
game/assets/levels/    Level definitions (JSON)
game/assets/dialogue/  Per-companion dialogue banks (JSON, localizable)
game/assets/sprites/   In-engine sprite sheets, incl. companion mood sheets
game/assets/audio/     Chiptune music loops + win/fail jingles (.ogg)
assets/art/companions/ Source portraits, animated profile GIFs, app icon art
art/aseprite/          Aseprite-ready sprite sources + headless import script
art/unreal/            Content-only Unreal preview project (Paper2D) for the same art
docs/                  Design, art direction, asset pipeline, build, and F-Droid docs
fastlane/              Shared Play Store / F-Droid store listing metadata
tools/                 Art + music generation scripts (placeholder art, companion art, chiptune)
```

## Status

Feature-complete gameplay loop: core simulation (`osp_sim`) is fully
implemented and tested; the Bevy front-end has a working state machine
(menu → playing → outage-repair → results), level loading, win/fail
condition checking with a live dB ledger UI, a companion-select menu,
an outage-repair loop (fault injection, live reroute under a countdown,
in-window vs. timed-out resolution), a results screen with win/fail
banners and companion dialogue, all four companions' dialogue/animation
systems (backed by real AI-generated portrait art and 64x64 in-engine
sprite sheets with per-mood accent tinting), on-board component icon
sprites for every placeable part, and a full Megaman-esque chiptune
soundtrack (menu/playing/outage loops, win/fail jingles) wired to every
state. See [`docs/ASSET_PIPELINE.md`](docs/ASSET_PIPELINE.md) for how
all art/audio is generated and how to take it further in Aseprite or
Unreal. Remaining before store submission is entirely account/asset
logistics rather than code: a signing keystore, real device
screenshots, and the Play Console/`fdroiddata` listing steps — see
[`docs/BUILD.md`](docs/BUILD.md) and [`docs/FDROID.md`](docs/FDROID.md).

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE) and
[`docs/CREDITS.md`](docs/CREDITS.md) for asset attribution.

## Store compliance

No ads, no in-app purchases, no tracking, no network permission requested —
the same build ships unmodified to both Google Play and F-Droid. See
[`docs/FDROID.md`](docs/FDROID.md) for the anti-features checklist.
