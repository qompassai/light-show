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
#
# Portrait filenames match what's actually committed at
# assets/art/companions/<key>_portrait.jpg (see docs/CREDITS.md) --
# this must stay in sync with that directory or the pipeline silently
# can't find its own source art.
COMPANIONS = [
    ("seraphine", "seraphine_portrait.jpg", (255, 111, 174)),
    ("ondine", "ondine_portrait.jpg", (46, 196, 182)),
    ("linka", "linka_portrait.jpg", (124, 58, 237)),
    ("lattice", "lattice_portrait.jpg", (37, 99, 235)),
]

HAZARD_RED = (255, 77, 77)
# Per-mood accent-strip colors so each row reads at a glance even at tiny
# in-game render sizes, instead of every non-alarmed row sharing the same
# character tint strip and only differing via a barely-visible
# brightness/saturation nudge.
MOOD_ACCENTS = {
    "idle": None,  # falls back to the character's own tint
    "blush": (255, 141, 187),
    "wink": (255, 209, 102),
    "pout": (94, 114, 148),
    "celebrate": (255, 209, 102),
    "alarmed": HAZARD_RED,
}


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

    # Each mood gets both a stronger color/brightness treatment *and* a
    # translucent color-overlay wash (like `alarmed` already had) so the
    # difference reads clearly at 64x64, not just under close inspection.
    if mood == "blush":
        working = enhancer_color.enhance(1.4)
        working = enhancer_bright.enhance(1.08)
        overlay = Image.new("RGBA", working.size, (255, 141, 187, 55))
        working = Image.alpha_composite(working.convert("RGBA"), overlay).convert("RGB")
    elif mood == "wink":
        working = enhancer_bright.enhance(1.2)
        overlay = Image.new("RGBA", working.size, (255, 209, 102, 45))
        working = Image.alpha_composite(working.convert("RGBA"), overlay).convert("RGB")
    elif mood == "pout":
        working = ImageEnhance.Color(working).enhance(0.55)
        working = ImageEnhance.Brightness(working).enhance(0.82)
        overlay = Image.new("RGBA", working.size, (94, 114, 148, 60))
        working = Image.alpha_composite(working.convert("RGBA"), overlay).convert("RGB")
    elif mood == "celebrate":
        working = enhancer_bright.enhance(1.3)
        working = enhancer_color.enhance(1.45)
        overlay = Image.new("RGBA", working.size, (255, 209, 102, 60))
        working = Image.alpha_composite(working.convert("RGBA"), overlay).convert("RGB")
    elif mood == "alarmed":
        overlay = Image.new("RGBA", working.size, HAZARD_RED + (70,))
        working = Image.alpha_composite(working.convert("RGBA"), overlay).convert("RGB")
        working = ImageEnhance.Brightness(working).enhance(1.08)
    # idle: base as-is

    tile.paste(working.convert("RGBA"), (0, bob))

    # Thick mood-accent underline strip (own color per mood, see
    # MOOD_ACCENTS) so each row is identifiable at a glance in the sprite
    # sheet / in tiny in-game render sizes, without needing to read the
    # subtler tint/brightness treatment above it.
    draw = ImageDraw.Draw(tile)
    accent = MOOD_ACCENTS.get(mood) or tint
    draw.rectangle([1, FRAME - 5, FRAME - 2, FRAME - 2], fill=accent)
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
        portrait_path = art_dir / portrait_name
        if not portrait_path.exists():
            raise FileNotFoundError(
                f"missing source portrait for {key!r}: {portrait_path} "
                "(see docs/CREDITS.md for how these are AI-generated)"
            )
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
