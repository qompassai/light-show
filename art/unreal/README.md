# Unreal preview/companion project

`LightShowArt.uproject` is a **content-only** Unreal Engine project (no
C++ modules, so it opens without a build step) whose only purpose is to
give you a real Unreal Editor environment for previewing, animating
further, or repurposing Light Show's pixel art and chiptune audio.

**The shipping game is not built in Unreal.** Light Show is a Bevy/Rust
game (see `game/` at the repository root) targeting Android (Play
Store) and F-Droid via `cargo-apk` — see `docs/BUILD.md`. This folder
exists because the original brief asked for art you could keep
iterating on "with Aseprite and/or Unreal Engine," so the assets are
packaged for both without implying a second game exists.

Because no Unreal Editor is available in the environment these files
were prepared in, this scaffold has been reviewed against Unreal's
documented `.uproject` schema and Paper2D import workflow but not
actually opened in the Editor — the first `Content/` `.uasset` files
(Textures, Sprites, Flipbooks) will be generated the first time you
open the project and import the PNGs below, which is standard: `.uasset`
binaries can only be produced by the Editor itself, never authored by
hand.

## Layout

```
art/unreal/
├── LightShowArt.uproject      # opens directly in UE 5.4+; Paper2D plugin pre-enabled
└── Content/
    ├── Sprites/Companions/    # the 4 companion sheets, 256x384 (64x64 x 4 cols x 6 rows)
    ├── Sprites/Components/    # the 9 OSP component icons, 64x64 single-frame
    └── Audio/                 # the 5 chiptune .ogg tracks
```

Because `LightShowArt.uproject` sits under this repo, the [Diver Neovim
Unreal tooling](https://github.com/qompassai/Diver/blob/main/lua/utils/games/unreal/util.lua)
(`find_uproject`, used by every action in
`utils.games.unreal.actions`) will discover it automatically via its
upward directory search from any file under `art/unreal/` — no
`NVIM_UNREAL_PROJECT` override needed as long as your cwd/buffer is
inside this subtree. If you want the root-level Unreal commands to
target this project from anywhere in the repo, set
`NVIM_UNREAL_PROJECT` to this file's absolute path.

## Importing the sprites

1. Open `LightShowArt.uproject` in Unreal Editor (5.4 or later; earlier
   5.x versions with Paper2D should also work — adjust
   `EngineAssociation` in the `.uproject` if pinning a different minor
   version).
2. Drag the PNGs from `Content/Sprites/Companions/` and
   `Content/Sprites/Components/` into the Content Browser (or use
   **Import**), or edit them in place with Unreal's built-in texture
   editor.
3. For each imported companion sheet's Texture2D, open **Sprite Actions
   → Extract Sprites** to slice it into 24 individual sprites (4
   columns × 6 rows @ 64×64) — this is the Paper2D equivalent of the
   Aseprite-side row/tag split in `art/aseprite/import_sheet.lua`.
4. Group each mood row's 4 extracted sprites into a **Paper Flipbook**
   (right-click → **Create Flipbook**) at the same 180ms-per-frame rate
   the Bevy build uses (`Timer::from_seconds(0.18, ...)` in
   `game/src/waifu/mod.rs`) so any Unreal-side preview or prototype
   matches the shipped game's animation speed.

## Pixel-art import settings

Set these on every imported Texture2D so pixel art stays crisp instead
of blurring under Unreal's default bilinear/mip-mapped settings:

- **Texture Group:** `2D Pixels (unfiltered)` if available, or manually
  set **Filter** to `Nearest`.
- **Mip Gen Settings:** `NoMipmaps` (these sprites are only ever shown
  at native or integer-multiple scale).
- **Compression Settings:** `UserInterface2D (RGBA)` to preserve exact
  alpha edges on the transparent-background component icons.
- **sRGB:** leave enabled — the PNGs were authored and hex-specified in
  sRGB space (see the palette table in `docs/ART_STYLE.md`).

## Palette and style reference

`docs/ART_STYLE.md` at the repository root is the canonical style
brief (palette hex values, companion roster, mood-row contract,
component iconography table) — treat it as the source of truth if
anything here and the live game diverge after further edits in either
Aseprite or Unreal.
