#!/usr/bin/env python3
"""SJK's emblem, built from its geometry instead of traced from a picture.

Sol's emblem (16/11/2026, after his "Can you make a simplified" and "Adding sun
to logo concepts" renders) is a monogram of circles and 45-degree strokes: two
rings, a pair of pillars with chamfered ends standing through them, and three
arms, a K on the right pillar and the peace sign's left arm on the other, in
steel before a burning sun. The renders it came as are AI pictures whose strokes
wander by a few pixels; this script rebuilds the same design from a handful of
numbers measured on them, so every size, from the 16-pixel icon to a 4K menu, is
drawn exactly:

- each piece is a signed distance field ([`Geometry`]): negative inside, in design
  units (the 2048 square), so edges are anti-aliased analytically at any size;
- the steel is shaded from that field: a bevel whose height follows the distance
  to the edge, lit like polished gunmetal, warmed by the sun behind ([`metal`]);
- the sun is procedural ([`sun`]): churning plasma with bright filaments, a hot
  limb, two layers of flame tongues, a corona, god rays, pointed spikes and
  embers, bloomed and tone-mapped like a film frame;
- small icons (32 pixels and under) use heavier strokes and a plain warm disc
  ([`SMALL`]), as the full drawing turns to mush there;
- the vector master is written as SVG from the same numbers ([`svg`]).

Usage:
    python scripts/sjk_emblem.py build [repository root, default .]
    python scripts/sjk_emblem.py preview <out dir> [size]
    python scripts/sjk_emblem.py svg <out.svg>

Needs Python 3 with Pillow, numpy and scipy. The output is deterministic.
"""
import math
import sys
from dataclasses import dataclass, replace
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

DESIGN = 2048.0
SQRT2 = math.sqrt(2.0)


# ---------------------------------------------------------------------------
# Geometry, in design units: the 2048 square of the source render, centre at the
# origin, y up.
# ---------------------------------------------------------------------------
@dataclass(frozen=True)
class Geometry:
    gap: float = 25.0  # half the slot between the pillars
    pillar: float = 54.0  # each pillar's width
    inner: tuple = (310.0, 364.0)  # the inner ring, welded to pillars and arms
    outer_thin: tuple = (403.0, 420.0)  # the outer ring's thin inner band
    outer: tuple = (425.0, 473.0)  # the outer ring's main band
    tip: float = 610.0  # the pillars' inner corners at each end; a 45-degree
    # chamfer runs from there to the outer corner
    reach: float = 337.0  # the arms run into the inner ring's middle
    outline: float = 9.0  # the dark line round each piece
    bevel: float = 13.0  # the bevel inside each edge
    lift: float = 0.0  # lighter steel, for contrast in the 48 and 64 icons
    arm_width: float = 0.0  # 0: the full design's rule ([`Geometry::arm`])
    arms: bool = True  # the 16-pixel icon leaves them out
    flat: bool = False  # plain light steel and a plain disc (the small icons)
    disc: float = 452.0  # the sun's radius: just under the outer ring's edge

    @property
    def out(self):
        """The pillars' outer edges."""
        return self.gap + self.pillar

    @property
    def arm(self):
        """The arms' width: the K's two arms then meet exactly on the right
        pillar's outer edge at the centre line, and the left arm's top edge
        reaches the left pillar there too."""
        return self.arm_width or self.out / SQRT2


FULL = Geometry()
# Icons of 32 pixels and under: the thin band goes, the strokes and the slot grow
# so each stays a pixel or more, the outline thins to a hairline.
SMALL = replace(
    FULL,
    gap=0.0,
    pillar=80.0,
    inner=(468.0, 580.0),
    outer_thin=(0.0, 0.0),
    outer=(0.0, 0.0),
    tip=700.0,
    reach=524.0,
    outline=42.0,
    bevel=40.0,
    arm_width=118.0,
    disc=560.0,
    flat=True,
)
# 48 and 64 pixels: the full drawing without the thin band, a little heavier.
MEDIUM = replace(
    FULL,
    gap=26.0,
    pillar=64.0,
    inner=(300.0, 372.0),
    outer_thin=(0.0, 0.0),
    outer=(414.0, 478.0),
    tip=620.0,
    outline=13.0,
    bevel=18.0,
    lift=0.22,
)
# 32 pixels: the small drawing keeps the pillars' slot, a pixel wide.
SMALL32 = replace(SMALL, gap=26.0, pillar=72.0, arm_width=104.0)
# 16 pixels: the ring and the stem on the sun; the arms would be a checkerboard.
TINY = replace(SMALL, arms=False, inner=(452.0, 576.0), pillar=92.0)
# The squares the icons show: the large ones keep the flames and cut the
# spikes; the small ones fit the stem's ends.
ICON_SPAN = 1640.0
SMALL_SPAN = 1440.0


