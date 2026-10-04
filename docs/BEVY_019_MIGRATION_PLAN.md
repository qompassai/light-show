# Light Show — Bevy 0.14 → 0.19.1 Migration Plan

**Status:** DRAFT — research and scoping only. No code changed. For Matt's review.
**Date:** 2026-10-03
**Decision context:** Matt approved migrating light-show from Bevy 0.14 to 0.19.1, starting only after the current animation/SFX work settles. The value he wants is the 0.19.1 bug fixes (2D flicker, MP4 audio, text measurement, iOS pink-screen, Steam OSK) plus the features the jump unlocks.

**How this was built:** each of the five official migration guides (bevy.org/learn/migration-guides) was read in full and every breaking change was grepped against `game/src/` (~8,265 lines). "Verified" below means checked against the guide text, docs.rs, or the actual repo — not inferred.

---

## Effort summary

| Step | Rating | Estimate | Character |
|------|--------|----------|-----------|
| 0.14 → 0.15 | **L** | 1–2 days | The heavy one. Text rework + bundle migrations. |
| 0.15 → 0.16 | **M** | half day | Waifu sprite/atlas rework + picking backend feature. |
| 0.16 → 0.17 | **S** | half day incl. visual check | Renames; one sneaky silent UI-transform break. |
| 0.17 → 0.18 | **S** | couple hours | Rename batch + one Cargo.toml feature. |
| 0.18 → 0.19 | **S** | couple hours | Mechanical `FontSize` wrapping + Android Cargo fix. |
| Cleanup pass | **S** | couple hours | Delete hand-ported easing, win-ring alpha, deprecation sweep. |
| **Total** | | **≈ 3–4 days** | |

Ratings: S < 2h · M = half-day · L = 1–2 days.

**Toolchain check:** primo has rustc 1.96.0 — well above Bevy 0.19's MSRV (~1.87). No toolchain work needed. Crate edition stays 2021.

---

## Silent runtime breaks — read first

These compile cleanly but break the game visually or functionally. Each step's verification pass must check them explicitly:

1. **0.15 — `TextureAtlas` as a component on sprite entities silently stops rendering.** The companion sprites (waifu/mod.rs) will vanish with no error. (Fix is part of the 0.15 sprite work below.)
2. **0.15 — `BackgroundColor` default becomes transparent** and no longer tints images. Menu title art and gold buttons need a visual check.
3. **0.16 — missing `bevy_ui_picking_backend` feature: every button silently stops responding.** No compile error. Must be verified on device/emulator.
4. **0.17 — UI entities move from `Transform` to `UiTransform`.** The results-screen entrance animations query `Option<&mut Transform>` — they compile fine but match nothing, so the animations silently stop. One file (results.rs), needs an eyeball check.
5. **0.18 — missing `bevy_gizmos_render` feature: board neon lines vanish.** The game uses `default-features = false`, which is exactly the affected configuration.

---

## Step 1 — 0.14 → 0.15 (L, 1–2 days)

The long pole. Do this step alone; gate on `cargo build` + full test suite + visual pass.

### 1A. Text rework — L
`Text` becomes a single-segment tuple struct; `TextSection` is gone; styling splits into `TextFont` (font, font_size) + `TextColor`; layout moves to `TextLayout`. Multi-section text becomes `TextSpan` child entities edited via `TextUiWriter`. `TextBundle` is a deprecated **empty struct** — `from_section` is gone, so all call sites break.

Hit sites:
- `TextBundle::from_section` ×14 — credits.rs:69, outage.rs:142,152, companion_select.rs:165,255,263, results.rs:341,371,391,411,510, board.rs:663,864
- `Text::from_section` ×2 (inside `Text2dBundle`) — board.rs:741,827
- `TextBundle { text: Text { sections, justify }, style }` literals — ui/neon.rs:92,112
- `TextSection::new` — ui/neon.rs:94,114
- `TextStyle { font, font_size, color }` ×12 — neon.rs:72 (`style_for` closure), credits.rs:71, outage.rs:144,154, companion_select.rs:167,257,265, results.rs:343,373,393,413,512, board.rs:665,743,829,866
- Section mutation — outage.rs:172, results.rs:161,190, ui/mod.rs:52 (`text.sections[0].value`, `txt.sections.iter_mut()`)

