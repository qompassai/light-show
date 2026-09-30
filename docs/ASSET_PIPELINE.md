# Asset pipeline: generation, source formats, and external tools

Light Show's shipped art and audio are all procedurally generated from
small, reviewable scripts rather than hand-authored binary assets with
no reproducible source — this keeps the whole pipeline auditable and
license-clean (see `CREDITS.md`), and means every asset can be
regenerated or tuned by editing a script instead of a lost source file.

| Asset class | Generator | Ships to | Editable source for further iteration |
|---|---|---|---|
| Companion sprite sheets, profile stills, animated GIFs | `tools/gen_companion_art.py` | `game/assets/sprites/*/`, `assets/art/companions/` | `art/aseprite/*_sheet.png` + `art/aseprite/import_sheet.lua` (slices into tagged `.aseprite` docs) |
| OSP component icons | `tools/gen_placeholder_art.py` | `game/assets/sprites/components/*.png` | `art/aseprite/component_icons/*.png` (single-frame, open directly in Aseprite) |
| Chiptune music (menu/playing/outage loops, win/fail jingles) | `tools/gen_chiptune_music.py` | `game/assets/audio/*.ogg` | Regenerate by editing the note-event `Channel` lists in the script itself — see its module docstring for the NES-2A03-inspired 4-channel model (2 pulse + 1 triangle + 1 noise) |

Regenerating any of these is a plain `python3 tools/<script>.py` run
(numpy + Pillow + ffmpeg on PATH for the music script; Pillow alone for
the art scripts) — no network calls, no paid API, nothing to license
beyond what's already declared in `CREDITS.md`.

## Taking assets further in Aseprite or Unreal

Both external-tool workflows are fully scaffolded under `art/`:

- **Aseprite** — `art/aseprite/README.md`. Turns the flat companion
  PNG sheets back into real multi-frame `.aseprite` documents (via the
  included `import_sheet.lua` headless script) with correctly named
  mood-row tags, ready to hand-edit and re-export through the
  [Diver Neovim Aseprite actions](https://github.com/qompassai/Diver/blob/main/lua/utils/games/aseprite/actions.lua).
- **Unreal Engine** — `art/unreal/README.md`. A content-only
  `LightShowArt.uproject` (Paper2D enabled) that Unreal's own
  [`.uproject`-upward-search tooling](https://github.com/qompassai/Diver/blob/main/lua/utils/games/unreal/util.lua)
  auto-discovers, pre-loaded with the same sprites and audio for
  Sprite/Flipbook extraction and pixel-perfect-import instructions.
  This does **not** mean Light Show has an Unreal build — the shipping
  game is the Bevy/Rust project at the repository root; this is purely
  an art-preview/iteration surface.

## Shipping a re-exported asset back into the game

1. Edit in Aseprite or Unreal, export a flat PNG (Aseprite: **Export
   sprite sheet + JSON**; Unreal: re-export the Texture2D/Flipbook
   frames as PNG) back to the same dimensions documented in
   `docs/ART_STYLE.md` (256×384 for companion sheets, 64×64 for
   component icons).
2. Overwrite the corresponding file under `game/assets/sprites/...` (or
   `game/assets/audio/...` for music re-exported from a DAW).
3. Run `cargo test --workspace` — several tests exist specifically to
   catch broken asset wiring before it reaches a device build:
   `board::tests::every_component_icon_path_actually_exists_under_game_assets`,
   `audio::tests::every_referenced_track_exists_on_disk`, and
   `waifu::tests::mood_sheet_rows_are_unique_and_within_bounds`. None
   of these assert exact pixel dimensions today, so also do a quick
   visual check against the sheet-layout contract in
   `docs/ART_STYLE.md` before committing.
4. Run `cargo fmt --all -- --check` and
   `cargo clippy --workspace --all-targets -- -D warnings` if any Rust
   changed alongside the asset (e.g. adding a new `Component` variant's
   icon mapping in `game/src/board.rs`).
