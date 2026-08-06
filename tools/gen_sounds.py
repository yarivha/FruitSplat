#!/usr/bin/env python3
# =============================================================================
# gen_sounds.py — procedurally generate every sound effect in Fruit Splat
#
# Pure standard library: no numpy, no samples, no external audio tools. Writes
# 16-bit mono 44.1kHz WAV files into assets/, which src/audio.rs then embeds
# into the binary with include_bytes!. The .wav files are a build-time artifact
# only — regenerate them with:
#     python3 tools/gen_sounds.py
# =============================================================================

import array
import math
import os
import random
import sys
import wave

SR = 44100
ASSETS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets")

# Pop pitch per fruit tier: 0 = blueberry (smallest, highest) … 4 = watermelon.
POP_BASE_HZ = [880.0, 720.0, 580.0, 460.0, 350.0]


# -----------------------------------------------------------------------------
# Signal helpers
# -----------------------------------------------------------------------------

def silence(dur):
    """A buffer of `dur` seconds of zeroes."""
    return [0.0] * int(dur * SR)


def lowpass(buf, cutoff_hz):
    """One-pole low-pass. Used to make noise bursts sound wet rather than hissy."""
    dt = 1.0 / SR
    rc = 1.0 / (2.0 * math.pi * cutoff_hz)
    a = dt / (rc + dt)
    out = [0.0] * len(buf)
    y = 0.0
    for i, x in enumerate(buf):
        y += a * (x - y)
        out[i] = y
    return out


def highpass(buf, cutoff_hz):
    """One-pole high-pass, for thin, crisp noise like hi-hats and clicks."""
    dt = 1.0 / SR
    rc = 1.0 / (2.0 * math.pi * cutoff_hz)
    a = rc / (rc + dt)
    out = [0.0] * len(buf)
    prev_x = 0.0
    y = 0.0
    for i, x in enumerate(buf):
        y = a * (y + x - prev_x)
        prev_x = x
        out[i] = y
    return out


def noise(n):
    return [random.uniform(-1.0, 1.0) for _ in range(n)]


def normalize(buf, peak=0.85):
    """Scale so the loudest sample sits at `peak`, avoiding clipping on write."""
    hi = max((abs(x) for x in buf), default=0.0)
    if hi < 1e-9:
        return buf
    k = peak / hi
    return [x * k for x in buf]


def tone(buf, freq, dur, vol=0.5, kind="sine", decay=25.0, sweep=1.0, offset=0.0):
    """
    Mix a decaying oscillator into `buf` starting at `offset` seconds.
    `sweep` bends the pitch multiplicatively over the note's life, so sweep<1
    falls and sweep>1 rises.
    """
    start = int(offset * SR)
    n = int(dur * SR)
    phase = 0.0
    for i in range(n):
        idx = start + i
        if idx >= len(buf):
            break
        t = i / SR
        env = math.exp(-t * decay)
        f = freq * (sweep ** (t / dur))
        phase += f / SR
        p = phase % 1.0
        if kind == "sine":
            s = math.sin(2.0 * math.pi * p)
        elif kind == "square":
            s = 1.0 if p < 0.5 else -1.0
        elif kind == "tri":
            s = 4.0 * abs(p - 0.5) - 1.0
        else:  # saw
            s = 2.0 * p - 1.0
        buf[idx] += s * env * vol
    return buf


def write_wav(name, buf):
    """Write a float buffer in [-1,1] out as 16-bit mono PCM."""
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

    print("  {:<18} {:>7.1f} KB".format(name, os.path.getsize(path) / 1024.0))


# -----------------------------------------------------------------------------
# The sounds themselves
# -----------------------------------------------------------------------------

def make_pop(tier):
    """
    A fruit bursting: a short tonal blip that sweeps downward, layered with a
    wet low-passed noise burst. Smaller fruit pop higher and shorter.
    """
    base = POP_BASE_HZ[tier]
    dur = 0.15 + tier * 0.025
    n = int(dur * SR)
    out = [0.0] * n

    # Tonal blip with a fast downward pitch sweep — the "pop".
    phase = 0.0
    for i in range(n):
        t = i / SR
        env = math.exp(-t * 34.0)
        f = base * (1.5 * math.exp(-t * 24.0) + 0.6)
        phase += f / SR
        out[i] += math.sin(2.0 * math.pi * phase) * env * 0.55

    # Low-passed noise — the wet "splat" of the pulp.
    wet = lowpass(noise(n), 2600.0 - tier * 300.0)
    for i in range(n):
        out[i] += wet[i] * math.exp(-(i / SR) * 26.0) * 0.5

    return normalize(out, 0.82)


