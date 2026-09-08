-- #################################################################
-- light-show/art/aseprite/import_sheet.lua
-- Headless Aseprite importer: turns one of this game's flat PNG sprite
-- sheets back into an editable .aseprite document with real frames and
-- named tags, so it can be opened, edited, and re-exported through the
-- Diver Neovim Aseprite actions (utils.games.aseprite.actions) —
-- specifically `export_sprite_sheet` / `export_sheet_type_preset`,
-- which both batch-export via `aseprite -b <sprite> --sheet out.png
-- --data out.json --format json-array`.
--
-- Run with real Aseprite (this sandbox has none installed, so this
-- script has not been executed here — only written and reviewed):
--
--   aseprite -b --script art/aseprite/import_sheet.lua -- \
--     art/aseprite/seraphine_sheet.png 64 64 \
--     Idle Blush Wink Pout Celebrate Alarmed
--
-- Positional script params (after `--`): <sheet.png> <frame_w> <frame_h>
-- <tag_name...> — one tag per row, top to bottom. Companion sheets are
-- 256x384 (4 cols x 6 rows @ 64x64) per docs/ART_STYLE.md's mood-row
-- contract, so pass all 6 mood names in that exact order. Component
-- icons are single-frame 64x64 PNGs with no rows/tags to slice — open
-- those directly in Aseprite instead of running this script on them.
--
-- Output: `<sheet>.aseprite` next to the source PNG, with one tag per
-- row (each tag spanning that row's 4 frames) and a 180ms default frame
-- duration (matches the `Timer::from_seconds(0.18, ...)` animation rate
-- in `game/src/waifu/mod.rs`, so preview playback timing in Aseprite
-- already matches in-game timing).
-- SPDX-License-Identifier: GPL-3.0-or-later
-- #################################################################

local params = app.params
local sheet_path = params["1"]
local frame_w = tonumber(params["2"])
local frame_h = tonumber(params["3"])

assert(sheet_path, "usage: -- <sheet.png> <frame_w> <frame_h> <tag_name...>")
assert(frame_w and frame_h, "frame_w/frame_h must be numbers")

local tags = {}
local i = 4
while params[tostring(i)] do
  tags[#tags + 1] = params[tostring(i)]
  i = i + 1
end
assert(#tags > 0, "pass at least one row tag name")

local source = Image { fromFile = sheet_path }
assert(source, "could not load " .. sheet_path)

local cols = math.floor(source.width / frame_w)
local rows = math.floor(source.height / frame_h)
assert(rows == #tags, ("sheet has %d rows but %d tags were given"):format(rows, #tags))

local sprite = Sprite(frame_w, frame_h, ColorMode.RGB)
sprite.filename = sheet_path:gsub("%.png$", ".aseprite")
local layer = sprite.layers[1]
layer.name = "sprite"

-- Sprite() starts with exactly one frame; add the rest up front so
-- tags can be assigned to a stable, fully-populated frame range.
for _ = 2, (cols * rows) do
  sprite:newEmptyFrame()
end

for row = 0, rows - 1 do
  for col = 0, cols - 1 do
    local frame_index = row * cols + col + 1
    local cel_image = Image(frame_w, frame_h, ColorMode.RGB)
    cel_image:drawImage(source, Point(-col * frame_w, -row * frame_h))
    sprite:newCel(layer, frame_index, cel_image)
    sprite.frames[frame_index].duration = 0.18
  end
end

for row, tag_name in ipairs(tags) do
  local from_frame = (row - 1) * cols + 1
  local to_frame = row * cols
  local tag = sprite:newTag(from_frame, to_frame)
  tag.name = tag_name
end

sprite:saveAs(sprite.filename)
print("wrote " .. sprite.filename .. (" (%dx%d frames, tags: %s)"):format(cols, rows, table.concat(tags, ", ")))
