-- Generate FX particle sprites for light-show
-- Aseprite 1.3 batch mode, Tiger Style: explicit, deterministic, no UI
-- Usage: aseprite -b --script gen_fx.lua (output dir via app.params["outdir"])
-- Palette: neon cyan #6ff2ff, gold #ffd166, dim #8e9eb8, error red #ff5577

local outdir = app.params["outdir"]
if outdir == nil or outdir == "" then
  print("FATAL: outdir param required")
  os.exit(1)
end

local function rgba(r, g, b, a)
  return app.pixelColor.rgba(r, g, b, a or 255)
end

local CYAN = rgba(111, 242, 255)
local CYAN_DIM = rgba(111, 242, 255, 128)
local CYAN_FAINT = rgba(111, 242, 255, 64)
local GOLD = rgba(255, 209, 102)
local GOLD_DIM = rgba(255, 209, 102, 128)
local GOLD_FAINT = rgba(255, 209, 102, 64)
local WHITE = rgba(255, 255, 255)
local RED = rgba(255, 85, 119)
local RED_DIM = rgba(255, 85, 119, 128)
local RED_FAINT = rgba(255, 85, 119, 64)

-- Draw a filled circle on image img centered at (cx,cy) radius r with color c
local function circle(img, cx, cy, r, c)
  for y = math.floor(cy - r), math.floor(cy + r) do
    for x = math.floor(cx - r), math.floor(cx + r) do
      local dx, dy = x - cx, y - cy
      if dx*dx + dy*dy <= r*r and x >= 0 and y >= 0 and x < img.width and y < img.height then
        img:drawPixel(x, y, c)
      end
    end
  end
end

-- Draw a ring (hollow circle) with thickness t
local function ring(img, cx, cy, r, t, c)
  for y = math.floor(cy - r - t), math.floor(cy + r + t) do
    for x = math.floor(cx - r - t), math.floor(cx + r + t) do
      local dx, dy = x - cx, y - cy
      local d = math.sqrt(dx*dx + dy*dy)
      if math.abs(d - r) <= t/2 and x >= 0 and y >= 0 and x < img.width and y < img.height then
        img:drawPixel(x, y, c)
      end
    end
  end
end

-- Draw a horizontal line
local function hline(img, x0, x1, y, c)
  for x = x0, x1 do
    if x >= 0 and y >= 0 and x < img.width and y < img.height then
      img:drawPixel(x, y, c)
    end
  end
end

-- Draw a vertical line
local function vline(img, x, y0, y1, c)
  for y = y0, y1 do
    if x >= 0 and y >= 0 and x < img.width and y < img.height then
      img:drawPixel(x, y, c)
    end
  end
end

local function new_sprite(w, h, frames)
  local spr = Sprite(w, h, ColorMode.RGB)
  for _ = 2, frames do spr:newEmptyFrame() end
  return spr
end

local function save(spr, name)
  local path = outdir .. "/" .. name .. ".aseprite"
  spr:saveAs(path)
  spr:close()
  print("wrote " .. path)
end

-- 1. connect_spark: 32x32, 4 frames — spark burst on connection
do
  local spr = new_sprite(32, 32, 4)
  local layer = spr.layers[1]
  local cx, cy = 15.5, 15.5
  -- Frame 1: small bright core
  local img = Image(32, 32, ColorMode.RGB)
  circle(img, cx, cy, 3, WHITE)
  circle(img, cx, cy, 5, CYAN_DIM)
  spr:newCel(layer, 1, img, Point(0, 0))
  -- Frame 2: expanding + spark lines
  img = Image(32, 32, ColorMode.RGB)
  circle(img, cx, cy, 2, WHITE)
  circle(img, cx, cy, 6, CYAN)
  circle(img, cx, cy, 8, CYAN_FAINT)
  hline(img, 4, 27, 15, CYAN_DIM); hline(img, 4, 27, 16, CYAN_DIM)
  vline(img, 15, 4, 27, CYAN_DIM); vline(img, 16, 4, 27, CYAN_DIM)
  spr:newCel(layer, 2, img, Point(0, 0))
  -- Frame 3: larger, fading
  img = Image(32, 32, ColorMode.RGB)
  circle(img, cx, cy, 7, CYAN_DIM)
  circle(img, cx, cy, 10, CYAN_FAINT)
  hline(img, 2, 29, 15, CYAN_FAINT)
  vline(img, 15, 2, 29, CYAN_FAINT)
  spr:newCel(layer, 3, img, Point(0, 0))
  -- Frame 4: faint remnant
  img = Image(32, 32, ColorMode.RGB)
  circle(img, cx, cy, 9, CYAN_FAINT)
  spr:newCel(layer, 4, img, Point(0, 0))
  save(spr, "connect_spark")