def grid(size, span=DESIGN):
    """Design coordinates of the pixel centres of a `size` square showing `span`
    design units, and the design units one pixel spans."""
    px = span / size
    c = (np.arange(size) + 0.5) * px - span / 2.0
    x, y = np.meshgrid(c, -c)
    return x, y, px


def annulus(r, band):
    mid = (band[0] + band[1]) / 2.0
    half = (band[1] - band[0]) / 2.0
    return np.abs(r - mid) - half


def pillar(g, x, y, side):
    xs = x * side
    return np.maximum.reduce(
        [xs - g.out, g.gap - xs, (np.abs(y) + (xs - g.gap) - g.tip) / SQRT2]
    )


def arm(g, x, r, s, side):
    """A band of the arms' width on one side of a line through the centre (`s`
    is the signed distance from that line), from inside the pillar to the inner
    ring's middle."""
    return np.maximum.reduce([-s, s - g.arm, g.gap + 35.0 - x * side, r - g.reach])


def body(g, x, y):
    """The front piece: inner ring, pillars and arms, one welded body."""
    r = np.hypot(x, y)
    pieces = [annulus(r, g.inner), pillar(g, x, y, 1.0), pillar(g, x, y, -1.0)]
    if g.arms:
        pieces += [
            arm(g, x, r, (x - y) / SQRT2, 1.0),  # the K's upper arm, under y = x
            arm(g, x, r, (x + y) / SQRT2, 1.0),  # the K's lower arm, over y = -x
            arm(g, x, r, (y - x) / SQRT2, -1.0),  # the left arm, over y = x
        ]
    return np.minimum.reduce(pieces)


def rings(g, x, y):
    """The outer ring's bands, behind the body."""
    r = np.hypot(x, y)
    d = np.full_like(r, 1e6)
    for band in (g.outer, g.outer_thin):
        if band[1] > band[0]:
            d = np.minimum(d, annulus(r, band))
    return d


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
def coverage(d, px):
    """Anti-aliased inside-ness of a distance field."""
    return np.clip(0.5 - d / px, 0.0, 1.0)


