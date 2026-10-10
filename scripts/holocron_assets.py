"""Make the Illuminate holocron's game files from Sol's two generated pictures,
and the four loot-box tiers' art from the shipped face.

Illuminate is SJK's own Force-wheel entry: a holocron that floats by the player's
shoulder and lights the way. Sol generated its pictures (07/10/2026): a holocron
icon in the style of Jedi Academy's Force icons (1024x1024, on a flat grey ground)
and one face of the holocron (2048x2048). This script writes, into the output
folder:

- `force_illuminate.png`: the icon cut out of its ground with its amber glow, cube
  at the stock icons' size, 128x128 like `gfx/mp/f_icon_*`;
- `holocron.jpg`: the face, 512x512;
- `holocron_glow.jpg`: the face's emblem alone on black, for the additive stage that
  keeps it lit in the dark;
- `holocron.md3`: a cube of HOLOCRON_EDGE units with the face on all six sides.

    python scripts/holocron_assets.py icon.jpg face.jpg crates/sjk-viewer/assets/holocron

That also remakes the tiers (below) from the new face. The tiers alone, from the
shipped `holocron.jpg` and `holocron_glow.jpg` in the folder (no originals needed):

    python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron

The tiers (uncommon green, rare blue, legendary purple, mythical gold) are variants of
the one holocron: the plate stays steel, the emblem, the ring's runes and a faint plate
tint take the tier's colour. Per tier it writes `holocron_<tier>.jpg` (the face),
`holocron_<tier>_glow.jpg`, `holocron_<tier>.png` (the icon, 256x256) and
`holocron_<tier>_small.png` (64x64), the one shared sheen stripe of the mythical
tier, and `holocron_tiers.shader`. The whole table (colours, glow ladder, shader
waves, point lights) is TIERS below; the output is deterministic.

Needs Pillow, numpy and scipy (pip install pillow numpy scipy).
"""

import math
import struct
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
from scipy.spatial import ConvexHull

ICON_SIZE = 128
# The icon's flat ground, and the share of the frame the cube's box takes (the
# stock icons' cubes span about two thirds of theirs).
ICON_GROUND = 39.0
ICON_CUBE_SHARE = 0.68
ICON_GLOW = np.array([255.0, 168.0, 72.0])
FACE_SIZE = 512
HOLOCRON_EDGE = 6.0
SHADER = "models/sjk/holocron"
JPEG_QUALITY = 88


def icon(source: Path, output: Path) -> None:
    a = np.asarray(Image.open(source).convert("RGB")).astype(float)
    r, b = a[..., 0], a[..., 2]
    # The cube is grey-blue and off the ground; its halo is orange (red over blue).
    cube = (np.abs(a - ICON_GROUND).max(-1) > 14) & (b >= r - 6)
    cube = ndimage.binary_opening(cube, iterations=2)
    labels, count = ndimage.label(cube)
    sizes = ndimage.sum(cube, labels, range(1, count + 1))
    cube = labels == 1 + int(np.argmax(sizes))
    ys, xs = np.nonzero(cube)
    points = np.stack([xs, ys], 1)
    hull = ConvexHull(points)
    mask = Image.new("L", (a.shape[1], a.shape[0]), 0)
    ImageDraw.Draw(mask).polygon([tuple(map(float, points[i])) for i in hull.vertices], fill=255)
    cube_alpha = ndimage.gaussian_filter(np.asarray(mask).astype(float) / 255.0, 0.8)
    # The halo's coverage from its red channel over the ground, in one amber.
    halo_alpha = np.clip((r - ICON_GROUND) / (ICON_GLOW[0] - ICON_GROUND), 0, 1)
    alpha = np.maximum(cube_alpha, halo_alpha)
    rgb = np.where(cube_alpha[..., None] > 0.5, a, ICON_GLOW)
    picture = Image.fromarray(np.dstack([rgb, alpha * 255]).clip(0, 255).astype(np.uint8), "RGBA")
    centre_x, centre_y = (xs.min() + xs.max()) / 2, (ys.min() + ys.max()) / 2
    side = max(xs.max() - xs.min(), ys.max() - ys.min()) / ICON_CUBE_SHARE
    box = tuple(
        round(v)
        for v in (centre_x - side / 2, centre_y - side / 2, centre_x + side / 2, centre_y + side / 2)
    )
    picture = picture.crop(box).convert("RGBa").resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS)
    picture.convert("RGBA").save(output / "force_illuminate.png", optimize=True)


