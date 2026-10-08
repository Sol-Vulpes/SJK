"""Synthesize the Sun blade's saber sounds: ignition, switching off, the hum and swings.

The Sun blade is SJK's first unlockable blade skin (docs/unlockables.md): a blade like a
small sun, so its sounds are a solar roar rather than the stock hum and buzz. They are
our own, made here from oscillators and filtered noise with a fixed seed, so the script
always writes the same files. The client bundles them and mounts them below the game
data at `sound/sjk/sabers/sun/` (crates/sjk-viewer/src/saber_skins.rs), so a PK3 with
the same paths replaces them. This writes, as 16-bit mono WAV at 22050 Hz, into the
output folder (default crates/sjk-viewer/assets/sabers/sun):

- `on.wav` (1 s): a bright snap, then a deep roar that rises in pitch and opens up,
  crackling, and settles into the hum's tone;
- `off.wav` (0.8 s): the roar collapsing and falling in pitch, its crackle dying out;
- `hum.wav` (2 s, looped): a warm low drone whose partials beat slowly against each
  other, with a faint plasma crackle. Every partial and modulation completes whole
  cycles in the loop and the noise is made circular, so the loop point does not click;
- `swing1.wav` to `swing3.wav` (0.45 to 0.6 s): hot whooshes, a resonant rush of air
  sweeping in pitch over the drone pushed up as if passing by, with fire crackle.

Swings, ignition and switching off peak at -3 dBFS like the stock saber's (which peak
near 0 dBFS); the hum, always playing, peaks at -12 dBFS
so that its average level (-17 dB) matches the stock hums'.

    python3 scripts/saber_skin_sounds.py [output-folder]

Needs only Python 3's standard library.
"""

import math
import random
import struct
import sys
import wave
from pathlib import Path

RATE = 22050
SEED = 0x50_1A_B1_AD
TAU = 2.0 * math.pi
# The hum's fundamental; the ignition settles on it and the swings push it up.
HUM_HZ = 60.0
LOOP_SECONDS = 2.0


def seconds(count):
    return [index / RATE for index in range(count)]


def lowpass(signal, cutoff):
    """One-pole low-pass; `cutoff` is a frequency in Hz or a per-sample list."""
    output = []
    state = 0.0
    for index, value in enumerate(signal):
        frequency = cutoff[index] if isinstance(cutoff, list) else cutoff
        keep = math.exp(-TAU * max(frequency, 1.0) / RATE)
        state = (1.0 - keep) * value + keep * state
        output.append(state)
    return output


def highpass(signal, cutoff):
    low = lowpass(signal, cutoff)
    return [value - smooth for value, smooth in zip(signal, low)]


def bandpass(signal, centre, q):
    """RBJ band-pass (constant peak gain) with a per-sample centre frequency list."""
    output = []
    x1 = x2 = y1 = y2 = 0.0
    for value, frequency in zip(signal, centre):
        omega = TAU * min(max(frequency, 20.0), RATE * 0.45) / RATE
        alpha = math.sin(omega) / (2.0 * q)
        a0 = 1.0 + alpha
        b0, b2 = alpha / a0, -alpha / a0
        a1, a2 = -2.0 * math.cos(omega) / a0, (1.0 - alpha) / a0
        y = b0 * value + b2 * x2 - a1 * y1 - a2 * y2
        x2, x1 = x1, value
        y2, y1 = y1, y
        output.append(y)
    return output


def circular(filter_signal, signal):
    """Filter `signal` as if it repeated forever: run it twice, keep the second pass."""
    return filter_signal(signal + signal)[len(signal):]


def white(rng, count):
    return [rng.uniform(-1.0, 1.0) for _ in range(count)]


def drone(times, pitch, harmonics, phases):
    """A warm tone: odd-heavy harmonics with 1/k^1.3 roll-off; `pitch` per sample."""
    output = []
    phase = 0.0
    for index in range(len(times)):
        phase += TAU * pitch[index] / RATE
        value = 0.0
        for harmonic in range(1, harmonics + 1):
            weight = harmonic ** -1.3 * (1.0 if harmonic % 2 else 0.6)
            value += weight * math.sin(harmonic * phase + phases[harmonic - 1])
        output.append(value)
    return output


def crackle(rng, count, rate, scale=1.0, wrap=False):
    """Sparse fire crackle: little bursts of bright, fast-decaying noise.

    `rate` gives each sample's expected bursts per second (a function of the time in
    seconds). With `wrap`, a burst running past the end continues from the start.
    """
    kernels = []
    for _ in range(8):
        length = rng.randint(int(0.002 * RATE), int(0.009 * RATE))
        decay = length / 4.0
        burst = [rng.uniform(-1.0, 1.0) * math.exp(-k / decay) for k in range(length)]
        kernels.append(highpass(burst, 2500.0))
    output = [0.0] * count
    for index in range(count):
        if rng.random() < rate(index / RATE) / RATE:
            kernel = rng.choice(kernels)
            amplitude = scale * (0.25 + 0.75 * rng.random() ** 2) * rng.choice((-1.0, 1.0))
            for offset, value in enumerate(kernel):
                position = index + offset
                if wrap:
                    position %= count
                elif position >= count:
                    break
                output[position] += amplitude * value
    return output