end

-- 2. success_burst: 64x64, 6 frames — gold expanding rings
do
  local spr = new_sprite(64, 64, 6)
  local layer = spr.layers[1]
  local cx, cy = 31.5, 31.5
  local radii = {4, 8, 12, 16, 20, 24}
  for f = 1, 6 do
    local img = Image(64, 64, ColorMode.RGB)
    local r = radii[f]
    local alpha_step = 6 - f
    -- Main ring + fading trail rings
    ring(img, cx, cy, r, 2, f <= 3 and GOLD or GOLD_DIM)
    if f > 1 then ring(img, cx, cy, radii[f-1], 1, GOLD_FAINT) end
    if f == 1 then circle(img, cx, cy, 3, WHITE) end
    -- Sparkle points on cardinal directions for frames 2-4
    if f >= 2 and f <= 4 then
      local sr = r + 4
      img:drawPixel(math.floor(cx - sr), math.floor(cy), GOLD)
      img:drawPixel(math.floor(cx + sr), math.floor(cy), GOLD)
      img:drawPixel(math.floor(cx), math.floor(cy - sr), GOLD)
      img:drawPixel(math.floor(cx), math.floor(cy + sr), GOLD)
    end
    spr:newCel(layer, f, img, Point(0, 0))
  end
  save(spr, "success_burst")
end

-- 3. error_flicker: 32x32, 4 frames — red alert flicker
do
  local spr = new_sprite(32, 32, 4)
  local layer = spr.layers[1]
  local cx, cy = 15.5, 15.5
  -- Frames alternate bright/dim for flicker effect
  local cfgs = {
    {core = RED, glow = RED_DIM, r = 6},
    {core = RED_DIM, glow = RED_FAINT, r = 5},
    {core = RED, glow = RED_DIM, r = 7},
    {core = RED_FAINT, glow = RED_FAINT, r = 4},
  }
  for f = 1, 4 do
    local img = Image(32, 32, ColorMode.RGB)
    local c = cfgs[f]
    -- X shape for "error"
    for i = -8, 8 do
      local x1, y1 = math.floor(cx + i), math.floor(cy + i)
      local x2, y2 = math.floor(cx + i), math.floor(cy - i)
      if x1 >= 0 and y1 >= 0 and x1 < 32 and y1 < 32 then img:drawPixel(x1, y1, c.core) end
      if x2 >= 0 and y2 >= 0 and x2 < 32 and y2 < 32 then img:drawPixel(x2, y2, c.core) end
    end
    circle(img, cx, cy, c.r, c.glow)
    spr:newCel(layer, f, img, Point(0, 0))
  end
  save(spr, "error_flicker")
end

-- 4. fiber_pulse: 32x32, 4 frames — animated light pulse (upgrade for pulse_dot)
do
  local spr = new_sprite(32, 32, 4)
  local layer = spr.layers[1]
  local cx, cy = 15.5, 15.5
  -- Pulse travels: core moves slightly + intensity oscillates
  local offsets = {0, 1, 0, -1}
  for f = 1, 4 do
    local img = Image(32, 32, ColorMode.RGB)
    local ox = offsets[f]
    circle(img, cx + ox, cy, 4, WHITE)
    circle(img, cx + ox, cy, 7, CYAN)
    circle(img, cx + ox, cy, 10, CYAN_DIM)
    circle(img, cx + ox, cy, 13, CYAN_FAINT)
    spr:newCel(layer, f, img, Point(0, 0))
  end
  save(spr, "fiber_pulse")
end

print("FX generation complete")