Fix: single-section sites → `commands.spawn((Text(value), TextFont { font, font_size, ..default() }, TextColor(color), TextLayout { justify, ..default() }, Node { … }))`. Multi-section/mutated sites → `TextUiWriter` system param. `spawn_neon_text` (neon.rs, spawns ~17 glow copies per call) needs the full rewrite: one `Text` root + `TextSpan` children.

Behavior note: cosmic-text replaces ab_glyph — the guide says divide existing font sizes by 1.2 for identical rendering. Visual pass required.

### 1B. UI bundles → required components — M
`Style` is renamed to `Node` (layout fields move); computed values move to `ComputedNode`. Deprecated `NodeBundle`/`ImageBundle`/`ButtonBundle` still exist but their `style:` field was renamed to `node:` — existing literals break.

Hit sites:
- `NodeBundle { style: Style { … } }` ×10 — credits.rs:49, outage.rs:126, menu.rs:65, companion_select.rs:90,122,228, results.rs:306,425, anim.rs:118,284
- `ImageBundle { style, image: UiImage::new(…) }` ×3 — menu.rs:~93, companion_select.rs:~109,~220
- `ButtonBundle { style, background_color }` ×6 — credits.rs:96, menu.rs:125,157, companion_select.rs:154,199, results.rs:500
- `Query<(Entity, &mut Style, …)>` — outage.rs:193,213 (mutates `style.top` for the banner slide)

Fix (mechanical): `Style { … }` → `Node { … }`; `&mut Style` → `&mut Node`; bundle literals → component tuples, e.g. `(Node { … }, BackgroundColor(c))`, `(Node, ImageNode::new(…))`, `(Button, Node, BackgroundColor)`.

Note: the `UiImage` → `ImageNode` rename is verified to land in the **0.16** guide, not 0.15. If the 0.15 compiler still accepts `UiImage`, leave it and do the rename in Step 2 — the compiler will tell you.

