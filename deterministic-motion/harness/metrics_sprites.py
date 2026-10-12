#!/usr/bin/env python3
"""Sprite-strip smoothness measurements for light-show (native timing only).

For every animated sprite strip shipped in the repo, compute per-frame
pixel-diff energy (mean absolute RGBA difference, 0..1) between
consecutive frames, the median intra-cycle delta, the max delta, and —
for looping strips — the loop-seam delta (last frame back to first)
against the median. No frames are interpolated or fabricated: strips are
measured exactly as shipped, at their authored frame timing.

Usage: metrics_sprites.py <repo_root> <out_json>
"""
import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageSequence

MOOD_ROWS = ["idle", "blush", "wink", "pout", "celebrate", "alarmed"]
COMPANIONS = ["seraphine", "ondine", "linka", "lattice"]


def frame_array(img: Image.Image) -> np.ndarray:
    return np.asarray(img.convert("RGBA"), dtype=np.float32) / 255.0


def diff_energy(a: np.ndarray, b: np.ndarray) -> float:
    return float(np.abs(a - b).mean())


def strip_metrics(frames, frame_secs, loop):
    deltas = [diff_energy(frames[i], frames[i + 1]) for i in range(len(frames) - 1)]
    median = float(np.median(deltas)) if deltas else 0.0
    seam = diff_energy(frames[-1], frames[0]) if (loop and len(frames) > 1) else None
    return {
        "frames": len(frames),
        "frame_secs": frame_secs,
        "loop": loop,
        "deltas": [round(d, 5) for d in deltas],
        "median_delta": round(median, 5),
        "mean_delta": round(float(np.mean(deltas)) if deltas else 0.0, 5),
        "max_delta": round(max(deltas), 5) if deltas else 0.0,
        "max_delta_at": (int(np.argmax(deltas)) + 1) if deltas else None,
        "seam_delta": round(seam, 5) if seam is not None else None,
        "seam_ratio": round(seam / median, 3) if (seam is not None and median > 0) else None,
        "static": bool(deltas and max(deltas) == 0.0),
    }


def slice_sheet(path: Path, cell_w: int, cell_h: int, cols: int, rows: int):
    img = Image.open(path).convert("RGBA")
    out = []
    for r in range(rows):
        row_frames = []
        for c in range(cols):
            box = (c * cell_w, r * cell_h, (c + 1) * cell_w, (r + 1) * cell_h)
            row_frames.append(frame_array(img.crop(box)))
        out.append(row_frames)
    return out


def load_sequence(paths):
    return [frame_array(Image.open(p)) for p in paths]


def aseprite_frame_durations(path: Path):
    """Parse per-frame durations (ms) from an .aseprite binary header chain."""
    data = path.read_bytes()
    if len(data) < 128 or struct.unpack_from("<H", data, 4)[0] != 0xA5E0:
        return None
    total_frames = struct.unpack_from("<H", data, 6)[0]
    durations = []
    off = 128
    for _ in range(total_frames):
        if off + 16 > len(data):
            break
        frame_size = struct.unpack_from("<I", data, off)[0]
        magic = struct.unpack_from("<H", data, off + 4)[0]
        if magic != 0xF1FA or frame_size <= 0:
            break
        durations.append(struct.unpack_from("<H", data, off + 8)[0])
        off += frame_size
    return durations or None


def gif_strip(path: Path):
    img = Image.open(path)
    frames, durations = [], []
    for fr in ImageSequence.Iterator(img):
        durations.append(fr.info.get("duration", 0) / 1000.0)
        frames.append(frame_array(fr))
    secs = durations[0] if durations and all(d == durations[0] for d in durations) else None
    return frames, secs, durations


def main() -> int:
    root = Path(sys.argv[1])
    out_path = Path(sys.argv[2])
    sprites = root / "game" / "assets" / "sprites"
    strips = []

    # 1. Runtime companion sheets (the files sprite_path() actually loads):
    #    288x576 cells, 4 cols x 6 mood rows, 0.18 s/frame (waifu/mod.rs).
    for char in COMPANIONS:
        sheet = sprites / char / f"{char}_sheet_fullbody_mature.png"
        rows = slice_sheet(sheet, 288, 576, 4, 6)
        for r, mood in enumerate(MOOD_ROWS):
            m = strip_metrics(rows[r], 0.18, True)
            strips.append({"id": f"companion-{char}-{mood}", "kind": "runtime-sheet-row",
                           "source": str(sheet.relative_to(root)), **m})

    # 2. FX frame sequences (fx.rs / results.rs timers).
    fx = sprites / "fx"
    for name, secs, loop in [("connect_spark", 0.06, False),
                             ("success_burst", 0.10, False),
                             ("win_ring", 0.12, False)]:
        paths = sorted(fx.glob(f"{name}_[0-9].png"))
        m = strip_metrics(load_sequence(paths), secs, loop)
        strips.append({"id": f"fx-{name}", "kind": "fx-sequence",
                       "source": str(paths[0].relative_to(root)), **m})

    # 3. Reserved/unwired FX cycles: timing from the .aseprite source.
    for name in ["fiber_pulse", "error_flicker"]:
        paths = sorted(fx.glob(f"{name}_[0-9].png"))
        durations = aseprite_frame_durations(fx / f"{name}.aseprite")
        secs = (durations[0] / 1000.0) if durations else None
        m = strip_metrics(load_sequence(paths), secs, True)
        strips.append({"id": f"fx-{name}-unwired", "kind": "fx-sequence-unwired",
                       "source": str(paths[0].relative_to(root)),
                       "aseprite_durations_ms": durations, **m})

    # 4. Companion-select silhouettes (companion_select.rs, 0.12 s/frame).
    for char in COMPANIONS:
        paths = sorted((sprites / "picker").glob(f"{char}_select_[0-9].png"))
        m = strip_metrics(load_sequence(paths), 0.12, True)
        strips.append({"id": f"select-{char}", "kind": "picker-select",
                       "source": str(paths[0].relative_to(root)), **m})

    # 5. README/store animated GIFs in the source-art tree (not runtime).
    gif_root = root / "assets" / "art"
    for gif in sorted(gif_root.glob("*/companions/*_animated_*.gif")):
        frames, secs, durations = gif_strip(gif)
        m = strip_metrics(frames, secs, True)
        strips.append({"id": f"gif-{gif.stem}", "kind": "readme-gif",
                       "source": str(gif.relative_to(root)),
                       "gif_durations_s": durations, **m})

    payload = {
        "metric": "per-frame mean absolute RGBA difference, values 0..1; "
                  "seam = last->first delta for looping strips",
        "strips": strips,
    }
    out_path.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"{len(strips)} strips measured -> {out_path}")
    # Ranked summary: worst seam ratios and max deltas first.
    ranked = sorted(strips, key=lambda s: (s["seam_ratio"] is not None,
                                           s["seam_ratio"] or 0.0), reverse=True)
    for s in ranked:
        print(f"{s['id']:34s} frames={s['frames']:2d} frame_secs={s['frame_secs']} "
              f"median={s['median_delta']:.5f} max={s['max_delta']:.5f} "
              f"seam={s['seam_delta']} seam_ratio={s['seam_ratio']} static={s['static']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
