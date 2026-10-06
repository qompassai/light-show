-- Generate UI accent sprites for light-show
-- Aseprite 1.3 batch mode. Palette: cyan #6ff2ff, gold #ffd166, dim #8e9eb8
-- Usage: aseprite -b --script-param outdir=/path --script gen_ui.lua

local outdir = app.params["outdir"]
if outdir == nil or outdir == "" then
  print("FATAL: outdir param required")
  os.exit(1)
end

local function rgba(r, g, b, a)
  return app.pixelColor.rgba(r, g, b, a or 255)
end

local CYAN = rgba(111, 242, 255)
local CYAN_DIM = rgba(111, 242, 255, 160)
local CYAN_FAINT = rgba(111, 242, 255, 80)
local GOLD = rgba(255, 209, 102)
local GOLD_DIM = rgba(255, 209, 102, 160)
local DIM = rgba(142, 158, 184)
local DIM_DARK = rgba(142, 158, 184, 100)
local WHITE = rgba(255, 255, 255)

local function hline(img, x0, x1, y, c)
  for x = x0, x1 do
    if x >= 0 and y >= 0 and x < img.width and y < img.height then
      img:drawPixel(x, y, c)
    end
  end
end

local function vline(img, x, y0, y1, c)
  for y = y0, y1 do
    if x >= 0 and y >= 0 and x < img.width and y < img.height then
      img:drawPixel(x, y, c)
    end
  end
end

local function rect(img, x0, y0, x1, y1, c)
  for y = y0, y1 do hline(img, x0, x1, y, c) end
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

-- 1. signal_bars: 48x32, 5 frames — signal strength 0-4 bars
do
  local spr = new_sprite(48, 32, 5)
  local layer = spr.layers[1]
  -- Bar geometry: 4 bars, 8px wide, 4px gap, heights 8/14/20/26, baseline y=28
  local bar_w, gap, base = 8, 4, 28
  local heights = {8, 14, 20, 26}
  for f = 1, 5 do
    local img = Image(48, 32, ColorMode.RGB)
    local active = f - 1  -- frame 1 = 0 bars, frame 5 = 4 bars
    for b = 1, 4 do
      local x0 = (b - 1) * (bar_w + gap) + 2
      local h = heights[b]
      local y0 = base - h
      local c = (b <= active) and CYAN or DIM_DARK
      local edge = (b <= active) and WHITE or DIM
      -- Bar body
      rect(img, x0 + 1, y0 + 1, x0 + bar_w - 2, base, c)
      -- Bright top edge for active bars
      if b <= active then hline(img, x0 + 1, x0 + bar_w - 2, y0, WHITE) end
      -- Dim outline for inactive
      if b > active then
        hline(img, x0, x0 + bar_w - 1, y0, edge)
        hline(img, x0, x0 + bar_w - 1, base, edge)
        vline(img, x0, y0, base, edge)
        vline(img, x0 + bar_w - 1, y0, base, edge)
      end
    end
    spr:newCel(layer, f, img, Point(0, 0))
  end
  save(spr, "signal_bars")
end

-- 2. panel_corner: 24x24, 1 frame — neon corner accent (rotatable in game)
do
  local spr = new_sprite(24, 24, 1)
  local layer = spr.layers[1]
  local img = Image(24, 24, ColorMode.RGB)
  -- L-shaped corner: top edge + left edge, cyan with white core
  hline(img, 0, 23, 0, WHITE)
  hline(img, 0, 23, 1, CYAN)
  hline(img, 0, 23, 2, CYAN_FAINT)
  vline(img, 0, 0, 23, WHITE)
  vline(img, 1, 0, 23, CYAN)
  vline(img, 2, 0, 23, CYAN_FAINT)
  -- Small gold dot at the inner corner joint
  img:drawPixel(4, 4, GOLD)
  img:drawPixel(5, 4, GOLD_DIM)
  img:drawPixel(4, 5, GOLD_DIM)
  spr:newCel(layer, 1, img, Point(0, 0))
  save(spr, "panel_corner")
end

-- 3. button_glow: 64x32, 3 frames — normal / hover / pressed
do
  local spr = new_sprite(64, 32, 3)
  local layer = spr.layers[1]
  local states = {
    {edge = CYAN_DIM, fill = false},   -- normal: dim outline
    {edge = CYAN, fill = false},        -- hover: bright outline + glow
    {edge = GOLD, fill = true},         -- pressed: gold fill tint
  }
  for f = 1, 3 do
    local img = Image(64, 32, ColorMode.RGB)
    local s = states[f]
    -- Rounded rect outline (2px inset)
    local x0, y0, x1, y1 = 2, 2, 61, 29
    hline(img, x0 + 2, x1 - 2, y0, s.edge)
    hline(img, x0 + 2, x1 - 2, y1, s.edge)
    vline(img, x0, y0 + 2, y1 - 2, s.edge)
    vline(img, x1, y0 + 2, y1 - 2, s.edge)
    -- Corner pixels
    for _, px in ipairs({{x0+1,y0+1},{x1-1,y0+1},{x0+1,y1-1},{x1-1,y1-1}}) do
      img:drawPixel(px[1], px[2], s.edge)
    end
    if f == 2 then
      -- Hover: outer glow
      hline(img, x0 + 2, x1 - 2, y0 - 1, CYAN_FAINT)
      hline(img, x0 + 2, x1 - 2, y1 + 1, CYAN_FAINT)
      vline(img, x0 - 1, y0 + 2, y1 - 2, CYAN_FAINT)
      vline(img, x1 + 1, y0 + 2, y1 - 2, CYAN_FAINT)
    elseif f == 3 then
      -- Pressed: subtle fill
      for y = y0 + 1, y1 - 1 do
        hline(img, x0 + 1, x1 - 1, y, rgba(255, 209, 102, 30))
      end
    end
    spr:newCel(layer, f, img, Point(0, 0))
  end
  save(spr, "button_glow")
end

-- 4. progress_tick: 16x16, 2 frames — progress indicator (empty/filled)
do
  local spr = new_sprite(16, 16, 2)
  local layer = spr.layers[1]
  -- Frame 1: empty diamond outline
  local img = Image(16, 16, ColorMode.RGB)
  local cx, cy = 7.5, 7.5
  for i = -5, 5 do
    local w = 5 - math.abs(i)
    if w >= 0 then
      local y = math.floor(cy + i)
      for x = math.floor(cx - w), math.floor(cx + w) do
        -- Only draw the edge (diamond outline)
        if math.abs(x - cx) + math.abs(i) >= 4.5 and math.abs(x - cx) + math.abs(i) <= 5.5 then
          if x >= 0 and y >= 0 and x < 16 and y < 16 then
            img:drawPixel(x, y, DIM)
          end
        end
      end
    end
  end
  spr:newCel(layer, 1, img, Point(0, 0))
  -- Frame 2: filled cyan diamond
  img = Image(16, 16, ColorMode.RGB)
  for i = -5, 5 do
    local w = 5 - math.abs(i)
    if w >= 0 then
      local y = math.floor(cy + i)
      for x = math.floor(cx - w), math.floor(cx + w) do
        if x >= 0 and y >= 0 and x < 16 and y < 16 then
          local edge = (math.abs(x - cx) + math.abs(i) >= 4.5)
          img:drawPixel(x, y, edge and WHITE or CYAN)
        end
      end
    end
  end
  spr:newCel(layer, 2, img, Point(0, 0))
  save(spr, "progress_tick")
end

print("UI generation complete")
