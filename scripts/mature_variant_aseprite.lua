-- mature_variant_aseprite.lua
--
-- Aseprite-native "mature appearance" variant generator for the light-show
-- full-body companion sheets. Replicates scripts/gen_mature_variant.py
-- (PIL/numpy) using only the Aseprite Lua API plus hand-rolled resampling.
--
-- Why hand-rolled resampling: on Aseprite 1.3.18.6-dev, Image:resize()
-- performs nearest-neighbor resampling no matter which method string is
-- passed ("bilinear", "rotsprite", even "bogus" all return identical
-- pixels; probed 2026-09-30). Nearest-neighbor would shatter the smooth
-- head/body proportions, so this script implements separable bicubic
-- (body, cf. PIL BICUBIC) and Lanczos-3 (head, cf. PIL LANCZOS) itself,
-- plus the jaw-squeeze remap and unsharp mask from the Python pipeline.
--
-- Parity notes (honest limits, see the trial report for measurements):
--   * PIL quantizes its resample filter tables and computes in float32;
--     this script uses float64 textbook kernels with edge clamping.
--   * Like PIL, the filter support widens by the downscale factor so the
--     head scale-down (e.g. seraphine 89 -> 49 at 0.55x) anti-aliases
--     instead of shimmering.
--   * The jaw squeeze truncates toward zero like numpy astype(uint8);
--     resample stages round half up like PIL's clip.
--   * The unsharp mask uses a true Gaussian (sigma = radius); PIL's
--     kernel discretization differs in the last bit.
--
-- Headless:
--   aseprite -b --script-param character=seraphine \
--            --script-param input=seraphine_sheet_fullbody.aseprite \
--            --script-param out_aseprite=seraphine_sheet_fullbody_mature.aseprite \
--            --script-param out_png=seraphine_sheet_fullbody_mature.png \
--            --script mature_variant_aseprite.lua
--
-- Interactive: run from File > Scripts. A dialog collects the four params;
-- the mature sprite is left open for inspection and the files are written.
--
-- Contract:
--   Input:  24-frame 96x192 RGBA .aseprite, one cel per frame, as built by
--           scripts/build_fullbody_aseprite.lua. The input is never modified.
--   Output: mature .aseprite (24 frames, 96x192, 180 ms/frame,
--           6 mood tags) and a 384x1152 RGBA mature PNG sheet.
--   (Aseprite's RGB color mode is 32-bit: the alpha channel rides along;
--   the old script simply ignored it.)
--   Alpha handling: pixel arrays carry premultiplied straight-from-Aseprite
--   RGBA (Aseprite stores straight alpha; the script premultiplies on read
--   so transparent pixels contribute nothing to the resample filters, then
--   un-premultiplies on write). All filters run identically on all four
--   channels, so fully-opaque regions produce bit-identical RGB to the old
--   RGB-only pipeline.
--   Prints "OK <out_aseprite>" / "OK <out_png>" on success. Any failure
--   raises a Lua error (non-zero exit in batch mode).

local CELL_W_PX = 96
local CELL_H_PX = 192
local COLS = 4
local ROWS = 6
local FRAME_COUNT = COLS * ROWS -- 24
local FRAME_DURATION_S = 0.18 -- 180 ms

local MOOD_BAR_H_PX = 6
-- Uniform head scale-down (reworked 2026-10-01): the old vertical squash
-- (96x89 -> 90x28) flattened eyes into slits. A uniform HEAD_SCALE keeps
-- facial proportions intact while the smaller head still reads adult.
-- The head height is derived per character as round(head_bottom_px *
-- HEAD_SCALE), e.g. seraphine 89 -> 49; the width as round(96 * HEAD_SCALE).
local HEAD_SCALE = 0.55
-- Head/body overlap (2026-10-01): the narrow uniform-scale head would
-- otherwise sit on the full-width shoulders with a visible seam. Overlapping
-- the head down over the body's top rows tucks the chin into the neckline.
local HEAD_OVERLAP_PX = 10
-- Vertical feather (2026-10-01): the head crop's bottom edge is a straight
-- cut; fading the head's bottom rows into the overlapped body hides it.
local HEAD_FEATHER_PX = 6
local JAW_BAND_FRAC = 0.40
local JAW_SQUEEZE = 0.88
local FACE_CX_PX = 48

