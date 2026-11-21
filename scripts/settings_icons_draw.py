"""Draw the settings menu's icons: grey metal discs, each with one embossed symbol.

Every icon comes out of this script, so the whole set has one look and a new icon is
one line in ICONS: the disc (a bright bevelled rim, a dark channel, a thin inner ring,
a darker face engraved with faint spokes, lit from the top left) is computed, and the
symbol is a glyph of the Phosphor Icons "fill" font (MIT, https://phosphoricons.com),
laid on it as a raised white plate with a soft shadow. A few symbols Phosphor has no
glyph for (the crossed lightsabers) are drawn here. ICONS are whole atlas cells
(128x128); SMALL, the sub-headings' own, are 64x64.

    python scripts/settings_icons_draw.py <phosphor> crates/sjk-viewer/assets/settings

<phosphor> is the unpacked npm package @phosphor-icons/web (2.1.1 was used):

    curl -sL https://registry.npmjs.org/@phosphor-icons/web/-/web-2.1.1.tgz | tar -xz
    python scripts/settings_icons_draw.py package crates/sjk-viewer/assets/settings

Add `--sheet board.png` for a labelled contact sheet of the set. The font is not kept
in the repository; only the pictures drawn from it are. Needs Pillow and numpy.
"""

import json
import math
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

# The UI atlas cell, and how many times larger the icon is drawn before it is scaled
# down to it.
CELL = 128
SS = 4
N = CELL * SS


def sabers(draw):
    """Two crossed lightsabers, hilts down: the Gameplay symbol."""
    for sign in (-1, 1):
        layer = Image.new("L", (N, N), 0)
        pen = ImageDraw.Draw(layer)
        cx = N / 2
        # Blade, emitter collar, grip with two bands, pommel.
        pen.rounded_rectangle([cx - 0.034 * N, 0.14 * N, cx + 0.034 * N, 0.58 * N], 0.034 * N, fill=255)
        pen.rectangle([cx - 0.066 * N, 0.585 * N, cx + 0.066 * N, 0.63 * N], fill=255)
        pen.rounded_rectangle([cx - 0.048 * N, 0.645 * N, cx + 0.048 * N, 0.82 * N], 0.014 * N, fill=255)
        for y in (0.69, 0.75):
            pen.rectangle([cx - 0.048 * N, y * N, cx + 0.048 * N, (y + 0.02) * N], fill=0)
        pen.rounded_rectangle([cx - 0.060 * N, 0.83 * N, cx + 0.060 * N, 0.875 * N], 0.012 * N, fill=255)
        draw.paste(255, mask=layer.rotate(sign * 40, resample=Image.BICUBIC, center=(N / 2, N * 0.50)))


# Each icon: its file name, then its symbol: a Phosphor glyph's name, or
# (glyph, size) where the glyph reads better larger or smaller than the usual 0.70
# of the disc, or a function drawing it. Grouped as the menus use them.
ICONS = {
    # The Settings screen and its categories.
    "settings": "gear",
    "first_setup": "compass-rose",
    "video": "frame-corners",
    "graphics": "desktop",
    "sound": "speaker-high",
    "mouse_joystick": "mouse",
    "key_bindings": "keyboard",
    "gameplay": sabers,
    "game_options": "cube",
    "interface": "browsers",
    "hud": "scan",
    "text": "chat-circle-dots",
    "options": "puzzle-piece",
    "network": "broadcast",
    # The key bindings' groups.
    "movement": "person-simple-run",
    "interaction": "hand-tap",
    "weapons": "crosshair-simple",
    "force_powers": "hand-waving",
    "other_controls": "wrench",
    # Graphics' parts.
    "image": "aperture",
    "lighting": "lightbulb",
    "shadows": "circle-half",
    "weather": "cloud-rain",
    # The HUD's parts and the other screens.
    "scoreboard": "ranking",
    "camera": "video-camera",
    "crosshair": "crosshair",
    "nameplates": "identification-badge",
    "identity": "fingerprint",
    "changelog": "scroll",
    "credits": "medal",
    "update": "arrows-clockwise",
    "import": "tray-arrow-down",
}