### 1C. Sprite bundle migration — S
`SpriteBundle` is deprecated but its `texture: Handle<Image>` field was **removed** — the waifu literal breaks. Separately, `TextureAtlas` as a component on sprite entities will no longer render (silent break #1 above).

Hit sites: waifu/mod.rs:294 (return type), :305–312 (`SpriteBundle { texture, transform, .. }` + `TextureAtlas { layout, index }`), :365 (`Query<(&CompanionSprite, &mut TextureAtlas)>`), :413 (test query).

Fix: return `(CompanionSprite, Sprite, Transform)`; `Sprite { image: texture, texture_atlas: Some(TextureAtlas { layout, index }), ..default() }`; queries become `&mut Sprite` / `&Sprite` with `sprite.texture_atlas.as_mut().unwrap().index = …`.

### 1D. Audio bundle removal — S
`AudioBundle` is **removed** in 0.15 (not just deprecated). `AudioPlayer<T>` component + auto-inserted `PlaybackSettings`.

Hit sites: audio.rs:122–129 (`spawn_track`), :312–318 (`Sfx::play` — the new SFX system).

Fix: `commands.spawn((AudioPlayer(asset_server.load(path)), PlaybackSettings { mode: PlaybackMode::Loop, ..default() }))` and the same with `PlaybackMode::Despawn` + `Volume` for one-shots.

### 1E. One-liners — S total
- `Handle` is no longer a `Component` — audio.rs:485 (test) `Query<Entity, With<Handle<AudioSource>>>` → `With<AudioPlayer<AudioSource>>`.
- `Camera::viewport_to_world_2d` now returns `Result` — board.rs:904, wrap with `.ok()`.
- `or_else` run condition renamed to `or` — ui/mod.rs:23.
- `ZIndex::Global(i32::MAX)` split — anim.rs:126 → `GlobalZIndex(i32::MAX)` component.

### 1F. Android activity default — M
0.15 makes **GameActivity** the default, replacing NativeActivity; `cargo-apk` is replaced by `cargo-ndk`. Light-show ships the NativeActivity path (`#[bevy_main]`, game-activity glue, cargo-apk docs).

Fix: pin the NativeActivity path explicitly by adding `"android-native-activity"` to the android bevy feature list in game/Cargo.toml (do NOT add `android-game-activity` — that would switch activities). Migrate the documented build flow from `cargo-apk` to `cargo-ndk` (docs/BUILD.md already documents the cargo-ndk flow as Option B; docs/FDROID.md, the publish skill, and `nix/apps/release.sh` reference cargo-apk). Verify on-device.

Hit sites: game/Cargo.toml android features, docs/BUILD.md:31–47, docs/FDROID.md:40–75, publish skill docs, `nix/apps/release.sh`.

### 1G. Optional deprecations (still compile)
- `Camera2dBundle` → `Camera2d` — menu.rs:59, playing.rs:174. S, do it now while touching cameras.
- `SpatialBundle` → `(Transform, Visibility)` — board.rs:674. S.

### Verified not applicable in 0.15
`EventReader`/`EventWriter` (message rename is 0.17); states API; `Touches`; `ButtonInput<MouseButton>`; `Window::cursor_position()`; `Window { title, resolution, ..default() }`; `RenderAdapterInfo` / `get_sub_app_mut(RenderApp)`; `TextureAtlasLayout::from_grid`; wgpu settings in bench.rs.

**Gate:** `cargo build` clean, `cargo test` 108/108, visual pass (text layout/sizes, menu art, buttons, companion sprites render, banner slide).

---

## Step 2 — 0.15 → 0.16 (M, half day)

### 2A. Waifu sprite/atlas rework — M
`TextureAtlas` loses its `Component` impl and moves into `bevy_image`; sprite-side atlas lives in `Sprite.texture_atlas`.

Hit sites: waifu/mod.rs:22,126,130,294–313,365,399,413; waifu/sprite.rs:15–16; playthrough.rs:72.

Fix: `bevy::sprite::TextureAtlas{,Layout}` → `bevy::image::TextureAtlas{,Layout}`; atlas sync via `sprite.texture_atlas.as_mut().unwrap().index`. (This is the second half of Step 1's 1C — the import path changes here.)

### 2B. `bevy_ui_picking_backend` feature — S + device verification
`bevy_ui` no longer implies the picking backend. Without the feature, `Interaction` stops updating — **every button in the game silently stops responding** (silent break #3).

Fix: add `"bevy_ui_picking_backend"` to both bevy feature lists in game/Cargo.toml (desktop + android). No code change. Must verify button hover/press on device or emulator.

### 2C. Relationship renames — S total
- `Parent` → `ChildOf`, `.get()` → `.parent()` — results.rs:217,231; board.rs:1248,1255,1299,1316.
- `despawn_recursive()` → `despawn()` (now recursive) — credits.rs:130, outage.rs:222, menu.rs:223, companion_select.rs:381, results.rs:558, board.rs:880.
- `&mut ChildBuilder` → `&mut ChildSpawnerCommands` — companion_select.rs:178, results.rs:492, ui/neon.rs:65. (Side benefit: the manual detach-before-despawn workaround is now redundant — 0.16 auto-detaches on despawn. Optional simplification.)

### 2D. One-liners — S total
- `UiImage` → `ImageNode` (if not done in Step 1) — menu.rs:99, companion_select.rs:115,224,282,504, results.rs:217,335; field `texture` → `image` (results.rs:237, companion_select.rs:292).
- `Volume::new(x)` → `Volume::Linear(x)` (`Volume` is now an enum) — audio.rs:314.
- `Anchor::Center` → `Anchor::CENTER` — board.rs:750.
- `bevy::core::TaskPoolPlugin` → `bevy::app::TaskPoolPlugin` — board.rs:2290,2343 (test code).
- Add `"std"` + `"async_executor"` to feature lists if the build complains (0.16 `no_std` split).

### 2E. Deprecation sweep (cheap, do now)
- `EventWriter::send` → `.write` — bench.rs:189, board.rs:947,969.
- `Query::get_single()` → `single()` — playing.rs:162,210, waifu/mod.rs:339, board.rs:893,899,1200,1311.

**Gate:** `cargo build` clean, `cargo test` 108/108, button hover/press verified working (device or emulator).

---

## Step 3 — 0.16 → 0.17 (S, half day incl. visual check)

### 3A. Event → Message rename — S
`Event` becomes observer-only; the broadcast API is renamed to `Message`. Use the new names directly (the 0.17 deprecated aliases die in 0.18 — don't use them).

Hit sites: waifu/mod.rs:33 (`.add_event` → `.add_message`), :52 (`#[derive(Event)]` → `#[derive(Message)]`), :104 (`EventReader` → `MessageReader`); board.rs:926 (`EventWriter` → `MessageWriter`), :947,969 (`.send` → `.write`), :1828 (test: `Events<SpliceReaction>` → `Messages<SpliceReaction>`); bench.rs:153,189 (`EventWriter<AppExit>` → `MessageWriter`, `.send` → `.write`).

### 3B. `UiTransform` for UI entities — S + visual check (silent break #4)
UI entities now carry `UiTransform` instead of `Transform`. The results-screen entrance animations query `Option<&mut Transform>` — they compile but match nothing, so the banner-pop and win-ring animations silently stop.

Hit sites: results.rs:~135 (entrance scale ease — `tr.scale = Vec3::splat(…)` → `UiTransform`, `Vec2::splat(…)`), :~217 (`animate_win_ring` — same conversion).

Fix: `&mut Transform` → `&mut UiTransform`; `Vec3::splat` → `Vec2::splat`. Eyeball-check both animations after.

### 3C. One-liners — S total
- `Anchor` is now a required component on `Sprite`; `text_anchor:` field removed from text spawns — board.rs:750 → add `Anchor::CENTER` as a separate component.
- `WindowResolution`: `(720.0_f32, 1280.0_f32).into()` no longer compiles — lib.rs:107 → `(720, 1280).into()`.
- `JustifyText` → `Justify` — ui/neon.rs:57.
- `Text2d` moved to `bevy_sprite` — board.rs world-space text: `use bevy::sprite::Text2d`.

**Gate:** `cargo build` clean, `cargo test` 108/108, visual check of results-screen entrance + win ring.

---

## Step 4 — 0.17 → 0.18 (S, couple hours)

- `bevy_gizmos_render` feature — add to both feature lists in game/Cargo.toml. Without it (and `default-features = false`), the board's neon gizmo lines silently vanish (silent break #5). S.
- `remove_children` / `clear_children` → `detach_children` (deprecation in 0.18; hard break later) — results.rs:233, board.rs:1258,1305. S.
- `Event`/`EventReader`/`EventWriter` aliases are **removed** in 0.18 — already handled in Step 3 (we used the new names directly). Nothing to do; just don't introduce aliases.
- Verified no-hit: same-state-transition behavior (all our transitions are cross-state), `RenderTarget`, input features, `LineHeight`, audio, resources.

**Gate:** `cargo build` clean, `cargo test` 108/108, gizmo lines visible in-game.

---

## Step 5 — 0.18 → 0.19 (S, couple hours)

### 5A. `TextFont` field changes — S (mechanical, ~15 sites)
- `font: Handle<Font>` → `font: FontSource` — append `.into()` to every `asset_server.load(fonts::…)` in a `TextFont` literal.
- `font_size: f32` → `font_size: FontSize` enum — wrap every literal: `FontSize::Px(24.0)`.

Hit sites (re-grep `TextFont` at 0.18 — line numbers will have shifted from the 0.14 tree): credits.rs:69–71, outage.rs:142–154, companion_select.rs:165–167,255–265, results.rs:341–343,371–393,411–413,510–512, ui/neon.rs:72 (`style_for` closure — one fix covers all neon text), board.rs:665,740–743,826–829,866 (Text2d labels). The new `weight`/`width`/`style`/`font_variations` fields are covered by existing `..Default::default()` — no action. (Optional later: replace the separate `DISPLAY_BOLD` font file with `weight: FontWeight::BOLD`.)

The compiler points at every site. Recommend a visual once-over of text after (parley is the layout engine now).

### 5B. Android Cargo fix — S
`"android_shared_stdcxx"` was removed in 0.19 (Rodio 0.22 dropped oboe-shared-stdcxx). Delete the line from game/Cargo.toml's android feature set. Nothing replaces it. Then run the Android build to confirm. (Minimum API for `bevy_audio` is now 26 — already satisfied: min_sdk_version = 26.)

### 5C. Verify-only (no code changes)
- **Resources as components:** `Res`/`ResMut`/`init_resource`/`insert_resource`/`World::resource*` all keep their signatures. Verified: no double `#[derive(Component, Resource)]`, no unfiltered broad queries (`Query<Entity>` etc.), no non-send resources, no generic resource helpers. The whole job is a compile + test pass. S.
- **Render sub-app** (`lib.rs`): `RenderApp`, the `Render` schedule label ("the main render schedule"), and `RenderAdapterInfo` all still exist in 0.19 — verified on docs.rs. `get_sub_app_mut(RenderApp)` + `add_systems(Render, log_render_backend_once)` needs no change; confirm the `[light-show] render backend:` line still prints exactly once at startup. S.
- **BSN scenes:** confirmed additive and optional — light-show uses no scene APIs. Skip entirely.

**Gate:** `cargo build` clean, `cargo test` 108/108, Android build succeeds, render-backend log line fires once.

---

## Step 6 — Cleanup pass at 0.19 (S, couple hours)

Things the migration unlocks — do these after all five steps are green:

1. **Delete the hand-ported `Ease` enum** (game/src/anim.rs:30–66) → use `bevy::math::curve::EaseFunction` natively. Bonus: the engine enum adds `ElasticIn/Out`, `BackIn`, `Bounce*` etc. — directly useful for the pill-pop / mood-pop juice. S.
2. **Win-ring alpha fade becomes possible.** In 0.14 `UiImage` had no color property, so the win-ring fade was implemented as scale-only (see results.rs comment). `ImageNode` has full color — implement the intended alpha 1→0 fade. S.
3. **Remove the manual detach-before-despawn workaround** (kept through 0.16 for safety) — 0.16+ auto-detaches children on despawn. S.
4. **Deprecation sweep** — `cargo clippy` + fix any remaining deprecation warnings so the tree is warning-clean on 0.19. S.
5. **Optional, not required:** `bevy_picking` (first-party since 0.16) could replace the hand-rolled `DragState`/`PointerWorld` pointer code in board.rs — but the current code works and is well-tested. Recommend leaving it; revisit only if touch behavior needs work.

**Gate:** `cargo build` clean, `cargo clippy` clean, `cargo test` 108/108.

---

## Ecosystem risks — low

| Dependency | Bevy coupling | Action |
|------------|---------------|--------|
| `bevy` / `bevy_ecs` / `bevy_state` (0.14) | direct | Bump in lockstep each step: `bevy = "0.1x"`, `bevy_ecs = "0.1x"`, `bevy_state = "0.1x"`. |
| `osp_sim` (local path crate) | **none** — serde/serde_json/thiserror only | None. |
| `android_logger` 0.14 (optional) | **none** — depends only on `log`/`android_log-sys`/`env_filter` | None required. Optional bump to 0.15.1 (latest) — trivial, no API used beyond `init_once` + `Config`. |
| `serde` / `serde_json` | none | None. |
| Third-party Bevy plugins | **none used** | None — this is the big risk reducer. No waiting on ecosystem crates. |

Ecosystem health for 0.19 is good (bevy_egui, bevy_embedded_assets, bevy_vello all ship 0.19 releases), but light-show doesn't need any of them.

---

## Recommended sequencing

1. **Step 1 (0.14→0.15) alone.** It's the only L. Don't combine — the text rework deserves its own compile-debug-visual cycle.
2. **Step 2 (0.15→0.16) alone.** The picking-backend silent break needs a dedicated device/emulator verification.
3. **Step 3 (0.16→0.17) alone.** The UiTransform silent break needs a dedicated visual check.
4. **Steps 4+5 (0.17→0.18→0.19) combined.** Both are all-S mechanical work; one compile-debug cycle with the test suite as gate. Fall back to separate steps if anything is unclear.
5. **Step 6 cleanup** once 0.19.1 is green.

Each step: bump versions in game/Cargo.toml → `cargo build` → fix → `cargo test` → step-specific verification → commit. One commit per step (unpushed until Matt reviews, per standing rules — no standing push authorization exists for this program).

**Total: ≈ 3–4 days.** The 0.14→0.15 text rework is half the work. Everything after 0.16 is renames, one-liners, and verification.

---

## Open questions / verification notes (for the implementer)

- The `UiImage` → `ImageNode` rename is in the 0.16 guide; one researcher also saw it referenced for 0.15. Trust the compiler: if 0.15 still accepts `UiImage`, defer the rename to Step 2.
- The 0.15 researcher recommended adding `android-game-activity` to keep Android working; that is **wrong for light-show** — the game ships the NativeActivity path, so pin `android-native-activity` instead (verified against the 0.19 mobile docs pattern).
- `bench.rs`'s `WgpuSettings`/`RenderCreation` backend-forcing (bench.rs:21,84–85) had no guide hits in any step, but render settings churned across these versions — if it fails to compile, check the `RenderPlugin` fields for that version first.
- Re-grep `TextFont` at 0.18 before Step 5 — the line numbers in this plan are from the 0.14 tree.
- The `anim.rs` fade driver forwards `TransitionRequest` blindly to `next_state.set` — after 0.18's same-state-transition behavior change, a *future* same-state request would re-run OnEnter/OnExit. No current call site does this; noted, no action.
