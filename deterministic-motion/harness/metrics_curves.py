#!/usr/bin/env python3
"""Deterministic evaluation of light-show's continuous + step motion math.

Every visual motion function in the game is re-implemented here from the
Rust/WGSL source (file:line citations in inventory.md) and sampled at
240 Hz — the render rate of the deterministic ports. For each curve we
report the per-frame delta distribution: mean, max, coefficient of
variation, and a spikiness ratio (max/mean). Eased curves vary by
design; a *spike* (one frame's delta dwarfing its neighbours) is what
would read as judder. Step functions (sprite timers, typewriter) are
evaluated exactly as shipped — value(t) changes only at authored step
instants — and their step sizes are reported instead.

Easing formulas are Bevy 0.19.1's own (bevy_math::curve::easing):
  CubicOut   f(t) = 1 - (1-t)^3
  CubicInOut f(t) = 4t^3 (t<0.5) else 1 - (-2t+2)^3 / 2
  SineInOut  f(t) = -(cos(pi t) - 1) / 2
  BackOut    f(t) = 1 + 2.70158 (t-1)^3 + 1.70158 (t-1)^2

Usage: metrics_curves.py <out_json>
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

HZ = 240.0


def cubic_out(t):
    return 1.0 - (1.0 - t) ** 3


def cubic_in_out(t):
    return 4.0 * t ** 3 if t < 0.5 else 1.0 - (-2.0 * t + 2.0) ** 3 / 2.0


def sine_in_out(t):
    return -(math.cos(math.pi * t) - 1.0) / 2.0


def back_out(t):
    return 1.0 + 2.70158 * (t - 1.0) ** 3 + 1.70158 * (t - 1.0) ** 2


def clamp01(x):
    return min(1.0, max(0.0, x))


def summarize(cid, source, kind, duration, values, unit, notes=""):
    v = np.asarray(values, dtype=np.float64)
    deltas = np.abs(np.diff(v))
    mean = float(deltas.mean()) if len(deltas) else 0.0
    std = float(deltas.std()) if len(deltas) else 0.0
    return {
        "id": cid, "source": source, "kind": kind, "unit": unit,
        "duration_s": round(duration, 4), "sample_hz": HZ,
        "samples": int(len(v)),
        "max_delta": round(float(deltas.max()) if len(deltas) else 0.0, 6),
        "mean_delta": round(mean, 6),
        "delta_cv": round(std / mean, 3) if mean > 0 else 0.0,
        "spikiness_max_over_mean": round(float(deltas.max()) / mean, 2) if mean > 0 else 0.0,
        "peak_rate_per_sec": round(float(deltas.max()) * HZ, 3) if len(deltas) else 0.0,
        "notes": notes,
    }


def sample(fn, duration):
    n = int(round(duration * HZ))
    return [fn(i / HZ) for i in range(n + 1)]


def main() -> int:
    curves = []

    # --- Continuous eased one-shots -------------------------------------
    curves.append(summarize(
        "fade-envelope-out", "anim.rs drive_transition_fade (FADE_OUT_SECS=0.3)",
        "continuous", 0.3,
        sample(lambda t: cubic_in_out(clamp01(t / 0.3)), 0.3), "alpha"))
    curves.append(summarize(
        "mood-pop-scale", "waifu/mod.rs animate_mood_pop (0.15s, peak 1.08, BackOut halves)",
        "continuous", 0.15,
        sample(lambda t: (1.0 + 0.08 * back_out(2.0 * (t / 0.15))) if t < 0.075
               else (1.08 - 0.08 * back_out(2.0 * (t / 0.15 - 0.5))), 0.15), "scale"))
    curves.append(summarize(
        "outage-banner-slide-in", "outage.rs animate_banner_entrance (0.35s BackOut, -80px -> 0)",
        "continuous", 0.35,
        sample(lambda t: -80.0 + 80.0 * back_out(clamp01(t / 0.35)), 0.35), "px",
        notes="BackOut overshoots: top goes past 0 px briefly (the authored alarm feel)."))
    curves.append(summarize(
        "outage-banner-slide-out", "outage.rs animate_banner_exit (0.25s CubicInOut)",
        "continuous", 0.25,
        sample(lambda t: -80.0 * cubic_in_out(clamp01(t / 0.25)), 0.25), "px"))
    curves.append(summarize(
        "results-banner-pop", "results.rs EntranceKind::BannerPop (0.3s BackOut, 0.8 -> 1.0)",
        "continuous", 0.3,
        sample(lambda t: 0.8 + 0.2 * back_out(clamp01(t / 0.3)), 0.3), "scale"))
    curves.append(summarize(
        "results-fade-in", "results.rs EntranceKind::FadeIn (0.2s CubicOut)",
        "continuous", 0.2,
        sample(lambda t: cubic_out(clamp01(t / 0.2)), 0.2), "alpha"))
    curves.append(summarize(
        "results-slide-up", "results.rs EntranceKind::SlideUp (0.25s CubicOut, 20px)",
        "continuous", 0.25,
        sample(lambda t: 20.0 * (1.0 - cubic_out(clamp01(t / 0.25))), 0.25), "px"))

    # --- Continuous travellers / oscillators ----------------------------
    edge_len = 400.0  # representative edge; PULSE_SPEED=260 px/s (board.rs)
    trav = edge_len / 260.0
    curves.append(summarize(
        "signal-pulse-position", "board.rs move_signal_pulses (SineInOut, 260 px/s, 400px edge)",
        "continuous", trav,
        sample(lambda t: edge_len * sine_in_out(clamp01(t / trav)), trav), "px",
        notes="Peak velocity is (pi/2)x mean velocity by SineInOut construction."))
    curves.append(summarize(
        "glow-pulse-factor", "glow.wgsl (1 + 0.25 sin(3t))",
        "continuous", 2.0 * math.pi / 3.0,
        sample(lambda t: 1.0 + 0.25 * math.sin(3.0 * t), 2.0 * math.pi / 3.0), "factor"))
    curves.append(summarize(
        "talk-bounce-scale", "dialogue_ui.rs tick_dialogue (1 + 0.035 sin(18t))",
        "continuous", 1.0,
        sample(lambda t: 1.0 + 0.035 * math.sin(18.0 * t), 1.0), "scale"))
    curves.append(summarize(
        "advance-indicator-pulse", "dialogue_ui.rs tick_dialogue (0.55 + 0.45 sin(6t))",
        "continuous", 2.0 * math.pi / 6.0,
        sample(lambda t: 0.55 + 0.45 * math.sin(6.0 * t), 2.0 * math.pi / 6.0), "alpha"))
    curves.append(summarize(
        "atmosphere-drift", "atmosphere.wgsl (0.04 sin(0.25t) uv offset)",
        "continuous", 4.0,
        sample(lambda t: 0.04 * math.sin(0.25 * t), 4.0), "uv"))

    # Pulse-flow brightness profile (pulse.wgsl): mean |dB| over a 512-pt
    # u profile between consecutive 240 Hz frames, worst frame reported.
    us = np.linspace(0.0, 1.0, 512)

    def profile(t):
        head = (0.8 * t) % 1.0
        d = np.abs(us - head)
        d = np.minimum(d, 1.0 - d)
        return 0.25 + np.exp(-(d ** 2) / (0.08 ** 2))

    period = 1.25
    n = int(period * HZ)
    prof_deltas = [float(np.abs(profile((i + 1) / HZ) - profile(i / HZ)).mean())
                   for i in range(n)]
    curves.append({
        "id": "pulse-flow-brightness", "source": "pulse.wgsl + board.rs FiberFlow (speed 0.8, width 0.08)",
        "kind": "continuous-field", "unit": "brightness (mean over u profile)",
        "duration_s": period, "sample_hz": HZ, "samples": n + 1,
        "max_delta": round(max(prof_deltas), 6),
        "mean_delta": round(float(np.mean(prof_deltas)), 6),
        "delta_cv": round(float(np.std(prof_deltas) / np.mean(prof_deltas)), 3),
        "spikiness_max_over_mean": round(max(prof_deltas) / float(np.mean(prof_deltas)), 2),
        "peak_rate_per_sec": round(max(prof_deltas) * HZ, 3),
        "notes": "Head advances 0.8 cycles/s; per-frame head step at 240 Hz is 1/300 of the fiber.",
    })

    # --- Step functions, evaluated exactly as shipped --------------------
    def step_summary(cid, source, duration, step_secs, value_fn, unit, notes=""):
        vals = sample(lambda t: value_fn(min(int(t / step_secs), int(duration / step_secs) - 1)), duration)
        return summarize(cid, source, "step", duration, vals, unit, notes)

    curves.append(step_summary(
        "win-ring-scale", "results.rs animate_win_ring (0.12s steps, CubicOut by frame index)",
        0.72, 0.12, lambda i: 0.5 + 0.7 * cubic_out(i / 6.0), "scale",
        notes="Scale/alpha are continuous curves in the source but are only re-sampled at frame steps."))
    curves.append(step_summary(
        "success-burst-scale", "fx.rs animate_success_bursts (0.1s steps, linear by frame index)",
        0.6, 0.1, lambda i: 0.6 + 0.8 * (i / 6.0), "scale"))
    curves.append(step_summary(
        "companion-frame-index", "waifu/mod.rs animate_companion (0.18s steps, 4-frame loop)",
        0.72, 0.18, lambda i: float(i % 4), "frame index",
        notes="0.18s is 43.2 frames at 240 Hz: authored cadence is 5.56 fps."))
    curves.append(step_summary(
        "typewriter-chars", "dialogue_ui.rs Typewriter (30 chars/s)",
        1.0, 1.0 / 30.0, lambda i: float(i), "chars",
        notes="Quantised reveal by design; one char every 8 frames at 240 Hz."))

    # Storm streak fall is linear by construction: delta is a constant.
    curves.append({
        "id": "storm-streak-fall", "source": "board.rs StormStreak (520 u/s, wrap 1200 -> -900)",
        "kind": "continuous-linear", "unit": "world units", "duration_s": 2100.0 / 520.0,
        "sample_hz": HZ, "samples": int(2100.0 / 520.0 * HZ),
        "max_delta": round(520.0 / HZ, 6), "mean_delta": round(520.0 / HZ, 6),
        "delta_cv": 0.0, "spikiness_max_over_mean": 1.0,
        "peak_rate_per_sec": 520.0,
        "notes": "Constant velocity: per-frame delta is identical every frame (analytic).",
    })

    payload = {"sample_hz": HZ, "curves": curves}
    out = Path(sys.argv[1])
    out.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"{len(curves)} curves evaluated -> {out}")
    for c in curves:
        print(f"{c['id']:26s} kind={c['kind']:16s} max_delta={c['max_delta']:.6f} "
              f"mean={c['mean_delta']:.6f} cv={c['delta_cv']} spike={c['spikiness_max_over_mean']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
