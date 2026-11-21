"""Make the feats' medal pictures (`cg_feats`, SJK's own).

Each feat (`crates/sjk-viewer/src/feats.rs`, `Feat`) shows as a medal under the top
centre: this script draws one picture per feat, 128x128 on transparency, from shapes
only (nothing comes from another game): a round medal in the feat's colour with a rim,
and a mark on it:

- the chains (Double to Penta Kill) carry that many saber blades side by side, Massacre
  two crossed blades;
- the sprees (Killing Spree to Unstoppable) a star with one to four pips under it,
  Godlike a sun;
- First Blood a drop, Revenge an arrow turning back, Spree Ended a broken star,
  HOOKED! a grapple hook on its line.

    python scripts/feat_assets.py crates/sjk-viewer/assets/feats

The output is deterministic. Needs Pillow (pip install pillow). A PK3 replaces any of
them with `gfx/sjk/feats/<id>.png`.
"""

import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

SIZE = 128
SCALE = 4
S = SIZE * SCALE
C = S / 2

# Colours as in `Feat::accent` (sRGB-ish, 0-255).
FEATS = {
    "hooked": (90, 215, 240),
    "first_blood": (215, 30, 30),
    "double_kill": (255, 158, 46),
    "triple_kill": (255, 158, 46),
    "quad_kill": (255, 90, 30),
    "penta_kill": (255, 90, 30),
    "massacre": (255, 90, 30),
    "killing_spree": (240, 64, 76),
    "rampage": (240, 64, 76),
    "dominating": (190, 90, 255),
    "unstoppable": (190, 90, 255),
    "godlike": (255, 214, 76),
    "spree_ended": (153, 166, 184),
    "revenge": (140, 230, 102),
}

WHITE = (255, 255, 255, 255)


def shade(colour, factor):
    return tuple(max(0, min(255, int(c * factor))) for c in colour) + (255,)


def medal(colour):
    """The medal: a soft shadow, a dark rim, the face in two tones and a highlight."""
    image = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).ellipse(
        (C - 0.45 * S, C - 0.43 * S, C + 0.45 * S, C + 0.47 * S), fill=(0, 0, 0, 150)
    )
    image.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(S * 0.02)))
    draw = ImageDraw.Draw(image)
    r = 0.44 * S
    draw.ellipse((C - r, C - r, C + r, C + r), fill=shade(colour, 0.35))
    r = 0.40 * S
    draw.ellipse((C - r, C - r, C + r, C + r), fill=shade(colour, 0.95))
    r = 0.34 * S
    draw.ellipse((C - r, C - r, C + r, C + r), fill=shade(colour, 0.62))
    # A highlight across the top.
    light = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(light).ellipse(
        (C - 0.30 * S, C - 0.38 * S, C + 0.30 * S, C - 0.05 * S), fill=(255, 255, 255, 46)
    )
    image.alpha_composite(light.filter(ImageFilter.GaussianBlur(S * 0.03)))
    return image


def blade(draw, x, y0, y1, width, colour):
    """A saber blade: a white core in a coloured edge, with a short hilt below."""
    draw.rounded_rectangle((x - width, y0, x + width, y1), radius=width, fill=shade(colour, 1.25))
    core = width * 0.5
    draw.rounded_rectangle((x - core, y0 + core, x + core, y1 - core), radius=core, fill=WHITE)
    draw.rounded_rectangle(
        (x - width * 1.1, y1, x + width * 1.1, y1 + 0.07 * S), radius=width * 0.4, fill=(40, 40, 46, 255)
    )


def blades(draw, count, colour):
    spread = 0.40 * S
    width = 0.022 * S if count > 3 else 0.028 * S
    for i in range(count):
        x = C + (i - (count - 1) / 2) * spread / max(count - 1, 1) if count > 1 else C
        blade(draw, x, C - 0.24 * S, C + 0.14 * S, width, colour)


def crossed(image, colour):
    for angle in (-38, 38):
        layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        blade(ImageDraw.Draw(layer), C, C - 0.27 * S, C + 0.16 * S, 0.03 * S, colour)
        image.alpha_composite(layer.rotate(angle, resample=Image.BICUBIC, center=(C, C)))


