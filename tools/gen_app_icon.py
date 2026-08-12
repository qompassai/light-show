#!/usr/bin/env python3
"""Generate Android launcher icon resources (legacy + adaptive) from the
AI-generated 4-companion group portrait.

Produces, under android/app/src/main/res/:
  mipmap-mdpi/ic_launcher.png        (48x48)   + ic_launcher_round.png
  mipmap-hdpi/ic_launcher.png        (72x72)   + ic_launcher_round.png
  mipmap-xhdpi/ic_launcher.png       (96x96)   + ic_launcher_round.png
  mipmap-xxhdpi/ic_launcher.png      (144x144) + ic_launcher_round.png
  mipmap-xxxhdpi/ic_launcher.png     (192x192) + ic_launcher_round.png
  mipmap-anydpi-v26/ic_launcher.xml            (adaptive icon, API 26+)
  mipmap-anydpi-v26/ic_launcher_round.xml
  mipmap-xxxhdpi/ic_launcher_foreground.png    (432x432 adaptive foreground)

Source art has no proprietary SDK / network dependency -- fully static
AI-generated PNG baked at build time, so this keeps the app F-Droid
eligible.

Usage:
    python3 tools/gen_app_icon.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT.parent / "app_icon_group.png"
if not SOURCE.exists():
    SOURCE = ROOT / "app_icon_group.png"

RES = ROOT / "android" / "app" / "src" / "main" / "res"

# (density, legacy launcher px size)
DENSITIES = [
    ("mdpi", 48),
    ("hdpi", 72),
    ("xhdpi", 96),
    ("xxhdpi", 144),
    ("xxxhdpi", 192),
]

ADAPTIVE_FOREGROUND_PX = 432  # standard adaptive icon foreground canvas
BACKGROUND_COLOR = "#141420"  # deep navy, matches game's dark-blue UI theme

ADAPTIVE_ICON_XML = """<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
</adaptive-icon>
"""

COLORS_XML = """<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="ic_launcher_background">{color}</color>
</resources>
"""


def make_round_mask(size: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, size - 1, size - 1), fill=255)
    return mask


def legacy_icons(source: Image.Image) -> None:
    for density, size in DENSITIES:
        out_dir = RES / f"mipmap-{density}"
        out_dir.mkdir(parents=True, exist_ok=True)

        square = source.resize((size, size), Image.LANCZOS)
        square.save(out_dir / "ic_launcher.png")

        round_mask = make_round_mask(size)
        round_icon = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        round_icon.paste(square, (0, 0), round_mask)
        round_icon.save(out_dir / "ic_launcher_round.png")

        print(f"wrote {out_dir / 'ic_launcher.png'} ({size}x{size})")
        print(f"wrote {out_dir / 'ic_launcher_round.png'} ({size}x{size})")


def adaptive_foreground(source: Image.Image) -> None:
    """Adaptive icons render the foreground within a safe zone that is
    roughly 66% of the full canvas (the outer ring gets cropped by the
    launcher's mask), so scale the group art down and center it on a
    transparent canvas sized to the full foreground spec."""
    canvas_size = ADAPTIVE_FOREGROUND_PX
    safe_zone = int(canvas_size * 0.66)

    art = source.convert("RGBA").resize((safe_zone, safe_zone), Image.LANCZOS)
    canvas = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    offset = ((canvas_size - safe_zone) // 2, (canvas_size - safe_zone) // 2)
    canvas.paste(art, offset, art)

    out_dir = RES / "mipmap-xxxhdpi"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "ic_launcher_foreground.png"
    canvas.save(out_path)
    print(f"wrote {out_path} ({canvas_size}x{canvas_size})")


def adaptive_icon_xml_and_colors() -> None:
    anydpi_dir = RES / "mipmap-anydpi-v26"
    anydpi_dir.mkdir(parents=True, exist_ok=True)
    (anydpi_dir / "ic_launcher.xml").write_text(ADAPTIVE_ICON_XML)
    (anydpi_dir / "ic_launcher_round.xml").write_text(ADAPTIVE_ICON_XML)
    print(f"wrote {anydpi_dir / 'ic_launcher.xml'}")
    print(f"wrote {anydpi_dir / 'ic_launcher_round.xml'}")

    values_dir = RES / "values"
    values_dir.mkdir(parents=True, exist_ok=True)
    colors_path = values_dir / "ic_launcher_colors.xml"
    colors_path.write_text(COLORS_XML.format(color=BACKGROUND_COLOR))
    print(f"wrote {colors_path}")


def main() -> None:
    source = Image.open(SOURCE).convert("RGBA")
    legacy_icons(source)
    adaptive_foreground(source)
    adaptive_icon_xml_and_colors()


if __name__ == "__main__":
    main()