def face(source: Path, output: Path) -> None:
    picture = Image.open(source).convert("RGB").resize((FACE_SIZE, FACE_SIZE), Image.LANCZOS)
    a = np.asarray(picture).astype(float)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    ys, xs = np.mgrid[0:FACE_SIZE, 0:FACE_SIZE]
    from_centre = np.hypot(xs - FACE_SIZE / 2, ys - FACE_SIZE / 2) / FACE_SIZE
    # The emblem and its baked halo are warm; the rays' cores are near white, so
    # inside the emblem's circle brightness counts too. Rust specks on the plates
    # are warm as well, hence the circles.
    warm = np.clip((r - b - 12) / 90.0, 0, 1) * (from_centre < 0.37)
    luminance = 0.3 * r + 0.59 * g + 0.11 * b
    bright = np.clip((luminance - 150) / 70.0, 0, 1) * (from_centre < 0.246)
    emblem = ndimage.gaussian_filter(np.maximum(warm, bright), 0.7)
    picture.save(output / "holocron.jpg", quality=92)
    glow = (a * emblem[..., None]).clip(0, 255).astype(np.uint8)
    Image.fromarray(glow).save(output / "holocron_glow.jpg", quality=92)


def encode_normal(n) -> int:
    # The inverse of sjk-model's `decode_normal` (stock's MD3 normal): the high
    # byte is the angle around Z, the low byte the angle from +Z.
    around = round(math.atan2(n[1], n[0]) * 255 / (2 * math.pi)) & 0xFF
    from_up = round(math.acos(max(-1.0, min(1.0, n[2]))) * 255 / (2 * math.pi)) & 0xFF
    return (around << 8) | from_up


def cube_md3(output: Path) -> None:
    h = HOLOCRON_EDGE / 2
    # Each face: outward normal and the picture's up; right = up x normal, so the
    # picture reads upright seen from outside.
    faces = [
        ((1, 0, 0), (0, 0, 1)),
        ((-1, 0, 0), (0, 0, 1)),
        ((0, 1, 0), (0, 0, 1)),
        ((0, -1, 0), (0, 0, 1)),
        ((0, 0, 1), (1, 0, 0)),
        ((0, 0, -1), (1, 0, 0)),
    ]
    positions, normals, uvs, triangles = [], [], [], []
    for normal, up in faces:
        n, u = np.array(normal, float), np.array(up, float)
        right = np.cross(u, n)
        base = len(positions)
        for s, t in ((0, 0), (1, 0), (1, 1), (0, 1)):
            positions.append(n * h + right * (2 * s - 1) * h + u * (1 - 2 * t) * h)
            normals.append(n)
            uvs.append((s, t))
        # Clockwise seen from outside: stock's front side.
        triangles += [(base, base + 1, base + 2), (base, base + 2, base + 3)]

    def name(text: str, size: int) -> bytes:
        return text.encode("ascii").ljust(size, b"\0")

    shaders = name(SHADER, 64) + struct.pack("<i", 0)
    tris = b"".join(struct.pack("<3i", *t) for t in triangles)
    sts = b"".join(struct.pack("<2f", *uv) for uv in uvs)
    verts = b"".join(
        struct.pack("<3hH", *(round(c * 64) for c in p), encode_normal(n))
        for p, n in zip(positions, normals)
    )
    surface_header = 108
    ofs_shaders = surface_header
    ofs_tris = ofs_shaders + len(shaders)
    ofs_st = ofs_tris + len(tris)
    ofs_verts = ofs_st + len(sts)
    surface_end = ofs_verts + len(verts)
    surface = (
        b"IDP3"
        + name("holocron", 64)
        + struct.pack(
            "<10i", 0, 1, 1, len(positions), len(triangles),
            ofs_tris, ofs_shaders, ofs_st, ofs_verts, surface_end,
        )
        + shaders + tris + sts + verts
    )
    frame = struct.pack("<10f", -h, -h, -h, h, h, h, 0, 0, 0, h * math.sqrt(3)) + name("holocron", 16)
    header_size = 108
    ofs_frames = header_size
    ofs_surfaces = ofs_frames + len(frame)
    end = ofs_surfaces + len(surface)
    header = (
        b"IDP3"
        + struct.pack("<i", 15)
        + name("models/sjk/holocron.md3", 64)
        + struct.pack("<9i", 0, 1, 0, 1, 0, ofs_frames, ofs_surfaces, ofs_surfaces, end)
    )
    (output / "holocron.md3").write_bytes(header + frame + surface)


