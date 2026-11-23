"""Make the Force box's model and pictures (`docs/mystery-boxes.md`, SJK's own).

Since 23/11/2026 the box is an ancient Sith tomb (Sol: "more of an ancient sith tomb
thing, dark, and with red effects, stone door sound when it opens"; the Jedi Force box of
temple stone and old gold before, the wooden chest before that): a sarcophagus of black
basalt on two steps, its walls leaning in, a horn of stone at each corner, glyphs and a
sigil cut into it that glow red, and a ridged lid in two slabs that part along its ridge.
Nothing here comes from another game or film: the stone, the glyphs and the sigil are
drawn from noise and shapes, the glyphs random strokes of no alphabet. This script writes,
into the output folder:

- `mystery_box.jpg`: the tomb's picture, 1024x1024 (the regions below);
- `mystery_box_glow.jpg`: the same layout, black but for what glows red (the shader adds
  it, pulsing);
- `mystery_box_n.png`: the same layout, the stone's relief as a normal map (+t down);
- `mystery_box_halo.jpg`: the halo's light, bright at its foot and fading upwards, in
  soft rays; `mystery_box_fire.jpg`, the same for the fire of an open tomb, in sharper
  tongues; `mystery_box_ember.jpg`, the embers glowing inside it (tiling);
- `mystery_box.md3`: the tomb, LENGTH along y, DEPTH along x (its front, a long side,
  facing +X), its origin in the middle of its bottom, open at the top under its lid
  (HEIGHT up), and a faint red halo rising over it (a second surface,
  `models/sjk/mystery_box_halo`) so it is found from afar;
- `mystery_box_lid.md3`: half the lid, its origin on the middle of the seam's bottom
  edge, reaching +X; the client draws it twice, the second turned half round, at
  (0, 0, HEIGHT), and slides the two apart;
- `mystery_box_fire.md3`: what an open tomb lets out, its origin on the embers (the
  client stands it FIRE_FLOOR up and stretches it up as the lid parts): a sheet of embers
  (`models/sjk/mystery_box_ember`) and two sleeves of red fire
  (`models/sjk/mystery_box_fire`);
- `toy_boxhunt.png`: the Box hunt toy's icon, 128x128: the tomb's front, on
  transparency.

    python scripts/mystery_box_model.py crates/sjk-viewer/assets/mystery_box

The output is deterministic (a fixed seed). Needs Pillow and numpy
(pip install pillow numpy). The shader (`mystery_box.shader`) is written by hand.
"""

import math
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter

# The tomb's body at its foot (the steps reach further), and where its lid lies.
LENGTH = 54.0
DEPTH = 28.0
HEIGHT = 27.0
SHADER = "models/sjk/mystery_box"
HALO_SHADER = "models/sjk/mystery_box_halo"
FIRE_SHADER = "models/sjk/mystery_box_fire"
EMBER_SHADER = "models/sjk/mystery_box_ember"
ATLAS = 1024
JPEG_QUALITY = 90
ICON_SIZE = 128
# Atlas regions, pixels (left, top, right, bottom).
WALL = (0, 0, 1024, 330)
LID = (0, 330, 1024, 630)
STRIP = (0, 630, 1024, 690)
END = (0, 690, 400, 940)
RIB = (0, 940, 400, 1024)
PYLON = (400, 690, 520, 1024)
STONE = (520, 690, 860, 1024)
INSIDE = (860, 690, 1024, 850)
HOT = (860, 850, 1024, 1024)
# Black basalt, and the red its cuts glow (their heart near white).
BASALT = np.array([54.0, 49.0, 53.0])
RED = np.array([255.0, 26.0, 10.0])
HEART = np.array([255.0, 190.0, 130.0])
# The halo's and the fire's light at their brightest (the shaders add them, so this is
# all they light), and their pictures' size: across (round them) and up.
HALO_LIGHT = np.array([170.0, 16.0, 8.0])
FIRE_LIGHT = np.array([255.0, 60.0, 24.0])
HALO_SIZE = (128, 256)
EMBER_SIZE = 256


def noise(rng, size, scale: float) -> np.ndarray:
    """Smooth value noise of `size` (h, w), about `scale` pixels per feature, in 0..1."""
    h, w = size
    gh, gw = int(h / scale) + 2, int(w / scale) + 2
    grid = Image.fromarray((rng.random((gh, gw)) * 255).astype(np.uint8))
    big = grid.resize((int(gw * scale) + 1, int(gh * scale) + 1), Image.BICUBIC)
    return np.asarray(big, dtype=float)[:h, :w] / 255.0


def blur(a: np.ndarray, radius: float) -> np.ndarray:
    """`a` (0..1 floats) blurred."""
    image = Image.fromarray((np.clip(a, 0, 1) * 255).astype(np.uint8))
    return np.asarray(image.filter(ImageFilter.GaussianBlur(radius)), dtype=float) / 255.0


def to_array(image: Image.Image) -> np.ndarray:
    return np.asarray(image, dtype=float) / 255.0


