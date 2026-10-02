-- build_fullbody_aseprite_hires.lua
--
-- Builds a tagged .aseprite project from a 1152x3456 full-body sprite sheet (hires)
-- (4 columns x 6 rows of 288x576 cells). Run headless:
--
--   aseprite -b --script-param input=<sheet.png> \
--            --script-param output=<name>.aseprite \
--            --script build_fullbody_aseprite.lua
--
-- Contract:
--   Input:  app.params.input  (path to 1152x3456 RGBA PNG sheet)
--           app.params.output (path for the .aseprite to write)
--   Output: 24-frame sprite, 288x576, 180 ms per frame, 6 mood tags
--           (idle, blush, wink, pout, celebrate, alarmed), saved via
--           saveCopyAs. Prints "OK <output>" on success; any failure
--           raises a Lua error (non-zero exit).
--   State:  closes both sprites; leaves no open documents.

local FRAME_W_PX = 288
local FRAME_H_PX = 576
local COLS = 4
local ROWS = 6
local FRAME_COUNT = 24
local FRAME_DURATION_S = 0.18
local SHEET_W_PX = FRAME_W_PX * COLS -- 1152
local SHEET_H_PX = FRAME_H_PX * ROWS -- 3456

local MOOD_TAGS = { "idle", "blush", "wink", "pout", "celebrate", "alarmed" }
assert(#MOOD_TAGS == ROWS, "tag count must match row count")

---@param params table app.params table from the CLI
---@return string input_path, string output_path
local function validate_params(params)
    local input_path = params.input
    local output_path = params.output
    assert(type(input_path) == "string" and input_path ~= "",
        "missing or empty param: input")
    assert(type(output_path) == "string" and output_path ~= "",
        "missing or empty param: output")
    return input_path, output_path
end

---@param spr Sprite the opened sheet sprite
local function validate_sheet(spr)
    assert(spr ~= nil, "cannot open input sheet")
    assert(spr.width == SHEET_W_PX and spr.height == SHEET_H_PX,
        string.format("sheet must be %dx%d, got %dx%d",
            SHEET_W_PX, SHEET_H_PX, spr.width, spr.height))
    assert(spr.colorMode == ColorMode.RGB,
        "sheet must be RGB, got " .. tostring(spr.colorMode))
    assert(#spr.cels == 1, "sheet must hold a single cel")
end

local function main()
    local input_path, output_path = validate_params(app.params)

    local src = app.open(input_path)
    validate_sheet(src)
    local src_image = src.cels[1].image

    local dst = Sprite(FRAME_W_PX, FRAME_H_PX, ColorMode.RGB)
    assert(dst ~= nil, "failed to create destination sprite")

    for _ = 2, FRAME_COUNT do
        dst:newFrame()
    end
    assert(#dst.frames == FRAME_COUNT, "frame count mismatch")

    for _, frame in ipairs(dst.frames) do
        frame.duration = FRAME_DURATION_S
    end

    local layer = dst.layers[1]
    assert(layer ~= nil, "destination has no layer")

    app.transaction("import full-body sheet cells", function()
        for row = 0, ROWS - 1 do
            for col = 0, COLS - 1 do
                local cell_index = row * COLS + col -- 0-based
                local frame_number = cell_index + 1 -- 1-based
                local crop_rect = Rectangle(
                    col * FRAME_W_PX, row * FRAME_H_PX,
                    FRAME_W_PX, FRAME_H_PX)
                local cell_image = Image(src_image, crop_rect)
                dst:newCel(layer, frame_number, cell_image)
            end
        end
    end)

    for row = 0, ROWS - 1 do
        local tag = dst:newTag(row * COLS + 1, (row + 1) * COLS, MOOD_TAGS[row + 1])
        assert(tag ~= nil, "failed to create tag " .. MOOD_TAGS[row + 1])
    end

    dst:saveCopyAs(output_path)
    dst:close()
    src:close()
    print("OK " .. output_path)
end

main()