local UNSHARP_RADIUS = 1.2
local UNSHARP_PERCENT = 45
local UNSHARP_THRESHOLD = 2

-- Bottom of the head region per character, in cell pixels. MUST sit below
-- the chin/mouth so the whole face compresses as one unit (same table as
-- the Python pipeline; a boundary through the mouth distorts it).
local HEAD_BOTTOM_PX = {
    lattice = 76,
    linka = 88,
    ondine = 76,
    seraphine = 89,
}

local MOOD_TAGS = { "idle", "blush", "wink", "pout", "celebrate", "alarmed" }

local BICUBIC_SUPPORT = 2
local LANCZOS_SUPPORT = 3
local GAUSS_HALF_WIDTH = 4 -- covers +/-3 sigma at sigma = 1.2

---@class MatureConfig
---@field character string
---@field head_bottom_px integer
---@field input string
---@field out_aseprite string
---@field out_png string

---@class ResampleGeom
---@field body_src_h_px integer
---@field body_y_px integer
---@field body_h_px integer
---@field body_wrows table
---@field head_w_px integer
---@field head_h_px integer
---@field head_x_px integer
---@field head_wcols table
---@field head_wrows table

-- ---------------------------------------------------------------------------
-- Param validation (external input: assert loudly, batch contract)
-- ---------------------------------------------------------------------------

---@param params table app.params in batch mode, dialog data in GUI mode
---@return MatureConfig
local function validate_params(params)
    local character = params.character
    assert(type(character) == "string" and HEAD_BOTTOM_PX[character] ~= nil,
        "param 'character' must be one of: lattice, linka, ondine, seraphine")
    local input = params.input
    assert(type(input) == "string" and input ~= "",
        "param 'input' must be a non-empty path")
    local out_aseprite = params.out_aseprite
    assert(type(out_aseprite) == "string" and out_aseprite ~= "",
        "param 'out_aseprite' must be a non-empty path")
    local out_png = params.out_png
    assert(type(out_png) == "string" and out_png ~= "",
        "param 'out_png' must be a non-empty path")
    return {
        character = character,
        head_bottom_px = HEAD_BOTTOM_PX[character],
        input = input,
        out_aseprite = out_aseprite,
        out_png = out_png,
    }
end