def envelope(times, points):
    """Piecewise-linear envelope through (seconds, level) points."""
    output = []
    for time in times:
        if time <= points[0][0]:
            output.append(points[0][1])
            continue
        level = points[-1][1]
        for (t0, l0), (t1, l1) in zip(points, points[1:]):
            if t0 <= time <= t1:
                level = l0 + (l1 - l0) * (time - t0) / max(t1 - t0, 1e-9)
                break
        output.append(level)
    return output


def mix(*parts):
    return [sum(values) for values in zip(*parts)]


def scaled(signal, gain):
    if isinstance(gain, list):
        return [value * level for value, level in zip(signal, gain)]
    return [value * gain for value in signal]


def saturate(signal, drive):
    norm = math.tanh(drive)
    return [math.tanh(value * drive) / norm for value in signal]


def peak_to(signal, dbfs):
    peak = max(abs(value) for value in signal) or 1.0
    gain = 10.0 ** (dbfs / 20.0) / peak
    return [value * gain for value in signal]


def fades(signal, fade_in, fade_out):
    count = len(signal)
    rise = max(1, int(fade_in * RATE))
    fall = max(1, int(fade_out * RATE))
    output = list(signal)
    for index in range(min(rise, count)):
        output[index] *= index / rise
    for index in range(min(fall, count)):
        output[count - 1 - index] *= index / fall
    return output


def ignition(rng):
    count = int(1.0 * RATE)
    times = seconds(count)
    # The snap: a bright noise burst and a falling zap, gone in 80 ms.
    snap_level = [math.exp(-t / 0.018) for t in times]
    snap = scaled(highpass(white(rng, count), 1800.0), snap_level)
    zap_pitch = [300.0 + 2200.0 * math.exp(-t / 0.025) for t in times]
    zap = scaled(drone(times, zap_pitch, 3, [0.0] * 3), [math.exp(-t / 0.035) for t in times])
    # The roar: noise opening from a rumble to a blaze, then settling.
    cutoff = envelope(times, [(0.0, 160.0), (0.28, 1700.0), (0.6, 800.0), (1.0, 450.0)])
    roar_level = envelope(times, [(0.0, 0.0), (0.03, 0.6), (0.22, 1.0), (0.5, 0.35), (1.0, 0.0)])
    roar = lowpass(lowpass(white(rng, count), cutoff), cutoff)
    roar = scaled(roar, [level * 4.0 for level in roar_level])
    # The deep tone rising to the hum's pitch.
    pitch = [HUM_HZ * (0.55 + 0.45 * (1.0 - math.exp(-t / 0.12))) for t in times]
    tone_level = envelope(times, [(0.0, 0.0), (0.04, 0.8), (0.25, 1.0), (0.6, 0.45), (1.0, 0.0)])
    tone = scaled(drone(times, pitch, 9, [rng.uniform(0, TAU) for _ in range(9)]), tone_level)
    sparks = crackle(rng, count, lambda t: 120.0 * math.exp(-t / 0.25) + 10.0, 0.8)
    body = saturate(mix(scaled(tone, 0.7), scaled(roar, 0.45), scaled(sparks, 0.45)), 1.6)
    out = mix(scaled(body, 0.6), scaled(snap, 1.1), scaled(zap, 0.3))
    return peak_to(fades(out, 0.001, 0.12), -3.0)


def extinction(rng):
    count = int(0.8 * RATE)
    times = seconds(count)
    # A short inward suck, then the roar collapsing.
    suck = scaled(highpass(white(rng, count), 1500.0), envelope(times, [(0.0, 0.6), (0.06, 0.0)]))
    cutoff = envelope(times, [(0.0, 1500.0), (0.12, 1100.0), (0.45, 260.0), (0.8, 100.0)])
    roar_level = envelope(times, [(0.0, 0.7), (0.05, 1.0), (0.3, 0.35), (0.7, 0.0)])
    roar = lowpass(lowpass(white(rng, count), cutoff), cutoff)
    roar = scaled(roar, [level * 4.0 for level in roar_level])
    pitch = [HUM_HZ * (0.4 + 0.6 * math.exp(-t / 0.22)) for t in times]
    tone_level = envelope(times, [(0.0, 1.0), (0.1, 0.85), (0.5, 0.25), (0.78, 0.0)])
    tone = scaled(drone(times, pitch, 9, [rng.uniform(0, TAU) for _ in range(9)]), tone_level)
    sparks = crackle(rng, count, lambda t: max(0.0, 90.0 * (1.0 - t / 0.6)), 0.8)
    body = saturate(mix(scaled(tone, 0.6), scaled(roar, 0.6), scaled(sparks, 0.5)), 1.5)
    out = mix(scaled(body, 0.75), scaled(suck, 0.5))
    return peak_to(fades(out, 0.002, 0.08), -3.0)


