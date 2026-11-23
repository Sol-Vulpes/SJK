# Mystery box

The Force box (the mystery box), an ancient Sith tomb since 23/11/2026 (black basalt,
red glyphs, a lid in two slabs that part; a Jedi box of temple stone, old gold and blue
runes from 20/11/2026, a wooden chest before), and the Box hunt toy's icon (SJK's own, [mystery-boxes.md](../../../../docs/mystery-boxes.md)),
made by [mystery_box_model.py](../../../../scripts/mystery_box_model.py) and
[teddy_bear_model.py](../../../../scripts/teddy_bear_model.py) from noise and shapes;
nothing is taken from another game, and only `mystery_box.shader` is written by hand. The
box's sounds are Jedi Academy's own retail files, read from the player's game and not
shipped (the music box tune and sparkle bundled here before, and the extras pack's
opening, roll and closing, are no longer played); its laugh is still Sol's, in the
optional SJK extras pack.

| File | Game path | What |
| --- | --- | --- |
| `mystery_box.md3` | `models/sjk/mystery_box.md3` | The tomb, its body 54 long (y) x 28 deep (x, its front facing +X) at its foot, on two steps (80 x 41 on the floor), its origin in the middle of its bottom: walls leaning in to a rim 27 up, a pilaster a quarter in from each end of the long sides, a horn at each corner leaning in to 47 up, open at the top (lit walls and a floor inside); and a second surface, the halo: a band of red light round its top rising 64 units |
| `mystery_box_lid.md3` | `models/sjk/mystery_box_lid.md3` | Half the lid: a slab from the seam (its origin, the middle of the seam's bottom edge) reaching 15 to +X, 55 long, sloping from the ridge (8 up) to its edge (3), three ribs with clasps; the client draws it twice, the second turned half round, 27 up, and slides the two apart |
| `mystery_box_fire.md3` | `models/sjk/mystery_box_fire.md3` | What an open tomb lets out, its origin on its embers (the client stands it 13 up and stretches it up as the lid parts): a sheet of embers (`models/sjk/mystery_box_ember`) and two sleeves of fire rising 62 and 78 (`models/sjk/mystery_box_fire`) |
| `teddy_bear.md3` | `models/sjk/teddy_bear.md3` | The teddy bear a box stops on when it blows up, about 18 units tall, facing +Y (made by [teddy_bear_model.py](../../../../scripts/teddy_bear_model.py)) |
| `teddy_bear.jpg` | `models/sjk/teddy_bear.jpg` | 256x256: its fur, light muzzle and belly, black eyes and nose, tan pads and red bow |
| `mystery_box.jpg` | `models/sjk/mystery_box.jpg` | 1024x1024: a long side (basalt blocks, the sigil, glyphs, lines of light), a slope of the lid, a strip for the steps and edges, an end, a rib, a horn's face, plain blocks, the inside, the hot seam |
| `mystery_box_glow.jpg` | `models/sjk/mystery_box_glow.jpg` | The same layout, black but for what glows red |
| `mystery_box_n.png` | `models/sjk/mystery_box_n.png` | The same layout, the stone's relief (joints, cuts, cracks) as a normal map, +t down |
| `mystery_box_halo.jpg` | `models/sjk/mystery_box_halo.jpg` | 128x256: the halo's light, soft red rays brightest at the foot, gone three quarters up (squared: the client adds it in linear light, where a faint grey shows as a pane) |
| `mystery_box_fire.jpg` | `models/sjk/mystery_box_fire.jpg` | 128x256: the fire's tongues, each dying at its own height |
| `mystery_box_ember.jpg` | `models/sjk/mystery_box_ember.jpg` | 256x256, tiling: veins of red embers |
| `mystery_box.shader` | `shaders/sjk_mystery_box.shader` | `models/sjk/mystery_box`: lit as a player model is through its normal map, the glow added on top, pulsing, in the dynamic glow; `models/sjk/mystery_box_halo`: added, two-sided, its rays drifting, breathing; `models/sjk/mystery_box_fire` and `_ember`: added, two layers each moving against the other; `models/sjk/teddy_bear`, lit the same; `models/sjk/box_loot`, lit with a sliding glint |
| `toy_boxhunt.png` | `gfx/sjk/toy_boxhunt.png` | The Box hunt toy's icon, 128x128: the tomb's front under a slope of its lid |
| `box_loot.jpg` | `models/sjk/box_loot.jpg` | 256x256 swatches: the coins' gold and rim (made by [box_loot_model.py](../../../../scripts/box_loot_model.py), 11/11/2026) |
| `coins_small.md3`, `coins_medium.md3`, `coins_big.md3` | `models/sjk/coins_*.md3` | Heaps of 5, 14 and 30 gold coins, the credits a box pays (50, 200, 500) |
