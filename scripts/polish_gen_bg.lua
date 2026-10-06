-- Generate seamless circuit/fiber background tile for light-show
-- 128x128, dark with subtle cyan circuit traces. Must tile seamlessly.
-- Aseprite 1.3 batch mode.

local outdir = app.params["outdir"]
if outdir == nil or outdir == "" then
  print("FATAL: outdir param required")
  os.exit(1)
end

local function rgba(r, g, b, a)
  return app.pixelColor.rgba(r, g, b, a or 255)
end

local INK = rgba(10, 14, 26)           -- background base
local INK_LIGHT = rgba(16, 22, 38)     -- slight variation
local TRACE = rgba(111, 242, 255, 36)  -- faint cyan traces
local TRACE_DIM = rgba(111, 242, 255, 20)
local NODE = rgba(111, 242, 255, 60)    -- via points
local GOLD_T = rgba(255, 209, 102, 28)  -- rare gold accent

local W, H = 128, 128
local spr = Sprite(W, H, ColorMode.RGB)
local layer = spr.layers[1]
local img = Image(W, H, ColorMode.RGB)

-- Base fill with subtle noise variation
for y = 0, H - 1 do
  for x = 0, W - 1 do
    -- Deterministic pseudo-noise from coordinates
    local n = ((x * 73856093) ~ (y * 19349663)) % 100
    if n < 8 then
      img:drawPixel(x, y, INK_LIGHT)
    else
      img:drawPixel(x, y, INK)
    end
  end
end

-- Helper: draw with wraparound for seamless tiling
local function px(x, y, c)
  x = ((x % W) + W) % W
  y = ((y % H) + H) % H
  img:drawPixel(x, y, c)
end

local function hline_w(x0, x1, y, c)
  for x = x0, x1 do px(x, y, c) end
end

local function vline_w(x, y0, y1, c)
  for y = y0, y1 do px(x, y, c) end
end

-- Circuit traces: horizontal and vertical lines that wrap at edges
-- Trace 1: horizontal at y=32, spans full width (wraps seamlessly)
hline_w(0, W - 1, 32, TRACE)
hline_w(0, W - 1, 33, TRACE_DIM)
-- Trace 2: horizontal at y=96
hline_w(0, W - 1, 96, TRACE)
-- Trace 3: vertical at x=64
vline_w(64, 0, H - 1, TRACE)
vline_w(65, 0, H - 1, TRACE_DIM)
-- Trace 4: vertical at x=16, partial (with wrapped segment)
vline_w(16, 0, 48, TRACE_DIM)
vline_w(16, 80, H - 1, TRACE_DIM)
vline_w(16, 0, 16, TRACE_DIM)  -- wraps to bottom
-- Trace 5: L-shaped trace (wraps)
hline_w(80, W - 1, 64, TRACE_DIM)
hline_w(0, 24, 64, TRACE_DIM)
vline_w(80, 64, 96, TRACE_DIM)

-- Via points (small squares) at intersections
local function via(x, y, c)
  c = c or NODE
  for dy = -1, 1 do
    for dx = -1, 1 do
      px(x + dx, y + dy, c)
    end
  end
  px(x, y, rgba(180, 250, 255, 90))
end

via(64, 32)
via(64, 96)
via(16, 32)
via(80, 64)
via(32, 96, GOLD_T)  -- one gold accent via
via(96, 32)

-- Small fiber dots scattered (deterministic positions)
local dots = {{10, 10}, {50, 70}, {90, 110}, {110, 50}, {30, 120}, {70, 20}}
for _, d in ipairs(dots) do
  px(d[1], d[2], TRACE_DIM)
end

spr:newCel(layer, 1, img, Point(0, 0))
local path = outdir .. "/circuit_tile.aseprite"
spr:saveAs(path)
spr:close()
print("wrote " .. path)
print("BG generation complete")