# ---------------------------------------------------------------------------
# The loot-box tiers.

TIER_ICON_SIZE = 256
TIER_SMALL_SIZE = 64
# Seen from the centre of the face, the ring is an octagon: flat sides at m = |dx| or
# |dy| and corners cut where (|dx| + |dy|) / RING_CUT = m.
RING_CUT = 1.475
# The icon's own octagon is cut a little deeper.
ICON_CUT = 1.5
# The icon's octagon, as half its width in 256 pixels; the rest is the halo's room.
ICON_HALF = 108.0
SHEEN_SIZE = 128
SHEEN_NAME = "holocron_mythical_sheen"


@dataclass(frozen=True)
class Tier:
    """One tier's look. Colours are RGB 0..255, the rest scales.

    `ramp` is the emblem's gradient from its dark edge to its white-hot core; the
    tiers climb in how much they glow (`ring`: the share of the ring's runes and bevels
    lit, `halo`: the emblem's light on the plate, `tint`: the plate's tint, `emblem`:
    the emblem's brightness), so they tell apart by that and by lightness, not by hue
    alone. `light` is the point light's colour in illuminate.rs's COLOR scale
    ([1.5, 1.3, 1.0]). `self_glow` and `breath` are the shader's waves (base,
    amplitude, phase, frequency): the face's own light, and the emblem's.
    """

    id: str
    label: str
    ramp: tuple
    emblem: float
    ring: float
    halo: float
    tint: float
    light: tuple
    self_glow: tuple
    breath: tuple


TIERS = (
    Tier(
        "uncommon", "Uncommon (green)",
        ((6, 46, 18), (52, 178, 76), (150, 240, 160), (226, 255, 228)),
        emblem=0.80, ring=0.12, halo=0.08, tint=0.10,
        light=(0.55, 1.50, 0.60),
        self_glow=(0.35, 0.0, 0.0, 0.0), breath=(0.40, 0.10, 0.0, 0.30),
    ),
    Tier(
        "rare", "Rare (blue)",
        ((4, 24, 88), (36, 120, 255), (140, 196, 255), (222, 240, 255)),
        emblem=0.90, ring=0.40, halo=0.16, tint=0.14,
        light=(0.50, 1.00, 1.75),
        self_glow=(0.40, 0.0, 0.0, 0.0), breath=(0.50, 0.15, 0.0, 0.35),
    ),
    Tier(
        "legendary", "Legendary (purple)",
        ((34, 6, 82), (176, 64, 255), (226, 160, 255), (250, 228, 255)),
        emblem=1.00, ring=0.72, halo=0.26, tint=0.18,
        light=(1.25, 0.45, 1.75),
        self_glow=(0.45, 0.0, 0.0, 0.0), breath=(0.60, 0.20, 0.0, 0.40),
    ),
    Tier(
        "mythical", "Mythical (gold, shiny)",
        ((92, 40, 0), (255, 186, 38), (255, 232, 130), (255, 252, 226)),
        emblem=1.00, ring=1.00, halo=0.38, tint=0.22,
        light=(1.85, 1.30, 0.40),
        self_glow=(0.45, 0.20, 0.25, 0.35), breath=(0.85, 0.35, 0.0, 0.55),
    ),
)