# The pictures only the Settings sub-headings wear (`settings/section_icons.rs`). They
# are drawn small there, so each is kept at half the cell, 64x64, in `small/`, and the
# client packs four to an atlas cell: the atlas is full (8192 texels tall, the most a
# device is asked for), and 32 more whole cells do not fit.
SMALL = {
    "frame_rate": "gauge",
    "view": "eye",
    "quality": "faders",
    "textures": "wall",
    "effects": "sparkle",
    "sunlight": "sun",
    "time": "clock",
    "game_sounds": "waveform",
    "headphones": "headphones",
    "emotes": "mask-happy",
    "toys": "robot",
    "notifications": "bell",
    "players": "users",
    "duels": "sword",
    "maps": "map-trifold",
    "prediction": "activity",
    "away": "moon-stars",
    "theme": "palette",
    "console": "terminal-window",
    "cards": "identification-card",
    "playstyle": "heart",
    "readouts": "chart-bar",
    "match": "flag-banner",
    "status_bar": "rows",
    "dnd": "bell-slash",
    "chat_look": "text-aa",
    "chat_tabs": "tabs",
    "sjk_chat": "chats",
    "packs": "package",
    "server_mods": "plug",
    "jof": "cloud-arrow-down",
    "online": "globe-hemisphere-west",
}
SMALL_SIZE = CELL // 2


def glyphs(package):
    """Each Phosphor fill glyph's character, by its name without the "-fill"."""
    data = json.load(open(Path(package) / "src/fill/selection.json", encoding="utf8"))
    found = {}
    for icon in data["icons"]:
        for name in icon["properties"]["name"].split(", "):
            found[name.removesuffix("-fill")] = chr(icon["properties"]["code"])
    return found


def disc():
    """The grey metal disc every icon stands on."""
    yy, xx = np.mgrid[0:N, 0:N].astype(np.float32)
    c = (N - 1) / 2
    dx, dy = (xx - c) / (N / 2), (yy - c) / (N / 2)
    r = np.hypot(dx, dy)
    angle = np.arctan2(dy, dx)
    lit = (-(dx + dy) / math.sqrt(2)) * 0.5 + 0.5

    def band(inner, outer):
        return np.clip((r - inner) / (outer - inner), 0, 1)

    out_r, rim_in, ring_out, ring_in = 0.985, 0.905, 0.865, 0.838
    # Rim: rounded, its outer slope catching the light, its inner slope the reverse.
    t = band(rim_in, out_r)
    rim = 0.58 + 0.20 * np.sin(t * math.pi) + 0.30 * (lit - 0.5) * 2 * -np.cos(t * math.pi)
    # Inner ring: a thin raised line.
    u = band(ring_in, ring_out)
    ring = 0.46 + 0.14 * np.sin(u * math.pi) + 0.20 * (lit - 0.5) * 2 * -np.cos(u * math.pi)
    # Face: dark, lighter towards the light, with sixteen engraved spokes and a ring.
    face = 0.31 + 0.05 * (lit - 0.5) * 2 - 0.03 * r
    spoke = np.clip(1 - np.abs(np.sin(angle * 8)) / (0.020 / np.maximum(r, 0.15)), 0, 1)
    face = face + 0.085 * spoke * np.clip((r - 0.10) / 0.08, 0, 1)
    face = face + 0.045 * np.clip(1 - np.abs(r - 0.52) / 0.010, 0, 1)
    channel = 0.15 + 0.04 * lit
    value = np.where(
        r > rim_in, rim, np.where(r > ring_out, channel, np.where(r > ring_in, ring, face))
    )
    # A dark outer edge, and the ring's shadow on the face.
    value = value * (1 - 0.6 * np.clip((r - (out_r - 0.018)) / 0.018, 0, 1))
    value = value * (1 - 0.30 * np.clip(1 - (ring_in - r) / 0.05, 0, 1) * (r < ring_in))
    alpha = np.clip((out_r - r) / (1.5 / (N / 2)), 0, 1)
    grey = np.clip(value, 0, 1)
    rgb = np.stack([grey * 0.975, grey * 0.99, grey], axis=-1)
    return Image.fromarray((np.dstack([rgb, alpha]) * 255).astype(np.uint8), "RGBA")


