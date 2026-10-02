-- verify_roundtrip_hires.lua
--
-- Verifies a built .aseprite project round-trips pixel-identical to its
-- source 1152x3456 sheet (hires). Run headless:
--
--   aseprite -b --script-param sheet=<sheet.png> \
--            --script-param project=<name>.aseprite \
--            --script verify_roundtrip.lua
--
-- Prints "IDENTICAL <project>" or raises with the first mismatch.

local FRAME_W_PX, FRAME_H_PX = 288, 576
local COLS, ROWS = 4, 6

local function main()
    local sheet_path = app.params.sheet
    local proj_path = app.params.project
    assert(type(sheet_path) == "string" and sheet_path ~= "", "missing param: sheet")
    assert(type(proj_path) == "string" and proj_path ~= "", "missing param: project")

    local sheet_spr = app.open(sheet_path)
    local proj_spr = app.open(proj_path)
    assert(#proj_spr.frames == COLS * ROWS, "frame count mismatch")
    assert(proj_spr.width == FRAME_W_PX and proj_spr.height == FRAME_H_PX,
        "frame size mismatch")

    local src_img = sheet_spr.cels[1].image
    local layer = proj_spr.layers[1]

    for f = 1, COLS * ROWS do
        local cel = layer:cel(f)
        assert(cel ~= nil, "missing cel for frame " .. f)
        local img = cel.image
        local col = (f - 1) % COLS
        local row = math.floor((f - 1) / COLS)
        for y = 0, FRAME_H_PX - 1 do
            for x = 0, FRAME_W_PX - 1 do
                local a = img:getPixel(x, y)
                local b = src_img:getPixel(col * FRAME_W_PX + x, row * FRAME_H_PX + y)
                if a ~= b then
                    error(string.format(
                        "MISMATCH %s frame %d at (%d,%d): %x vs %x",
                        proj_path, f, x, y, a, b))
            end
        end
    end
    end

    proj_spr:close()
    sheet_spr:close()
    print("IDENTICAL " .. proj_path)
end

main()
