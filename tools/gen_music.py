#!/usr/bin/env python3
# =============================================================================
# gen_music.py — procedurally generate the two Fruit Splat music loops
#
# Pure standard library. Builds a small subtractive synth and a step sequencer,
# then renders two seamless loops as 16-bit mono 44.1kHz WAV into assets/:
#   music_game.wav  — 16 bars at 132 BPM, drums + bass + arpeggio + lead
#   music_menu.wav  —  8 bars at  96 BPM, soft pad + gentle arpeggio, no drums
#
# Both loop cleanly because every voice decays inside its own bar. Regenerate:
#     python3 tools/gen_music.py
# =============================================================================

import array
import math
import os
import random
import sys
import wave

SR = 44100
ASSETS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets")

NOTE_OFFSETS = {
    "C": 0, "C#": 1, "D": 2, "D#": 3, "E": 4, "F": 5,
    "F#": 6, "G": 7, "G#": 8, "A": 9, "A#": 10, "B": 11,
}


# -----------------------------------------------------------------------------
# Note and buffer helpers
# -----------------------------------------------------------------------------

def nf(name):
    """Frequency of a note name like 'A4' or 'F#3'. A4 = 440Hz."""
    pitch, octave = name[:-1], int(name[-1])
    semitones = NOTE_OFFSETS[pitch] + (octave - 4) * 12 - 9
    return 440.0 * (2.0 ** (semitones / 12.0))


def normalize(buf, peak=0.82):
    hi = max((abs(x) for x in buf), default=0.0)
    if hi < 1e-9:
        return buf
    k = peak / hi
    return [x * k for x in buf]


def write_wav(name, buf):
    os.makedirs(ASSETS, exist_ok=True)
    path = os.path.join(ASSETS, name)

    ints = array.array("h", (int(max(-1.0, min(1.0, x)) * 32767) for x in buf))
    if sys.byteorder == "big":
        ints.byteswap()

    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(ints.tobytes())

    size_kb = os.path.getsize(path) / 1024.0
    print("  {:<18} {:>7.1f} KB  ({:.1f}s)".format(name, size_kb, len(buf) / SR))


# -----------------------------------------------------------------------------
# Voices
# -----------------------------------------------------------------------------

def add_note(buf, at, dur, freq, kind="tri", vol=0.2, decay=6.0, detune=0.0):
    """
    Mix one plucked note into `buf` at `at` seconds. `decay` sets how fast it
    dies away; a small `detune` adds a second voice slightly sharp for width.
    """
    start = int(at * SR)
    n = int(dur * SR)
    voices = [freq] if detune <= 0.0 else [freq, freq * (1.0 + detune)]

    for v in voices:
        phase = 0.0
        for i in range(n):
            idx = start + i
            if idx >= len(buf):
                break
            t = i / SR
            # Short attack keeps plucks from clicking at note onset.
            attack = min(1.0, t / 0.006)
            env = attack * math.exp(-t * decay)
            phase += v / SR
            p = phase % 1.0
            if kind == "sine":
                s = math.sin(2.0 * math.pi * p)
            elif kind == "square":
                s = 1.0 if p < 0.5 else -1.0
            elif kind == "tri":
                s = 4.0 * abs(p - 0.5) - 1.0
            else:
                s = 2.0 * p - 1.0
            buf[idx] += s * env * vol / len(voices)


def add_pad(buf, at, dur, freq, vol=0.12):
    """A soft sustained sine with slow swell — used for the menu chords."""
    start = int(at * SR)
    n = int(dur * SR)
    phase = 0.0
    for i in range(n):
        idx = start + i
        if idx >= len(buf):
            break
        t = i / SR
        # Swell in over the first 25% and back out over the last 35%.
        swell = min(1.0, t / (dur * 0.25)) * min(1.0, (dur - t) / (dur * 0.35))
        vib = 1.0 + 0.0015 * math.sin(2.0 * math.pi * 4.5 * t)
        phase += freq * vib / SR
        buf[idx] += math.sin(2.0 * math.pi * phase) * swell * vol


def add_kick(buf, at, vol=0.5):
    """Sine sweeping 130Hz → 45Hz: the thump on beats 1 and 3."""
    start = int(at * SR)
    n = int(0.14 * SR)
    phase = 0.0
    for i in range(n):
        idx = start + i
        if idx >= len(buf):
            break
        t = i / SR
        f = 45.0 + 85.0 * math.exp(-t * 32.0)
        phase += f / SR
        buf[idx] += math.sin(2.0 * math.pi * phase) * math.exp(-t * 16.0) * vol


def add_snare(buf, at, vol=0.3):
    """Noise burst plus a 190Hz body, on beats 2 and 4."""
    start = int(at * SR)
    n = int(0.13 * SR)
    phase = 0.0
    for i in range(n):
        idx = start + i
        if idx >= len(buf):
            break
        t = i / SR
        env = math.exp(-t * 26.0)
        phase += 190.0 / SR
        body = math.sin(2.0 * math.pi * phase) * 0.35
        buf[idx] += (random.uniform(-1.0, 1.0) * 0.65 + body) * env * vol


