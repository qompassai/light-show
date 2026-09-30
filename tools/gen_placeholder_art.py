#!/usr/bin/env python3
"""Generate palette-correct 64x64 pixel-art icons for Light Show's OSP
board components.

These are programmatically drawn (not hand-painted / AI-illustrated like
the companion portraits — see `gen_companion_art.py` for that pipeline),
but they follow the iconography brief in docs/ART_STYLE.md exactly: one
motif per component, filled silhouettes (not just outlines) so they read
clearly at the ~48px on-board render size, transparent backgrounds so
they composite cleanly over the gizmo-drawn pill circle in
`game/src/board.rs`, and a consistent 3px outline weight across the set.

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
    "apc_green": (86, 214, 122),
    "metal": (196, 206, 224),
    "metal_dark": (120, 132, 158),
}

OUTLINE_W = 3
CX, CY = FRAME // 2, FRAME // 2


def _fiber_line(draw: ImageDraw.ImageDraw, y: int = CY, x0: int = 6, x1: int = 58) -> None:
    """The light-carrying fiber strand running through most icons."""
    draw.line([x0, y, x1, y], fill=PALETTE["light_warm"], width=3)


def draw_olt(draw: ImageDraw.ImageDraw) -> None:
    """Server-rack silhouette with light rays leaving the front — the
    network's transmitting end."""
    draw.rectangle(
        [14, 12, 44, 52], fill=PALETTE["metal_dark"], outline=PALETTE["board_accent"], width=OUTLINE_W
    )
    for y in (20, 28, 36, 44):
        draw.rectangle([18, y, 40, y + 4], fill=PALETTE["board_accent"])
    for i, dx in enumerate((0, 6, 12)):
        draw.line(
            [46, 32 - dx, 58 - i * 3, 20 - dx], fill=PALETTE["light_hot"], width=2
        )
        draw.line(
            [46, 32 + dx, 58 - i * 3, 44 + dx], fill=PALETTE["light_hot"], width=2
        )


