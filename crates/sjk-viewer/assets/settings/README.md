# Settings icons

The settings menu's icons ([settings_icons.rs](../../src/settings_icons.rs)): grey
metal discs in the quick wheels' style ([assets/wheel](../wheel/README.md)), one for
every Settings category, every sub-heading of a category
([section_icons.rs](../../src/settings/section_icons.rs)), the key bindings' groups
and a few other screens (the in-game menu, the news).

Since 21/11/2026 every one of them is drawn by
[settings_icons_draw.py](../../../../scripts/settings_icons_draw.py), so the set has
one look and a new icon is one line in the script:

- the disc (a bright bevelled rim, a dark channel, a thin inner ring, a darker face
  engraved with sixteen faint spokes, lit from the top left) is computed;
- the symbol is a glyph of the [Phosphor Icons](https://phosphoricons.com) "fill"
  font, by the Phosphor Icons authors, under the MIT License
  ([Phosphor-LICENSE.txt](Phosphor-LICENSE.txt)), laid on the disc as a raised white
  plate with a soft shadow; the crossed lightsabers are drawn by the script itself.

There are two sizes. The 32 in this folder are 128x128, the UI atlas cell: the
categories', the key bindings' groups' and the ones other screens draw large. The 32
in [small/](small) are 64x64: the pictures only Settings' sub-headings wear, drawn
small there. The client packs those four to an atlas cell at start
(`settings_icons::small_cells`) and draws each from its quarter, because the atlas is
full: it is as tall as a device is asked to allow, so 32 more whole cells do not fit.
All are on transparency with an anti-aliased round edge. The font is not in the
repository: only the pictures drawn from it are.

Until then the icons were cut from an icon board Sol generated on 07/10/2026
(`scripts/settings_icons.py`, removed with it; gold-on-navy and holocron-face sets
replaced them on 13/10/2026 and Sol brought the grey discs back on 15/10/2026). The
board had 32 discs and no way to add one in the same hand; the quick wheel's icons
still come from their own boards.

## Regenerating, or adding an icon

With Python 3, Pillow and numpy (`pip install pillow numpy`), from the repository
root:

```sh
curl -sL https://registry.npmjs.org/@phosphor-icons/web/-/web-2.1.1.tgz | tar -xz
python scripts/settings_icons_draw.py package crates/sjk-viewer/assets/settings
```

`--sheet board.png` also writes a labelled contact sheet of the whole set.

To add one: a line in the script's `ICONS` or `SMALL` (its file name and a Phosphor
glyph's name, as on phosphoricons.com), the same name in `settings_icons.rs`'s `ICONS`
or `SMALL`, and run the script. The atlas keeps 40 cells for them (`ui_renderer::icons`,
`SETTINGS_ICON_CELLS`, eight of them taken from the quick wheel's spare ones): 32 whole
icons and room for 32 small ones, all in use. A further icon needs a cell found
elsewhere in the atlas; a test checks the count and the sizes.
