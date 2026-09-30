#!/usr/bin/env python3
"""Derive 'mature' full-body variant sheets from the standard full-body set.

For each character's 384x1152 sheet (96x192 cells, 6 mood rows x 4 frames):
  - compress the head/hair-mass region vertically (youthful ~1:3.2 -> adult ~1:7)
  - stretch the body region so legs/torso lengthen and feet stay planted
  - narrow the head slightly and squeeze the jaw band for a less rounded face
  - copy the 6px mood bar verbatim; all mood overlays ride along with the head

Run on primo:  python3 derive_mature.py
Reads:  game/assets/sprites/<name>/<name>_sheet_fullbody.png
Writes: game/assets/sprites/<name>/<name>_sheet_fullbody_mature.png
"""
import os
import numpy as np
from PIL import Image, ImageFilter

REPO = "/home/phaedrus/workspace/repos/light-show"
CHARS = ["lattice", "linka", "ondine", "seraphine"]

CELL_W, CELL_H = 96, 192
COLS, ROWS = 4, 6
MOOD_BAR_H = 6          # bottom rows preserved verbatim
NEW_HEAD_H = 28         # head/hair mass height in output (~1:6.9 head ratio)
HEAD_XSCALE = 0.94      # slight overall head narrowing
JAW_BAND_FRac = 0.40    # lower fraction of head region treated as jaw
JAW_SQUEEZE = 0.88      # extra horizontal squeeze in the jaw band

# Bottom of the head region per character (cell coords, measured).
# MUST sit below the chin/mouth so the whole face compresses as one unit;
# a boundary through the mouth splits it across the compress/stretch seam
# and distorts it (seen 2026-09-30 on ondine/linka).
HEAD_BOTTOM = {
    "lattice": 76,    # below chin (mouth ends ~69)
    "linka": 88,      # below chin (mouth ends ~83)
    "ondine": 76,     # below chin (mouth ends ~72)
    "seraphine": 89,  # below chin (mouth ends ~84)
}
FACE_CX = 48


def squeeze_jaw(head: Image.Image, cx: int) -> Image.Image:
    """Narrow the lower (jaw) band of a head crop around cx. Returns new image."""
    a = np.asarray(head).astype(np.float32)
    h, w, _ = a.shape
    jaw_start = int(h * (1.0 - JAW_BAND_FRac))
    out = a.copy()
    for y in range(jaw_start, h):
        # ramp squeeze from 1.0 at band top to JAW_SQUEEZE at chin
        t = (y - jaw_start) / max(1, h - 1 - jaw_start)
        s = 1.0 - (1.0 - JAW_SQUEEZE) * t
        xs = np.arange(w)
        src_x = cx + (xs - cx) / s
        src_x = np.clip(src_x, 0, w - 1)
        x0 = np.floor(src_x).astype(int)
        x1 = np.minimum(x0 + 1, w - 1)
        f = (src_x - x0)[..., None]
        out[y] = a[y, x0] * (1 - f) + a[y, x1] * f
    return Image.fromarray(np.clip(out, 0, 255).astype(np.uint8))


def mature_cell(cell: Image.Image, head_bottom: int) -> Image.Image:
    body_h = CELL_H - MOOD_BAR_H - NEW_HEAD_H
    out = Image.new("RGB", (CELL_W, CELL_H))

    # 1. Body: stretch from below head to above the mood bar.
    body = cell.crop((0, head_bottom, CELL_W, CELL_H - MOOD_BAR_H))
    body = body.resize((CELL_W, body_h), Image.BICUBIC)
    out.paste(body, (0, NEW_HEAD_H))

    # 2. Head: jaw-squeeze, then shrink to adult proportion.
    head = cell.crop((0, 0, CELL_W, head_bottom))
    head = squeeze_jaw(head, FACE_CX)
    new_w = int(CELL_W * HEAD_XSCALE)
    head = head.resize((new_w, NEW_HEAD_H), Image.LANCZOS)
    out.paste(head, ((CELL_W - new_w) // 2, 0))

    # 3. Mood bar: verbatim.
    bar = cell.crop((0, CELL_H - MOOD_BAR_H, CELL_W, CELL_H))
    out.paste(bar, (0, CELL_H - MOOD_BAR_H))

    # 4. Restore pixel-art crispness lost to resampling.
    return out.filter(ImageFilter.UnsharpMask(radius=1.2, percent=45, threshold=2))


def main() -> None:
    for name in CHARS:
        d = os.path.join(REPO, "game", "assets", "sprites", name)
        src = Image.open(os.path.join(d, f"{name}_sheet_fullbody.png")).convert("RGB")
        assert src.size == (CELL_W * COLS, CELL_H * ROWS), f"bad size {src.size}"
        dst = Image.new("RGB", src.size)
        hb = HEAD_BOTTOM[name]
        for row in range(ROWS):
            for col in range(COLS):
                cell = src.crop((col * CELL_W, row * CELL_H,
                                 (col + 1) * CELL_W, (row + 1) * CELL_H))
                dst.paste(mature_cell(cell, hb),
                          (col * CELL_W, row * CELL_H))
        out_path = os.path.join(d, f"{name}_sheet_fullbody_mature.png")
        dst.save(out_path)
        print("wrote", out_path)


if __name__ == "__main__":
    main()