def add_hat(buf, at, vol=0.12):
    """A very short, very bright noise tick on the eighths."""
    start = int(at * SR)
    n = int(0.03 * SR)
    prev = 0.0
    for i in range(n):
        idx = start + i
        if idx >= len(buf):
            break
        x = random.uniform(-1.0, 1.0)
        # Crude high-pass: the sample-to-sample difference keeps only the fizz.
        hp = x - prev
        prev = x
        buf[idx] += hp * math.exp(-(i / SR) * 120.0) * vol


# -----------------------------------------------------------------------------
# Arrangements
# -----------------------------------------------------------------------------

# I – V – vi – IV in C major: the chord tones each section arpeggiates over.
GAME_CHORDS = [
    ("C2", ["C4", "E4", "G4"]),
    ("G2", ["G3", "B3", "D4"]),
    ("A2", ["A3", "C4", "E4"]),
    ("F2", ["F3", "A3", "C4"]),
]

# Which chord tone each eighth-note of a bar plays — an up-down arpeggio.
ARP_PATTERN = [0, 1, 2, 1, 0, 2, 1, 2]


def build_game_track():
    """16 bars at 132 BPM. Two passes over the progression; the lead joins on
    the second so the loop has somewhere to build to."""
    bpm = 132.0
    beat = 60.0 / bpm
    bar = beat * 4.0
    bars = 16
    buf = [0.0] * int(bar * bars * SR)

    for b in range(bars):
        at = b * bar
        root_name, chord = GAME_CHORDS[(b // 2) % len(GAME_CHORDS)]
        second_pass = b >= 8

        # Drums — kick on 1 and 3, snare on 2 and 4, hats on every eighth.
        add_kick(buf, at + 0.0 * beat)
        add_kick(buf, at + 2.0 * beat)
        add_snare(buf, at + 1.0 * beat)
        add_snare(buf, at + 3.0 * beat)
        for e in range(8):
            add_hat(buf, at + e * beat * 0.5, vol=0.10 if e % 2 else 0.14)

        # Bass — root on the beat, with an octave lift on the offbeats.
        root = nf(root_name)
        for e in range(8):
            f = root if e % 4 != 3 else root * 2.0
            add_note(buf, at + e * beat * 0.5, beat * 0.45, f,
                     kind="square", vol=0.16, decay=9.0)

        # Arpeggio — the harmonic bed, one chord tone per eighth.
        for e, step in enumerate(ARP_PATTERN):
            add_note(buf, at + e * beat * 0.5, beat * 0.5, nf(chord[step]),
                     kind="tri", vol=0.13, decay=7.0, detune=0.004)

        # Lead — sparse quarter notes an octave up, second pass only.
        if second_pass:
            for e in (0, 2, 3):
                step = ARP_PATTERN[(e * 2) % len(ARP_PATTERN)]
                add_note(buf, at + e * beat, beat * 0.9, nf(chord[step]) * 2.0,
                         kind="tri", vol=0.11, decay=4.0, detune=0.006)

    return normalize(buf, 0.80)


# Softer, slower progression for the title screen.
MENU_CHORDS = [
    ["C4", "E4", "G4", "B4"],
    ["A3", "C4", "E4", "G4"],
    ["F3", "A3", "C4", "E4"],
    ["G3", "B3", "D4", "F4"],
]


def build_menu_track():
    """8 bars at 96 BPM: sustained pad chords with a gentle arpeggio, no drums."""
    bpm = 96.0
    beat = 60.0 / bpm
    bar = beat * 4.0
    bars = 8
    buf = [0.0] * int(bar * bars * SR)

    for b in range(bars):
        at = b * bar
        chord = MENU_CHORDS[b % len(MENU_CHORDS)]

        # Pad — the whole chord held for the bar.
        for name in chord:
            add_pad(buf, at, bar * 0.97, nf(name), vol=0.10)

        # Low root, an octave and a half below the chord.
        add_pad(buf, at, bar * 0.97, nf(chord[0]) * 0.25, vol=0.09)

        # Arpeggio — one note per beat, drifting up through the chord.
        for e in range(4):
            name = chord[e % len(chord)]
            add_note(buf, at + e * beat, beat * 1.4, nf(name) * 2.0,
                     kind="sine", vol=0.09, decay=3.2, detune=0.003)

    return normalize(buf, 0.72)


def main():
    # Fixed seed so the noise in the drums is identical on every regeneration.
    random.seed(20260806)

    print("Generating music into assets/ …")
    write_wav("music_game.wav", build_game_track())
    write_wav("music_menu.wav", build_menu_track())
    print("Done.")


if __name__ == "__main__":
    main()