---@param spr Sprite opened input sprite
local function validate_input_sprite(spr)
    assert(spr ~= nil, "cannot open input sprite")
    assert(spr.width == CELL_W_PX and spr.height == CELL_H_PX,
        string.format("input must be %dx%d, got %dx%d",
            CELL_W_PX, CELL_H_PX, spr.width, spr.height))
    assert(spr.colorMode == ColorMode.RGB, "input must be RGB")
    assert(#spr.frames == FRAME_COUNT, "input must have 24 frames")
    assert(#spr.layers >= 1, "input must have at least one layer")
end

-- ---------------------------------------------------------------------------
-- Pixel array helpers (flat {r,g,b,a} quadruples, row-major; rgb stored
-- premultiplied by alpha so filtering never bleeds hidden backdrop colors
-- from fully-transparent pixels into visible edges)
-- ---------------------------------------------------------------------------

---@param img Image
---@return table flat premultiplied-rgba array
local function read_rgba(img)
    local px = app.pixelColor
    local out = {}
    local n = 0
    for it in img:pixels() do
        local p = it()
        local a = px.rgbaA(p)
        local k = a / 255
        out[n + 1] = px.rgbaR(p) * k
        out[n + 2] = px.rgbaG(p) * k
        out[n + 3] = px.rgbaB(p) * k
        out[n + 4] = a
        n = n + 4
    end
    assert(n == img.width * img.height * 4, "pixel iterator count mismatch")
    return out
end

---@param img Image destination, already the right size
---@param arr table flat premultiplied-rgba array
local function write_rgba(img, arr)
    local px = app.pixelColor
    local w = img.width
    local n = 0
    for y = 0, img.height - 1 do
        for x = 0, w - 1 do
            local a = arr[n + 4]
            local r, g, b = 0, 0, 0
            if a > 0 then
                -- Un-premultiply; round half up like the resample stages.
                r = math.floor(math.min(255, arr[n + 1] * 255 / a) + 0.5)
                g = math.floor(math.min(255, arr[n + 2] * 255 / a) + 0.5)
                b = math.floor(math.min(255, arr[n + 3] * 255 / a) + 0.5)
            end
            a = math.floor(math.min(255, math.max(0, a)) + 0.5)
            img:drawPixel(x, y, px.rgba(r, g, b, a))
            n = n + 4
        end
    end
end

---@param src table flat rgba array, sw/sh dims
---@return table cropped flat rgba array
local function crop_array(src, sw, x, y, w, h)
    local out = {}
    local n = 0
    for row = 0, h - 1 do
        local s4 = ((y + row) * sw + x) * 4
        for col = 0, w - 1 do
            out[n + 1] = src[s4 + 1]
            out[n + 2] = src[s4 + 2]
            out[n + 3] = src[s4 + 3]
            out[n + 4] = src[s4 + 4]
            n = n + 4
            s4 = s4 + 4
        end
    end
    return out
end

---@param dst table flat rgba array being composed
---@param src table flat rgba array to paste
local function blit(dst, dst_w, src, x, y, w, h)
    for row = 0, h - 1 do
        local d4 = ((y + row) * dst_w + x) * 4
        local s4 = row * w * 4
        for col = 0, w - 1 do
            dst[d4 + 1] = src[s4 + 1]
            dst[d4 + 2] = src[s4 + 2]
            dst[d4 + 3] = src[s4 + 3]
            dst[d4 + 4] = src[s4 + 4]
            d4 = d4 + 4
            s4 = s4 + 4
        end
    end
end

-- src-over-dst composite for premultiplied RGBA (the head over the body).
---@param dst table flat rgba array being composed
---@param src table flat rgba array to composite
local function blit_over(dst, dst_w, src, x, y, w, h)
    for row = 0, h - 1 do
        local d4 = ((y + row) * dst_w + x) * 4
        local s4 = row * w * 4
        for col = 0, w - 1 do
            local sa = src[s4 + 4] / 255
            local da = 1 - sa
            dst[d4 + 1] = src[s4 + 1] + dst[d4 + 1] * da
            dst[d4 + 2] = src[s4 + 2] + dst[d4 + 2] * da
            dst[d4 + 3] = src[s4 + 3] + dst[d4 + 3] * da
            dst[d4 + 4] = src[s4 + 4] + dst[d4 + 4] * da
            d4 = d4 + 4
            s4 = s4 + 4
        end
    end
end

-- Fade the bottom rows of a premultiplied head to transparent so the
-- head/body crop seam melts into the overlapped body instead of cutting.
---@param head table flat rgba, modified in place
local function feather_bottom(head, w, h, feather_px)
    for y = h - feather_px, h - 1 do
        local t = (h - 1 - y) / feather_px
        for x = 0, w - 1 do
            local i4 = (y * w + x) * 4
            for c = 1, 4 do
                head[i4 + c] = head[i4 + c] * t
            end
        end
    end
end

-- ---------------------------------------------------------------------------
-- Resampling kernels (textbook; PIL uses quantized tables, hence last-bit
-- differences documented in the parity notes)
-- ---------------------------------------------------------------------------

---@param x number distance from tap center
---@return number Keys bicubic weight, a = -0.5
local function cubic_kernel(x)
    local ax = math.abs(x)
    if ax <= 1 then
        return 1.5 * ax * ax * ax - 2.5 * ax * ax + 1
    elseif ax < 2 then
        return -0.5 * ax * ax * ax + 2.5 * ax * ax - 4 * ax + 2
    end
    return 0
end

---@param x number
---@return number sinc(x)
local function sinc(x)
    if x == 0 then
        return 1
    end
    local pix = math.pi * x
    return math.sin(pix) / pix
end

---@param x number distance from tap center
---@return number Lanczos-3 weight
local function lanczos3_kernel(x)
    if math.abs(x) < 3 then
        return sinc(x) * sinc(x / 3)
    end
    return 0
end

---@param in_n integer input samples
---@param out_n integer output samples
---@param kernel function weight kernel
---@param support integer kernel support radius at unit scale
---@return table per output index, array of {src=integer, w=number}
local function resample_weights(in_n, out_n, kernel, support)
    local scale = in_n / out_n
    -- Widen the filter when downscaling (scale > 1), the way PIL does:
    -- the kernel is stretched over support*scale input samples so high
    -- frequencies are attenuated before decimation instead of aliasing.
    -- A fixed-support filter on a 3x downscale (head 89 -> 28) shimmers.
    local eff_support = support
    local stretch = 1
    if scale > 1 then
        eff_support = support * scale
        stretch = scale
    end
    local rows = {}
    for o = 0, out_n - 1 do
        local center = (o + 0.5) * scale - 0.5
        local taps = {}
        local sum = 0
        local lo = math.max(0, math.ceil(center - eff_support))
        local hi = math.min(in_n - 1, math.floor(center + eff_support))
        for s = lo, hi do
            local w = kernel((center - s) / stretch)
            if w ~= 0 then
                taps[#taps + 1] = { src = s, w = w }
                sum = sum + w
            end
        end
        assert(#taps > 0, "empty resample tap list")
        for _, tap in ipairs(taps) do
            tap.w = tap.w / sum
        end
        rows[o + 1] = taps
    end
    return rows
end

---@param src table flat rgba, sw/sh dims
---@param dw integer target width
---@param dh integer target height
---@param wcols table|nil precomputed horizontal weights (nil = identity ok)
---@param wrows table|nil precomputed vertical weights (nil = identity ok)
---@return table flat rgba dw*dh
local function resize_separable(src, sw, sh, dw, dh, wcols, wrows)
    -- Horizontal pass; skipped when the width is unchanged.
    local mid = src
    local mw = sw
    if sw ~= dw then
        assert(wcols ~= nil, "missing horizontal weights")
        mid = {}
        mw = dw
        for y = 0, sh - 1 do
            local base = y * sw * 4
            for x = 0, dw - 1 do
                local acc = { 0, 0, 0, 0 }
                for _, tap in ipairs(wcols[x + 1]) do
                    local s4 = base + tap.src * 4
                    for c = 1, 4 do
                        acc[c] = acc[c] + src[s4 + c] * tap.w
                    end
                end
                local d4 = (y * dw + x) * 4
                for c = 1, 4 do
                    mid[d4 + c] = acc[c]
                end
            end
        end
    end
    -- Vertical pass with round-half-up quantization (matches PIL resample).
    assert(wrows ~= nil or sh == dh, "missing vertical weights")
    local dst = {}
    for y = 0, dh - 1 do
        local taps = wrows[y + 1]
        for x = 0, mw - 1 do
            local acc = { 0, 0, 0, 0 }
            for _, tap in ipairs(taps) do
                local s4 = (tap.src * mw + x) * 4
                for c = 1, 4 do
                    acc[c] = acc[c] + mid[s4 + c] * tap.w
                end
            end
            local d4 = (y * mw + x) * 4
            for c = 1, 4 do
                dst[d4 + c] = math.floor(math.min(255, math.max(0, acc[c])) + 0.5)
            end
        end
    end
    return dst
end

-- ---------------------------------------------------------------------------
-- Jaw squeeze: horizontal bilinear remap around the face center, ramped
-- from 1.0 at the band top to JAW_SQUEEZE at the chin. Mirrors the Python
-- pipeline exactly, including truncation toward zero (numpy astype(uint8)).
-- ---------------------------------------------------------------------------

---@param head table flat rgba, w/h dims
---@return table squeezed flat rgba
local function jaw_squeeze(head, w, h, cx)
    local jaw_start = math.floor(h * (1 - JAW_BAND_FRAC))
    local out = {}
    local n = w * h * 4
    for i = 1, n do
        out[i] = head[i]
    end
    local denom = math.max(1, h - 1 - jaw_start)
    for y = jaw_start, h - 1 do
        local t = (y - jaw_start) / denom
        local s = 1 - (1 - JAW_SQUEEZE) * t
        for x = 0, w - 1 do
            local src_x = cx + (x - cx) / s
            src_x = math.min(w - 1, math.max(0, src_x))
            local x0 = math.floor(src_x)
            local x1 = math.min(x0 + 1, w - 1)
            local f = src_x - x0
            local d4 = (y * w + x) * 4
            local a4 = (y * w + x0) * 4
            local b4 = (y * w + x1) * 4
            for c = 1, 4 do
                local v = head[a4 + c] * (1 - f) + head[b4 + c] * f
                v = math.min(255, math.max(0, v))
                out[d4 + c] = math.floor(v) -- truncate like astype(uint8)
            end
        end
    end
    return out
end

-- ---------------------------------------------------------------------------
-- Unsharp mask: separable Gaussian blur (sigma = radius), then
-- out = orig + (orig - blur) * percent/100 where |orig - blur| >= threshold.
-- Operates in place on the composed cell, exactly like the Python pipeline.
-- ---------------------------------------------------------------------------

---@return table array of {off=integer, w=number}, normalized
local function gaussian_weights(sigma, half)
    local taps = {}
    local sum = 0
    for i = -half, half do
        local w = math.exp(-(i * i) / (2 * sigma * sigma))
        taps[#taps + 1] = { off = i, w = w }
        sum = sum + w
    end
    for _, tap in ipairs(taps) do
        tap.w = tap.w / sum
    end
    return taps
end

---@param cell table flat rgba, modified in place
local function unsharp_mask(cell, w, h, radius, percent, threshold)
    local taps = gaussian_weights(radius, GAUSS_HALF_WIDTH)
    local strength = percent / 100
    -- Horizontal blur into tmp.
    local tmp = {}
    for y = 0, h - 1 do
        for x = 0, w - 1 do
            local acc = { 0, 0, 0, 0 }
            for _, tap in ipairs(taps) do
                local sx = math.min(w - 1, math.max(0, x + tap.off))
                local s4 = (y * w + sx) * 4
                for c = 1, 4 do
                    acc[c] = acc[c] + cell[s4 + c] * tap.w
                end
            end
            local d4 = (y * w + x) * 4
            for c = 1, 4 do
                tmp[d4 + c] = acc[c]
            end
        end
    end
    -- Vertical blur + sharpen, in place.
    for y = 0, h - 1 do
        for x = 0, w - 1 do
            local d4 = (y * w + x) * 4
            for c = 1, 4 do
                local bl = 0
                for _, tap in ipairs(taps) do
                    local sy = math.min(h - 1, math.max(0, y + tap.off))
                    bl = bl + tmp[(sy * w + x) * 4 + c] * tap.w
                end
                local orig = cell[d4 + c]
                local diff = orig - bl
                local v = orig
                if math.abs(diff) >= threshold then
                    v = orig + diff * strength
                end
                cell[d4 + c] = math.floor(math.min(255, math.max(0, v)) + 0.5)
            end
        end
    end
end

-- ---------------------------------------------------------------------------
-- Per-character geometry, precomputed once (identical for all 24 frames)
-- ---------------------------------------------------------------------------

---@param head_bottom_px integer
---@return ResampleGeom
local function precompute_geometry(head_bottom_px)
    -- Uniform HEAD_SCALE head, centered: round half up so 96 * 0.55 -> 53
    -- (x offset 21) and e.g. seraphine 89 * 0.55 -> 49.
    local head_w = math.floor(CELL_W_PX * HEAD_SCALE + 0.5)
    local head_h = math.floor(head_bottom_px * HEAD_SCALE + 0.5)
    local body_src_h = CELL_H_PX - MOOD_BAR_H_PX - head_bottom_px
    -- The body starts HEAD_OVERLAP_PX above the head's bottom so the chin
    -- tucks into the neckline instead of seaming against the shoulders.
    local body_y = head_h - HEAD_OVERLAP_PX
    local body_h = CELL_H_PX - MOOD_BAR_H_PX - body_y
    return {
        body_src_h_px = body_src_h,
        body_y_px = body_y,
        body_h_px = body_h,
        body_wrows = resample_weights(body_src_h, body_h, cubic_kernel,
            BICUBIC_SUPPORT),
        head_w_px = head_w,
        head_h_px = head_h,
        head_x_px = math.floor((CELL_W_PX - head_w) / 2),
        head_wcols = resample_weights(CELL_W_PX, head_w, lanczos3_kernel,
            LANCZOS_SUPPORT),
        head_wrows = resample_weights(head_bottom_px, head_h,
            lanczos3_kernel, LANCZOS_SUPPORT),
    }
end

-- ---------------------------------------------------------------------------
-- One cell: body stretch, head squeeze+shrink, verbatim mood bar, sharpen
-- ---------------------------------------------------------------------------

---@param src_img Image 96x192 source cel image
---@param head_bottom_px integer
---@param geom ResampleGeom
---@return Image 96x192 mature cel image
local function mature_cell(src_img, head_bottom_px, geom)
    local src = read_rgba(src_img)
    local out = {}
    for i = 1, CELL_W_PX * CELL_H_PX * 4 do
        out[i] = 0
    end
    -- 1. Body: stretch from below the head to above the mood bar.
    local body = crop_array(src, CELL_W_PX, 0, head_bottom_px, CELL_W_PX,
        geom.body_src_h_px)
    local body_rs = resize_separable(body, CELL_W_PX, geom.body_src_h_px,
        CELL_W_PX, geom.body_h_px, nil, geom.body_wrows)
    blit(out, CELL_W_PX, body_rs, 0, geom.body_y_px, CELL_W_PX, geom.body_h_px)
    -- 2. Head: jaw-squeeze, shrink to the adult proportion, feather the
    -- crop seam, then composite over the overlapped body.
    local head = crop_array(src, CELL_W_PX, 0, 0, CELL_W_PX, head_bottom_px)
    head = jaw_squeeze(head, CELL_W_PX, head_bottom_px, FACE_CX_PX)
    local head_rs = resize_separable(head, CELL_W_PX, head_bottom_px,
        geom.head_w_px, geom.head_h_px, geom.head_wcols, geom.head_wrows)
    feather_bottom(head_rs, geom.head_w_px, geom.head_h_px, HEAD_FEATHER_PX)
    blit_over(out, CELL_W_PX, head_rs, geom.head_x_px, 0, geom.head_w_px,
        geom.head_h_px)
    -- 3. Mood bar: verbatim.
    local bar = crop_array(src, CELL_W_PX, 0, CELL_H_PX - MOOD_BAR_H_PX,
        CELL_W_PX, MOOD_BAR_H_PX)
    blit(out, CELL_W_PX, bar, 0, CELL_H_PX - MOOD_BAR_H_PX, CELL_W_PX,
        MOOD_BAR_H_PX)
    -- 4. Restore pixel-art crispness lost to resampling.
    unsharp_mask(out, CELL_W_PX, CELL_H_PX, UNSHARP_RADIUS, UNSHARP_PERCENT,
        UNSHARP_THRESHOLD)
    local img = Image(CELL_W_PX, CELL_H_PX, ColorMode.RGB)
    write_rgba(img, out)
    return img
end

-- ---------------------------------------------------------------------------
-- Sprite assembly and sheet export
-- ---------------------------------------------------------------------------

---@param src_spr Sprite validated 24-frame input
---@param cfg MatureConfig
---@param geom ResampleGeom
---@return Sprite mature sprite (caller owns it)
local function build_mature_sprite(src_spr, cfg, geom)
    local src_layer = src_spr.layers[1]
    local dst = Sprite(CELL_W_PX, CELL_H_PX, ColorMode.RGB)
    assert(dst ~= nil, "failed to create destination sprite")
    for _ = 2, FRAME_COUNT do
        dst:newFrame()
    end
    assert(#dst.frames == FRAME_COUNT, "frame count mismatch")
    for _, frame in ipairs(dst.frames) do
        frame.duration = FRAME_DURATION_S
    end
    local dst_layer = dst.layers[1]
    app.transaction("mature variant transform", function()
        for f = 1, FRAME_COUNT do
            local cel = src_layer:cel(f)
            assert(cel ~= nil, "input missing cel for frame " .. f)
            local img = cel.image
            assert(img.width == CELL_W_PX and img.height == CELL_H_PX,
                "input cel has wrong size")
            dst:newCel(dst_layer, f, mature_cell(img, cfg.head_bottom_px, geom))
            if f % 6 == 0 then
                print(string.format("frame %d/%d", f, FRAME_COUNT))
            end
        end
    end)
    for row = 0, ROWS - 1 do
        -- newTag's name arg is silently ignored on 1.3.18.6-dev; assign it.
        local tag = dst:newTag(row * COLS + 1, (row + 1) * COLS,
            MOOD_TAGS[row + 1])
        assert(tag ~= nil, "failed to create tag " .. MOOD_TAGS[row + 1])
        tag.name = MOOD_TAGS[row + 1]
    end
    return dst
end

---@param mature_spr Sprite 24-frame mature sprite
---@param out_png string destination path for the 384x1152 sheet
local function export_sheet_png(mature_spr, out_png)
    local sheet_w = COLS * CELL_W_PX
    local sheet_h = ROWS * CELL_H_PX
    local sheet_img = Image(sheet_w, sheet_h, ColorMode.RGB)
    local layer = mature_spr.layers[1]
    for f = 1, FRAME_COUNT do
        local cel = layer:cel(f)
        assert(cel ~= nil, "mature sprite missing cel " .. f)
        local col = (f - 1) % COLS
        local row = math.floor((f - 1) / COLS)
        sheet_img:drawImage(cel.image, col * CELL_W_PX, row * CELL_H_PX)
    end
    local sheet = Sprite(sheet_w, sheet_h, ColorMode.RGB)
    sheet:newCel(sheet.layers[1], 1, sheet_img)
    sheet:saveCopyAs(out_png)
    sheet:close()
end

-- ---------------------------------------------------------------------------
-- Entry points
-- ---------------------------------------------------------------------------

local function run_batch()
    local cfg = validate_params(app.params)
    local geom = precompute_geometry(cfg.head_bottom_px)
    local src_spr = app.open(cfg.input)
    validate_input_sprite(src_spr)
    local mature_spr = build_mature_sprite(src_spr, cfg, geom)
    mature_spr:saveCopyAs(cfg.out_aseprite)
    export_sheet_png(mature_spr, cfg.out_png)
    src_spr:close()
    mature_spr:close()
    print("OK " .. cfg.out_aseprite)
    print("OK " .. cfg.out_png)
end

local function run_interactive()
    local dlg = Dialog({ title = "Mature variant (Aseprite-native)" })
    dlg:combobox({
        id = "character",
        label = "Character:",
        options = { "lattice", "linka", "ondine", "seraphine" },
        option = "seraphine",
    })
    dlg:file({ id = "input", label = "Standard .aseprite:", open = true })
    dlg:file({
        id = "out_aseprite",
        label = "Mature .aseprite:",
        save = true,
        filename = "seraphine_sheet_fullbody_mature.aseprite",
    })
    dlg:file({
        id = "out_png",
        label = "Mature PNG sheet:",
        save = true,
        filename = "seraphine_sheet_fullbody_mature.png",
    })
    dlg:button({ id = "ok", text = "Generate" })
    dlg:button({ id = "cancel", text = "Cancel" })
    dlg:show()
    local data = dlg.data
    if not data.ok then
        print("cancelled")
        return
    end
    local cfg = validate_params(data)
    local geom = precompute_geometry(cfg.head_bottom_px)
    local src_spr = app.open(cfg.input)
    validate_input_sprite(src_spr)
    local mature_spr = build_mature_sprite(src_spr, cfg, geom)
    mature_spr:saveCopyAs(cfg.out_aseprite)
    export_sheet_png(mature_spr, cfg.out_png)
    src_spr:close()
    -- In GUI mode the result stays open for inspection.
    print("OK " .. cfg.out_aseprite)
    print("OK " .. cfg.out_png)
end

local function main()
    assert(app.apiVersion ~= nil,
        "Aseprite scripting API unavailable (app.apiVersion is nil)")
    if app.isUIAvailable then
        run_interactive()
    else
        run_batch()
    end
end

main()
