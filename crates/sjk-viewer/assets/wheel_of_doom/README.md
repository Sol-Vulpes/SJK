# WHEEL OF DOOM

The WHEEL OF DOOM toy (SJK's own, [wheel-of-doom.md](../../../../docs/wheel-of-doom.md)).
The wheel no longer stands on maps (its DOOM coin rolls at a Force box), so its model,
pictures, shader and spin sounds were removed on 23/11/2026. What is left is drawn by
[wheel_of_doom_model.py](../../../../scripts/wheel_of_doom_model.py) (the icon; the script
still draws the model it is cut from) and
[wheel_of_doom_assets.py](../../../../scripts/wheel_of_doom_assets.py) (the quack; it still
writes the three spin sounds), from shapes and synthesized sound, nothing taken from
another game.

| File | Game path | What |
| --- | --- | --- |
| `toy_wheelofdoom.png` | `gfx/sjk/toy_wheelofdoom.png` | The toy's icon (Collection > Toys, key bindings), 128x128 |
| `quack.wav` | `sound/sjk/wheel_of_doom/quack.wav` | The Quack effect |

The client mounts them in memory below all game data
([wheel_of_doom.rs](../../src/wheel_of_doom.rs)), so a PK3 with the same paths replaces
them.
