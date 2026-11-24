# WHEEL OF DOOM

The WHEEL OF DOOM's outcomes (SJK's own, [wheel-of-doom.md](../../../../docs/wheel-of-doom.md)),
which a DOOM coin rolls at a Force box. The wheel no longer stands on maps, so its model,
pictures, shader and spin sounds were removed on 23/11/2026, its toy icon and model script
with the `toy_wheelofdoom` toy the same day. What is left is the Quack effect's quack,
synthesized by [wheel_of_doom_assets.py](../../../../scripts/wheel_of_doom_assets.py),
nothing taken from another game.

| File | Game path | What |
| --- | --- | --- |
| `quack.wav` | `sound/sjk/wheel_of_doom/quack.wav` | The Quack effect |

The client mounts it in memory below all game data
([wheel_of_doom.rs](../../src/wheel_of_doom.rs)), so a PK3 with the same path replaces
it.
