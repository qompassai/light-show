#!/usr/bin/env python3
"""Procedurally synthesize Light Show's Megaman-esque chiptune soundtrack.

This is a from-scratch 4-channel synthesizer modeled on the NES 2A03 sound
chip's channel layout (two pulse/square oscillators with selectable duty
cycle, one triangle wave, one noise channel) — the same chip that drove
the Mega Man series' scores. Composing directly against that channel
layout (bouncy duty-cycle lead + fast arpeggiated harmony + triangle bass
+ noise percussion) is what gives the output its "Megaman-esque" identity,
rather than trying to imitate the timbre after the fact.

Fully procedural and license-free (no samples, no third-party audio) —
see docs/CREDITS.md for the corresponding entry.

Output: `.wav` files rendered here, then transcoded to `.ogg` (Vorbis,
matching the `vorbis` Cargo feature already enabled on `bevy_audio` in
game/Cargo.toml) via `ffmpeg`, written to `game/assets/audio/`.

Usage:
    python3 tools/gen_chiptune_music.py

Requires: numpy, ffmpeg on PATH.
"""

from __future__ import annotations

import subprocess
import tempfile
import wave
from dataclasses import dataclass
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "game" / "assets" / "audio"
SR = 44100

# --- Note frequencies -------------------------------------------------
# Standard 12-TET, A4 = 440 Hz. `None` in a channel's event list means a
# rest of that duration.
_NOTE_INDEX = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def note(name: str) -> float:
    """`"A4"` -> 440.0, `"C#3"` -> 138.59, etc."""
    if name[1] in ("#", "b"):
        letter, accidental, octave = name[0], name[1], int(name[2:])
    else:
        letter, accidental, octave = name[0], "", int(name[1:])
    semitone = _NOTE_INDEX[letter] + {"#": 1, "b": -1, "": 0}[accidental]
    midi = (octave + 1) * 12 + semitone
    return 440.0 * 2.0 ** ((midi - 69) / 12.0)


REST = None

# --- Oscillators --------------------------------------------------------


def pulse_wave(freq: float, seconds: float, duty: float, sr: int = SR) -> np.ndarray:
    t = np.arange(int(seconds * sr)) / sr
    phase = (t * freq) % 1.0
    return np.where(phase < duty, 1.0, -1.0).astype(np.float32)


def triangle_wave(freq: float, seconds: float, sr: int = SR) -> np.ndarray:
    t = np.arange(int(seconds * sr)) / sr
    phase = (t * freq) % 1.0
    return (4.0 * np.abs(phase - 0.5) - 1.0).astype(np.float32)


def noise_burst(seconds: float, sr: int = SR, seed: int = 0) -> np.ndarray:
    rng = np.random.default_rng(seed)
    return rng.uniform(-1.0, 1.0, int(seconds * sr)).astype(np.float32)