def symbol(font_path, found, spec):
    """The symbol's shape, white on black, centred on the disc."""
    mask = Image.new("L", (N, N), 0)
    if callable(spec):
        spec(mask)
        return mask
    name, size = spec if isinstance(spec, tuple) else (spec, 0.70)
    font = ImageFont.truetype(str(font_path), int(N * size))
    draw = ImageDraw.Draw(mask)
    box = draw.textbbox((0, 0), found[name], font=font)
    draw.text(
        ((N - (box[2] - box[0])) / 2 - box[0], (N - (box[3] - box[1])) / 2 - box[1]),
        found[name],
        font=font,
        fill=255,
    )
    return mask


def emboss(base, mask):
    """`mask` on the disc `base` as a raised white plate, lit from the top left."""
    out = base.copy()
    shadow = ImageChops.offset(mask, int(N * 0.018), int(N * 0.026)).filter(
        ImageFilter.GaussianBlur(N * 0.014)
    )
    out = Image.composite(
        Image.new("RGBA", (N, N), (0, 0, 0, 255)), out, shadow.point(lambda v: int(v * 0.62))
    )
    # White at the top to pale grey at the bottom.
    shade = np.linspace(1.06, 0.66, N, dtype=np.float32)[:, None] * np.ones((1, N), np.float32)
    body = np.clip(np.stack([shade * 0.985, shade * 0.99, shade], -1), 0, 1)
    out = Image.composite(
        Image.fromarray((body * 255).astype(np.uint8), "RGB").convert("RGBA"), out, mask
    )
    # A bright edge towards the light, a darker one away from it.
    soft = mask.filter(ImageFilter.GaussianBlur(N * 0.006))
    step = int(N * 0.010)
    bright = ImageChops.multiply(ImageChops.subtract(soft, ImageChops.offset(soft, step, step)), mask)
    dark = ImageChops.multiply(ImageChops.subtract(soft, ImageChops.offset(soft, -step, -step)), mask)
    out = Image.composite(
        Image.new("RGBA", (N, N), (255, 255, 255, 255)), out, bright.point(lambda v: int(v * 0.9))
    )
    out = Image.composite(
        Image.new("RGBA", (N, N), (70, 74, 80, 255)), out, dark.point(lambda v: int(v * 0.55))
    )
    out.putalpha(base.getchannel("A"))
    return out


def main():
    args = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
    package, out = Path(args[0]), Path(args[1])
    sheet_path = None
    if "--sheet" in sys.argv:
        sheet_path = Path(sys.argv[sys.argv.index("--sheet") + 1])
        args = [arg for arg in args if arg != str(sheet_path)]
    found = glyphs(package)
    font = package / "src/fill/Phosphor-Fill.ttf"
    base = disc()
    out.mkdir(parents=True, exist_ok=True)
    drawn = {}
    for name, spec in ICONS.items():
        drawn[name] = emboss(base, symbol(font, found, spec)).resize((CELL, CELL), Image.LANCZOS)
        drawn[name].save(out / f"{name}.png", optimize=True)
    (out / "small").mkdir(exist_ok=True)
    for name, spec in SMALL.items():
        picture = emboss(base, symbol(font, found, spec))
        picture.resize((SMALL_SIZE, SMALL_SIZE), Image.LANCZOS).save(
            out / "small" / f"{name}.png", optimize=True
        )
        drawn[name] = picture.resize((CELL, CELL), Image.LANCZOS)
    print(f"{len(ICONS)} icons and {len(SMALL)} small ones in {out}")
    if sheet_path:
        columns = 8
        rows = (len(drawn) + columns - 1) // columns
        sheet = Image.new("RGBA", (columns * 150, rows * 160), (20, 20, 20, 255))
        pen = ImageDraw.Draw(sheet)
        for index, (name, picture) in enumerate(drawn.items()):
            x, y = (index % columns) * 150 + 11, (index // columns) * 160 + 6
            sheet.paste(picture, (x, y), picture)
            pen.text((x, y + 134), name, fill=(200, 200, 200, 255))
        sheet.save(sheet_path)


if __name__ == "__main__":
    main()
