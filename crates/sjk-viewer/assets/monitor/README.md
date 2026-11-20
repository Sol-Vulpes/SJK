# Force Throw

The Force Throw toy (the Monitor until 21/11/2026, its commands, cvars and this folder's
name kept; SJK's own, [client.md](../../../../docs/client.md#monitor)) hurls junk with the
Force that sends the players it hits flying as ragdolls. What it hurls (a banded crate, a
wrecked R5 and a wrecked GONK droid), the pieces a crate bursts into and its sounds are
Jedi Academy's own retail files, read from the player's game by path and not shipped
([monitor.rs](../../src/monitor.rs), `JUNK`, `CHUNKS` and the sounds). Only the two files
here are SJK's, made by [force_throw_assets.py](../../../../scripts/force_throw_assets.py)
from shapes and synthesized sound.

| File | Game path | What |
| --- | --- | --- |
| `toy_monitor.png` | `gfx/sjk/toy_monitor.png` | The toy's icon (quick wheel, Collection, key bindings), 128x128: a banded crate flying, blue rings of the Force behind it |
| `hit.wav` | `sound/sjk/monitor/hit.wav` | The thrower's hit sound when their throw strikes someone: a bright tick, 22050 Hz mono |

The client mounts them in memory below all game data, so a PK3 that has the same paths
replaces them. The old CRT monitor's model, picture, shader and synthesized sounds
(`monitor_assets.py`) went with the rework.