class Slab:
    """One atlas region being cut: `height` (1 the stone's face, lower where cut), `glow`
    (what glows red, 0..1) and `tone` (how light each block is)."""

    def __init__(self, rng, region, rows: int, cols: int):
        self.rng = rng
        self.region = region
        self.h, self.w = region[3] - region[1], region[2] - region[0]
        self.height = np.ones((self.h, self.w))
        self.glow = np.zeros((self.h, self.w))
        self.tone = np.ones((self.h, self.w))
        self.blocks(rows, cols)
        self.cracks(max(self.w * self.h // 26000, 1))

    def blocks(self, rows: int, cols: int) -> None:
        """Courses of fitted blocks, each course's joints offset from the last."""
        joints = Image.new("L", (self.w, self.h), 0)
        draw = ImageDraw.Draw(joints)
        rng = self.rng
        for i in range(rows):
            top, bottom = i * self.h // rows, (i + 1) * self.h // rows
            if i:
                draw.line([(0, top), (self.w, top)], fill=255, width=2)
            step = self.w / cols
            offset = step * (0.5 if i % 2 else 0.0) + rng.uniform(-0.12, 0.12) * step
            edges = [0] + [int(offset + k * step) for k in range(cols + 1)] + [self.w]
            edges = sorted(e for e in set(edges) if 0 <= e <= self.w)
            for left, right in zip(edges, edges[1:]):
                self.tone[top:bottom, left:right] = 0.84 + 0.3 * rng.random()
                if 0 < left:
                    draw.line([(left, top), (left, bottom)], fill=255, width=2)
        cut = to_array(joints.filter(ImageFilter.GaussianBlur(1.6)))
        self.height -= 0.55 * np.clip(cut * 2.2, 0, 1)

    def cracks(self, count: int) -> None:
        """Fine cracks wandering over the stone, a few of them faintly red."""
        lines = Image.new("L", (self.w, self.h), 0)
        hot = Image.new("L", (self.w, self.h), 0)
        rng = self.rng
        for index in range(count):
            x, y = rng.uniform(0, self.w), rng.uniform(0, self.h)
            angle = rng.uniform(0, 2 * math.pi)
            points = [(x, y)]
            for _ in range(int(rng.uniform(6, 16))):
                angle += rng.uniform(-0.7, 0.7)
                x += math.cos(angle) * rng.uniform(5, 14)
                y += math.sin(angle) * rng.uniform(5, 14)
                points.append((x, y))
            ImageDraw.Draw(lines).line(points, fill=255, width=1)
            if index % 3 == 0:
                ImageDraw.Draw(hot).line(points, fill=255, width=1)
        self.height -= 0.5 * to_array(lines.filter(ImageFilter.GaussianBlur(0.7)))
        self.glow = np.maximum(self.glow, 0.3 * to_array(hot.filter(ImageFilter.GaussianBlur(0.8))))

    def sink(self, mask: Image.Image, depth: float, soft: float = 2.0) -> None:
        """Cut `mask` (white where cut) `depth` into the stone."""
        self.height -= depth * to_array(mask.filter(ImageFilter.GaussianBlur(soft)))

    def burn(self, mask: Image.Image, depth: float = 0.6, light: float = 1.0) -> None:
        """Cut `mask` into the stone and make it glow."""
        self.sink(mask, depth, 1.4)
        self.glow = np.maximum(self.glow, light * to_array(mask.filter(ImageFilter.GaussianBlur(0.9))))

    def mask(self) -> tuple:
        image = Image.new("L", (self.w, self.h), 0)
        return image, ImageDraw.Draw(image)

    def frame(self, inset: int, width: int, depth: float = 0.35) -> None:
        """A groove round the slab, `inset` from its edge."""
        image, draw = self.mask()
        draw.rectangle((inset, inset, self.w - inset, self.h - inset), outline=255, width=width)
        self.sink(image, depth)

    def panel(self, box, depth: float = 0.3) -> None:
        """A sunk panel."""
        image, draw = self.mask()
        draw.rectangle(box, fill=255)
        self.sink(image, depth, 3.0)

    def paste(self, picture: Image.Image, centre, depth: float = 0.6, light: float = 1.0) -> None:
        """Cut `picture` (white on black) into the stone about `centre`, glowing."""
        image, _ = self.mask()
        image.paste(picture, (int(centre[0] - picture.width / 2), int(centre[1] - picture.height / 2)))
        self.burn(image, depth, light)


def glyph(rng, height: int) -> Image.Image:
    """A glyph of the tomb, white on black, `height` tall and about half as wide: a stem
    with hooks, bars and a wedge on a lattice, drawn at random; of no alphabet."""
    scale = 4
    w, h = height // 2 * scale, height * scale
    image = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(image)
    cols, rows = 3, 5
    pad = h * 0.08
    at = lambda c, r: (pad + c * (w - 2 * pad) / (cols - 1), pad + r * (h - 2 * pad) / (rows - 1))
    width = max(int(h * 0.055), 2)
    stem = int(rng.integers(0, cols))
    top, bottom = int(rng.integers(0, 2)), int(rng.integers(3, rows))
    strokes = [(at(stem, top), at(stem, bottom))]
    for _ in range(int(rng.integers(2, 5))):
        r = int(rng.integers(0, rows))
        c = int(rng.integers(0, cols))
        if c == stem:
            c = (stem + 1) % cols
        r2 = min(max(r + int(rng.integers(-1, 2)), 0), rows - 1)
        strokes.append((at(stem, r), at(c, r2)))
        if rng.random() < 0.45:
            r3 = min(max(r2 + int(rng.choice([-1, 1])), 0), rows - 1)
            strokes.append((at(c, r2), at(c, r3)))
    for a, b in strokes:
        draw.line([a, b], fill=255, width=width)
        for x, y in (a, b):
            draw.regular_polygon((x, y, width * 0.75), 4, rotation=45, fill=255)
    # A wedge, the chisel's first bite.
    x, y = at(int(rng.integers(0, cols)), int(rng.integers(0, rows)))
    s = width * 1.6
    draw.polygon([(x - s, y - s * 0.6), (x + s, y), (x - s, y + s * 0.6)], fill=255)
    return image.resize((w // scale, h // scale), Image.LANCZOS)


def sigil(size: int) -> Image.Image:
    """The tomb's sigil, white on black, `size` across: a tall blade of a diamond, split
    down its middle, between two horns curving up round it, in a broken ring; drawn here,
    of no order or emblem."""
    scale = 4
    s = size * scale
    image = Image.new("L", (s, s), 0)
    draw = ImageDraw.Draw(image)
    c = s / 2
    width = max(int(s * 0.03), 2)
    r = s * 0.46
    for start, end in ((200, 340), (20, 160)):
        draw.arc((c - r, c - r, c + r, c + r), start, end, fill=255, width=width)
    # The diamond, split.
    for side in (-1, 1):
        gap = s * 0.018
        draw.polygon(
            [
                (c + side * gap, c - s * 0.4),
                (c + side * s * 0.11, c - s * 0.02),
                (c + side * gap, c + s * 0.36),
            ],
            fill=255,
        )
    # The horns: two arcs rising round the diamond, thick at the foot.
    for side in (-1, 1):
        points = []
        for i in range(13):
            t = i / 12
            a = math.radians(-70 + 150 * t)
            points.append((c + side * (s * 0.13 + s * 0.2 * math.cos(a)), c + s * 0.08 - s * 0.3 * math.sin(a)))
        for i, (a, b) in enumerate(zip(points, points[1:])):
            draw.line([a, b], fill=255, width=max(int(width * (2.4 - 1.8 * i / 12)), 2))
    # A notch each side of the ring.
    for side in (-1, 1):
        x = c + side * r
        draw.polygon([(x - width * 2, c), (x, c - width * 1.5), (x + width * 2, c), (x, c + width * 1.5)], fill=255)
    return image.resize((size, size), Image.LANCZOS)


def chevrons(slab: Slab, cx: float, top: float, bottom: float, width: float, count: int) -> None:
    """A column of chevrons pointing up, cut glowing down `slab`."""
    image, draw = slab.mask()
    step = (bottom - top) / count
    for i in range(count):
        y = top + (i + 0.5) * step
        tip = step * 0.3
        draw.line([(cx - width / 2, y + tip), (cx, y - tip), (cx + width / 2, y + tip)], fill=255, width=3)
    slab.burn(image, 0.5, 0.8)


def wall(rng) -> Slab:
    """A long side: a framed panel, the sigil in a sunk disc at its middle, glyphs either
    side, a line of light along its top and bottom; the pilasters stand at 1/4 and 3/4."""
    slab = Slab(rng, WALL, 3, 5)
    w, h = slab.w, slab.h
    slab.frame(14, 6)
    posts = (int(w * (0.5 - 13 / LENGTH)), int(w * (0.5 + 13 / LENGTH)))
    half = int(2.4 / LENGTH * w)
    for left, right in ((30, posts[0] - half - 10), (posts[0] + half + 10, posts[1] - half - 10), (posts[1] + half + 10, w - 30)):
        slab.panel((left, 44, right, h - 44))
        image, draw = slab.mask()
        for y in (58, h - 58):
            draw.line([(left + 14, y), (right - 14, y)], fill=255, width=3)
        slab.burn(image, 0.5, 0.75)
    disc, draw = slab.mask()
    draw.ellipse((w / 2 - 112, h / 2 - 112, w / 2 + 112, h / 2 + 112), fill=255)
    slab.sink(disc, 0.25, 3.0)
    slab.paste(sigil(200), (w / 2, h / 2))
    size = 150
    for cx in (70, 150, 362, 662, 874, 954):
        slab.paste(glyph(rng, size), (cx, h / 2))
    return slab


def lid_slope(rng) -> Slab:
    """One slope of the lid, the ridge along its top: a frame, glyphs in the two panels
    between the ribs, chevrons pointing at the ridge."""
    slab = Slab(rng, LID, 3, 6)
    w, h = slab.w, slab.h
    slab.frame(12, 5)
    for cx in (w * 0.365, w * 0.635):
        slab.panel((cx - 88, 40, cx + 88, h - 40))
        for dx in (-44, 44):
            slab.paste(glyph(rng, 120), (cx + dx, h / 2 + 40))
        chevrons(slab, cx, 50, 110, 120, 2)
    for cx in (w * 0.13, w * 0.87):
        chevrons(slab, cx, h * 0.5, h - 40, 70, 3)
    return slab


def end_wall(rng) -> Slab:
    """An end: a frame and the sigil."""
    slab = Slab(rng, END, 3, 3)
    slab.frame(12, 5)
    slab.paste(sigil(170), (slab.w / 2, slab.h / 2))
    return slab


def pylon_face(rng) -> Slab:
    """A horn's or a pilaster's face: a slit of light down it, notched."""
    slab = Slab(rng, PYLON, 6, 1)
    w, h = slab.w, slab.h
    image, draw = slab.mask()
    draw.line([(w / 2, h * 0.16), (w / 2, h * 0.9)], fill=255, width=5)
    for i in range(7):
        y = h * (0.2 + 0.1 * i)
        draw.line([(w / 2 - 13, y - 8), (w / 2, y), (w / 2 + 13, y - 8)], fill=255, width=3)
    slab.burn(image, 0.6)
    return slab


def rib(rng) -> Slab:
    """A rib of the lid, along it: a groove of light."""
    slab = Slab(rng, RIB, 1, 3)
    w, h = slab.w, slab.h
    image, draw = slab.mask()
    draw.line([(26, h / 2), (w - 26, h / 2)], fill=255, width=4)
    slab.burn(image, 0.5, 0.85)
    slab.frame(6, 3, 0.3)
    return slab


def inside(rng) -> Slab:
    """The tomb's inside: stone lit from below by its embers."""
    slab = Slab(rng, INSIDE, 3, 2)
    up = 1.0 - np.arange(slab.h)[:, None] / (slab.h - 1)
    slab.tone *= 0.55
    slab.glow = np.maximum(slab.glow, np.clip((1.0 - up) ** 1.6, 0, 1) * (0.5 + 0.3 * noise(rng, (slab.h, slab.w), 14)))
    return slab


def hot(rng) -> Slab:
    """The lid's seam: stone glowing through, as hot as the embers under it."""
    slab = Slab(rng, HOT, 1, 1)
    veins = noise(rng, (slab.h, slab.w), 9)
    slab.glow = np.clip(0.55 + 0.6 * (veins - 0.4), 0, 1)
    return slab


def colour(slab: Slab) -> np.ndarray:
    """`slab`'s picture: basalt, darker in its cuts and joints, its edges worn pale, what
    glows painted in (the glow adds its light on top)."""
    rng, size = slab.rng, (slab.h, slab.w)
    grit = noise(rng, size, 2.2)
    wear = noise(rng, size, 30)
    stain = noise(rng, size, 90)
    level = slab.height * 0.5 + 0.5
    cavity = np.clip(blur(level, 5) - level, 0, 1)
    edge = np.clip(level - blur(level, 2.5), 0, 1)
    shade = (0.7 + 0.22 * grit + 0.3 * (wear - 0.5) - 0.2 * np.clip(stain - 0.55, 0, 1)) * slab.tone
    shade = shade * (1.0 - 2.4 * cavity) + 1.5 * edge
    rgb = BASALT[None, None, :] * np.clip(shade, 0.08, 1.6)[:, :, None]
    # A faint rust of red in the low stains.
    rgb[:, :, 0] += 16 * np.clip(stain - 0.5, 0, 1)
    spill = blur(slab.glow, 12)
    rgb += RED[None, None, :] * (0.3 * spill + 0.65 * slab.glow)[:, :, None]
    return rgb


def light(slab: Slab) -> np.ndarray:
    """`slab`'s glow: red, near white at the heart of each cut, spilling a little over
    the stone round it."""
    glow = slab.glow
    heart = np.clip(blur(glow, 1.2) * 1.2 - 0.25, 0, 1) ** 2
    spill = blur(glow, 10) * 0.45 + blur(glow, 3) * 0.35
    rgb = RED[None, None, :] * np.clip(glow + spill, 0, 1)[:, :, None]
    return rgb * (1 - 0.6 * heart[:, :, None]) + HEART[None, None, :] * (0.6 * heart[:, :, None])


def normals(height: np.ndarray, strength: float) -> np.ndarray:
    """A height's normal map (+s right, +t down), RGB floats 0..255."""
    soft = blur16(height * 0.5 + 0.5, 0.8)
    dy, dx = np.gradient(soft)
    n = np.dstack([-dx * strength, -dy * strength, np.ones_like(dx)])
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    return (n * 0.5 + 0.5) * 255.0


def blur16(a: np.ndarray, radius: float) -> np.ndarray:
    """`a` (floats) blurred, kept finer than a byte so a slope has no steps."""
    reach = max(int(radius * 3), 1)
    kernel = np.exp(-0.5 * (np.arange(-reach, reach + 1) / radius) ** 2)
    kernel /= kernel.sum()
    for axis in (0, 1):
        a = np.apply_along_axis(lambda row: np.convolve(np.pad(row, reach, mode="edge"), kernel, mode="valid"), axis, a)
    return a


def pictures(output: Path) -> tuple:
    rng = np.random.default_rng(66)
    atlas = np.zeros((ATLAS, ATLAS, 3))
    glow = np.zeros((ATLAS, ATLAS, 3))
    relief = np.full((ATLAS, ATLAS, 3), (127.5, 127.5, 255.0))
    slabs = [
        wall(rng),
        lid_slope(rng),
        Slab(rng, STRIP, 1, 9),
        end_wall(rng),
        rib(rng),
        pylon_face(rng),
        Slab(rng, STONE, 4, 3),
        inside(rng),
        hot(rng),
    ]
    for slab in slabs:
        left, top, right, bottom = slab.region
        atlas[top:bottom, left:right] = colour(slab)
        glow[top:bottom, left:right] = light(slab)
        relief[top:bottom, left:right] = normals(slab.height, 8.0)
    as_image = lambda a: Image.fromarray(np.clip(a, 0, 255).astype(np.uint8))
    image, lit = as_image(atlas), as_image(glow)
    image.save(output / "mystery_box.jpg", quality=JPEG_QUALITY)
    lit.save(output / "mystery_box_glow.jpg", quality=JPEG_QUALITY)
    as_image(relief).save(output / "mystery_box_n.png", optimize=True)
    rays(rng, HALO_LIGHT, 16, 1.4).save(output / "mystery_box_halo.jpg", quality=96)
    rays(rng, FIRE_LIGHT, 40, 1.0, tongues=True).save(output / "mystery_box_fire.jpg", quality=96)
    embers(rng).save(output / "mystery_box_ember.jpg", quality=JPEG_QUALITY)
    return image, lit


def rays(rng, colour_at_foot: np.ndarray, count: int, fade: float, tongues: bool = False) -> Image.Image:
    """A sleeve of light: `count` soft rays round it (wrapping), brightest at the foot
    (the bottom row) and gone at the top; as `tongues`, each ray dies at its own height."""
    w, h = HALO_SIZE
    across = rng.random(count)
    s = np.arange(w) / w * count
    i = s.astype(int)
    f = s - i
    f = f * f * (3 - 2 * f)
    ray = across[i % count] * (1 - f) + across[(i + 1) % count] * f
    t = 1.0 - np.arange(h)[:, None] / (h - 1)
    if tongues:
        reach = 0.35 + 0.65 * ray[None, :]
        up = np.clip(1.0 - t / reach, 0, 1) ** fade * np.clip(t / 0.04, 0.0, 1.0)
        level = up * (0.35 + 0.65 * ray[None, :] ** 2)
    else:
        up = np.clip(1.0 - t / 0.75, 0, 1) ** fade * np.clip(t / 0.06, 0.0, 1.0)
        level = up * (0.6 + 0.4 * ray[None, :])
    # The client adds this in linear light, where a faint grey is far from black: square
    # it, so the light dies away to nothing rather than to a red pane.
    level = level**2
    rgb = level[:, :, None] * colour_at_foot[None, None, :]
    # The hottest of it burns toward white.
    rgb += np.clip(level - 0.6, 0, 1)[:, :, None] * np.array([0.0, 150.0, 110.0])
    return Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8))


def embers(rng) -> Image.Image:
    """Embers seen from above, tiling: veins of red through the dark."""
    n = EMBER_SIZE
    field = np.zeros((n, n))
    for cells in (4, 8, 16):
        grid = rng.random((cells, cells))
        grid = np.pad(grid, ((1, 2), (1, 2)), mode="wrap")
        size = (cells + 3) * (n // cells)
        big = np.asarray(Image.fromarray((grid * 255).astype(np.uint8)).resize((size, size), Image.BICUBIC), dtype=float) / 255.0
        start = n // cells
        field += big[start : start + n, start : start + n] / cells**0.5
    field = (field - field.min()) / (field.max() - field.min())
    veins = np.clip(1.0 - np.abs(field - 0.5) * 7.0, 0, 1) ** 2
    level = 0.25 * field + 0.9 * veins
    rgb = level[:, :, None] * RED[None, None, :] + (veins**3)[:, :, None] * np.array([0.0, 140.0, 90.0])
    return Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8))


def encode_normal(n) -> int:
    # The inverse of sjk-model's `decode_normal` (stock's MD3 normal), as in
    # cardboard_box_assets.py.
    around = round(math.atan2(n[1], n[0]) * 255 / (2 * math.pi)) & 0xFF
    from_up = round(math.acos(max(-1.0, min(1.0, n[2]))) * 255 / (2 * math.pi)) & 0xFF
    return (around << 8) | from_up


def write_md3(path: Path, name: str, surfaces: list) -> None:
    """`surfaces`: (name, shader, quads), `quads` holding (corners, four normals, None,
    four (s, t)); a triangle repeats its last corner. Each is turned clockwise seen from
    outside, stock's front side, if it is not."""

    def text(value: str, size: int) -> bytes:
        return value.encode("ascii").ljust(size, b"\0")

    every = []
    blobs = []
    for surface_name, shader, quads in surfaces:
        positions, blob = md3_surface(surface_name, shader, quads, text)
        every += positions
        blobs.append(blob)
    surface_bytes = b"".join(blobs)
    points = np.array(every)
    mins, maxs = points.min(axis=0), points.max(axis=0)
    centre = (mins + maxs) / 2
    radius = float(np.linalg.norm(maxs - centre))
    frame = struct.pack("<10f", *mins, *maxs, *centre, radius) + text(name, 16)
    header_size = 108
    ofs_frames = header_size
    ofs_surfaces = ofs_frames + len(frame)
    end = ofs_surfaces + len(surface_bytes)
    header = (
        b"IDP3"
        + struct.pack("<i", 15)
        + text(f"models/sjk/{name}.md3", 64)
        + struct.pack(
            "<9i", 0, 1, 0, len(surfaces), 0, ofs_frames, ofs_surfaces, ofs_surfaces, end
        )
    )
    path.write_bytes(header + frame + surface_bytes)


def md3_surface(name: str, shader: str, quads: list, text) -> tuple:
    """One MD3 surface of `quads` (as [`write_md3`] takes them) with `shader`: its
    positions and its bytes."""
    positions, normals, uvs, triangles = [], [], [], []
    for corners, corner_normals, _, sts in quads:
        p = [np.array(c, dtype=float) for c in corners]
        outward = np.sum(np.array(corner_normals, dtype=float), axis=0)
        cross = np.cross(p[1] - p[0], p[2] - p[0])
        order = [0, 1, 2, 3] if np.dot(cross, outward) <= 0 else [0, 3, 2, 1]
        base = len(positions)
        for i in order:
            positions.append(tuple(float(v) for v in corners[i]))
            normals.append(tuple(corner_normals[i]))
            uvs.append(tuple(sts[i]))
        triangles.append((base, base + 1, base + 2))
        if positions[base + 2] != positions[base + 3]:
            triangles.append((base, base + 2, base + 3))

    shaders = text(shader, 64) + struct.pack("<i", 0)
    tris = b"".join(struct.pack("<3i", *t) for t in triangles)
    sts = b"".join(struct.pack("<2f", *uv) for uv in uvs)
    verts = b"".join(
        struct.pack("<3hH", *(round(c * 64) for c in p), encode_normal(n))
        for p, n in zip(positions, normals)
    )
    surface_header = 108
    ofs_tris = surface_header + len(shaders)
    ofs_st = ofs_tris + len(tris)
    ofs_verts = ofs_st + len(sts)
    surface_end = ofs_verts + len(verts)
    surface = (
        b"IDP3"
        + text(name, 64)
        + struct.pack(
            "<10i", 0, 1, 1, len(positions), len(triangles),
            ofs_tris, surface_header, ofs_st, ofs_verts, surface_end,
        )
        + shaders + tris + sts + verts
    )
    return positions, surface


# The steps the tomb stands on: half depth, half length and top of each, each STEP_LIP
# wider at its foot.
STEPS = ((20.0, 37.0, 3.0), (17.0, 33.0, 6.0))
STEP_LIP = 0.5
# The body's walls lean in from its foot to its top, under the rim.
BODY_TOP = (12.0, 25.0, 24.0)
RIM = (13.5, 26.5)
# The opening under the lid, and the floor inside.
OPENING = (10.5, 23.5)
FLOOR = 9.0
# How long a strip of STRIP is, so long thin faces share its grain.
STRIP_LENGTH = 74.0
# The horns at the corners: where each stands, half its foot, half its top and how far
# that leans in, how high its shaft and its tip reach.
HORN_AT = (15.5, 32.5)
HORN_FOOT = 3.5
HORN_TOP = 1.9
HORN_LEAN = (1.6, 1.0)
HORN_SHAFT = 38.0
HORN_TIP = 47.0
# Half the lid: how far it reaches from the seam, half its length, its edge's thickness,
# the ridge's height and half length, the strip of light along the ridge, and its ribs.
LID_REACH = 15.0
LID_HALF = 27.5
LID_EDGE = 3.0
LID_RIDGE = 8.0
LID_RIDGE_HALF = 19.0
LID_SEAM = 0.7
RIBS = (-15.0, 0.0, 15.0)
RIB_HALF = 2.25
RIB_OUT = 0.7
# The halo over the tomb: how far round it stands out of the tomb's top, how tall it
# rises, and in how many strips round.
HALO_OUT = 2.0
HALO_RISE = 64.0
HALO_STEPS = 32
# The fire of an open tomb: how tall its sleeves rise from its embers (the client
# stretches them).
FIRE_RISE = 62.0


def face(corners, region, inside, full_u=None, full_v=None) -> tuple:
    """A flat face (a triangle repeats its last corner) with `region` laid flat on it:
    across it level (along y on a level face) and down it, fitted to the face, or
    `full_u` / `full_v` units to the region's width / height, centred. Its normal points
    away from `inside`."""
    p = [np.array(c, dtype=float) for c in corners]
    normal = np.zeros(3)
    for i in range(len(p)):
        normal += np.cross(p[i], p[(i + 1) % len(p)])
    normal /= np.linalg.norm(normal)
    middle = np.mean(p, axis=0)
    if np.dot(normal, middle - np.array(inside, dtype=float)) < 0:
        normal = -normal
    if abs(normal[2]) > 0.95:
        u_axis, v_axis = np.array([0.0, 1.0, 0.0]), np.array([1.0, 0.0, 0.0])
    else:
        u_axis = np.cross([0.0, 0.0, 1.0], normal)
        u_axis /= np.linalg.norm(u_axis)
        v_axis = np.cross(u_axis, normal)
        if v_axis[2] > 0:
            v_axis = -v_axis
    left, top, right, bottom = region
    sts = []
    for axis, full, low, high in ((u_axis, full_u, left, right), (v_axis, full_v, top, bottom)):
        along = [float(np.dot(c, axis)) for c in p]
        if full is None:
            span = max(max(along) - min(along), 1e-6)
            part = [(a - min(along)) / span for a in along]
        else:
            mid = (max(along) + min(along)) / 2
            part = [min(max(0.5 + (a - mid) / full, 0.0), 1.0) for a in along]
        # Half a pixel in, so a neighbouring region never bleeds over the edge.
        sts.append([(low + 0.5 + f * (high - low - 1)) / ATLAS for f in part])
    return ([tuple(float(v) for v in c) for c in corners], [tuple(normal)] * 4, None, list(zip(*sts)))


def rect(hx: float, hy: float, z: float, cx: float = 0.0, cy: float = 0.0) -> list:
    """A level rectangle's corners, counter-clockwise seen from above."""
    return [(cx + hx, cy - hy, z), (cx + hx, cy + hy, z), (cx - hx, cy + hy, z), (cx - hx, cy - hy, z)]


def hexa(bottom: list, top: list, sides=None, lid=None, floor=None, **how) -> list:
    """A block between two quads (corner for corner): its sides with region `sides` (one
    region, or one per side), a top with `lid` and a bottom with `floor`, where given."""
    inside = np.mean(np.array(bottom + top, dtype=float), axis=0)
    quads = []
    for i in range(4):
        region = sides[i] if isinstance(sides, list) else sides
        if region is not None:
            j = (i + 1) % 4
            quads.append(face([top[i], top[j], bottom[j], bottom[i]], region, inside, **how))
    if lid is not None:
        quads.append(face(top, lid, inside, **how))
    if floor is not None:
        quads.append(face(bottom, floor, inside, **how))
    return quads


def ring(outer: list, inner: list) -> list:
    """A level border between two rectangles, mitred, facing up, STRIP's grain along
    each side."""
    left, top, right, bottom = STRIP
    quads = []
    for i in range(4):
        j = (i + 1) % 4
        corners = [outer[i], outer[j], inner[j], inner[i]]
        # Along the side: y for the sides at +x and -x (0 and 2), x for the others.
        along = 1 if i % 2 == 0 else 0
        sts = []
        for k, c in enumerate(corners):
            u = min(max(0.5 + c[along] / STRIP_LENGTH, 0.0), 1.0)
            v = 0.0 if k < 2 else 1.0
            sts.append(((left + 0.5 + u * (right - left - 1)) / ATLAS, (top + 0.5 + v * (bottom - top - 1)) / ATLAS))
        quads.append(([tuple(float(v) for v in c) for c in corners], [(0.0, 0.0, 1.0)] * 4, None, sts))
    return quads


def tomb() -> list:
    quads = []
    strip = {"full_u": STRIP_LENGTH}
    small = {"full_u": 14.0, "full_v": 14.0}
    # The two steps, each with a border of its top showing round what stands on it.
    (hx1, hy1, z1), (hx2, hy2, z2) = STEPS
    quads += hexa(rect(hx1 + STEP_LIP, hy1 + STEP_LIP, 0.0), rect(hx1, hy1, z1), STRIP, **strip)
    quads += ring(rect(hx1, hy1, z1), rect(hx2, hy2, z1))
    quads += hexa(rect(hx2 + STEP_LIP, hy2 + STEP_LIP, z1), rect(hx2, hy2, z2), STRIP, **strip)
    hx, hy = DEPTH / 2, LENGTH / 2
    quads += ring(rect(hx2, hy2, z2), rect(hx, hy, z2))
    # The body, its walls leaning in: the long sides, then the ends.
    tx, ty, tz = BODY_TOP
    quads += hexa(rect(hx, hy, z2), rect(tx, ty, tz), [WALL, END, WALL, END])
    # A pilaster a quarter in from each end of the long sides, a slit of light down it.
    for sx in (-1, 1):
        for y in (-13.0, 13.0):
            foot = [(sx * (hx - 0.4), y - 2.4, z2), (sx * (hx - 0.4), y + 2.4, z2), (sx * (hx + 1.5), y + 2.0, z2), (sx * (hx + 1.5), y - 2.0, z2)]
            head = [(sx * (tx - 0.4), y - 2.2, tz), (sx * (tx - 0.4), y + 2.2, tz), (sx * (tx + 1.1), y + 1.8, tz), (sx * (tx + 1.1), y - 1.8, tz)]
            quads += hexa(foot, head, [None, STONE, PYLON, STONE], **small)
    # The rim round the opening: its sides, its top round the opening.
    rx, ry = RIM
    ox, oy = OPENING
    quads += hexa(rect(rx - 1.0, ry - 1.0, tz), rect(rx, ry, tz + 1.0), STRIP, **strip)
    quads += hexa(rect(rx, ry, tz + 1.0), rect(rx, ry, HEIGHT), STRIP, **strip)
    quads += ring(rect(rx, ry, HEIGHT), rect(ox, oy, HEIGHT))
    # Inside: walls facing in and a floor, lit by the embers.
    walls = hexa(rect(ox, oy, FLOOR), rect(ox, oy, HEIGHT), INSIDE)
    quads += [(corners, [tuple(-np.array(n)) for n in normal], region, sts) for corners, normal, region, sts in walls]
    quads.append(face(rect(ox, oy, FLOOR), HOT, (0, 0, -100)))
    # The horns: a shaft leaning in over each corner, a collar, sharpened to a tip.
    for sx in (-1, 1):
        for sy in (-1, 1):
            cx, cy = sx * HORN_AT[0], sy * HORN_AT[1]
            tx2, ty2 = cx - sx * HORN_LEAN[0], cy - sy * HORN_LEAN[1]
            foot = rect(HORN_FOOT, HORN_FOOT, z1, cx, cy)
            head = rect(HORN_TOP, HORN_TOP, HORN_SHAFT, tx2, ty2)
            quads += hexa(foot, head, PYLON)
            collar = HORN_TOP + 0.7
            quads += hexa(
                rect(collar, collar, HORN_SHAFT - 2.2, tx2, ty2),
                rect(collar, collar, HORN_SHAFT, tx2, ty2),
                STONE, lid=STONE, floor=STONE, **small,
            )
            tip = (tx2 - sx * 0.6, ty2 - sy * 0.4, HORN_TIP)
            inside = (tx2, ty2, HORN_SHAFT)
            for i in range(4):
                a, b = head[i], head[(i + 1) % 4]
                quads.append(face([a, b, tip, tip], STONE, inside, **small))
    return quads


def halo() -> list:
    """The halo: a band of light round the tomb's top (an ellipse standing HALO_OUT out
    of it), rising HALO_RISE from the rim's top; its picture wraps once round it."""
    return sleeve(DEPTH / 2 + HALO_OUT, LENGTH / 2 + HALO_OUT, 1.0, HEIGHT, HEIGHT + HALO_RISE, 1)


def sleeve(rx: float, ry: float, flare: float, z0: float, z1: float, wraps: int) -> list:
    """A band of light: an ellipse `rx` by `ry` at `z0`, `flare` times as wide at `z1`,
    its picture wrapping `wraps` times round it."""
    quads = []
    for i in range(HALO_STEPS):
        a0, a1 = (2 * math.pi * k / HALO_STEPS for k in (i, i + 1))
        p0 = (rx * math.cos(a0), ry * math.sin(a0))
        p1 = (rx * math.cos(a1), ry * math.sin(a1))
        n0 = (math.cos(a0), math.sin(a0), 0.0)
        n1 = (math.cos(a1), math.sin(a1), 0.0)
        s0, s1 = wraps * i / HALO_STEPS, wraps * (i + 1) / HALO_STEPS
        corners = [(p0[0] * flare, p0[1] * flare, z1), (p1[0] * flare, p1[1] * flare, z1), (*p1, z0), (*p0, z0)]
        sts = [(s0, 0.0), (s1, 0.0), (s1, 1.0), (s0, 1.0)]
        quads.append((corners, [n0, n1, n1, n0], None, sts))
    return quads


def fire() -> tuple:
    """The fire of an open tomb, about its embers: the sheet of embers filling the
    opening, and two sleeves of fire rising out of it, one inside the other."""
    ox, oy = OPENING
    sheet = [(rect(ox - 0.2, oy - 0.2, 0.0), [(0.0, 0.0, 1.0)] * 4, None, [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (0.0, 1.0)])]
    sleeves = sleeve(ox - 2.0, oy - 3.0, 1.15, 0.0, FIRE_RISE, 2)
    sleeves += sleeve(ox - 5.5, oy - 9.0, 0.8, 0.0, FIRE_RISE * 1.25, 1)
    return sheet, sleeves


def lid() -> list:
    """Half the lid, from the seam (x = 0) out to its edge."""
    reach, half, edge, ridge, ridge_half, seam = LID_REACH, LID_HALF, LID_EDGE, LID_RIDGE, LID_RIDGE_HALF, LID_SEAM
    inside = (reach / 2, 0.0, 1.0)
    strip = {"full_u": STRIP_LENGTH}
    small = {"full_u": 14.0, "full_v": 14.0}
    hip = {"full_u": 30.0, "full_v": 30.0}
    quads = []
    # The strip of light along the ridge, and the slope down to the edge.
    quads.append(face([(0, -ridge_half, ridge), (0, ridge_half, ridge), (seam, ridge_half, ridge), (seam, -ridge_half, ridge)], HOT, inside))
    quads.append(face([(seam, -ridge_half, ridge), (seam, ridge_half, ridge), (reach, half, edge), (reach, -half, edge)], LID, inside))
    for s in (-1, 1):
        # The hip at each end, and the sliver of it beside the strip of light.
        quads.append(face([(0, s * ridge_half, ridge), (reach, s * half, edge), (0, s * half, edge), (0, s * half, edge)], STONE, inside, **hip))
        quads.append(face([(0, s * ridge_half, ridge), (seam, s * ridge_half, ridge), (reach, s * half, edge), (reach, s * half, edge)], STONE, inside, **hip))
        # The end's edge.
        quads.append(face([(0, s * half, edge), (reach, s * half, edge), (reach, s * half, 0), (0, s * half, 0)], STRIP, inside, **strip))
    # The long edge, the underside, and the seam's face (hot where the stone is thick).
    quads.append(face([(reach, -half, edge), (reach, half, edge), (reach, half, 0), (reach, -half, 0)], STRIP, inside, **strip))
    quads.append(face([(0, -half, 0), (0, half, 0), (reach, half, 0), (reach, -half, 0)], INSIDE, inside))
    quads.append(face([(0, -half, edge), (0, half, edge), (0, half, 0), (0, -half, 0)], HOT, inside))
    quads.append(face([(0, -ridge_half, ridge), (0, ridge_half, ridge), (0, half, edge), (0, -half, edge)], HOT, inside))
    # Ribs down the slope, each ending in a clasp over the edge.
    out = np.array([ridge - edge, 0.0, reach - seam])
    out /= np.linalg.norm(out)
    on_slope = lambda x, y: np.array([x, y, ridge + (x - seam) / (reach - seam) * (edge - ridge)])
    left, top_px, right, bottom_px = RIB
    uv = lambda u, v: ((left + 0.5 + u * (right - left - 1)) / ATLAS, (top_px + 0.5 + v * (bottom_px - top_px - 1)) / ATLAS)
    for y in RIBS:
        y0, y1 = y - RIB_HALF, y + RIB_HALF
        x0, x1 = 1.8, reach
        base = [on_slope(x1, y0), on_slope(x1, y1), on_slope(x0, y1), on_slope(x0, y0)]
        top = [tuple(c + out * RIB_OUT) for c in base]
        base = [tuple(c) for c in base]
        quads += hexa(base, top, [None, STONE, STONE, STONE], **small)
        # Its top, the rib's picture along it (the ridge end at the picture's left).
        quads.append((top, [tuple(out)] * 4, None, [uv(1, 0), uv(1, 1), uv(0, 1), uv(0, 0)]))
        quads += hexa(
            rect(0.55, RIB_HALF, -0.9, reach + 0.25, y),
            rect(0.55, RIB_HALF, edge + 0.95, reach + 0.25, y),
            STONE, lid=STONE, floor=STONE, **small,
        )
    return quads


def models(output: Path) -> None:
    write_md3(
        output / "mystery_box.md3",
        "mystery_box",
        [("mystery_box", SHADER, tomb()), ("mystery_box_halo", HALO_SHADER, halo())],
    )
    write_md3(output / "mystery_box_lid.md3", "mystery_box_lid", [("mystery_box_lid", SHADER, lid())])
    sheet, sleeves = fire()
    write_md3(
        output / "mystery_box_fire.md3",
        "mystery_box_fire",
        [("mystery_box_ember", EMBER_SHADER, sheet), ("mystery_box_fire", FIRE_SHADER, sleeves)],
    )


def icon(output: Path, atlas: Image.Image, lit: Image.Image) -> None:
    """The tomb's front with its lid's slope above, glowing, on transparency."""
    scale = 4
    size = ICON_SIZE * scale
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    glowing = ImageChops.add(atlas.point(lambda v: min(int(v * 1.5), 255)), lit)
    front = glowing.crop(WALL)
    width = int(size * 0.92)
    height = int(width * (WALL[3] - WALL[1]) / (WALL[2] - WALL[0]) * 1.25)
    lid_height = height // 2
    top = (size - height - lid_height) // 2
    left = (size - width) // 2
    slope = glowing.crop(LID).resize((width, lid_height)).convert("RGBA")
    # The lid's slope narrows to its ridge.
    mask = Image.new("L", (width, lid_height), 0)
    inset = int(width * (1 - LID_RIDGE_HALF / LID_HALF) / 2)
    ImageDraw.Draw(mask).polygon([(inset, 0), (width - inset, 0), (width, lid_height), (0, lid_height)], fill=255)
    canvas.paste(slope, (left, top), mask)
    canvas.paste(front.resize((width, height)).convert("RGBA"), (left, top + lid_height))
    alpha = canvas.split()[3]
    halo = alpha.filter(ImageFilter.MaxFilter(9)).filter(ImageFilter.GaussianBlur(3))
    ground = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ground.putalpha(halo.point(lambda v: int(v * 0.7)))
    ground.alpha_composite(canvas)
    ground.resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS).save(output / "toy_boxhunt.png")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    atlas, lit = pictures(output)
    models(output)
    icon(output, atlas, lit)


if __name__ == "__main__":
    main()