def envelope(n: int, attack: float, release: float) -> np.ndarray:
    """Linear attack/release envelope, `attack`/`release` in seconds."""
    env = np.ones(n, dtype=np.float32)
    a = min(int(attack * SR), n // 2 if n > 0 else 0)
    r = min(int(release * SR), n - a if n > a else 0)
    if a > 0:
        env[:a] = np.linspace(0.0, 1.0, a, dtype=np.float32)
    if r > 0:
        env[n - r :] = np.linspace(1.0, 0.0, r, dtype=np.float32)
    return env


# --- Sequencer -----------------------------------------------------------

Event = tuple  # (note_name_or_None, duration_in_sixteenths)


@dataclass
class Channel:
    kind: str  # "pulse", "triangle", "noise"
    events: list[Event]
    volume: float = 0.25
    duty: float = 0.5  # only used by "pulse"


def render_channel(channel: Channel, bpm: float, sr: int = SR) -> np.ndarray:
    sixteenth = 60.0 / bpm / 4.0
    chunks: list[np.ndarray] = []
    seed = 0
    for pitch, dur_sixteenths in channel.events:
        seconds = sixteenth * dur_sixteenths
        n = max(int(seconds * sr), 1)
        if pitch is None:
            chunks.append(np.zeros(n, dtype=np.float32))
            continue
        freq = note(pitch)
        if channel.kind == "pulse":
            wave_data = pulse_wave(freq, seconds, channel.duty, sr)
        elif channel.kind == "triangle":
            wave_data = triangle_wave(freq, seconds, sr)
        else:
            wave_data = noise_burst(seconds, sr, seed=seed)
            seed += 1
        wave_data = wave_data[:n] * envelope(n, attack=0.004, release=min(seconds * 0.35, 0.05))
        chunks.append(wave_data * channel.volume)
    return np.concatenate(chunks) if chunks else np.zeros(0, dtype=np.float32)


def mix(channels: list[Channel], bpm: float, loop: bool, sr: int = SR) -> np.ndarray:
    rendered = [render_channel(c, bpm, sr) for c in channels]
    length = max((len(r) for r in rendered), default=0)
    buf = np.zeros(length, dtype=np.float32)
    for r in rendered:
        buf[: len(r)] += r
    peak = np.max(np.abs(buf)) if length else 0.0
    if peak > 1e-6:
        buf = buf / peak * 0.9
    # A tiny fade at both ends guarantees a click-free seam when Bevy's
    # `PlaybackSettings::LOOP` splices the end of the buffer straight
    # back to the start with no crossfade of its own.
    fade_n = min(int(0.012 * sr), length // 2 if length else 0)
    if fade_n > 0:
        buf[:fade_n] *= np.linspace(0.0, 1.0, fade_n, dtype=np.float32)
        buf[-fade_n:] *= np.linspace(1.0, 0.0, fade_n, dtype=np.float32)
    del loop  # loop-safety is handled unconditionally above; kept for call-site clarity
    return buf


def write_wav(path: Path, buf: np.ndarray, sr: int = SR) -> None:
    pcm = np.clip(buf * 32767.0, -32768, 32767).astype(np.int16)
    with wave.open(str(path), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(sr)
        f.writeframes(pcm.tobytes())


def to_ogg(wav_path: Path, ogg_path: Path) -> None:
    subprocess.run(
        [
            "ffmpeg",
            "-y",
            "-loglevel",
            "error",
            "-i",
            str(wav_path),
            "-c:a",
            "libvorbis",
            "-q:a",
            "4",
            str(ogg_path),
        ],
        check=True,
    )


# --- Compositions -------------------------------------------------------
# Every melodic/bass/arp channel's event durations must sum to the same
# total, and that total must match across all channels in a track, or
# the mix silently truncates to the shortest one. `_assert_channels_align`
# catches that at generation time instead of at first (silent) playtest.


def _assert_channels_align(channels: list[Channel]) -> None:
    totals = {sum(d for _, d in c.events) for c in channels}
    assert len(totals) == 1, f"channel lengths disagree (in 16th notes): {totals}"


def menu_theme() -> tuple[list[Channel], float]:
    """Bright, bouncy title-screen loop — 2 bars at 116 BPM."""
    lead = Channel(
        "pulse",
        [
            ("E5", 2), ("G5", 2), ("A5", 2), ("G5", 2),
            ("E5", 2), ("D5", 2), ("E5", 4),
            ("G5", 2), ("A5", 2), ("C6", 2), ("B5", 2),
            ("A5", 2), ("G5", 2), ("E5", 4),
        ],
        volume=0.22,
        duty=0.5,
    )
    harmony = Channel(
        "pulse",
        [
            ("C5", 4), ("E5", 4), ("C5", 4), ("D5", 4),
            ("E5", 4), ("G5", 4), ("C5", 4), ("B4", 4),
        ],
        volume=0.12,
        duty=0.25,
    )
    bass = Channel(
        "triangle",
        [
            ("C3", 4), ("C3", 4), ("A2", 4), ("A2", 4),
            ("F2", 4), ("F2", 4), ("G2", 4), ("G2", 4),
        ],
        volume=0.28,
    )
    drums = Channel(
        "noise",
        [(None, 2), ("C2", 1), (None, 1)] * 8,
        volume=0.10,
    )
    channels = [lead, harmony, bass, drums]
    _assert_channels_align(channels)
    return channels, 116.0


def playing_theme() -> tuple[list[Channel], float]:
    """Steady, determined puzzle-solving groove — minor key, 122 BPM."""
    lead = Channel(
        "pulse",
        [
            ("A4", 3), ("C5", 1), ("D5", 2), ("C5", 2),
            ("A4", 2), (None, 2), ("G4", 3), ("A4", 1),
            ("A4", 3), ("C5", 1), ("E5", 2), ("D5", 2),
            ("C5", 2), (None, 2), ("A4", 4),
        ],
        volume=0.20,
        duty=0.5,
    )
    arp = Channel(
        "pulse",
        [("A3", 1), ("C4", 1), ("E4", 1), ("C4", 1)] * 8,
        volume=0.10,
        duty=0.125,
    )
    bass = Channel(
        "triangle",
        [
            ("A2", 4), ("A2", 4), ("F2", 4), ("F2", 4),
            ("A2", 4), ("A2", 4), ("G2", 4), ("E2", 4),
        ],
        volume=0.30,
    )
    drums = Channel(
        "noise",
        [("C2", 1), (None, 1), (None, 1), ("C2", 1)] * 8,
        volume=0.11,
    )
    channels = [lead, arp, bass, drums]
    _assert_channels_align(channels)
    return channels, 122.0


def outage_theme() -> tuple[list[Channel], float]:
    """Urgent, faster alarm-state loop — 150 BPM, tighter/more dissonant."""
    lead = Channel(
        "pulse",
        [
            ("D5", 1), ("F5", 1), ("D5", 1), ("F5", 1),
            ("C5", 1), ("F5", 1), ("C5", 1), ("F5", 1),
            ("Bb4", 1), ("F5", 1), ("Bb4", 1), ("F5", 1),
            ("A4", 1), ("F5", 1), ("A4", 1), ("F5", 1),
        ]
        * 2,
        volume=0.20,
        duty=0.25,
    )
    arp = Channel(
        "pulse",
        [("D4", 1), ("F4", 1), ("Ab4", 1), ("F4", 1)] * 8,
        volume=0.12,
        duty=0.5,
    )
    bass = Channel(
        "triangle",
        [("D2", 2), ("D2", 2)] * 8,
        volume=0.30,
    )
    drums = Channel(
        "noise",
        [("C2", 1), ("C2", 1), (None, 1), ("C2", 1)] * 8,
        volume=0.14,
    )
    channels = [lead, arp, bass, drums]
    _assert_channels_align(channels)
    return channels, 150.0


def victory_jingle() -> tuple[list[Channel], float]:
    """Short ascending triumphant fanfare — non-looping, ~2.3s at 140 BPM."""
    lead = Channel(
        "pulse",
        [("C5", 2), ("E5", 2), ("G5", 2), ("C6", 4), (None, 2), ("G5", 1), ("C6", 5)],
        volume=0.26,
        duty=0.5,
    )
    harmony = Channel(
        "pulse",
        [("E4", 6), ("G4", 4), (None, 2), ("E5", 6)],
        volume=0.14,
        duty=0.25,
    )
    bass = Channel(
        "triangle",
        [("C3", 6), ("G2", 4), (None, 2), ("C3", 6)],
        volume=0.30,
    )
    channels = [lead, harmony, bass]
    _assert_channels_align(channels)
    return channels, 140.0


def failure_jingle() -> tuple[list[Channel], float]:
    """Short descending "not this time" cue — non-looping, ~2s at 110 BPM."""
    lead = Channel(
        "pulse",
        [("A4", 3), ("G4", 3), ("F4", 3), ("E4", 3), ("D4", 6)],
        volume=0.22,
        duty=0.5,
    )
    bass = Channel(
        "triangle",
        [("A2", 6), ("F2", 6), ("D2", 6)],
        volume=0.28,
    )
    channels = [lead, bass]
    _assert_channels_align(channels)
    return channels, 110.0


TRACKS = {
    "menu_theme": (menu_theme, True),
    "playing_theme": (playing_theme, True),
    "outage_theme": (outage_theme, True),
    "victory_jingle": (victory_jingle, False),
    "failure_jingle": (failure_jingle, False),
}


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # Intermediate WAVs are scratch data only the OGG transcode step needs;
    # a real temp directory (auto-removed even if `to_ogg` raises) keeps the
    # repo tree clean instead of leaving a stray `tools/_audio_tmp/` behind.
    with tempfile.TemporaryDirectory(prefix="light_show_audio_") as tmp:
        tmp_dir = Path(tmp)
        for name, (compose, loop) in TRACKS.items():
            channels, bpm = compose()
            buf = mix(channels, bpm, loop=loop)
            wav_path = tmp_dir / f"{name}.wav"
            ogg_path = OUT_DIR / f"{name}.ogg"
            write_wav(wav_path, buf)
            to_ogg(wav_path, ogg_path)
            print(f"wrote {ogg_path} ({len(buf) / SR:.2f}s)")


if __name__ == "__main__":
    main()
