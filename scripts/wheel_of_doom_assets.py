"""Make the WHEEL OF DOOM's sound (`docs/wheel-of-doom.md`, SJK's own).

Nothing here comes from another game: the sound is synthesized. This script writes,
into the output folder, 22050 Hz mono 16-bit:

- `quack.wav`: the Quack effect's quack, a nasal buzz falling in pitch.

    python scripts/wheel_of_doom_assets.py crates/sjk-viewer/assets/wheel_of_doom

The wheel's tick, stop and fanfare sounds went with the placed wheel on 23/11/2026 (the
DOOM coin rolls at a Force box). The output is deterministic. Needs numpy (pip install
numpy); it uses the WAV writer of mystery_box_assets.py.
"""

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import mystery_box_assets as box  # noqa: E402

RATE = box.RATE


def seconds(length: float) -> np.ndarray:
    return np.arange(int(length * RATE)) / RATE


def quack() -> np.ndarray:
    t = seconds(0.28)
    # A falling buzz through two nasal formants.
    pitch = 520 * (1 - 0.35 * t / t[-1])
    phase = 2 * np.pi * np.cumsum(pitch) / RATE
    buzz = np.sign(np.sin(phase)) * 0.5 + np.sin(phase * 2) * 0.3
    spectrum = np.fft.rfft(buzz)
    freqs = np.fft.rfftfreq(len(buzz), 1 / RATE)
    shape = np.exp(-((freqs - 1100) / 380) ** 2) + 0.7 * np.exp(-((freqs - 2600) / 500) ** 2)
    voiced = np.fft.irfft(spectrum * shape, len(buzz))
    envelope = np.minimum(t / 0.015, 1.0) * np.exp(-t / 0.16)
    return voiced * envelope


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    folder = Path(sys.argv[1])
    folder.mkdir(parents=True, exist_ok=True)
    box.write_wav(folder / "quack.wav", quack())


if __name__ == "__main__":
    main()