def make_shoot():
    """A soft, very short thwip. Kept quiet — many towers fire at once."""
    n = int(0.07 * SR)
    out = [0.0] * n
    air = highpass(noise(n), 1800.0)
    for i in range(n):
        out[i] += air[i] * math.exp(-(i / SR) * 70.0) * 0.5
    tone(out, 620.0, 0.06, vol=0.3, kind="tri", decay=60.0, sweep=0.45)
    return normalize(out, 0.45)


def make_knife():
    """A thin metallic whoosh — brighter and sharper than the seed thwip."""
    n = int(0.13 * SR)
    out = [0.0] * n

    air = highpass(noise(n), 3200.0)
    for i in range(n):
        out[i] += air[i] * math.exp(-(i / SR) * 40.0) * 0.6

    # A brief ring on top so it reads as a blade rather than just moving air.
    tone(out, 2200.0, 0.10, vol=0.22, kind="tri", decay=45.0, sweep=0.6)
    tone(out, 3300.0, 0.08, vol=0.12, kind="sine", decay=55.0, sweep=0.6)
    return normalize(out, 0.50)


def make_splash():
    """The Blender's pulp impact: wetter and longer than a single pop."""
    n = int(0.4 * SR)
    out = [0.0] * n
    wet = lowpass(noise(n), 1500.0)
    for i in range(n):
        out[i] += wet[i] * math.exp(-(i / SR) * 11.0) * 0.75
    tone(out, 190.0, 0.3, vol=0.4, kind="sine", decay=14.0, sweep=0.5)
    return normalize(out, 0.8)


def make_freeze():
    """A glassy shimmer for the Freezer pulse: detuned high sines with tremolo."""
    dur = 0.55
    n = int(dur * SR)
    out = [0.0] * n
    for freq, vol in ((1568.0, 0.30), (2093.0, 0.22), (2637.0, 0.16)):
        phase = 0.0
        for i in range(n):
            t = i / SR
            env = math.exp(-t * 7.0)
            trem = 0.75 + 0.25 * math.sin(2.0 * math.pi * 18.0 * t)
            phase += freq / SR
            out[i] += math.sin(2.0 * math.pi * phase) * env * trem * vol
    return normalize(out, 0.6)


def make_place():
    """A solid wooden thunk confirming a tower went down."""
    n = int(0.2 * SR)
    out = [0.0] * n
    tone(out, 220.0, 0.18, vol=0.6, kind="tri", decay=26.0, sweep=0.55)
    click = highpass(noise(int(0.02 * SR)), 2500.0)
    for i, x in enumerate(click):
        out[i] += x * math.exp(-(i / SR) * 140.0) * 0.4
    return normalize(out, 0.75)


def make_deny():
    """A short low buzz for an illegal placement."""
    n = int(0.18 * SR)
    out = [0.0] * n
    tone(out, 130.0, 0.16, vol=0.5, kind="square", decay=16.0, sweep=0.8)
    return normalize(out, 0.55)


def make_leak():
    """A falling, slightly detuned tone: a fruit got through and cost lives."""
    n = int(0.55 * SR)
    out = [0.0] * n
    tone(out, 440.0, 0.5, vol=0.45, kind="tri", decay=6.0, sweep=0.45)
    tone(out, 442.5, 0.5, vol=0.30, kind="sine", decay=6.0, sweep=0.45)
    return normalize(out, 0.7)


def make_wave_start():
    """A short rising three-note call announcing the incoming wave."""
    n = int(0.55 * SR)
    out = [0.0] * n
    for i, f in enumerate((392.0, 523.25, 659.25)):
        tone(out, f, 0.22, vol=0.4, kind="tri", decay=13.0, offset=i * 0.10)
    return normalize(out, 0.72)


def make_wave_clear():
    """A bright major arpeggio with a sparkle on top for surviving a wave."""
    n = int(1.0 * SR)
    out = [0.0] * n
    for i, f in enumerate((523.25, 659.25, 783.99, 1046.5)):
        tone(out, f, 0.42, vol=0.34, kind="tri", decay=7.0, offset=i * 0.09)
    tone(out, 2093.0, 0.5, vol=0.12, kind="sine", decay=6.0, offset=0.34)
    return normalize(out, 0.78)


