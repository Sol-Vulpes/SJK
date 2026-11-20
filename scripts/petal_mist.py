#!/usr/bin/env python3
"""Draw the lavender mist's picture (`crates/sjk-viewer/assets/petals/mist.png`,
`saber_petals.rs`, the Varykino Blossom's Blossom mist extra, 20/11/2026): a soft wisp of
cloud, near white (the particle tints it lavender), its alpha a few overlapping soft lobes
broken by smooth noise and gone well before the picture's edge, so a billboard of it never
shows a square. It is shaded as a lit volume (Kushina: the mist must read as 3D, not a flat
pattern): lighter at its top and where it is thick, deeper toward its underside and thin
edges, so each wisp looks round; the particle keeps it nearly upright so the light stays
above. Its colour is premultiplied by its alpha (`blendFunc GL_ONE GL_ONE_MINUS_SRC_ALPHA`):
the wisp both glows softly and veils what is behind it, so on a dark map it reads as pale
light, not grey smoke.

    python3 scripts/petal_mist.py
"""
from pathlib import Path

import numpy as np
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "crates/sjk-viewer/assets/petals"
SIZE = 128


def value_noise(shape, cells, rng, across=None):
    """Smooth value noise: a random grid of `cells` cells down (`across` across, as many
    when not given), smoothstep-interpolated."""
    across = across or cells
    grid = rng.random((cells + 1, across + 1))
    h, w = shape
    y = np.linspace(0, cells, h, endpoint=False)
    x = np.linspace(0, across, w, endpoint=False)
    yi, xi = np.floor(y).astype(int), np.floor(x).astype(int)
    fy, fx = y - yi, x - xi
    fy, fx = fy * fy * (3 - 2 * fy), fx * fx * (3 - 2 * fx)
    a = grid[yi][:, xi]
    b = grid[yi][:, xi + 1]
    c = grid[yi + 1][:, xi]
    d = grid[yi + 1][:, xi + 1]
    top = a + (b - a) * fx[None, :]
    bottom = c + (d - c) * fx[None, :]
    return top + (bottom - top) * fy[:, None]


def mist():
    rng = np.random.default_rng(20261120)
    y, x = np.mgrid[0:SIZE, 0:SIZE]
    x = (x + 0.5) / SIZE * 2.0 - 1.0
    y = (y + 0.5) / SIZE * 2.0 - 1.0
    # A few soft lobes, so the wisp is not a disc.
    lobes = np.zeros((SIZE, SIZE))
    for cx, cy, rx, ry in ((0.0, 0.0, 0.62, 0.36), (-0.3, 0.16, 0.4, 0.24), (0.32, -0.14, 0.42, 0.22)):
        lobes = np.maximum(lobes, np.exp(-(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2)))
    # Smooth noise, stretched sideways, tears it into wisps with gaps between them.
    noise = (
        0.5 * value_noise((SIZE, SIZE), 6, rng, across=3)
        + 0.3 * value_noise((SIZE, SIZE), 9, rng)
        + 0.2 * value_noise((SIZE, SIZE), 18, rng)
    )
    torn = np.clip((noise - 0.32) / 0.4, 0.0, 1.0)
    torn = torn * torn * (3 - 2 * torn)
    wisp = lobes * (0.15 + 0.85 * torn)
    # Gone well inside the edge.
    r = np.hypot(x, y)
    edge = np.clip((0.98 - r) / 0.4, 0.0, 1.0)
    edge = edge * edge * (3 - 2 * edge)
    alpha = np.clip(wisp * edge, 0.0, 1.0)
    alpha = alpha / alpha.max() * 0.9
    # Light from above: brighter toward the top (y is -1 at the top) and where the wisp is
    # thick, deeper toward its underside and thin edges.
    thick = np.clip(alpha / 0.9, 0.0, 1.0)
    above = np.clip(0.5 - 0.5 * y, 0.0, 1.0)
    light = np.clip(0.76 + 0.14 * above + 0.12 * thick, 0.0, 1.0)
    rgba = np.zeros((SIZE, SIZE, 4), dtype=np.uint8)
    rgba[..., :3] = np.round(light * alpha * 255).astype(np.uint8)[..., None]
    rgba[..., 3] = np.round(alpha * 255).astype(np.uint8)
    return Image.fromarray(rgba, "RGBA")


mist().save(OUT / "mist.png", optimize=True)
print("written mist")
