# Mystery box

The Force box's chest and lid (the mystery box, an ancient Jedi Force box since
20/11/2026: a placeholder in temple stone, old gold and blue runes until Sol's own model)
and the Box hunt toy's icon (SJK's own, [mystery-boxes.md](../../../../docs/mystery-boxes.md)),
made by [mystery_box_model.py](../../../../scripts/mystery_box_model.py) and
[teddy_bear_model.py](../../../../scripts/teddy_bear_model.py) from noise and shapes;
nothing is taken from another game, and only `mystery_box.shader` is written by hand. The
box's sounds are Jedi Academy's own retail files, read from the player's game and not
shipped (the music box tune and sparkle bundled here before, and the extras pack's
opening, roll and closing, are no longer played); its laugh is still Sol's, in the
optional SJK extras pack.

| File | Game path | What |
| --- | --- | --- |
| `mystery_box.md3` | `models/sjk/mystery_box.md3` | The chest, 60 long (y) x 26 deep (x, its front facing +X) x 24 tall, its origin in the middle of its bottom: feet, a plinth, raised gold bands, corner posts, a rim, a lock plate and end handles, open at the top (dark walls and a floor inside); and a second surface, the halo: a band of light round its top rising 52 units |
| `mystery_box_lid.md3` | `models/sjk/mystery_box_lid.md3` | The lid, arched, banded, its origin on the hinge (the chest's back top edge), reaching +X; the client pitches it open |
| `teddy_bear.md3` | `models/sjk/teddy_bear.md3` | The teddy bear a box stops on when it blows up, about 18 units tall, facing +Y (made by [teddy_bear_model.py](../../../../scripts/teddy_bear_model.py)) |
| `teddy_bear.jpg` | `models/sjk/teddy_bear.jpg` | 256x256: its fur, light muzzle and belly, black eyes and nose, tan pads and red bow |
| `mystery_box.jpg` | `models/sjk/mystery_box.jpg` | 1024x1024: a long side (temple stone, gold bands, two runes, grooves along it), the lid's top, an end, the dark inside, the gold edge |
| `mystery_box_glow.jpg` | `models/sjk/mystery_box_glow.jpg` | The same layout, black but for the runes' and grooves' blue glow |
| `mystery_box_halo.jpg` | `models/sjk/mystery_box_halo.jpg` | 128x256: the halo's light, soft blue rays brightest at the foot, fading to black at the top |
| `mystery_box.shader` | `shaders/sjk_mystery_box.shader` | `models/sjk/mystery_box`: lit as a player model is, the glow added on top, pulsing; `models/sjk/mystery_box_halo`: added, two-sided, its rays drifting, breathing; `models/sjk/teddy_bear`, lit the same; `models/sjk/box_loot`, lit with a sliding glint |
| `toy_boxhunt.png` | `gfx/sjk/toy_boxhunt.png` | The Box hunt toy's icon, 128x128: the chest's front |
| `box_loot.jpg` | `models/sjk/box_loot.jpg` | 256x256 swatches: the coins' gold and rim (made by [box_loot_model.py](../../../../scripts/box_loot_model.py), 11/11/2026) |
| `coins_small.md3`, `coins_medium.md3`, `coins_big.md3` | `models/sjk/coins_*.md3` | Heaps of 5, 14 and 30 gold coins, the credits a box pays (50, 200, 500) |
