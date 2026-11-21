"""Make the Force Throw toy's own game files (`toy_monitor`, SJK's own).

Force Throw (the Monitor until 21/11/2026, its commands kept) hurls Jedi Academy's own
junk with the Force, and plays the game's own sounds; only two files are SJK's, drawn and
synthesized here, nothing taken from another game. This script writes, into the output
folder:

- `toy_monitor.png`: the toy's icon, 128x128 like `gfx/mp/f_icon_*`: a banded crate
  flying, blue rings of the Force pushing it from behind, on transparency;
- `hit.wav`: the short bright tick the thrower hears when their throw strikes someone,
  22050 Hz mono 16-bit (the Monitor's, as it was).

    python scripts/force_throw_assets.py crates/sjk-viewer/assets/monitor

The output is deterministic (fixed seeds). Needs Pillow and numpy
(pip install pillow numpy).
"""

import math
import sys
import wave
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ICON_SIZE = 128
RATE = 22050
WOOD = (122, 92, 58, 255)
WOOD_DARK = (74, 54, 34, 255)
BAND = (96, 100, 108, 255)
FORCE = (120, 200, 255)


def icon(output: Path) -> None:
    """A crate flying to the right, a little tilted, three rings of the Force behind it."""
    scale = 4
    size = ICON_SIZE * scale
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    # The Force: arcs opening to the right, behind the crate, fading as they trail.
    for index, (x, alpha) in enumerate(((0.2, 255), (0.04, 190), (-0.12, 110))):
        r = size * (0.2 + 0.05 * index)
        cx, cy = size * x, size * 0.5
        draw.arc((cx - r, cy - r, cx + r, cy + r), -55, 55,
                 fill=FORCE + (alpha,), width=int(size * 0.035))
    glow = canvas.filter(ImageFilter.GaussianBlur(10))
    canvas = Image.alpha_composite(glow, canvas)
    # The crate: planks, a dark frame and two metal bands.
    crate = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(crate)
    x0, y0, x1, y1 = size * 0.44, size * 0.26, size * 0.9, size * 0.72
    d.rectangle((x0, y0, x1, y1), fill=WOOD, outline=WOOD_DARK, width=int(size * 0.025))
    for k in range(1, 4):
        y = y0 + (y1 - y0) * k / 4
        d.line((x0, y, x1, y), fill=WOOD_DARK, width=int(size * 0.008))
    for x in (x0 + (x1 - x0) * 0.22, x1 - (x1 - x0) * 0.22):
        d.rectangle((x - size * 0.03, y0, x + size * 0.03, y1), fill=BAND)
        for y in (y0 + size * 0.05, y1 - size * 0.05):
            d.ellipse((x - size * 0.012, y - size * 0.012, x + size * 0.012, y + size * 0.012),
                      fill=(170, 175, 184, 255))
    crate = crate.rotate(-12, resample=Image.BICUBIC, center=((x0 + x1) / 2, (y0 + y1) / 2))
    canvas.alpha_composite(crate)
    alpha = canvas.split()[3]
    halo = alpha.filter(ImageFilter.MaxFilter(9)).filter(ImageFilter.GaussianBlur(3))
    ground = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ground.putalpha(halo.point(lambda v: int(v * 0.7)))
    ground.alpha_composite(canvas)
    ground.resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS).save(output / "toy_monitor.png")


def write_wav(path: Path, samples: np.ndarray) -> None:
    peak = np.max(np.abs(samples))
    if peak > 0:
        samples = samples / peak * 0.9
    data = (np.clip(samples, -1, 1) * 32767).astype("<i2").tobytes()
    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(data)


def lowpass(signal: np.ndarray, cutoff: float) -> np.ndarray:
    """A one-pole low pass at `cutoff` Hz."""
    a = 1.0 - math.exp(-2 * math.pi * cutoff / RATE)
    out = np.empty_like(signal)
    y = 0.0
    for i, x in enumerate(signal):
        y += a * (x - y)
        out[i] = y
    return out


def hit(output: Path) -> None:
    """The thrower's hit: a short bright tick."""
    rng = np.random.default_rng(11)
    n = int(RATE * 0.22)
    t = np.arange(n) / RATE
    tick = ((np.sin(2 * np.pi * 1870 * t) + 0.5 * np.sin(2 * np.pi * 2810 * t))
            * np.exp(-t * 28))
    click = lowpass(rng.standard_normal(n), 7000) * np.exp(-t * 400)
    write_wav(output / "hit.wav", tick + click * 0.8)


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    icon(output)
    hit(output)


if __name__ == "__main__":
    main()
