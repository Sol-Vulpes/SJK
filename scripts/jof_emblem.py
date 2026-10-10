"""Render the JoF clan emblem (J, o, F) to assets/branding/jof-emblem.png.

White on a transparent square, so the client tints it. The shapes are the clan's
SVG path: two polygons and a ring (even-odd), in a 120-unit box.

Usage: python scripts/jof_emblem.py [out.png] [size]
"""

import sys

from PIL import Image, ImageDraw

J = [(38, 9), (38, 66), (10, 94), (10, 109), (50, 69), (50, 9)]
F = [(58, 9), (58, 69), (69, 80), (69, 56), (84, 41), (84, 26), (69, 41), (69, 33),
     (87, 15), (87, 0), (69, 18), (69, 9)]
RING = ((54, 90), 16.5, 8.0)  # centre, outer and inner radius
# The emblem's box: x 10..87, y 0..109 (the ring reaches 106.5).
LEFT, TOP, WIDTH, HEIGHT = 10, 0, 77, 109
SUPERSAMPLE = 8


def render(size: int) -> Image.Image:
    big = size * SUPERSAMPLE
    margin = big * 0.04
    scale = (big - 2 * margin) / max(WIDTH, HEIGHT)
    ox = (big - WIDTH * scale) / 2
    oy = (big - HEIGHT * scale) / 2

    def at(point):
        return (ox + (point[0] - LEFT) * scale, oy + (point[1] - TOP) * scale)

    mask = Image.new("L", (big, big), 0)
    draw = ImageDraw.Draw(mask)
    draw.polygon([at(p) for p in J], fill=255)
    draw.polygon([at(p) for p in F], fill=255)
    (cx, cy), outer, inner = RING
    x, y = at((cx, cy))
    draw.ellipse([x - outer * scale, y - outer * scale, x + outer * scale, y + outer * scale], fill=255)
    draw.ellipse([x - inner * scale, y - inner * scale, x + inner * scale, y + inner * scale], fill=0)
    mask = mask.resize((size, size), Image.LANCZOS)
    out = Image.new("RGBA", (size, size), (255, 255, 255, 0))
    out.putalpha(mask)
    return out


if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "assets/branding/jof-emblem.png"
    size = int(sys.argv[2]) if len(sys.argv) > 2 else 128
    render(size).save(path, optimize=True)
