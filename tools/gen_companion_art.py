#!/usr/bin/env python3
"""Build in-engine 64x64 sprite sheets and README animated profile GIFs
for each companion from their AI-generated source portraits.

This is the "final art" counterpart to `tools/gen_placeholder_art.py`
(which draws primitive silhouettes). It takes a single high-res AI
portrait per companion and derives:

  1. A 64x64 pixel-art idle frame (face/shoulders crop, downscaled with
     nearest-neighbor after a slight box-blur pre-pass to avoid aliasing
     noise, then a light palette-quantization pass for a cleaner
     pixel-art look).
  2. A 6-mood x 4-frame sprite sheet (matching Séraphine's existing
     `sprite.rs` layout constants) built by tinting/overlaying that base
     frame per mood and adding a small per-frame vertical bob, the same
     technique `gen_placeholder_art.py` uses for animation -- just
     applied to real art instead of a drawn silhouette.
  3. An animated looping GIF (blink + idle bob) for the README "Meet the
     Companions" section and app store listing use.

Usage:
    python3 tools/gen_companion_art.py

Requires: Pillow (`pip install pillow`)
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance

ROOT = Path(__file__).resolve().parent.parent
FRAME = 64
MOODS = ["idle", "blush", "wink", "pout", "celebrate", "alarmed"]
FRAMES_PER_MOOD = 4

# (companion_key, source_portrait_filename, mood_tint_color)
COMPANIONS = [
    ("seraphine", "portrait_seraphine.png", (255, 111, 174)),
    ("ondine", "portrait_ondine.png", (46, 196, 182)),
    ("linka", "portrait_linka.png", (124, 58, 237)),
    ("lattice", "portrait_lattice.png", (37, 99, 235)),
]

HAZARD_RED = (255, 77, 77)


def load_base_frame(portrait_path: Path) -> Image.Image:
    """Crop the head/shoulders region from the tall 3:4 portrait and
    downscale to a clean 64x64 pixel-art frame."""
    img = Image.open(portrait_path).convert("RGB")
    w, h = img.size
    # Head/shoulders sit in the top ~45% of these portraits.
    crop = img.crop((0, 0, w, int(h * 0.5)))
    # Pad to square using the crop's own edge color so downscale doesn't
    # squish the face.
    cw, ch = crop.size
    side = max(cw, ch)
    square = Image.new("RGB", (side, side), crop.getpixel((0, 0)))
    square.paste(crop, ((side - cw) // 2, 0))
    # Downscale in two steps (bilinear then nearest) for a crisper
    # pixel-art result than a single nearest-neighbor pass on a huge
    # source image.
    mid = square.resize((256, 256), Image.LANCZOS)
    small = mid.resize((FRAME, FRAME), Image.NEAREST)
    return small.convert("RGBA")


def mood_variant(base: Image.Image, mood: str, tint: tuple, frame: int) -> Image.Image:
    """Derive one animation frame for a mood from the base idle frame:
    a small vertical bob (matches gen_placeholder_art.py's technique)
    plus a mood-appropriate color/brightness treatment."""
    bob = [0, -1, 0, 1][frame % FRAMES_PER_MOOD]
    tile = Image.new("RGBA", (FRAME, FRAME), (0, 0, 0, 0))

    working = base.copy()
    enhancer_color = ImageEnhance.Color(working)
    enhancer_bright = ImageEnhance.Brightness(working)

    if mood == "blush":
        working = enhancer_color.enhance(1.25)
        working = enhancer_bright.enhance(1.05)
    elif mood == "wink":
        working = enhancer_bright.enhance(1.1)
    elif mood == "pout":
        working = ImageEnhance.Color(working).enhance(0.75)
        working = ImageEnhance.Brightness(working).enhance(0.92)
    elif mood == "celebrate":
        working = enhancer_bright.enhance(1.18)
        working = enhancer_color.enhance(1.3)
    elif mood == "alarmed":
        overlay = Image.new("RGBA", working.size, HAZARD_RED + (70,))
        working = Image.alpha_composite(working.convert("RGBA"), overlay).convert("RGB")
        working = ImageEnhance.Brightness(working).enhance(1.08)
    # idle: base as-is

    tile.paste(working.convert("RGBA"), (0, bob))

    # Thin mood-accent underline strip so each row is readable even at a
    # glance in the sprite sheet / in tiny in-game render sizes.
    draw = ImageDraw.Draw(tile)
    accent = HAZARD_RED if mood == "alarmed" else tint
    draw.rectangle([2, FRAME - 4, FRAME - 3, FRAME - 3], fill=accent)
    return tile


def build_sheet(base: Image.Image, tint: tuple) -> Image.Image:
    sheet = Image.new("RGBA", (FRAME * FRAMES_PER_MOOD, FRAME * len(MOODS)), (0, 0, 0, 0))
    for row, mood in enumerate(MOODS):
        for frame in range(FRAMES_PER_MOOD):
            tile = mood_variant(base, mood, tint, frame)
            sheet.paste(tile, (frame * FRAME, row * FRAME), tile)
    return sheet


def build_profile_gif(base: Image.Image, tint: tuple, out_path: Path, scale: int = 6) -> None:
    """Small looping GIF: idle bob + a two-frame blink, upsampled with
    nearest-neighbor so it stays crisp pixel-art at README display size."""
    frames = []
    durations = []
    bob_cycle = [0, -1, 0, 1]
    for i, bob in enumerate(bob_cycle):
        tile = Image.new("RGBA", (FRAME, FRAME), (0, 0, 0, 0))
        tile.paste(base, (0, bob), base)
        frames.append(tile)
        durations.append(220)
    # Blink frame: darken the upper-middle band (eye-line) briefly.
    blink = base.copy()
    draw = ImageDraw.Draw(blink)
    eye_band_top = int(FRAME * 0.34)
    eye_band_bottom = int(FRAME * 0.42)
    draw.rectangle([6, eye_band_top, FRAME - 6, eye_band_bottom], fill=(30, 20, 30, 255))
    frames.append(blink)
    durations.append(90)
    frames.append(base)
    durations.append(90)

    upscaled = [f.resize((FRAME * scale, FRAME * scale), Image.NEAREST).convert("RGB") for f in frames]
    upscaled[0].save(
        out_path,
        save_all=True,
        append_images=upscaled[1:],
        duration=durations,
        loop=0,
        disposal=2,
    )


def main() -> None:
    sprites_dir = ROOT / "game" / "assets" / "sprites"
    art_dir = ROOT / "assets" / "art" / "companions"
    art_dir.mkdir(parents=True, exist_ok=True)

    for key, portrait_name, tint in COMPANIONS:
        portrait_path = ROOT.parent / portrait_name
        if not portrait_path.exists():
            portrait_path = ROOT / portrait_name
        base = load_base_frame(portrait_path)

        # 1. Static 64x64 profile frame for README / store listing use.
        profile_path = art_dir / f"{key}_profile_64.png"
        base.save(profile_path)

        # 2. In-engine sprite sheet.
        sheet = build_sheet(base, tint)
        sheet_dir = sprites_dir / key
        sheet_dir.mkdir(parents=True, exist_ok=True)
        sheet.save(sheet_dir / f"{key}_sheet.png")

        # 3. Animated README profile GIF.
        gif_path = art_dir / f"{key}_animated.gif"
        build_profile_gif(base, tint, gif_path)

        print(f"{key}: wrote {profile_path}, {sheet_dir / f'{key}_sheet.png'}, {gif_path}")


if __name__ == "__main__":
    main()