def star_points(cx, cy, outer, inner, points=5, turn=-90):
    out = []
    for i in range(points * 2):
        radius = outer if i % 2 == 0 else inner
        angle = math.radians(turn + i * 180 / points)
        out.append((cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
    return out


def star(draw, pips, colour):
    draw.polygon(star_points(C, C - 0.04 * S, 0.22 * S, 0.095 * S), fill=WHITE)
    r = 0.025 * S
    gap = 0.075 * S
    for i in range(pips):
        x = C + (i - (pips - 1) / 2) * gap
        y = C + 0.22 * S
        draw.ellipse((x - r, y - r, x + r, y + r), fill=shade(colour, 1.4))


def sun(draw):
    for i in range(12):
        angle = math.radians(i * 30)
        tip = (C + 0.27 * S * math.cos(angle), C + 0.27 * S * math.sin(angle))
        side = math.radians(i * 30 + 90)
        w = 0.035 * S
        base = (C + 0.13 * S * math.cos(angle), C + 0.13 * S * math.sin(angle))
        draw.polygon(
            [
                (base[0] + w * math.cos(side), base[1] + w * math.sin(side)),
                tip,
                (base[0] - w * math.cos(side), base[1] - w * math.sin(side)),
            ],
            fill=WHITE,
        )
    r = 0.13 * S
    draw.ellipse((C - r, C - r, C + r, C + r), fill=WHITE)


def drop(draw):
    r = 0.15 * S
    cy = C + 0.07 * S
    draw.ellipse((C - r, cy - r, C + r, cy + r), fill=WHITE)
    draw.polygon([(C - r * 0.93, cy - r * 0.35), (C, C - 0.27 * S), (C + r * 0.93, cy - r * 0.35)], fill=WHITE)


def arrow_back(draw):
    width = int(0.06 * S)
    box = (C - 0.2 * S, C - 0.2 * S, C + 0.2 * S, C + 0.2 * S)
    draw.arc(box, start=-200, end=60, fill=WHITE, width=width)
    # Its head at the arc's start, pointing back.
    angle = math.radians(-200)
    x, y = C + 0.2 * S * math.cos(angle) + width * 0.5, C + 0.2 * S * math.sin(angle)
    h = 0.12 * S
    draw.polygon([(x - h, y), (x + h * 0.5, y - h * 0.9), (x + h * 0.5, y + h * 0.9)], fill=WHITE)


def broken_star(image):
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(layer).polygon(star_points(C, C, 0.25 * S, 0.105 * S), fill=WHITE)
    # Cut it in two along a jagged line and part the halves.
    cut = Image.new("L", (S, S), 0)
    ImageDraw.Draw(cut).polygon(
        [(0, 0), (C - 0.02 * S, 0), (C + 0.04 * S, C - 0.1 * S), (C - 0.04 * S, C + 0.02 * S),
         (C + 0.03 * S, S), (0, S)],
        fill=255,
    )
    left = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    left.paste(layer, (0, 0), cut)
    right = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    right.paste(layer, (0, 0), Image.eval(cut, lambda v: 255 - v))
    image.alpha_composite(left.rotate(8, resample=Image.BICUBIC, center=(C, C)), (-int(0.03 * S), 0))
    image.alpha_composite(right.rotate(-8, resample=Image.BICUBIC, center=(C, C)), (int(0.03 * S), 0))


def hook(draw):
    width = int(0.045 * S)
    # The line from the top, the shank, and the hook's curve ending in a barb.
    draw.line((C, C - 0.33 * S, C, C + 0.05 * S), fill=(220, 220, 225, 255), width=int(0.02 * S))
    draw.rounded_rectangle((C - 0.04 * S, C - 0.17 * S, C + 0.04 * S, C - 0.11 * S), radius=0.015 * S, fill=WHITE)
    box = (C - 0.17 * S, C - 0.06 * S, C + 0.17 * S, C + 0.26 * S)
    draw.arc(box, start=0, end=180, fill=WHITE, width=width)
    draw.line((C + 0.17 * S - width / 2, C + 0.10 * S, C + 0.17 * S - width / 2, C - 0.02 * S), fill=WHITE, width=width)
    draw.line((C - 0.17 * S + width / 2, C + 0.10 * S, C - 0.17 * S + width / 2, C - 0.10 * S), fill=WHITE, width=width)
    tip = (C - 0.17 * S + width / 2, C - 0.16 * S)
    draw.polygon([(tip[0] - 0.05 * S, C - 0.07 * S), tip, (tip[0] + 0.05 * S, C - 0.07 * S)], fill=WHITE)


def picture(feat, colour):
    image = medal(colour)
    draw = ImageDraw.Draw(image)
    chains = {"double_kill": 2, "triple_kill": 3, "quad_kill": 4, "penta_kill": 5}
    sprees = {"killing_spree": 1, "rampage": 2, "dominating": 3, "unstoppable": 4}
    if feat in chains:
        blades(draw, chains[feat], colour)
    elif feat == "massacre":
        crossed(image, colour)
    elif feat in sprees:
        star(draw, sprees[feat], colour)
    elif feat == "godlike":
        sun(draw)
    elif feat == "first_blood":
        drop(draw)
    elif feat == "revenge":
        arrow_back(draw)
    elif feat == "spree_ended":
        broken_star(image)
    elif feat == "hooked":
        hook(draw)
    return image.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for feat, colour in FEATS.items():
        picture(feat, colour).save(out / f"{feat}.png", optimize=True)


if __name__ == "__main__":
    main()