def hum(rng):
    count = int(LOOP_SECONDS * RATE)
    times = seconds(count)
    # Every frequency is a multiple of 1 / LOOP_SECONDS, so each completes whole cycles.
    partials = [
        (HUM_HZ, 1.0),
        (HUM_HZ + 0.5, 0.55),  # beats with the fundamental twice a loop
        (2 * HUM_HZ, 0.42),
        (3 * HUM_HZ, 0.3),
        (3 * HUM_HZ + 1.0, 0.18),  # a faster shimmer on the third
        (4 * HUM_HZ, 0.12),
        (5 * HUM_HZ, 0.14),
        (7 * HUM_HZ, 0.06),
        (9 * HUM_HZ, 0.03),
    ]
    phases = [rng.uniform(0, TAU) for _ in partials]
    tone = [
        sum(level * math.sin(TAU * frequency * t + phase)
            for (frequency, level), phase in zip(partials, phases))
        for t in times
    ]
    breathing = [1.0 + 0.12 * math.sin(TAU * t / LOOP_SECONDS) for t in times]
    tone = saturate(scaled(tone, breathing), 1.3)
    # A soft solar wind: circular band-passed noise, with a slow swell of its own.
    wind = circular(lambda signal: lowpass(highpass(signal, 500.0), 1400.0), white(rng, count))
    swell = [0.7 + 0.3 * math.sin(TAU * 2.0 * t / LOOP_SECONDS + 1.0) for t in times]
    wind = scaled(wind, [level * 2.2 for level in swell])
    sparks = crackle(rng, count, lambda t: 18.0, 0.22, wrap=True)
    out = mix(scaled(tone, 0.5), scaled(wind, 0.12), sparks)
    return peak_to(out, -12.0)


def swing(rng, duration, sweep, centre, rush_at):
    count = int(duration * RATE)
    times = seconds(count)
    shape = [
        math.sin(math.pi * min(1.0, t / rush_at) * 0.5) ** 2 if t < rush_at
        else math.exp(-(t - rush_at) / ((duration - rush_at) * 0.38))
        for t in times
    ]
    # The rush of air: band-passed noise whose centre follows the swing.
    centre_hz = [centre * (1.0 + sweep * (level - 0.5)) for level in shape]
    air = bandpass(white(rng, count), centre_hz, 2.4)
    air = scaled(air, [level * 3.0 for level in shape])
    # The drone pushed up in pitch as the blade passes, roughened by saturation.
    pitch = [HUM_HZ * (1.0 + 0.7 * level) for level in shape]
    tone = drone(times, pitch, 9, [rng.uniform(0, TAU) for _ in range(9)])
    tone = scaled(saturate(tone, 2.0), [0.25 + 0.75 * level for level in shape])
    flames = lowpass(highpass(white(rng, count), 900.0), 3500.0)
    flames = scaled(flames, [level ** 2 * 1.4 for level in shape])
    sparks = crackle(rng, count, lambda t: 25.0 + 160.0 * shape[min(int(t * RATE), count - 1)], 0.7)
    out = mix(scaled(air, 0.8), scaled(tone, 0.45), scaled(flames, 0.35), scaled(sparks, 0.45))
    return peak_to(fades(saturate(out, 1.2), 0.004, 0.05), -3.0)


def write(path, signal):
    frames = b"".join(
        struct.pack("<h", max(-32767, min(32767, round(value * 32767.0)))) for value in signal
    )
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(RATE)
        output.writeframes(frames)


def main():
    folder = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("crates/sjk-viewer/assets/sabers/sun")
    folder.mkdir(parents=True, exist_ok=True)
    rng = random.Random(SEED)
    sounds = {
        "on": ignition(rng),
        "off": extinction(rng),
        "hum": hum(rng),
        "swing1": swing(rng, 0.45, 1.6, 700.0, 0.16),
        "swing2": swing(rng, 0.52, -1.1, 1100.0, 0.2),
        "swing3": swing(rng, 0.6, 2.2, 520.0, 0.24),
    }
    looped = sounds["hum"]
    largest_step = max(abs(b - a) for a, b in zip(looped, looped[1:]))
    seam = abs(looped[0] - looped[-1])
    assert seam <= largest_step, f"the hum's loop point jumps {seam:.4f} (> {largest_step:.4f})"
    total = 0
    for name, signal in sounds.items():
        path = folder / f"{name}.wav"
        write(path, signal)
        size = path.stat().st_size
        total += size
        print(f"{path}: {len(signal) / RATE:.2f} s, {size} bytes")
    print(f"total {total} bytes")


if __name__ == "__main__":
    main()
