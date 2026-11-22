# Feats

The feats' medal pictures (`cg_feats`, [client.md](../../../../docs/client.md#feats)), one
per feat, 128x128 on transparency. All of them are made by
[feat_assets.py](../../../../scripts/feat_assets.py) from shapes (nothing taken from another
game); none is edited by hand.

| File | Game path | What |
| --- | --- | --- |
| `hooked.png` | `gfx/sjk/feats/hooked.png` | HOOKED! (a JA+ grapple kill): a grapple hook on its line |
| `first_blood.png` | `gfx/sjk/feats/first_blood.png` | First Blood: a drop |
| `double_kill.png` to `penta_kill.png` | `gfx/sjk/feats/<id>.png` | Double to Penta Kill: two to five saber blades |
| `massacre.png` | `gfx/sjk/feats/massacre.png` | Massacre: two crossed blades |
| `killing_spree.png` to `unstoppable.png` | `gfx/sjk/feats/<id>.png` | Killing Spree, Rampage, Dominating, Unstoppable: a star with one to four pips |
| `godlike.png` | `gfx/sjk/feats/godlike.png` | Godlike: a sun |
| `spree_ended.png` | `gfx/sjk/feats/spree_ended.png` | Spree Ended: a broken star |
| `revenge.png` | `gfx/sjk/feats/revenge.png` | Revenge: an arrow turning back |

The client mounts them in memory below all game data ([feats.rs](../../src/feats.rs)), so a
PK3 that has the same paths replaces them.