def make_upgrade():
    """Two rising notes with a shimmer on top — a tower just got stronger."""
    n = int(0.5 * SR)
    out = [0.0] * n
    for i, f in enumerate((523.25, 783.99)):
        tone(out, f, 0.30, vol=0.40, kind="tri", decay=9.0, offset=i * 0.09)
    tone(out, 1567.98, 0.30, vol=0.15, kind="sine", decay=8.0, offset=0.18)
    return normalize(out, 0.72)


def make_sell():
    """A short descending blip for cashing a tower back in."""
    n = int(0.32 * SR)
    out = [0.0] * n
    for i, f in enumerate((659.25, 440.0)):
        tone(out, f, 0.18, vol=0.40, kind="tri", decay=16.0, offset=i * 0.08)
    return normalize(out, 0.60)


def make_spikes():
    """Metallic scatter of caltrops hitting the dirt."""
    n = int(0.26 * SR)
    out = [0.0] * n

    # Several short metallic ticks at slightly different times and pitches.
    for i, (freq, at) in enumerate(((2600.0, 0.0), (1900.0, 0.035), (3100.0, 0.07))):
        tone(out, freq, 0.08, vol=0.22, kind="tri", decay=50.0, sweep=0.7, offset=at)

    grit = highpass(noise(n), 2400.0)
    for i in range(n):
        out[i] += grit[i] * math.exp(-(i / SR) * 22.0) * 0.35
    return normalize(out, 0.52)


def make_victory():
    """A rising fanfare for clearing a whole route: a major triad walked up,
    then the octave held with a sparkle over it."""
    n = int(2.2 * SR)
    out = [0.0] * n

    for i, f in enumerate((523.25, 659.25, 783.99, 1046.5)):
        tone(out, f, 0.55, vol=0.34, kind="tri", decay=5.0, offset=i * 0.16)
        tone(out, f * 0.5, 0.55, vol=0.16, kind="sine", decay=4.5, offset=i * 0.16)

    # Held final chord.
    for f in (523.25, 659.25, 783.99, 1046.5):
        tone(out, f, 1.1, vol=0.20, kind="tri", decay=2.2, offset=0.72)
    tone(out, 2093.0, 0.9, vol=0.10, kind="sine", decay=3.0, offset=0.86)
    return normalize(out, 0.80)


def make_game_over():
    """A descending minor figure for being overrun."""
    n = int(1.8 * SR)
    out = [0.0] * n
    for i, f in enumerate((440.0, 392.0, 349.23, 261.63)):
        tone(out, f, 0.75, vol=0.38, kind="tri", decay=3.4, offset=i * 0.28)
        tone(out, f * 0.5, 0.75, vol=0.22, kind="sine", decay=3.0, offset=i * 0.28)
    return normalize(out, 0.8)


# -----------------------------------------------------------------------------
# Entry point
# -----------------------------------------------------------------------------

def main():
    # Fixed seed so regenerating produces byte-identical assets.
    random.seed(20260806)

    print("Generating sound effects into assets/ …")
    for tier in range(5):
        write_wav("pop_{}.wav".format(tier), make_pop(tier))

    write_wav("shoot.wav", make_shoot())
    write_wav("knife.wav", make_knife())
    write_wav("splash.wav", make_splash())
    write_wav("freeze.wav", make_freeze())
    write_wav("place.wav", make_place())
    write_wav("deny.wav", make_deny())
    write_wav("upgrade.wav", make_upgrade())
    write_wav("sell.wav", make_sell())
    write_wav("leak.wav", make_leak())
    write_wav("wave_start.wav", make_wave_start())
    write_wav("wave_clear.wav", make_wave_clear())
    write_wav("game_over.wav", make_game_over())

    # NOTE: new sounds go at the END of this list. Every generator draws from
    # one shared seeded stream, so inserting a call earlier shifts the random
    # numbers every later sound gets and silently rewrites unrelated .wav files.
    write_wav("spikes.wav", make_spikes())
    write_wav("victory.wav", make_victory())
    print("Done.")


if __name__ == "__main__":
    main()
