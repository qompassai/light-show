#!/usr/bin/env python3
"""Generate palette-correct 64x64 placeholder sprites for Light Show.

This produces solid-silhouette placeholders (NOT final art) so the game is
playable and screenshot-able before hand-drawn/commissioned art lands. See
docs/ART_STYLE.md for the real art direction brief and docs/CREDITS.md for
the licensing checklist that must be filled in before a tagged release.

Usage:
    python3 tools/gen_placeholder_art.py

Requires: Pillow (`pip install pillow`)
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
FRAME = 64

PALETTE = {
    "board_bg": (13, 13, 30),
    "board_line": (28, 37, 65),
    "board_accent": (91, 192, 235),
    "light_warm": (255, 209, 102),
    "light_hot": (255, 251, 230),
    "seraphine_magenta": (255, 111, 174),
    "seraphine_cyan": (111, 214, 255),
    "hazard_red": (255, 77, 77),
    "hazard_orange": (255, 184, 77),
}

MOODS = ["idle", "blush", "wink", "pout", "celebrate", "alarmed"]
FRAMES_PER_MOOD = 4


def draw_seraphine_frame(draw: ImageDraw.ImageDraw, mood: str, frame: int) -> None:
    """Very simple placeholder silhouette: a ponytail-hologram blob whose
    color shifts per mood, with a small bob offset per frame so idle
    animation reads even before final art exists."""
    bob = [0, -2, 0, 2][frame % FRAMES_PER_MOOD]
    accent = PALETTE["seraphine_magenta"] if mood != "alarmed" else PALETTE["hazard_red"]
    base_y = 20 + bob

    # Head
    draw.ellipse([18, base_y, 46, base_y + 28], fill=accent, outline=PALETTE["seraphine_cyan"])
    # Ponytail / cable braid
    draw.line([44, base_y + 6, 58, base_y + 20], fill=PALETTE["seraphine_cyan"], width=3)
    # Visor glow
    draw.rectangle([21, base_y + 10, 43, base_y + 16], fill=PALETTE["light_hot"])


def draw_component_icon(draw: ImageDraw.ImageDraw, name: str) -> None:
    cx, cy = FRAME // 2, FRAME // 2
    if name == "olt":
        draw.rectangle([16, 20, 48, 44], outline=PALETTE["board_accent"], width=3)
        draw.line([16, 28, 48, 28], fill=PALETTE["light_warm"], width=2)
    elif name == "ont":
        draw.polygon([(32, 14), (14, 32), (50, 32)], outline=PALETTE["board_accent"], width=3)
        draw.rectangle([18, 32, 46, 50], outline=PALETTE["board_accent"], width=3)
    elif name == "fusion_splice":
        draw.line([10, cy, 54, cy], fill=PALETTE["light_warm"], width=3)
        draw.ellipse([cx - 6, cy - 6, cx + 6, cy + 6], outline=PALETTE["hazard_orange"], width=3)
    elif name == "mechanical_splice":
        draw.line([10, cy, 54, cy], fill=PALETTE["light_warm"], width=3)
        draw.rectangle([cx - 8, cy - 6, cx + 8, cy + 6], outline=PALETTE["board_accent"], width=3)
    elif name == "splitter":
        draw.line([10, cy, cx, cy], fill=PALETTE["light_warm"], width=3)
        for dy in (-14, 0, 14):
            draw.line([cx, cy, 54, cy + dy], fill=PALETTE["light_warm"], width=2)
    elif name == "macrobend":
        draw.line([10, cy, 26, cy - 10, 38, cy + 10, 54, cy], fill=PALETTE["hazard_red"], width=3, joint="curve")
    elif name == "water_intrusion":
        draw.ellipse([cx - 10, cy - 4, cx + 10, cy + 16], outline=PALETTE["hazard_orange"], width=3)
        draw.polygon([(cx, cy - 16), (cx - 6, cy - 4), (cx + 6, cy - 4)], fill=PALETTE["board_accent"])


def main() -> None:
    out_dir = ROOT / "game" / "assets" / "sprites"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Seraphine sheet: 4 frames per mood row, 6 mood rows.
    sheet = Image.new("RGBA", (FRAME * FRAMES_PER_MOOD, FRAME * len(MOODS)), (0, 0, 0, 0))
    for row, mood in enumerate(MOODS):
        for frame in range(FRAMES_PER_MOOD):
            tile = Image.new("RGBA", (FRAME, FRAME), (0, 0, 0, 0))
            draw = ImageDraw.Draw(tile)
            draw_seraphine_frame(draw, mood, frame)
            sheet.paste(tile, (frame * FRAME, row * FRAME), tile)
    seraphine_dir = out_dir / "seraphine"
    seraphine_dir.mkdir(exist_ok=True)
    sheet.save(seraphine_dir / "seraphine_sheet.png")
    print(f"wrote {seraphine_dir / 'seraphine_sheet.png'}")

    # Component icons.
    icons_dir = out_dir / "components"
    icons_dir.mkdir(exist_ok=True)
    for name in [
        "olt",
        "ont",
        "fusion_splice",
        "mechanical_splice",
        "splitter",
        "macrobend",
        "water_intrusion",
    ]:
        img = Image.new("RGBA", (FRAME, FRAME), PALETTE["board_bg"])
        draw = ImageDraw.Draw(img)
        draw_component_icon(draw, name)
        path = icons_dir / f"{name}.png"
        img.save(path)
        print(f"wrote {path}")


if __name__ == "__main__":
    main()