def draw_ont(draw: ImageDraw.ImageDraw) -> None:
    """House silhouette with light rays entering — the subscriber end."""
    draw.polygon(
        [(32, 10), (12, 30), (52, 30)],
        fill=PALETTE["board_accent"],
        outline=PALETTE["light_hot"],
    )
    draw.rectangle(
        [16, 30, 48, 54], fill=PALETTE["metal_dark"], outline=PALETTE["board_accent"], width=OUTLINE_W
    )
    draw.rectangle([27, 38, 37, 54], fill=PALETTE["board_bg"])
    for i, dx in enumerate((0, 6, 12)):
        draw.line([2, 20 - dx, 14, 28 - dx // 2], fill=PALETTE["light_hot"], width=2)


def draw_fusion_splice(draw: ImageDraw.ImageDraw) -> None:
    """Two fiber ends meeting inside a splicer clamp — the "clean" joint."""
    _fiber_line(draw)
    draw.rectangle(
        [CX - 14, CY - 10, CX - 2, CY + 10],
        outline=PALETTE["metal"],
        width=OUTLINE_W,
    )
    draw.rectangle(
        [CX + 2, CY - 10, CX + 14, CY + 10],
        outline=PALETTE["metal"],
        width=OUTLINE_W,
    )
    # Fusion glow at the meeting point.
    draw.ellipse(
        [CX - 6, CY - 6, CX + 6, CY + 6],
        fill=PALETTE["light_hot"],
        outline=PALETTE["hazard_orange"],
        width=2,
    )


def draw_mechanical_splice(draw: ImageDraw.ImageDraw) -> None:
    """Two fiber ends inside a gel-filled sleeve — faster, less clean."""
    _fiber_line(draw)
    draw.rounded_rectangle(
        [CX - 16, CY - 9, CX + 16, CY + 9],
        radius=6,
        fill=PALETTE["board_line"],
        outline=PALETTE["board_accent"],
        width=OUTLINE_W,
    )
    # Visible gel-fill seam, slightly off-center to read as "less precise".
    draw.line([CX + 2, CY - 9, CX + 2, CY + 9], fill=PALETTE["hazard_orange"], width=2)


def draw_connector(draw: ImageDraw.ImageDraw, apc: bool) -> None:
    """A UPC (flat-face) or APC (angled-face, green-keyed) connector tip."""
    _fiber_line(draw, x1=CX + 2)
    draw.rectangle(
        [CX - 18, CY - 8, CX - 2, CY + 8], fill=PALETTE["metal"], outline=PALETTE["metal_dark"], width=2
    )
    key_color = PALETTE["apc_green"] if apc else PALETTE["board_accent"]
    if apc:
        # Angled 8-degree face, drawn as a slanted polygon tip.
        draw.polygon(
            [(CX - 2, CY - 10), (CX + 10, CY - 4), (CX + 10, CY + 4), (CX - 2, CY + 10)],
            fill=key_color,
            outline=PALETTE["board_line"],
        )
    else:
        # Flat face, drawn as a plain circle tip.
        draw.ellipse([CX - 4, CY - 10, CX + 12, CY + 10], fill=key_color, outline=PALETTE["board_line"])
    draw.rectangle([CX - 16, CY - 11, CX - 12, CY + 11], fill=key_color)


def draw_splitter(draw: ImageDraw.ImageDraw) -> None:
    """A single line fanning into N lines inside a rounded box."""
    draw.line([6, CY, CX - 6, CY], fill=PALETTE["light_warm"], width=3)
    draw.rounded_rectangle(
        [CX - 6, CY - 16, CX + 6, CY + 16],
        radius=4,
        fill=PALETTE["board_line"],
        outline=PALETTE["board_accent"],
        width=OUTLINE_W,
    )
    for dy in (-14, 0, 14):
        draw.line([CX + 6, CY, 58, CY + dy], fill=PALETTE["light_warm"], width=2)


def draw_macrobend(draw: ImageDraw.ImageDraw) -> None:
    """A kinked fiber line with a small warning glyph — an over-tight bend
    that leaks light out the side of the cladding."""
    draw.line(
        [8, CY, 24, CY - 12, 40, CY + 12, 56, CY],
        fill=PALETTE["hazard_red"],
        width=4,
        joint="curve",
    )
    # Light leaking out at the kinks.
    for x, y in ((24, CY - 12), (40, CY + 12)):
        draw.line([x, y, x - 4, y - 10], fill=PALETTE["light_hot"], width=2)
    draw.polygon(
        [(CX, 8), (CX - 6, 20), (CX + 6, 20)],
        fill=PALETTE["hazard_orange"],
        outline=PALETTE["board_bg"],
    )
    draw.text((CX - 2, 9), "!", fill=PALETTE["board_bg"])


def draw_water_intrusion(draw: ImageDraw.ImageDraw) -> None:
    """A droplet glyph over a splice enclosure — moisture degrading a
    joint over time."""
    draw.rounded_rectangle(
        [CX - 16, CY, CX + 16, CY + 20], radius=4, fill=PALETTE["board_line"], outline=PALETTE["board_accent"], width=OUTLINE_W
    )
    draw.polygon(
        [(CX, CY - 22), (CX - 8, CY - 6), (CX + 8, CY - 6)],
        fill=PALETTE["hazard_orange"],
        outline=PALETTE["board_bg"],
    )
    draw.ellipse([CX - 8, CY - 10, CX + 8, CY - 2], fill=PALETTE["hazard_orange"])


ICONS = {
    "olt": draw_olt,
    "ont": draw_ont,
    "fusion_splice": draw_fusion_splice,
    "mechanical_splice": draw_mechanical_splice,
    "upc_connector": lambda d: draw_connector(d, apc=False),
    "apc_connector": lambda d: draw_connector(d, apc=True),
    "splitter": draw_splitter,
    "macrobend": draw_macrobend,
    "water_intrusion": draw_water_intrusion,
}


def main() -> None:
    icons_dir = ROOT / "game" / "assets" / "sprites" / "components"
    icons_dir.mkdir(parents=True, exist_ok=True)

    for name, draw_fn in ICONS.items():
        img = Image.new("RGBA", (FRAME, FRAME), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)
        draw_fn(draw)
        path = icons_dir / f"{name}.png"
        img.save(path)
        print(f"wrote {path}")


if __name__ == "__main__":
    main()