def smoothstep(edge0, edge1, x):
    t = np.clip((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def octagon_metric(n: int, cut: float):
    ys, xs = np.mgrid[0:n, 0:n]
    dx, dy = np.abs(xs - (n - 1) / 2), np.abs(ys - (n - 1) / 2)
    return np.maximum(np.maximum(dx, dy), (dx + dy) / cut)


def face_masks(face_rgb: np.ndarray):
    """The shipped face's emblem (with its halo) and ring (runes and bevels), 0..1."""
    n = face_rgb.shape[0]
    r, g, b = face_rgb[..., 0], face_rgb[..., 1], face_rgb[..., 2]
    ys, xs = np.mgrid[0:n, 0:n]
    from_centre = np.hypot(xs - n / 2, ys - n / 2) / n
    luminance = 0.3 * r + 0.59 * g + 0.11 * b
    # As face() cuts the emblem, but inside the emblem's own reach only: the base's
    # glow picture keeps a few warm specks of the ring, a tier's must not.
    warm = np.clip((r - b - 12) / 90.0, 0, 1)
    bright = np.clip((luminance - 150) / 70.0, 0, 1) * (from_centre < 0.246)
    reach = 1.0 - smoothstep(0.27, 0.31, from_centre)
    emblem = ndimage.gaussian_filter(np.maximum(warm, bright) * reach, 0.7)
    # The ring: the bevels' and runes' highlights inside the octagonal band.
    metric = octagon_metric(n, RING_CUT)
    band = smoothstep(126, 130, metric) * (1.0 - smoothstep(179, 183, metric))
    high_pass = luminance - ndimage.gaussian_filter(luminance, 6)
    ring = np.clip(high_pass / 25.0, 0, 1) * band * (1.0 - emblem)
    return emblem, ring


def ramp(values: np.ndarray, stops) -> np.ndarray:
    """Colour the 0..1 `values` along the four stops of a tier's ramp."""
    at = [0.12, 0.50, 0.80, 0.97]
    return np.stack([np.interp(values, at, [stop[c] for stop in stops]) for c in range(3)], -1)


def tier_layers(face_rgb: np.ndarray, tier: Tier):
    """The tier's face and glow pictures (float RGB 0..255) from the base face."""
    emblem, ring = face_masks(face_rgb)
    luminance = 0.3 * face_rgb[..., 0:1] + 0.59 * face_rgb[..., 1:2] + 0.11 * face_rgb[..., 2:3]
    paint = ramp(luminance[..., 0] / 255.0, tier.ramp) * tier.emblem
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    # The plate: steel with a faint tint of the tier's colour, the emblem's light
    # spilling over it, the ring's bevels and runes lit.
    plate = face_rgb + tier.tint * (luminance * (mid / mid.max()) - face_rgb)
    spill = ndimage.gaussian_filter(emblem, 14)[..., None]
    plate = plate + tier.halo * spill * pale
    ring_light = tier.ring * ring[..., None]
    plate = plate + ring_light * (mid * 0.55 + pale * 0.45)
    face = plate * (1.0 - emblem[..., None]) + paint * emblem[..., None]
    # The glow picture, on black: the emblem and the lit ring (what the additive
    # stage and the glow pass pick up).
    glow = paint * emblem[..., None] + ring_light * (mid * 0.55 + pale * 0.45)
    return face.clip(0, 255), glow.clip(0, 255)


def tier_icon(face_rgb: np.ndarray, glow_rgb: np.ndarray, tier: Tier) -> Image.Image:
    """The 256x256 icon: the face in a chamfered octagon with a bevelled rim and a
    halo in the tier's colour, made for a dark UI."""
    size, oversample = TIER_ICON_SIZE, 4
    big = size * oversample
    half = ICON_HALF * oversample
    # The face, the ring's octagon filling the icon, with its glow added (the icon
    # is lit by it).
    lit = (face_rgb + 0.35 * glow_rgb).clip(0, 255).astype(np.uint8)
    side = round(2 * half)
    shown = np.asarray(Image.fromarray(lit).crop((60, 60, 452, 452)).resize((side, side), Image.LANCZOS))
    canvas = np.zeros((big, big, 3))
    offset = (big - side) // 2
    canvas[offset : offset + side, offset : offset + side] = shown
    metric = octagon_metric(big, ICON_CUT)
    inside = (metric <= half).astype(float)
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    ys, xs = np.mgrid[0:big, 0:big]
    # The rim: a bevel, lit from the top left, and a dark line inside it.
    rim_width = 7.0 * oversample
    rim = inside * smoothstep(half - rim_width - 1, half - rim_width + 1, metric)
    slope = ((xs + ys) / (2.0 * big))[..., None]
    bevel = mid * (1.35 - 0.9 * slope) + 40.0 * (1.0 - slope)
    line = inside * smoothstep(half - rim_width - 3 * oversample, half - rim_width, metric) * (1.0 - rim)
    picture = canvas * (1.0 - 0.55 * line[..., None])
    picture = picture * (1.0 - rim[..., None]) + bevel * rim[..., None]
    # A gloss across the top left.
    gloss = np.clip(1.0 - (xs + ys) / (0.9 * big), 0, 1)[..., None] ** 2
    picture = picture + 22.0 * gloss * inside[..., None]
    solid = inside
    if tier.id == "mythical":
        # A second, thin outer line in the pale gold.
        outer = smoothstep(half + 2 * oversample, half + 3 * oversample, metric) * (
            1.0 - smoothstep(half + 4 * oversample, half + 5 * oversample, metric)
        )
        picture = picture * (1.0 - outer[..., None]) + pale * outer[..., None]
        solid = np.maximum(inside, outer)
    # The halo outside, in the tier's light; wider the more the tier glows.
    spread = ndimage.gaussian_filter(solid, 9.0 * oversample)
    halo = np.clip(spread * 2.0 * (0.45 + 0.55 * tier.ring), 0, 0.85) * (1.0 - solid)
    colour = picture * solid[..., None] + mid * halo[..., None]
    alpha = np.clip(solid + halo, 0, 1)
    if tier.id == "mythical":
        star = sparkle(big, (0.80 * big, 0.17 * big), 0.13 * big)
        colour = colour * (1.0 - star[..., None]) + np.array((255.0, 250.0, 225.0)) * star[..., None]
        alpha = np.maximum(alpha, star)
    # Average in premultiplied terms down to the icon's size.
    flat = np.dstack([colour * alpha[..., None], alpha * 255.0])
    reduced = flat.reshape(size, oversample, size, oversample, 4).mean((1, 3))
    a = reduced[..., 3:4] / 255.0
    rgb = np.where(a > 1e-4, reduced[..., :3] / np.maximum(a, 1e-4), 0.0)
    return Image.fromarray(np.dstack([rgb, reduced[..., 3]]).clip(0, 255).round().astype(np.uint8), "RGBA")


def sparkle(n: int, centre, reach: float) -> np.ndarray:
    """A four-pointed glint, 0..1."""
    ys, xs = np.mgrid[0:n, 0:n]
    dx, dy = xs - centre[0], ys - centre[1]

    def arm(along, across):
        return np.exp(-3.0 * (along / reach) ** 2) * np.exp(-((across / (reach * 0.07)) ** 2))

    core = np.exp(-((np.hypot(dx, dy) / (reach * 0.22)) ** 2))
    return np.clip(arm(dx, dy) + arm(dy, dx) + core, 0, 1)


def small_icon(picture: Image.Image) -> Image.Image:
    """The 64x64 icon: the large one resized in premultiplied colour."""
    small = picture.convert("RGBa").resize((TIER_SMALL_SIZE, TIER_SMALL_SIZE), Image.LANCZOS)
    return small.convert("RGBA")


def sheen(output: Path) -> None:
    """A soft diagonal stripe that tiles along (s + t): the mythical shader scrolls it
    over the face, one stripe crossing at a time."""
    n = SHEEN_SIZE
    ys, xs = np.mgrid[0:n, 0:n]
    phase = ((xs + ys + 1.0) / n) % 1.0
    distance = np.abs(phase - 0.5)
    band = np.exp(-((distance / 0.07) ** 2)) + 0.18 * np.exp(-((distance / 0.22) ** 2))
    picture = np.clip(band[..., None] * np.array((255.0, 232.0, 160.0)), 0, 255)
    Image.fromarray(picture.astype(np.uint8)).save(output / f"{SHEEN_NAME}.jpg", quality=JPEG_QUALITY)


def wave(values) -> str:
    return "rgbGen wave sin " + " ".join(f"{value:g}" for value in values)


def tier_shader() -> str:
    lines = [
        "// The holocron's loot-box tiers (SJK; see README.md): holocron.shader once per",
        "// tier. Lit steel that also glows faintly of itself, and an emblem and ring kept",
        "// bright and in the dynamic glow, breathing; the tiers climb in how bright.",
        "// Mythical also breathes the whole face and has a sheen stripe sliding across.",
        "// Made by scripts/holocron_assets.py from its TIERS table: edit that, not this.",
    ]
    for tier in TIERS:
        name = f"models/sjk/holocron_{tier.id}"
        lines += [
            name,
            "{",
            "\tq3map_nolightmap",
            "\t{",
            f"\t\tmap {name}",
            "\t\trgbGen lightingDiffuse",
            "\t}",
            "\t{",
            f"\t\tmap {name}",
            "\t\tblendFunc GL_ONE GL_ONE",
            f"\t\t{wave(tier.self_glow)}",
            "\t}",
            "\t{",
            f"\t\tmap {name}_glow",
            "\t\tblendFunc GL_ONE GL_ONE",
            f"\t\t{wave(tier.breath)}",
            "\t\tglow",
            "\t}",
        ]
        if tier.id == "mythical":
            lines += [
                "\t{",
                f"\t\tmap models/sjk/{SHEEN_NAME}",
                "\t\tblendFunc GL_ONE GL_ONE",
                "\t\ttcMod scale 0.5 0.5",
                "\t\ttcMod scroll 0.06 0.06",
                "\t\trgbGen wave sin 0.5 0.15 0 0.25",
                "\t}",
            ]
        lines.append("}")
    return "\n".join(lines) + "\n"


def tiers(output: Path) -> None:
    """The tiers' pictures and shaders, from the folder's holocron.jpg."""
    base = np.asarray(Image.open(output / "holocron.jpg").convert("RGB")).astype(float)
    for tier in TIERS:
        face_picture, glow_picture = tier_layers(base, tier)
        Image.fromarray(face_picture.round().astype(np.uint8)).save(
            output / f"holocron_{tier.id}.jpg", quality=JPEG_QUALITY
        )
        Image.fromarray(glow_picture.round().astype(np.uint8)).save(
            output / f"holocron_{tier.id}_glow.jpg", quality=JPEG_QUALITY
        )
        picture = tier_icon(face_picture, glow_picture, tier)
        picture.save(output / f"holocron_{tier.id}.png", optimize=True)
        small_icon(picture).save(output / f"holocron_{tier.id}_small.png", optimize=True)
    sheen(output)
    (output / "holocron_tiers.shader").write_text(tier_shader(), encoding="ascii", newline="\n")


def main() -> None:
    if len(sys.argv) == 3 and sys.argv[1] == "tiers":
        tiers(Path(sys.argv[2]))
        return
    icon_source, face_source, output = (Path(arg) for arg in sys.argv[1:4])
    output.mkdir(parents=True, exist_ok=True)
    icon(icon_source, output)
    face(face_source, output)
    cube_md3(output)
    tiers(output)


if __name__ == "__main__":
    main()
