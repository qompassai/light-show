#!/usr/bin/env python3
"""Generate Light Show's audio pack: new SFX, per-companion chiptune themes,
per-companion ambient hums.

Tool choice (deliberate, per job):
- SFX one-shots: numpy synthesis (exact envelopes, reproducible; matches the
  square-wave provenance of the existing SFX set).
- Character themes: the proven NES-2A03 engine in tools/gen_chiptune_music.py
  (imported, not forked) so the new motifs are stylistically identical to
  the existing soundtrack.
- Ambience hums: numpy (layered sines + filtered noise, seamless loops).
- OGG transcode: ffmpeg/libvorbis, same settings as gen_chiptune_music.py.

Outputs (all 44.1 kHz; SFX mono 16-bit WAV like the existing set):
    game/assets/sfx/*.wav            9 new one-shots
    game/assets/music/themes/*.ogg   8 companion theme loops
    game/assets/music/ambience/*.ogg 8 companion ambience hum loops

Usage: python3 tools/gen_audio_pack.py   (run from the repo root)
Requires: numpy, ffmpeg on PATH.
"""

from __future__ import annotations

import sys
import tempfile
import wave
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import gen_chiptune_music as chip  # noqa: E402  (the proven 2A03 engine)

SR = 44100
SFX_DIR = ROOT / "game" / "assets" / "sfx"
THEME_DIR = ROOT / "game" / "assets" / "music" / "themes"
AMB_DIR = ROOT / "game" / "assets" / "music" / "ambience"


# --- numpy SFX helpers -----------------------------------------------------

def _t(dur: float) -> np.ndarray:
    return np.arange(int(dur * SR), dtype=np.float64) / SR


def _norm(buf: np.ndarray, peak: float = 0.9) -> np.ndarray:
    m = np.max(np.abs(buf))
    if m > 1e-9:
        buf = buf / m * peak
    return buf.astype(np.float32)


