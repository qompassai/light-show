# Light Show — extra Google Play listing assets

App: Light Show (ai.qompass.lightshow)
Produced: 2026-10-13 (America/Los_Angeles), on primo.
Status: staged locally only — nothing uploaded to Play Console, nothing committed to git.
All screenshots are genuine captures of the game running (Android emulator / desktop Linux build). No mock-ups, no AI-generated screenshots, no composited fakes.

## Confirmed specifications (primary sources, fetched 2026-10-13)

General + tablet/desktop screenshot rules — Google Play Console Help, "Add preview assets to showcase your app"
(https://support.google.com/googleplay/android-developer/answer/9866151):

- Screenshots: JPEG or 24-bit PNG (no alpha); minimum dimension 320 px; maximum dimension 3840 px; the longest side may be at most twice the shortest side; up to 8 per device type.
- Large screens (tablets and Chromebook/desktop): screenshots between 1,080 and 7,680 px per side; 16:9 aspect ratio for landscape, 9:16 for portrait; at least 4 screenshots recommended to demonstrate the in-app experience.
- Games recommendation: at least three 16:9 landscape screenshots at minimum 1920x1080, or three 9:16 portrait at minimum 1080x1920.

Play Games on PC game-card assets — Android Developers, "Submit your Google Play Games on PC game"
(https://developer.android.com/games/playgames/development-submit):

- PGPC logo: PNG, exactly 600x400 px, up to 8 MB, transparent background; the image represents the game name and is overlaid on the feature graphic.
- PGPC feature graphic (game-card background): PNG or JPG, 16:9 aspect ratio, each side between 720 and 7,680 px, up to 15 MB; should represent the game cover and contain no text (see the production note under that category below).

Note: the working assumption at task start (PGPC logo 512x512) was superseded by the official spec above (600x400); the official spec governed production.

## Categories

### 1. 7-inch tablet screenshots — `tablet-7-inch/`

Spec: PNG, 9:16 portrait, each side 1,080–7,680 px (large-screen rule) and within the general 320–3,840 px envelope; up to 8.
Capture source: API 36 emulator, AVD `ls_tab7_api36` (1080x1920 @ 320 dpi, ~6.9 in panel), portrait, `-no-audio`.
All files verified with ImageMagick `identify`: PNG, 1080x1920, no alpha channel (24-bit; `adb screencap` emits RGBA, so the fully-opaque alpha channel was stripped losslessly after capture to meet the 24-bit/no-alpha rule — pixel values unchanged).

| File | Dimensions | Size (bytes) | Subject |
|---|---|---|---|
| ls_tab7_01_menu.png | 1080x1920 | 2,540,370 | Main menu / title (portrait key art, full frame) |
| ls_tab7_02_companion_select.png | 1080x1920 | 1,162,227 | Companion select ("choose your discipline") |
| ls_tab7_03_credits.png | 1080x1920 | 305,707 | Credits |
| ls_tab7_04_level_select.png | 1080x1920 | 1,394,012 | Level select — Séraphine, World 1 |
| ls_tab7_05_gameplay_world1.png | 1080x1920 | 407,369 | Gameplay — World 1 "First Light", splice placed, link lit, ledger PASS |
| ls_tab7_06_gameplay_world2.png | 1080x1920 | 354,960 | Gameplay — World 2 "Unity Gain", +20 dB amp placed, all checks PASS, Cable ID panel open |
| ls_tab7_07_result.png | 1080x1920 | 160,097 | Result — SERVICE RESTORED, World 1 |
| ls_tab7_08_warehouse.png | 1080x1920 | 1,573,176 | The Warehouse (Bianca's test bench) |

### 2. 10-inch tablet screenshots — `tablet-10-inch/`

Spec: PNG, 16:9 landscape, each side 1,080–7,680 px and within the general envelope; up to 8.
Capture source: API 36 emulator, AVD `ls_tab10_api36` (2560x1440 @ 300 dpi, ~9.8 in panel), landscape, `-no-audio`, native panel resolution (no `wm size` override was needed).
All files verified: PNG, 2560x1440, no alpha channel (stripped losslessly as above).

| File | Dimensions | Size (bytes) | Subject |
|---|---|---|---|
| ls_tab10_01_menu.png | 2560x1440 | 4,536,061 | Main menu / title (landscape key art fitted to the full frame) |
| ls_tab10_02_companion_select.png | 2560x1440 | 2,630,681 | Companion select — 2x2 discipline grid |
| ls_tab10_03_credits.png | 2560x1440 | 288,307 | Credits (full page on one screen) |
| ls_tab10_04_level_select.png | 2560x1440 | 2,857,231 | Level select — Séraphine, World 1 |
| ls_tab10_05_gameplay_world1.png | 2560x1440 | 431,621 | Gameplay — World 1 "First Light", link lit, ledger PASS |
| ls_tab10_06_gameplay_world2.png | 2560x1440 | 358,288 | Gameplay — World 2 "Unity Gain", +20 dB amp placed, IN WINDOW |
| ls_tab10_07_result.png | 2560x1440 | 150,641 | Result — SERVICE RESTORED, World 1 |
| ls_tab10_08_warehouse.png | 2560x1440 | 3,047,473 | The Warehouse (full wide scene) |

### 3. Google Play Games on PC logo — `pgpc-logo/`

Spec: PNG exactly 600x400, transparent background, ≤ 8 MB.
Source art: game/assets/sprites/ui/title_logo.png (the in-game title logo, 400x240, palette PNG with alpha).
Derivation: logo placed unscaled (native 400x240, pixel art kept crisp) centered on a 600x400 transparent canvas; no resampling.

| File | Dimensions | Size (bytes) | Notes |
|---|---|---|---|
| lightshow_pgpc_logo.png | 600x400 | 5,202 | PNG with alpha (transparent background, as the spec requires) |

### 4. Google Play Games on PC feature graphic — `pgpc-feature-graphic/`

Spec: PNG or JPG, 16:9, each side 720–7,680 px, ≤ 15 MB.
Source art: game/assets/sprites/ui/title_artwork_landscape.png — the game's official landscape cover/key art, already exactly 1920x1080; used unmodified (byte-identical copy, sha256 92b46b6cc103323c65a456d0ace62097fa6fd2ee1f02b1224274e4cf3d2eb22e).
Production note: Google guidance says the game-card background should represent the cover and not contain text, because the logo is overlaid on it. The game's cover art has the LIGHT SHOW title baked into the artwork itself, and no text-free plate of the cover exists in the repo (both title artwork files carry the baked title; the fastlane feature graphic is a separate 1024x500 design). The cover art was used as-is rather than fabricating a text-free variant; swapping in a future text-free plate is a drop-in replacement at the same dimensions.

| File | Dimensions | Size (bytes) | Notes |
|---|---|---|---|
| lightshow_pgpc_feature_graphic.png | 1920x1080 | 3,439,266 | PNG, no alpha; unmodified key art |

### 5. Google Play Games on PC screenshots — `pgpc-screenshots/`

Spec: PNG, 16:9 landscape 1920x1080 (large-screen range; meets the games recommendation of ≥1920x1080 16:9); up to 8.
Capture source: desktop Linux build of the game at 1920x1080 under Xvfb (NVIDIA Vulkan) — the same genuine captures as the desktop set below, staged for both categories (Play Games on PC runs the desktop-class experience; no separate PGPC client build exists to capture from).
All files verified: PNG, 1920x1080, no alpha channel.

| File | Dimensions | Size (bytes) | Subject |
|---|---|---|---|
| ls_desk_01_menu.png | 1920x1080 | 3,181,238 | Main menu / title |
| ls_desk_02_companion_select.png | 1920x1080 | 2,286,869 | Companion select |
| ls_desk_03_credits.png | 1920x1080 | 132,103 | Credits |
| ls_desk_04_level_select.png | 1920x1080 | 2,297,453 | Level select — Séraphine |
| ls_desk_05_gameplay_world1.png | 1920x1080 | 232,870 | Gameplay — World 1, link lit |
| ls_desk_06_gameplay_world2.png | 1920x1080 | 190,844 | Gameplay — World 2, +20 dB amp placed |
| ls_desk_07_result.png | 1920x1080 | 125,909 | Result — SERVICE RESTORED |
| ls_desk_08_warehouse.png | 1920x1080 | 2,028,917 | The Warehouse |

### 6. Desktop screenshots — `desktop-screenshots/`

Spec: PNG, 16:9 landscape, each side 1,080–7,680 px (Chromebook/large-screen grouping); up to 8. Produced at 1920x1080.
Capture source: desktop Linux build of the game at 1920x1080 under Xvfb (NVIDIA Vulkan, `VK_ICD_FILENAMES` pinned to the NVIDIA ICD; audio muted — see the audio note below), window resized from its 1280x720 launch size to 1920x1080 before capture.
All files verified: PNG, 1920x1080, no alpha channel. Same files as category 5.

| File | Dimensions | Size (bytes) | Subject |
|---|---|---|---|
| ls_desk_01_menu.png | 1920x1080 | 3,181,238 | Main menu / title |
| ls_desk_02_companion_select.png | 1920x1080 | 2,286,869 | Companion select |
| ls_desk_03_credits.png | 1920x1080 | 132,103 | Credits |
| ls_desk_04_level_select.png | 1920x1080 | 2,297,453 | Level select — Séraphine |
| ls_desk_05_gameplay_world1.png | 1920x1080 | 232,870 | Gameplay — World 1, link lit |
| ls_desk_06_gameplay_world2.png | 1920x1080 | 190,844 | Gameplay — World 2, +20 dB amp placed |
| ls_desk_07_result.png | 1920x1080 | 125,909 | Result — SERVICE RESTORED |
| ls_desk_08_warehouse.png | 1920x1080 | 2,028,917 | The Warehouse |

## Intentionally NOT produced

- Android XR screenshots — Light Show is not an Android XR app; it has no XR build or XR form factor.
- Spatial XR video — not an XR app, and the slot is a YouTube-URL field, not a file upload.
- Non-spatial XR video — not an XR app, and the slot is a YouTube-URL field, not a file upload.
- Google Play Games on PC video — the slot is a YouTube-URL field, not a file upload; no video file can be produced for it.

## Production notes

- **Build provenance (all captures):** every screenshot was captured from the large-screen-fixed tree — branch `pax/large-screen-api36-20261013` @ commit `9c21ca1` (local commit, not pushed) — because that is the build in which the large-screen defects (portrait orientation lock, companion-picker overflow in short landscape, title backdrop not refitting after a size/orientation change) are fixed; the shipped main tree still shows cropped backdrops at these form factors. Android sets: Gradle debug APK (265,765,953 bytes) carrying a release-flavor native library (`liblight_show.so`), installed on both AVDs. Desktop/PGPC sets: originally captured from that worktree's debug binary; **re-captured 2026-10-17** from the dialogue-fix worktree (`~/workspace/repos/light-show-dialoguefix-wt`) @ commit `b0cd4fb` — the same `9c21ca1` tree plus the WebP-decoder fix (see the correction note below) — debug binary (`target/debug/light-show`) built from that worktree.
- **Desktop asset install was stale; synced before capture.** The desktop build reads its assets from `~/.local/share/light-show/assets` (XDG data dir). That install predated the landscape title art: `sprites/ui/title_artwork_landscape.png` was missing, so the title backdrop fell back to the portrait artwork and rendered visibly zoomed/cropped at 1920x1080 (exactly two asset errors in the run log — the landscape art and `art/seraphine/companions/seraphine_portrait.jpg`). The current assets were synced from the capture worktree with `rsync -a` (additive, no deletions) before the desktop session; the capture run then logged zero asset errors and the backdrop selected the landscape art and fit the full frame.
- **Save data protected.** The desktop profile's real `save.json` was backed up before the desktop session and restored byte-identical afterward (md5 b6b0c6534bc15ce51b6b4be491d338ef, both copies); the erase-save menu item was never used. The tablet sets ran on disposable emulator profiles.
- **Audio note.** The first desktop capture instance briefly played game audio through primo's speakers (Xvfb virtualizes the display only, not sound). It was killed, and all later desktop instances were launched with `SDL_AUDIODRIVER=dummy` plus a null ALSA default device, with `pactl` verified to show no light-show sink input before capturing. Emulator captures ran with `-no-audio` throughout.
- **Desktop dialogue-portrait defect — corrected 2026-10-17.** In the desktop captures as originally produced on 2026-10-13, the portrait slot inside the in-game dialogue box rendered empty on every desktop dialogue frame (including well after the line finished typing), while the same companion portrait rendered correctly in the desktop level-select header and the slot was populated in the Android captures. Root cause: the desktop build's Bevy image features enabled `png`/`jpeg` but not `webp`, so every `.webp` face under `game/assets/art/*/faces/` failed to load on desktop (the level-select header uses a `.jpg` portrait, and Android finds no on-device asset root and falls back to `.jpg`, which is why only desktop dialogue frames were affected). Fixed in commit `b0cd4fb` (webp feature added to the desktop and Android feature lists, plus a regression test decoding all 330 shipped faces). All eight desktop/PGPC screenshots were re-captured from the fixed build on 2026-10-17: the dialogue frames (05, 06, 07) now show the companion's face in the portrait slot, and the Warehouse host pill (08) renders its portrait as before. The re-captured Warehouse frame shows a different one of the shipped warehouse backdrop variants than the Oct 13 frame. The Android/tablet captures were not affected and were not re-shot.
- **Board framing.** In landscape gameplay the board camera centers the circuit panel with flat gray bands at the sides; that is the game's own rendered framing and appears in the World 1 / World 2 landscape shots (tablet 10-inch and desktop/PGPC sets).
- **Teardown.** Both emulators and the Xvfb display used for capture were shut down at the end of the session; no capture processes were left running. This staging folder is untracked in the repo (`git status` shows `?? listing-assets-extra/`) — nothing was added, committed, pushed, or uploaded.
