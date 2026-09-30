# Credits & Licensing

## Code

Original code © Qompass AI, licensed under GPL-3.0-or-later (see
`LICENSE`). Third-party Rust crates retain their own licenses as declared
in `Cargo.lock` (all permissive/copyleft-compatible: MIT/Apache-2.0 for the
Bevy ecosystem).

## Engineering reference data

The dB loss figures used in `crates/osp_sim` (fusion/mechanical splice
loss, UPC/APC connector loss, PLC splitter insertion loss, G.652 fiber
attenuation coefficients, GPON ONT receive sensitivity per ITU-T G.984.2)
are drawn from publicly published industry-standard specifications and
common OSP field practice figures — they are engineering constants, not
copyrightable expression.

## Art assets

- **OSP component iconography** (fusion/mechanical splice, UPC/APC
  connector, splitter, OLT, ONT, macrobend, water intrusion): filled,
  transparent-background 64x64 sprites generated deterministically by
  `tools/gen_placeholder_art.py` (original tool code, GPL-3.0-or-later, no
  external asset dependency, no AI generation involved). Wired into
  gameplay as the on-board per-pill icons in `game/src/board.rs`. Editable
  sources for further hand-painting live at `art/aseprite/component_icons/`
  — see `docs/ASSET_PIPELINE.md`.
- **Companion portraits and derived art** (Séraphine, Ondine, Linka,
  Lattice): the four source portraits
  (`assets/art/companions/<name>_portrait.jpg`), the four-companion group
  portrait used for the app icon (`assets/art/companions/app_icon_group.jpg`),
  and everything derived from them (64x64 in-engine sprite sheets under
  `game/assets/sprites/<name>/`, animated README profile GIFs, and the
  Android launcher icon set) were generated with an AI image-generation
  model (OpenAI `gpt-image-1`-family, invoked as `gpt_image_2` through
  Perplexity's Comet/Computer image tooling) from text prompts written for
  this project, with no third-party reference images or copyrighted
  characters used as input. `tools/gen_companion_art.py` and
  `tools/gen_app_icon.py` (both original code, GPL-3.0-or-later) perform the
  crop/pixelate/tint/icon-assembly steps deterministically from those
  source portraits.
  - **Before tagging a public Play Store / F-Droid release**, confirm the
    generating account's terms of service grant redistribution rights
    sufficient for a GPL-3.0 F-Droid-eligible project (OpenAI's API/consumer
    terms currently assign the requester usage/ownership rights to
    generated images, but re-verify current terms at release time since AI
    provider ToS changes over time) and record the confirmation date here.
  - No proprietary SDK or network call is required at build or run time to
    use this art — the generated PNGs/GIFs are committed as static assets,
    so this does not affect F-Droid's no-network-dependency requirement.

## Fonts

`game/assets/fonts/pixel.ttf` is **Press Start 2P** by The Press Start 2P
Project Authors (cody@zone38.net), licensed under the SIL Open Font License
1.1 (full text at `game/assets/fonts/PressStart2P-OFL.txt`), sourced from
the [Google Fonts repository](https://github.com/google/fonts/tree/main/ofl/pressstart2p).
OFL permits redistribution and use in both the Play Store and F-Droid
builds without additional licensing action.

## Audio

All five music tracks under `game/assets/audio/` (`menu_theme.ogg`,
`playing_theme.ogg`, `outage_theme.ogg`, `victory_jingle.ogg`,
`failure_jingle.ogg`) are procedurally synthesized from scratch by
`tools/gen_chiptune_music.py` (original tool code, GPL-3.0-or-later) —
a 4-channel synthesizer modeled on the NES 2A03 sound chip's channel
layout (two duty-cycle pulse oscillators, one triangle, one noise
channel), composed directly in Python and rendered to `.wav` then
transcoded to Vorbis `.ogg` via `ffmpeg` (LGPL/GPL-licensed, a build-time
tool only — not linked into or shipped inside the app). No samples,
loops, or third-party audio of any kind were used, so there is nothing
to attribute beyond this repository's own code and no F-Droid
free-audio concern. Editable composition source is the script itself —
see `docs/ASSET_PIPELINE.md` for how to retune or extend it.