def smoothstep(a, b, v):
    t = np.clip((v - a) / (b - a), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def ramp(t, stops):
    """Colour along `stops` [(position, (r, g, b)), ...] for each value of `t`."""
    out = np.zeros(t.shape + (3,))
    pos = [p for p, _ in stops]
    for c in range(3):
        out[..., c] = np.interp(t, pos, [col[c] for _, col in stops])
    return out


def blur(img, sigma_design, px):
    sigma = sigma_design / px
    if sigma < 0.3:
        return img.copy()
    if img.ndim == 3:
        return np.stack(
            [ndimage.gaussian_filter(img[..., c], sigma, mode="constant") for c in range(3)],
            axis=-1,
        )
    return ndimage.gaussian_filter(img, sigma, mode="constant")


def value_noise(x, y, seed):
    """Smooth value noise on the unit lattice."""
    rng = np.random.default_rng(seed)
    table = rng.random((256, 256))
    xi = np.floor(x).astype(np.int64)
    yi = np.floor(y).astype(np.int64)
    xf = x - xi
    yf = y - yi
    u = xf * xf * (3 - 2 * xf)
    v = yf * yf * (3 - 2 * yf)
    a = table[yi & 255, xi & 255]
    b = table[yi & 255, (xi + 1) & 255]
    c = table[(yi + 1) & 255, xi & 255]
    d = table[(yi + 1) & 255, (xi + 1) & 255]
    return (a + (b - a) * u) * (1 - v) + (c + (d - c) * u) * v


def fbm(x, y, octaves, seed, gain=0.5):
    total = np.zeros_like(x)
    amp = 0.5
    norm = 0.0
    for o in range(octaves):
        total += amp * value_noise(x, y, seed + o * 17)
        norm += amp
        x = x * 2.03 + 11.7
        y = y * 2.03 - 5.3
        amp *= gain
    return total / norm


def periodic(theta, freq, seed, octaves=4, offset=0.0):
    """Noise round the circle, seamless at the wrap (noise sampled on a circle
    of radius `freq`); `offset` slides along a second axis."""
    return fbm(np.cos(theta) * freq + 40.0 + offset, np.sin(theta) * freq + 40.0, octaves, seed)


def aces(x):
    """The ACES filmic curve (Narkowicz's fit)."""
    return np.clip((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0)


# ---------------------------------------------------------------------------
# The sun
# ---------------------------------------------------------------------------
DISC = 452.0  # the sun's radius: just under the outer ring's edge


def plasma(x, y, r, px):
    """The disc: slow swirls, granulation and bright filaments, brightest at the
    limb where it shows round the rings."""
    w1 = fbm(x / 230.0, y / 230.0, 4, 3)
    w2 = fbm(x / 230.0 + 5.2, y / 230.0 + 1.3, 4, 4)
    swirl = fbm(x / 95.0 + 3.2 * w1, y / 95.0 + 3.2 * w2, 5, 7)
    gran = fbm(x / 18.0, y / 18.0, 3, 8)
    fil = 1.0 - np.abs(2.0 * fbm(x / 70.0 + 2.4 * w2, y / 70.0 - 2.4 * w1, 5, 9) - 1.0)
    t = 0.72 * swirl + 0.16 * gran + 0.42 * fil**6 - 0.12
    stops = [
        (0.00, (0.30, 0.03, 0.00)),
        (0.30, (0.85, 0.20, 0.01)),
        (0.48, (1.35, 0.48, 0.04)),
        (0.62, (1.80, 0.85, 0.12)),
        (0.80, (2.40, 1.45, 0.40)),
        (1.00, (3.20, 2.40, 1.10)),
    ]
    colour = ramp(np.clip(t, 0.0, 1.0), stops)
    limb = 0.7 + 0.9 * smoothstep(DISC * 0.45, DISC, r) ** 2
    return colour * limb[..., None] * coverage(r - DISC, px)[..., None]


def flames(theta, out, outside, seed, freq, base, length, fade, root, tip, wobble):
    """A ring of flame tongues: wide at the root and pointed at the tip, curling
    as they rise, streaked along their length, gold at the root and red at the
    ends."""
    curl = theta + wobble * np.sin(out / 55.0 + 6.0 * periodic(theta, 1.6, seed + 1, 3))
    field = periodic(curl, freq, seed, 5)
    threshold = base + out / length
    tongue = smoothstep(threshold, threshold + 0.09, field) * np.exp(-out / fade)
    streak = 0.7 + 0.5 * periodic(curl - out / 900.0, freq * 5.0, seed + 2, 3)
    hue = np.exp(-out / (fade * 0.45))[..., None]
    colour = np.array(tip) + (np.array(root) - np.array(tip)) * hue
    return (tongue * streak * outside)[..., None] * colour


def spikes(theta, r, out, outside, rng):
    """Pointed rays: the vertical one the longest, as in Sol's render, the
    horizontal and diagonal ones shorter, and thin ones at uneven angles."""
    total = np.zeros_like(r)
    rays = [
        (math.pi / 2, 1100.0, 0.045, 1.6),
        (-math.pi / 2, 1100.0, 0.045, 1.6),
        (0.0, 620.0, 0.04, 1.1),
        (math.pi, 620.0, 0.04, 1.1),
    ]
    for k in range(4):
        rays.append((math.pi / 4 + k * math.pi / 2, 470.0, 0.035, 0.8))
    for _ in range(30):
        rays.append(
            (
                rng.uniform(-math.pi, math.pi),
                rng.uniform(150.0, 520.0),
                rng.uniform(0.012, 0.03),
                rng.uniform(0.3, 0.9),
            )
        )
    for angle, length, root, strength in rays:
        da = np.abs(np.angle(np.exp(1j * (theta - angle))))
        half = root * np.clip(1.0 - out / length, 0.0, 1.0) * (DISC / np.maximum(r, 1.0))
        core = smoothstep(0.0, 1.0, 1.0 - da / np.maximum(half, 1e-6)) * (out < length)
        total += strength * core * (0.6 + 0.4 * core)
    return (total * outside)[..., None] * np.array([1.8, 0.95, 0.3])


def embers(x, y, px, rng, count=120):
    """Sparks drifting out, each a short streak along its way."""
    light = np.zeros(x.shape + (3,))
    size_px = x.shape[0]
    for _ in range(count):
        angle = rng.uniform(-math.pi, math.pi)
        dist = DISC + 40.0 + 560.0 * rng.random() ** 1.6
        width = rng.uniform(1.4, 3.2)
        length = width * rng.uniform(2.0, 5.0)
        # At least a pixel across, so a small picture keeps its sparks.
        width, length = max(width, 0.9 * px), max(length, 2.2 * px)
        heat = rng.uniform(0.5, 1.6)
        ex, ey = dist * math.cos(angle), dist * math.sin(angle)
        # Only the patch round the spark.
        reach = length * 3.0
        col = int((ex - x[0, 0]) / px)
        row = int((y[0, 0] - ey) / px)
        rad = int(reach / px) + 2
        r0, r1 = max(row - rad, 0), min(row + rad + 1, size_px)
        c0, c1 = max(col - rad, 0), min(col + rad + 1, size_px)
        if r0 >= r1 or c0 >= c1:
            continue
        px_ = x[r0:r1, c0:c1] - ex
        py_ = y[r0:r1, c0:c1] - ey
        along = px_ * math.cos(angle) + py_ * math.sin(angle)
        across = -px_ * math.sin(angle) + py_ * math.cos(angle)
        glow = np.exp(-((along / length) ** 2 + (across / width) ** 2))
        colour = np.array([1.6, 0.6, 0.1]) if heat < 1.1 else np.array([2.2, 1.2, 0.35])
        light[r0:r1, c0:c1] += glow[..., None] * colour * heat
    return light


PARTS = ("disc", "flames", "rays", "embers")


def sun_parts(x, y, px, seed=0, parts=PARTS):
    """The sun's light behind the emblem, linear and unbounded, by part: the
    disc (plasma and its rim), the flames (both rings of tongues and the
    corona), the rays (god rays and spikes) and the embers. The menu draws each
    part as its own layer and moves it; the pictures add them up."""
    rng = np.random.default_rng(21 + seed)
    r = np.hypot(x, y)
    theta = np.arctan2(y, x)
    out = np.clip(r - DISC, 0.0, None)
    outside = 1.0 - coverage(r - DISC, px)
    light = {}
    if "disc" in parts:
        disc = plasma(x, y, r, px)
        # The chromosphere: a thin white-gold rim with spicules.
        spic = 0.6 + 0.6 * periodic(theta, 70.0, 31 + seed, 3)
        rim = np.exp(-np.abs(r - DISC - 2.0) / 7.0) * spic
        light["disc"] = disc + rim[..., None] * np.array([2.6, 1.6, 0.6])
    if "flames" in parts:
        # Two rings of flames: short dense tongues, then long curling ones.
        fl = flames(
            theta, out, outside, 41 + seed, 22.0, 0.22, 150.0, 120.0,
            (1.7, 0.85, 0.16), (1.1, 0.25, 0.02), 0.06,
        )
        fl += flames(
            theta, out, outside, 11 + seed, 8.5, 0.24, 360.0, 330.0,
            (1.5, 0.62, 0.1), (0.85, 0.13, 0.01), 0.13,
        )
        # The corona: close and bright, then wide and dim.
        fl += (np.exp(-out / 45.0) * outside)[..., None] * np.array([1.1, 0.42, 0.06])
        fl += (np.exp(-out / 280.0) * outside)[..., None] * np.array([0.45, 0.12, 0.01])
        light["flames"] = fl
    if "rays" in parts:
        # God rays: soft uneven beams, then the pointed spikes. The rays keep
        # their own seed, so the flames' two menu layers share one set.
        beams = smoothstep(0.45, 0.85, periodic(theta, 30.0, 51, 3)) * 0.5
        ray = (beams * np.exp(-out / 300.0) * smoothstep(0.0, 80.0, out) * outside)[
            ..., None
        ] * np.array([0.9, 0.38, 0.06])
        light["rays"] = ray + spikes(theta, r, out, outside, np.random.default_rng(21))
    if "embers" in parts:
        light["embers"] = embers(x, y, px, rng)
    return light


def sun(x, y, px, seed=0):
    """The whole sun's light: every part added."""
    return sum(sun_parts(x, y, px, seed).values())


def small_sun(x, y, px, radius):
    """The sun of the small icons: a hot disc and a soft glow, which still read
    at 16 pixels where flames and rays would be noise."""
    r = np.hypot(x, y)
    disc = coverage(r - radius, px)
    core = ramp(
        np.clip(r / radius, 0.0, 1.0),
        [(0.0, (2.2, 1.2, 0.3)), (0.6, (1.6, 0.6, 0.06)), (1.0, (1.2, 0.32, 0.02))],
    )
    light = core * disc[..., None]
    out = np.clip(r - radius, 0.0, None)
    light += (np.exp(-out / 70.0) * (1.0 - disc))[..., None] * np.array([1.4, 0.5, 0.06])
    return light


def bloom(light, px):
    """The glow round everything bright, in four widths."""
    bright = np.clip(light - 1.0, 0.0, None)
    glow = np.zeros_like(light)
    for sigma, weight in ((6.0, 0.3), (22.0, 0.22), (70.0, 0.14), (190.0, 0.1)):
        glow += blur(bright, sigma, px) * weight
    return glow


# ---------------------------------------------------------------------------
# Steel
# ---------------------------------------------------------------------------
def metal(g, d, x, y, px, env=None):
    """Polished gunmetal over the field `d`: a bevel inside each edge, lit from
    the upper left, lighter at the top, with a fine brushed grain. `env` (linear
    light round the piece, or None) warms its reflections and lights its edges
    from behind, as the sun does."""
    depth = -d
    if g.flat:
        # A plain light steel with a soft fall: bevels and glints would be a
        # pixel of white and a pixel of black at these sizes.
        sweep = np.clip(0.5 + y / 1400.0, 0.0, 1.0)
        top, foot = np.array([0.84, 0.84, 0.87]), np.array([0.44, 0.44, 0.48])
        return foot + (top - foot) * sweep[..., None]
    h = smoothstep(0.0, g.bevel, depth)
    gy, gx = np.gradient(h)  # per pixel; rows run downwards
    k = g.bevel / px * 0.9
    n = np.stack([-gx * k, gy * k, np.ones_like(h)], axis=-1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    key = np.array([-0.45, 0.62, 0.64])
    key /= np.linalg.norm(key)
    half = key + np.array([0.0, 0.0, 1.0])
    half /= np.linalg.norm(half)
    diffuse = np.clip(n @ key, 0.0, 1.0)
    spec = np.clip(n @ half, 0.0, 1.0)
    sweep = 0.5 + 0.5 * np.tanh((y + 140.0) / 360.0)
    grain = fbm(x / 1.5, y / 26.0, 2, 61) - 0.5
    if env is None:
        dark, light_ = np.array([0.19, 0.195, 0.21]), np.array([0.66, 0.67, 0.70])
    else:
        dark, light_ = np.array([0.07, 0.068, 0.07]), np.array([0.46, 0.45, 0.47])
        dark, light_ = dark + g.lift * 0.6, light_ + g.lift
    base = dark + (light_ - dark) * sweep[..., None]
    shade = base * (0.5 + 0.65 * diffuse[..., None]) * (1.0 + 0.12 * grain[..., None])
    shade += (spec**40 * 0.6 + spec**200 * 0.6)[..., None]
    # The polished rim just inside every edge.
    rim = np.exp(-(((depth - 2.2) / 1.3) ** 2))
    if env is None:
        shade += (rim * (0.35 + 0.65 * diffuse))[..., None] * np.array([0.55, 0.56, 0.6])
        return np.clip(shade, 0.0, 1.0)
    # The sun's light: the faces reflect its warmth, lower and outer parts the
    # most; the edges facing it catch it from behind.
    strength = np.max(env, axis=-1)
    warm = (strength / (1.0 + strength))[..., None] * np.array([1.0, 0.42, 0.09])
    outward = np.clip(
        (n[..., 0] * x + n[..., 1] * y) / np.maximum(np.hypot(x, y), 1.0), 0.0, 1.0
    )
    edge = np.exp(-np.clip(depth, 0.0, None) / 4.0)
    shade += warm * (0.04 + 0.10 * (1.0 - sweep))[..., None]
    shade += warm * (edge * (0.25 + 1.0 * outward))[..., None]
    shade += (rim * (0.25 + 0.75 * diffuse))[..., None] * (
        np.array([0.42, 0.42, 0.44]) + warm * 0.45
    )
    return np.clip(shade, 0.0, 1.0)


# ---------------------------------------------------------------------------
# Pictures
# ---------------------------------------------------------------------------
def render(size, with_sun, g=FULL, span=DESIGN, seed=0, steel=True):
    """The emblem at `size`: display RGB and alpha arrays (alpha from the light
    for the sun, so it can stand on any background)."""
    x, y, px = grid(size, span)
    d_body = body(g, x, y)
    d_rings = rings(g, x, y)
    if with_sun:
        if g.flat:
            light = small_sun(x, y, px, g.disc)
        else:
            light = sun(x, y, px, seed)
        light += bloom(light, px)
        env = blur(light, 40.0, px)
        rgb = aces(light)
        alpha = np.clip(np.max(rgb, axis=-1) * 1.4, 0.0, 1.0)
    else:
        env = None
        rgb = np.zeros((size, size, 3))
        alpha = np.zeros((size, size))
    if not steel:
        return rgb, alpha
    outline = np.array([0.015, 0.013, 0.013])
    # The body's shadow on the rings behind it.
    shadow = np.exp(-np.clip(d_body, 0.0, None) / 14.0) * 0.55
    for d, shaded in ((d_rings, True), (d_body, False)):
        line = coverage(d - g.outline, px)
        rgb = rgb * (1 - line[..., None]) + outline * line[..., None]
        alpha = np.maximum(alpha, line)
        face = coverage(d, px)
        steel_rgb = metal(g, d, x, y, px, env)
        if shaded:
            steel_rgb = steel_rgb * (1.0 - shadow[..., None])
        rgb = rgb * (1 - face[..., None]) + steel_rgb * face[..., None]
    if with_sun:
        # The glow wraps over the steel's edges a little, as light does in a lens.
        steel_cover = coverage(np.minimum(d_body, d_rings) - g.outline, px)
        wrap = aces(blur(np.clip(light - 1.5, 0.0, None), 10.0, px)) * 0.35
        wrap *= (1.0 - 0.85 * steel_cover)[..., None]
        rgb = 1.0 - (1.0 - rgb) * (1.0 - wrap)
    return np.clip(rgb, 0.0, 1.0), alpha


def save(rgb, alpha, path, background=None):
    if background is not None:
        bg = np.array(background) / 255.0
        out = np.clip(rgb + bg * (1 - alpha[..., None]), 0.0, 1.0)
        Image.fromarray((out * 255 + 0.5).astype(np.uint8), "RGB").save(path)
    else:
        # Un-multiply the glow's colour so it does not darken on light pages.
        safe = np.maximum(alpha, 1e-4)[..., None]
        straight = np.clip(rgb / safe, 0.0, 1.0)
        rgba = np.dstack([straight, alpha])
        Image.fromarray((rgba * 255 + 0.5).astype(np.uint8), "RGBA").save(path)


def downsample(rgb, alpha, size):
    """Shrink a premultiplied render to `size` with a Lanczos filter, so fine
    detail (embers, spikes, plasma) averages instead of aliasing."""
    rgba = np.dstack([rgb, alpha])
    channels = []
    for c in range(4):
        im = Image.fromarray(rgba[..., c].astype(np.float32), "F")
        channels.append(np.asarray(im.resize((size, size), Image.BOX)))
    out = np.clip(np.stack(channels, axis=-1), 0.0, 1.0)
    return out[..., :3], out[..., 3]


def icon(size):
    """The application icon at `size` pixels: premultiplied RGB and alpha."""
    if size <= 16:
        return render(size, True, TINY, SMALL_SPAN)
    if size < 32:
        return render(size, True, SMALL, SMALL_SPAN)
    if size == 32:
        return render(size, True, SMALL32, SMALL_SPAN)
    g = MEDIUM if size <= 64 else FULL
    big = min(max(size * 4, 512), 2048)
    rgb, alpha = render(big, True, g, ICON_SPAN)
    x, y, _ = grid(big, ICON_SPAN)
    fade = smoothstep(ICON_SPAN * 0.5, ICON_SPAN * 0.36, np.hypot(x, y))
    keep = np.maximum(fade, coverage(np.minimum(body(g, x, y), rings(g, x, y)) - g.outline, ICON_SPAN / big))
    return downsample(rgb * keep[..., None], alpha * keep, size)


def icon_sheet(path):
    """Every icon size, magnified, on a dark and a light background."""
    sizes = [256, 128, 64, 48, 32, 24, 20, 16]
    tiles = []
    for size in sizes:
        rgb, alpha = icon(size)
        tiles.append((size, rgb, alpha))
    cell = 256
    sheet = Image.new("RGB", ((cell + 16) * len(sizes), (cell + 16) * 2), (40, 41, 44))
    for row, bg in enumerate(((32, 33, 36), (236, 236, 238))):
        for i, (size, rgb, alpha) in enumerate(tiles):
            b = np.array(bg) / 255.0
            out = rgb + b * (1 - alpha[..., None])
            im = Image.fromarray((out * 255 + 0.5).astype(np.uint8))
            im = im.resize((cell, cell), Image.NEAREST)
            sheet.paste(im, (i * (cell + 16), row * (cell + 16)))
    sheet.save(path)


def svg(path, g=FULL):
    """The flat vector master: the same pieces, steel on a dark outline."""
    s = DESIGN / 2.0

    def pt(x_, y_):
        return f"{x_ + s:.2f},{s - y_:.2f}"

    def ring(band):
        a, b = band
        return (
            f"M{s - b:.2f},{s:.2f} a{b},{b} 0 1,0 {2 * b},0 a{b},{b} 0 1,0 {-2 * b},0 Z "
            f"M{s - a:.2f},{s:.2f} a{a},{a} 0 1,0 {2 * a},0 a{a},{a} 0 1,0 {-2 * a},0 Z"
        )

    def pillar_path(side):
        pts = [
            (g.gap * side, g.tip),
            (g.out * side, g.tip - g.pillar),
            (g.out * side, -(g.tip - g.pillar)),
            (g.gap * side, -g.tip),
        ]
        return "M" + " L".join(pt(*p) for p in pts) + " Z"

    def arm_path(angle, toward):
        ux, uy = math.cos(angle), math.sin(angle)
        nx, ny = toward
        t0 = (g.gap + 35.0) / abs(ux)
        t1 = g.reach
        pts = [
            (ux * t0, uy * t0),
            (ux * t1, uy * t1),
            (ux * t1 + nx * g.arm, uy * t1 + ny * g.arm),
            (ux * t0 + nx * g.arm, uy * t0 + ny * g.arm),
        ]
        return "M" + " L".join(pt(*p) for p in pts) + " Z"

    body_paths = [
        ring(g.inner),
        pillar_path(1),
        pillar_path(-1),
        arm_path(math.pi / 4, (1 / SQRT2, -1 / SQRT2)),
        arm_path(-math.pi / 4, (1 / SQRT2, 1 / SQRT2)),
        arm_path(-3 * math.pi / 4, (-1 / SQRT2, 1 / SQRT2)),
    ]
    ring_paths = [ring(g.outer)]
    if g.outer_thin[1] > g.outer_thin[0]:
        ring_paths.append(ring(g.outer_thin))

    def group(paths):
        strokes = "\n".join(
            f'    <path d="{p}" fill-rule="evenodd" fill="none" stroke="#050506" '
            f'stroke-width="{2 * g.outline:.0f}" stroke-linejoin="miter"/>'
            for p in paths
        )
        fills = "\n".join(
            f'    <path d="{p}" fill-rule="evenodd" fill="url(#steel)"/>' for p in paths
        )
        return strokes + "\n" + fills

    text = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {DESIGN:.0f} {DESIGN:.0f}">
  <!-- SJK's emblem, written by scripts/sjk_emblem.py from its geometry. -->
  <defs>
    <linearGradient id="steel" x1="0" y1="{s - 480:.0f}" x2="0" y2="{s + 640:.0f}" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#b4b6bc"/>
      <stop offset="0.55" stop-color="#7b7d84"/>
      <stop offset="1" stop-color="#3c3d42"/>
    </linearGradient>
  </defs>
  <g>
{group(ring_paths)}
  </g>
  <g>
{group(body_paths)}
  </g>
</svg>
"""
    Path(path).write_text(text, encoding="ascii")


# ---------------------------------------------------------------------------
# Every shipped file
# ---------------------------------------------------------------------------
# The menu's layers: (file, edge in pixels, design units it spans, sun part).
# The client draws each at its span relative to the steel's
# (`crates/sjk-viewer/src/menu/emblem.rs`, which keeps the same numbers).
LAYERS = (
    ("emblem-disc.png", 512, 960.0, "disc"),
    ("emblem-flames-a.png", 768, ICON_SPAN, "flames"),
    ("emblem-flames-b.png", 768, ICON_SPAN, "flames"),
    ("emblem-rays.png", 1024, 3200.0, "rays"),
    ("emblem-embers.png", 512, 2400.0, "embers"),
)
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
SPACE = (6, 10, 20)  # the site's deep-space gradient, bottom
SPACE_2 = (11, 20, 48)  # and top


def to_rgba(rgb, alpha):
    """A PIL image from premultiplied arrays, with straight colour."""
    safe = np.maximum(alpha, 1e-4)[..., None]
    straight = np.clip(rgb / safe, 0.0, 1.0)
    rgba = np.dstack([straight, alpha])
    return Image.fromarray((rgba * 255 + 0.5).astype(np.uint8), "RGBA")


def light_layer(size, span, part, seed):
    """One additive menu layer: the part's light, bloomed and tone-mapped, on
    black (black adds nothing), opaque."""
    x, y, px = grid(size, span)
    light = sun_parts(x, y, px, seed, (part,))[part]
    light = light + bloom(light, px)
    # Black at the edge of the disc it is drawn in, as the client turns it and
    # the corners of its turned square repeat the edge.
    light *= smoothstep(span * 0.5, span * 0.44, np.hypot(x, y))[..., None]
    rgb = aces(light)
    return Image.fromarray((rgb * 255 + 0.5).astype(np.uint8), "RGB").convert("RGBA")


def steel_layer(size):
    """The menu's steel: the emblem alone, warmed by the sun it stands before."""
    x, y, px = grid(size, ICON_SPAN)
    light = sun(x, y, px)
    light += bloom(light, px)
    env = blur(light, 40.0, px)
    rgb = np.zeros((size, size, 3))
    alpha = np.zeros((size, size))
    d_body = body(FULL, x, y)
    d_rings = rings(FULL, x, y)
    shadow = np.exp(-np.clip(d_body, 0.0, None) / 14.0) * 0.55
    outline = np.array([0.015, 0.013, 0.013])
    for d, shaded in ((d_rings, True), (d_body, False)):
        line = coverage(d - FULL.outline, px)
        rgb = rgb * (1 - line[..., None]) + outline * line[..., None]
        alpha = alpha * (1 - line) + line
        face = coverage(d, px)
        steel_rgb = metal(FULL, d, x, y, px, env)
        if shaded:
            steel_rgb = steel_rgb * (1.0 - shadow[..., None])
        rgb = rgb * (1 - face[..., None]) + steel_rgb * face[..., None]
    return to_rgba(rgb, alpha)


def backdrop(width, height):
    yy, xx = np.mgrid[0:height, 0:width].astype(np.float32)
    t = (yy / max(height - 1, 1))[..., None]
    base = np.array(SPACE_2, np.float32) * (1.0 - t) + np.array(SPACE, np.float32) * t
    return base / 255.0, xx, yy


def over(base, rgb, alpha, left, top):
    """Premultiplied `rgb`/`alpha` over the float RGB `base` at (left, top)."""
    h, w = alpha.shape
    region = base[top : top + h, left : left + w]
    base[top : top + h, left : left + w] = rgb + region * (1 - alpha[..., None])


def touch_icon(size=180):
    base, _, _ = backdrop(size, size)
    rgb, alpha = icon(size)
    over(base, rgb, alpha, 0, 0)
    return Image.fromarray((np.clip(base, 0, 1) * 255 + 0.5).astype(np.uint8), "RGB")


def social_preview(font_dir):
    """1200x630 Open Graph card: the emblem left, the name and line right."""
    from PIL import ImageDraw, ImageFont

    width, height = 1200, 630
    base, _, _ = backdrop(width, height)
    logo = 600
    rgb, alpha = icon(logo)
    over(base, rgb, alpha, 30, (height - logo) // 2)
    image = Image.fromarray((np.clip(base, 0, 1) * 255 + 0.5).astype(np.uint8), "RGB")
    draw = ImageDraw.Draw(image)
    semibold = ImageFont.truetype(str(font_dir / "Inter-SemiBold.ttf"), 112)
    regular = ImageFont.truetype(str(font_dir / "Inter-Regular.ttf"), 34)
    small = ImageFont.truetype(str(font_dir / "Inter-SemiBold.ttf"), 24)
    left = 64 + 520 + 48
    draw.text((left, 186), "STAR WARS JEDI KNIGHT: JEDI ACADEMY", font=small, fill=(168, 207, 255))
    draw.text((left - 4, 222), "Sol JK", font=semibold, fill=(255, 217, 122))
    lines = ("Native Rust client and server", "for Jedi Academy multiplayer,", "classic at heart.")
    for row, line in enumerate(lines):
        draw.text((left, 372 + row * 46), line, font=regular, fill=(226, 233, 246))
    return image


def save_png(image, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)
    print(f"{path}: {image.size[0]}x{image.size[1]}, {path.stat().st_size} bytes")


def save_ico(frames, path):
    frames[-1].save(path, format="ICO", sizes=[f.size for f in frames], append_images=frames[:-1])
    sizes = ", ".join(str(f.size[0]) for f in frames)
    print(f"{path}: {sizes}, {path.stat().st_size} bytes")


def build(root):
    """Write every shipped image (assets/branding/README.md lists them)."""
    branding = root / "assets" / "branding"
    site = root / "site" / "assets"
    fonts = root / "crates" / "sjk-viewer" / "assets" / "fonts"
    svg(branding / "sjk-emblem.svg")
    print(f"{branding / 'sjk-emblem.svg'}")
    icons = {size: to_rgba(*icon(size)) for size in sorted(set(ICO_SIZES) | {180, 512, 1024})}
    save_png(icons[1024], branding / "sjk-logo.png")
    save_png(icons[512], branding / "sjk-logo-512.png")
    save_png(icons[32], branding / "icon-32.png")
    save_png(icons[64], branding / "icon-64.png")
    save_png(to_rgba(*render(128, True, SMALL32, SMALL_SPAN)), branding / "emblem-chat.png")
    save_ico([icons[size] for size in ICO_SIZES], branding / "sjk.ico")
    save_png(steel_layer(1024), branding / "emblem-steel.png")
    for index, (name, size, span, part) in enumerate(LAYERS):
        seed = 7 if name.endswith("-b.png") else 0
        save_png(light_layer(size, span, part, seed), branding / name)
    save_png(icons[512], site / "sjk-logo-512.png")
    save_png(icons[32], site / "favicon-32.png")
    save_ico([icons[size] for size in (16, 32, 48)], site / "favicon.ico")
    save_png(touch_icon(), site / "apple-touch-icon.png")
    preview = social_preview(fonts)
    path = site / "social-preview.jpg"
    preview.save(path, quality=90, optimize=True, progressive=True)
    print(f"{path}: 1200x630, {path.stat().st_size} bytes")


def main(argv):
    if len(argv) in (1, 2) and argv[0] == "build":
        build(Path(argv[1] if len(argv) == 2 else "."))
        return 0
    if len(argv) >= 2 and argv[0] == "preview":
        out = Path(argv[1])
        out.mkdir(parents=True, exist_ok=True)
        size = int(argv[2]) if len(argv) > 2 else 1024
        rgb, alpha = render(size, False)
        save(rgb, alpha, out / "emblem-steel.png", background=(25, 25, 27))
        rgb, alpha = render(size, True)
        save(rgb, alpha, out / "emblem-sun.png", background=(0, 0, 0))
        icon_sheet(out / "icons.png")
        return 0
    if len(argv) == 2 and argv[0] == "svg":
        svg(argv[1])
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