def _write_wav(path: Path, buf: np.ndarray) -> None:
    pcm = np.clip(buf * 32767.0, -32768, 32767).astype(np.int16)
    with wave.open(str(path), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(SR)
        f.writeframes(pcm.tobytes())


def _sine(freq: float, dur: float) -> np.ndarray:
    t = _t(dur)
    return np.sin(2.0 * np.pi * freq * t)


def _square(freq: float, dur: float) -> np.ndarray:
    t = _t(dur)
    return np.sign(np.sin(2.0 * np.pi * freq * t)).astype(np.float64)


def _sweep(f0: float, f1: float, dur: float, kind: str = "sine") -> np.ndarray:
    """Exponential frequency sweep from f0 to f1."""
    t = _t(dur)
    # instantaneous frequency interpolated exponentially; integrate phase
    k = np.log(f1 / f0) / dur
    phase = 2.0 * np.pi * f0 * (np.exp(k * t) - 1.0) / k
    osc = np.sin(phase) if kind == "sine" else np.sign(np.sin(phase))
    return osc.astype(np.float64)


def _adsr(n: int, a: float, d: float, s: float, r: float) -> np.ndarray:
    """Attack/decay/sustain/release envelope, times in seconds."""
    env = np.ones(n)
    na, nd, nr = int(a * SR), int(d * SR), int(r * SR)
    ns = max(n - na - nd - nr, 0)
    if na:
        env[:na] = np.linspace(0, 1, na)
    if nd:
        env[na:na + nd] = np.linspace(1, s, nd)
    if nr:
        env[n - nr:] = np.linspace(s if ns else env[n - nr - 1], 0, nr)
    return env


def _place(base: np.ndarray, sig: np.ndarray, at: float) -> np.ndarray:
    i = int(at * SR)
    j = min(i + len(sig), len(base))
    base[i:j] += sig[: j - i]
    return base


# --- SFX definitions --------------------------------------------------------

def sfx_hover() -> np.ndarray:
    """Tiny high blip for button hover."""
    return _norm(_sine(1568.0, 0.07) * _adsr(int(0.07 * SR), 0.004, 0.01, 0.4, 0.05))


def sfx_error() -> np.ndarray:
    """Low detuned buzz for invalid actions."""
    dur, n = 0.40, int(0.40 * SR)
    t = _t(dur)
    glide = 1.0 - 0.12 * (t / dur)
    sig = np.sign(np.sin(2 * np.pi * 110.0 * glide * t)) + np.sign(
        np.sin(2 * np.pi * 116.5 * glide * t))
    return _norm(sig * 0.5 * _adsr(n, 0.008, 0.05, 0.7, 0.12))


def sfx_zap() -> np.ndarray:
    """Rising connection zap: click + upward sweep."""
    dur, n = 0.30, int(0.30 * SR)
    sig = _sweep(350.0, 2800.0, dur)
    click = np.random.default_rng(7).standard_normal(int(0.012 * SR)) * 0.6
    sig[: len(click)] += click
    return _norm(sig * _adsr(n, 0.004, 0.06, 0.8, 0.08))


def sfx_alarm_urgent() -> np.ndarray:
    """Fast two-tone klaxon (escalation of the existing outage alarm)."""
    dur = 0.90
    t = _t(dur)
    tone = np.where(((t / 0.15).astype(int) % 2) == 0, 660.0, 880.0)
    sig = np.sign(np.sin(2 * np.pi * tone * t))
    return _norm(sig * 0.5 * _adsr(len(t), 0.01, 0.05, 0.85, 0.10))


def sfx_alarm_soft() -> np.ndarray:
    """Gentle warning chime (non-critical alert)."""
    dur = 0.60
    sig = np.zeros(int(dur * SR))
    n1 = _sine(659.25, 0.25) * _adsr(int(0.25 * SR), 0.03, 0.05, 0.6, 0.10)
    n2 = _sine(523.25, 0.35) * _adsr(int(0.35 * SR), 0.03, 0.05, 0.6, 0.15)
    sig = _place(_place(sig, n1, 0.0), n2, 0.25)
    return _norm(sig)


def sfx_menu_open() -> np.ndarray:
    """Soft upward whoosh for opening panels."""
    dur = 0.25
    return _norm(_sweep(280.0, 880.0, dur) * _adsr(int(dur * SR), 0.03, 0.08, 0.6, 0.10))


def sfx_menu_close() -> np.ndarray:
    """Soft downward whoosh for closing panels."""
    dur = 0.25
    return _norm(_sweep(880.0, 280.0, dur) * _adsr(int(dur * SR), 0.03, 0.08, 0.6, 0.10))


def sfx_tab_switch() -> np.ndarray:
    """Click-slide for tab switches."""
    dur = 0.14
    n = int(dur * SR)
    sig = _sweep(900.0, 1400.0, dur) * 0.7
    click = np.random.default_rng(21).standard_normal(int(0.008 * SR)) * 0.5
    sig[: len(click)] += click
    return _norm(sig * _adsr(n, 0.003, 0.03, 0.5, 0.06))


def sfx_fanfare() -> np.ndarray:
    """Track-complete fanfare: rising arp + held chord."""
    dur = 1.20
    sig = np.zeros(int(dur * SR))
    arp_notes = ["C5", "E5", "G5", "C6"]
    for i, name in enumerate(arp_notes):
        f = chip.note(name)
        bit = _square(f, 0.13) * _adsr(int(0.13 * SR), 0.005, 0.03, 0.7, 0.05)
        sig = _place(sig, bit, i * 0.12)
    chord = sum(_square(chip.note(nm), 0.62) for nm in ["C5", "E5", "G5", "C6"]) / 4.0
    chord = chord * _adsr(int(0.62 * SR), 0.01, 0.10, 0.7, 0.25)
    sig = _place(sig, chord, 0.50)
    return _norm(sig)


SFX = {
    "hover": sfx_hover,
    "error": sfx_error,
    "zap": sfx_zap,
    "alarm_urgent": sfx_alarm_urgent,
    "alarm_soft": sfx_alarm_soft,
    "menu_open": sfx_menu_open,
    "menu_close": sfx_menu_close,
    "tab_switch": sfx_tab_switch,
    "fanfare": sfx_fanfare,
}


# --- Character themes (2A03 engine) ------------------------------------------
# 8 bars each (128 sixteenths). Lead motif per companion over a I–vi–IV–V-ish
# progression voiced for their discipline's mood.

def _arp(tones: list[str], bars2_dur: int = 32) -> list[tuple]:
    """Eighth-note arpeggio cycling chord tones for a 2-bar slot."""
    ev: list[tuple] = []
    i = 0
    while sum(d for _, d in ev) < bars2_dur:
        ev.append((tones[i % len(tones)], 2))
        i += 1
    return ev


def _theme(prog: list[tuple[str, list[str]]], lead: list[tuple],
           bass_octave_shift: int, bpm: float, duty: float = 0.5,
           drum_density: str = "quarter") -> tuple[list, float]:
    """prog: 4 × (bass_root, chord_tones), each covering 2 bars."""
    bass_ev: list[tuple] = []
    harm_ev: list[tuple] = []
    for root, tones in prog:
        bass_ev += [(root, 16), (root, 16)]
        harm_ev += _arp(tones)
    bass = chip.Channel("triangle", bass_ev, volume=0.30)
    harm = chip.Channel("pulse", harm_ev, volume=0.11, duty=0.25)
    lead_ch = chip.Channel("pulse", lead, volume=0.22, duty=duty)
    if drum_density == "quarter":
        drums = chip.Channel("noise", [(None, 2), ("C2", 1), (None, 1)] * 32, volume=0.09)
    elif drum_density == "eighth":
        drums = chip.Channel("noise", [("C2", 1), (None, 1)] * 64, volume=0.08)
    else:  # sparse
        drums = chip.Channel("noise", [(None, 6), ("C2", 1), (None, 1)] * 16, volume=0.08)
    channels = [lead_ch, harm, bass, drums]
    chip._assert_channels_align(channels)
    return channels, bpm


def theme_seraphine():
    """Warm C-major mentor theme, 112 BPM."""
    prog = [("C3", ["C5", "E5", "G5"]), ("F2", ["F4", "A4", "C5"]),
            ("G2", ["G4", "B4", "D5"]), ("C3", ["C5", "E5", "G5"])]
    phrase = [("E5", 4), ("G5", 4), ("A5", 4), ("G5", 4),
              ("E5", 4), ("D5", 4), ("E5", 8)]
    var = [("E5", 4), ("G5", 4), ("A5", 4), ("C6", 4),
           ("B5", 4), ("G5", 4), ("E5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 112.0)


def theme_ondine():
    """Fluid A-minor coax theme, 118 BPM."""
    prog = [("A2", ["A4", "C5", "E5"]), ("F2", ["F4", "A4", "C5"]),
            ("C3", ["C5", "E5", "G5"]), ("G2", ["G4", "B4", "D5"])]
    phrase = [("A5", 4), ("C6", 4), ("B5", 4), ("A5", 4),
              ("G5", 4), ("E5", 4), ("D5", 8)]
    var = [("A5", 4), ("C6", 4), ("D6", 4), ("C6", 4),
           ("B5", 4), ("A5", 4), ("G5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 118.0, duty=0.25)


def theme_linka():
    """Driving E-minor mobile theme, 134 BPM."""
    prog = [("E2", ["E4", "G4", "B4"]), ("C3", ["C5", "E5", "G5"]),
            ("G2", ["G4", "B4", "D5"]), ("D3", ["D5", "F#5", "A5"])]
    phrase = [("E5", 2), ("E5", 2), ("G5", 4), ("E5", 2), ("D5", 2), ("E5", 4),
              ("B4", 4), ("D5", 4), ("E5", 8)]
    var = [("E5", 2), ("E5", 2), ("G5", 4), ("A5", 4), ("G5", 4),
           ("F#5", 4), ("E5", 4), ("D5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 134.0, drum_density="eighth")


def theme_lattice():
    """Structured G-major ethernet march, 116 BPM."""
    prog = [("G2", ["G4", "B4", "D5"]), ("C3", ["C5", "E5", "G5"]),
            ("G2", ["G4", "B4", "D5"]), ("D3", ["D5", "F#5", "A5"])]
    phrase = [("G5", 4), ("B5", 4), ("D6", 4), ("B5", 4),
              ("A5", 4), ("G5", 4), ("F#5", 8)]
    var = [("G5", 4), ("B5", 4), ("D6", 4), ("G6", 4),
           ("F#6", 4), ("D6", 4), ("B5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 116.0)


def theme_clara():
    """Bouncy D-major provisioning theme, 124 BPM."""
    prog = [("D3", ["D5", "F#5", "A5"]), ("G2", ["G4", "B4", "D5"]),
            ("A2", ["A4", "C#5", "E5"]), ("D3", ["D5", "F#5", "A5"])]
    phrase = [("D5", 2), ("F#5", 2), ("A5", 4), ("F#5", 2), ("E5", 2), ("D5", 4),
              ("E5", 4), ("F#5", 4), ("D5", 8)]
    var = [("D5", 2), ("F#5", 2), ("A5", 4), ("D6", 4), ("C#6", 4),
           ("A5", 4), ("E5", 4), ("F#5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 124.0)


def theme_aino():
    """Sparse F#-minor NOC theme, 104 BPM."""
    prog = [("F#2", ["F#4", "A4", "C#5"]), ("D3", ["D5", "F#5", "A5"]),
            ("A2", ["A4", "C#5", "E5"]), ("E2", ["E4", "G#4", "B4"])]
    phrase = [("F#5", 8), ("A5", 8), ("G#5", 8), ("E5", 8)]
    var = [("F#5", 8), ("C#6", 8), ("B5", 8), ("A5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 104.0, duty=0.25, drum_density="sparse")


def theme_hikari():
    """Bright A-major pentatonic field theme, 128 BPM."""
    prog = [("A2", ["A4", "C#5", "E5"]), ("D3", ["D5", "F#5", "A5"]),
            ("E3", ["E5", "G#5", "B5"]), ("A2", ["A4", "C#5", "E5"])]
    phrase = [("A5", 4), ("C#6", 4), ("B5", 4), ("A5", 4),
              ("G#5", 4), ("E5", 4), ("F#5", 8)]
    var = [("A5", 4), ("C#6", 4), ("E6", 4), ("C#6", 4),
           ("B5", 4), ("A5", 4), ("G#5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 128.0)


def theme_lea():
    """Gentle F-major study theme, 96 BPM."""
    prog = [("F2", ["F4", "A4", "C5"]), ("Bb2", ["Bb4", "D5", "F5"]),
            ("C3", ["C5", "E5", "G5"]), ("F2", ["F4", "A4", "C5"])]
    phrase = [("F5", 8), ("A5", 4), ("G5", 4), ("F5", 8), ("E5", 8)]
    var = [("F5", 8), ("A5", 4), ("C6", 4), ("Bb5", 8), ("A5", 8)]
    return _theme(prog, phrase * 3 + var, 0, 96.0, duty=0.25, drum_density="sparse")


THEMES = {
    "seraphine": theme_seraphine,
    "ondine": theme_ondine,
    "linka": theme_linka,
    "lattice": theme_lattice,
    "clara": theme_clara,
    "aino": theme_aino,
    "hikari": theme_hikari,
    "lea": theme_lea,
}


# --- Ambient hums ------------------------------------------------------------
# Seamless 4 s loops: root sine + 2nd harmonic + slow-LFO'd filtered noise.

HUM_ROOTS = {
    "seraphine": 55.00,   # A1 warm
    "ondine": 65.41,      # C2 fluid
    "linka": 73.42,       # D2 driving
    "lattice": 49.00,      # G1 structured
    "clara": 58.27,        # Bb1 upbeat
    "aino": 46.25,        # F#1 cool
    "hikari": 61.74,      # B1 bright
    "lea": 43.65,         # F1 gentle
}

HUM_DUR = 4.0


def ambience_hum(root: float) -> np.ndarray:
    n = int(HUM_DUR * SR)
    t = np.arange(n, dtype=np.float64) / SR
    # Integer-cycle LFO (2 cycles per loop) keeps the loop seamless.
    lfo = 0.75 + 0.25 * np.sin(2.0 * np.pi * 2.0 * t / HUM_DUR)
    sig = np.sin(2 * np.pi * root * t) * 0.55
    sig += np.sin(2 * np.pi * root * 2 * t) * 0.18
    sig += np.sin(2 * np.pi * root * 3 * t) * 0.07
    rng = np.random.default_rng(int(root * 100))
    noise = rng.standard_normal(n)
    # One-pole lowpass at ~220 Hz for an airy bed.
    alpha = 1.0 - np.exp(-2.0 * np.pi * 220.0 / SR)
    bed = np.zeros(n)
    acc = 0.0
    for i in range(n):
        acc += alpha * (noise[i] - acc)
        bed[i] = acc
    sig = (sig + bed * 0.35) * lfo
    # Crossfade the seam: last 0.15 s blends into the first 0.15 s.
    xf = int(0.15 * SR)
    w = np.linspace(0, 1, xf)
    sig[-xf:] = sig[-xf:] * (1 - w) + sig[:xf] * w
    return _norm(sig, peak=0.5)


# --- main ---------------------------------------------------------------------

def main() -> None:
    SFX_DIR.mkdir(parents=True, exist_ok=True)
    THEME_DIR.mkdir(parents=True, exist_ok=True)
    AMB_DIR.mkdir(parents=True, exist_ok=True)

    manifest: list[str] = []

    print("== SFX ==")
    for name, fn in SFX.items():
        path = SFX_DIR / f"{name}.wav"
        _write_wav(path, fn())
        kb = path.stat().st_size // 1024
        manifest.append(f"sfx/{name}.wav ({kb} KB)")
        print(f"  {path.name}  {kb} KB")

    print("== themes ==")
    with tempfile.TemporaryDirectory(prefix="light_show_audio_pack_") as tmp:
        tmp_dir = Path(tmp)
        for name, compose in THEMES.items():
            channels, bpm = compose()
            buf = chip.mix(channels, bpm, loop=True)
            wav_path = tmp_dir / f"{name}.wav"
            chip.write_wav(wav_path, buf)
            ogg_path = THEME_DIR / f"{name}.ogg"
            chip.to_ogg(wav_path, ogg_path)
            kb = ogg_path.stat().st_size // 1024
            manifest.append(f"music/themes/{name}.ogg ({kb} KB)")
            print(f"  {ogg_path.name}  {kb} KB")

        print("== ambience ==")
        for name, root in HUM_ROOTS.items():
            buf = ambience_hum(root)
            wav_path = tmp_dir / f"{name}_hum.wav"
            chip.write_wav(wav_path, buf)
            ogg_path = AMB_DIR / f"{name}_hum.ogg"
            chip.to_ogg(wav_path, ogg_path)
            kb = ogg_path.stat().st_size // 1024
            manifest.append(f"music/ambience/{name}_hum.ogg ({kb} KB)")
            print(f"  {ogg_path.name}  {kb} KB")

    print(f"\nDone: {len(manifest)} files.")
    for m in manifest:
        print(f"  {m}")


if __name__ == "__main__":
    main()
