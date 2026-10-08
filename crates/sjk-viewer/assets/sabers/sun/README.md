# The Sun blade's sounds

The Sun blade is SJK's first blade skin ([unlockables.md](../../../../../docs/unlockables.md#the-sun-blade)).
Its sounds are SJK's own: synthesized from oscillators and filtered noise with a
fixed seed by [saber_skin_sounds.py](../../../../../scripts/saber_skin_sounds.py), not
edited by hand and not taken from any game data. All are 16-bit mono WAV at 22050 Hz.

| File | Game path | What |
| --- | --- | --- |
| `on.wav` | `sound/sjk/sabers/sun/on.wav` | Ignition, 1 s |
| `off.wav` | `sound/sjk/sabers/sun/off.wav` | Switching off, 0.8 s |
| `hum.wav` | `sound/sjk/sabers/sun/hum.wav` | The hum, a seamless 2 s loop |
| `swing1.wav` to `swing3.wav` | `sound/sjk/sabers/sun/swing1.wav` ... | Swings, 0.45 to 0.6 s |

The client mounts them in memory below all game data
([saber_skins.rs](../../../src/saber_skins.rs)), so a PK3 that has the same paths
replaces them.

## Regenerating

With Python 3 alone, from the repository root:

```sh
python3 scripts/saber_skin_sounds.py crates/sjk-viewer/assets/sabers/sun
```
