# SJK branding

SJK's emblem is Sol's monogram (16/11/2026): two steel rings, a pair of pillars
with chamfered ends standing through them, and three 45-degree arms, the K on
the right pillar and the peace sign's arm on the left, before a burning sun.
Sol supplied it as two AI renders, the steel alone and the steel before a sun;
neither is kept here. [sjk_emblem.py](../../scripts/sjk_emblem.py) rebuilds the
design from about ten numbers measured on them (the renders' strokes wander by
a few pixels; the rebuild's do not) and writes every image below, so none of
them is edited by hand.

| File | Size | Used by |
| --- | --- | --- |
| `sjk-emblem.svg` | vector | The flat master: the steel pieces on their dark outline |
| `sjk-logo.png` | 1024, transparent | The emblem with its sun, round, for anything large |
| `sjk-logo-512.png` | 512, transparent | README and release notes |
| `emblem-steel.png` | 1024, transparent | The menu emblem's steel ([emblem.rs](../../crates/sjk-viewer/src/menu/emblem.rs)), warmed by the sun behind it |
| `emblem-disc.png` | 512, additive | The menu sun's plasma disc, which turns |
| `emblem-flames-a.png`, `emblem-flames-b.png` | 768, additive | The menu sun's flames and corona: two versions that turn against each other and cross-fade |
| `emblem-rays.png` | 1024, additive | The menu sun's god rays and spikes |
| `emblem-embers.png` | 512, additive | The menu sun's sparks; two copies drift out in turn |
| `emblem-chat.png` | 128, transparent | SJK's emblem before SJK chat lines ([ui_renderer.rs](../../crates/sjk-viewer/src/ui_renderer.rs)): the small icons' drawing |
| `icon-32.png`, `icon-64.png` | 32, 64 | The client's window icons ([window_icon.rs](../../crates/sjk-viewer/src/window_icon.rs)) |
| `sjk.ico` | 16 to 256 | Windows icon of `sjk.exe` and `sjk-server.exe` (the crates' `build.rs`) |
| `jof-emblem.png` | 128, full colour | The JoF clan's emblem beside tagged names ([jof_tag.rs](../../crates/sjk-viewer/src/jof_tag.rs)); the clan's green J o F logo (80x93, supplied by Sol, not kept here) enlarged by `python scripts/clan_emblem.py <logo.png> <out.png>` (not SJK's emblem) |
| `jofa-emblem.png` | 128, full colour | JoF Academy's emblem beside JoFA-tagged names ([jof_tag.rs](../../crates/sjk-viewer/src/jof_tag.rs)); the academy's HD logo (1295x1214, supplied by the clan, not kept here) shrunk by `python scripts/clan_emblem.py <logo.png> <out.png>` (not SJK's emblem) |
| `jawa-emblem.png` | 128, full colour | The JAWA clan's emblem beside JAWA-tagged names ([jof_tag.rs](../../crates/sjk-viewer/src/jof_tag.rs)); the clan's forum icon, a Jawa's hood (60x60, supplied by Sol, not kept here), enlarged by `python scripts/clan_emblem.py <icon.png> <out.png>` (not SJK's emblem) |
| `s-emblem.png` | 128, full colour | The {{S}} clan's emblem beside `{{S}}`-tagged names ([jof_tag.rs](../../crates/sjk-viewer/src/jof_tag.rs)); the clan's red Sith emblem (178x93 with margins, supplied by Sol, not kept here) by `python scripts/clan_emblem.py <logo.png> <out.png>` (not SJK's emblem) |
| `jof-ejk-icon.png` | 64, full colour | JoF EternalJK's icon, which an SJK chat message sent from that client starts with in place of SJK's emblem ([sjk_line.rs](../../crates/sjk-viewer/src/chat/view/sjk_line.rs)); supplied by JoF as is (not SJK's emblem) |
| `sjk-launcher-icon.png` | 64, full colour | The SJK Launcher's icon, which an SJK chat message sent from the launcher's chat window starts with in place of SJK's emblem ([client_icons.rs](../../crates/sjk-viewer/src/chat/client_icons.rs)); made by `python scripts/launcher_icon.py <out.png> 64` in Sol-Vulpes/sjk-launcher (not SJK's emblem) |
| `site/assets/sjk-logo-512.png` | 512 | The site's hero, and Discord's rich presence picture |
| `site/assets/favicon.ico`, `favicon-32.png` | 16 to 48 | The site's favicon and navigation mark |
| `site/assets/apple-touch-icon.png` | 180 | Home-screen icon |
| `site/assets/social-preview.jpg` | 1200x630 | Open Graph and Twitter card |

## Regenerating

With Python 3 and Pillow, numpy and scipy (`pip install pillow numpy scipy`),
from the repository root (about four minutes):

```sh
python scripts/sjk_emblem.py build
```

`python scripts/sjk_emblem.py preview <folder>` writes quick looks instead: the
steel alone, the emblem with its sun, and every icon size magnified on a dark
and a light background.

## Choices

- **Geometry.** Every piece is a signed distance field, so each size is drawn
  exactly with analytic anti-aliasing instead of shrunk from a picture. The
  arms are exactly as wide as makes the K's two arms meet on the right pillar's
  edge at the centre line, as the renders intended.
- **Steel.** Polished gunmetal shaded from the distance field: a bevel inside
  every edge, a key light from the upper left, a fine brushed grain, and the
  sun's warmth on its faces and on the edges that face it.
- **The sun.** Procedural, in the Sun blade's spirit: plasma (slow swirls,
  granulation, bright filaments) brightest at the limb, a thin white-gold rim,
  two rings of flame tongues (short dense ones, long curling ones) with a
  corona, soft god rays, pointed spikes (the vertical one the longest, as in
  Sol's render) and embers, bloomed in four widths and tone-mapped with the
  ACES curve.
- **Moving in the menu.** The sun is cut into layers the client moves: the
  disc turns once in four minutes and breathes, the two flame layers turn
  against each other and cross-fade on an uneven beat, the rays shimmer and the
  embers grow outwards and fade (`menu/art/motion.rs`).
- **Small icons.** 48 and 64 pixels use slightly heavier strokes and lighter
  steel. At 32 pixels and under the full drawing turns to mush, so the icons
  draw a simpler emblem: plain light steel, no thin band, a hot disc in place of
  the flames; 32 keeps the pillars' slot, 20 and 24 join the pillars into one
  stem, and 16 leaves the arms out. Large icons are rendered four times larger
  and shrunk with an area filter, so their sparks and spikes do not alias, and
  their glow fades out before the square's edge so the icon stays round.
- **Formats.** PNG keeps the transparency (straight colour, un-multiplied from
  the glow); the menu's light layers are opaque, black adding nothing. The
  social card is a JPEG because it is opaque and photographic. Every `.ico`
  frame is PNG-compressed (Windows Vista and later).
