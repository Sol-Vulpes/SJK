# WHEEL OF DOOM

The WHEEL OF DOOM is no longer a thing on a map. Its outcomes are what a **DOOM coin**
rolls at a [Force box](mystery-boxes.md#the-doom-coin-roll): Use held 1 s at a box with a
coin (the teddy bear in a box brings one) asks the hub (`POST /v1/mystery-box/doom`), and
a punishment strikes the roller through the effects below (`wheel_of_doom_strike`).
`toy_wheelofdoom` (Collection > Toys, its "Coins" switch; the quick wheel; a key in
Settings > Controls) says the player's DOOM coins and points at the Force box. Code:
[wheel_of_doom.rs](../crates/sjk-viewer/src/wheel_of_doom.rs) (the outcome table, the
coins, the quack), [effects.rs](../crates/sjk-viewer/src/wheel_of_doom/effects.rs) (what
an outcome does to the roller), [draw.rs](../crates/sjk-viewer/src/wheel_of_doom/draw.rs)
(the HUD line, Blind's darkness), [mirror.rs](../crates/sjk-viewer/src/wheel_of_doom/mirror.rs)
(Mirror's flip), and the hub calls in
[sjk-identity's wheel_of_doom.rs](../crates/sjk-identity/src/wheel_of_doom.rs).

## History

- 07/11/2026: a toy. `toy_wheelofdoom` stood a wheel before the spinner, who paid 100
  credits, every 5 minutes, 16 segments, a 6-second spin.
- 09/11/2026: out of order (the hub's `--wheel-of-doom` switch off, Sol: bugged).
- 11/11/2026: wheels SJK's creator placed on maps, spun with Use for a DOOM coin, 20
  segments, a 34-second spin to Sol's music; 15/11/2026: a "Possible drops" panel.
- 20/11/2026: merged into the Force box (Sol). The client took no placed wheel any more
  and `wheelofdoom_place` refused.
- 23/11/2026, the clean-up ahead of v1: the placed wheels left the client. Removed: the
  two models, their pictures and shader, the tick, stop and fanfare sounds, the music's
  cue, the spin, its Use prompt and drops panel, the words over a wheel, the "out of
  order" path (the hub's `open` flag and `wheel_off` wording for a wheel), the spin
  request (`POST /v1/wheel-of-doom/spin`) and its feed event (`land_<wheel>_<outcome>`),
  `wheelofdoom_place` and `wheelofdoom_list`, and the world shots of a wheel. Kept:
  `wheelofdoom_remove <id>` (SJK's creator; the hub still stores the wheels placed before,
  for older clients, until it deletes them), the coin read (`GET /v1/wheel-of-doom`, of
  which only `coins` is read) and everything below.

## The outcomes

Twenty outcomes, in the hub's order (`OUTCOMES` in `wheel_of_doom.rs`; an id the client
does not know is told by its name and does nothing). Their chances are the hub's, at the
box's own odds ([mystery-boxes.md](mystery-boxes.md#the-doom-coin-roll)).

| # | Outcome | What happens to the roller |
| --- | --- | --- |
| 0 | `mythical` | The gold jackpot: a Mythical holocron (source `wheel`, uncapped, untradeable, as box ones), Sol's exception of 07/11/2026. "MYTHICAL HOLOCRON! In your Collection". The holocrons are read again at once, and it arrives as any new holocron does ([holocrons.md](holocrons.md#the-banner)); the hub's feed announces it to everyone as a Mythical drop |
| 1 | `reverse_keys` | 30 s: forward and back, left and right swapped in the user command |
| 2 | `holo_uncommon` | An Uncommon (green) holocron, as the Mythical one: source `wheel`, uncapped, untradeable |
| 3 | `invert_mouse` | 30 s: the mouse's pitch and yaw both turned the wrong way |
| 4 | `explode` | The box blows up, as its teddy bear does ([mystery-boxes.md](mystery-boxes.md)) |
| 5 | `holo_rare` | A Rare (blue) holocron, the same |
| 6 | `mirror` | 30 s: the whole picture flipped left to right, and the strafe and yaw swapped, so it plays mirrored |
| 7 | `upside_down` | 20 s: the view rolled 180 degrees |
| 8 | `doom_coin` | "DOOM COIN BACK! Spin it again": the coin spent comes back |
| 9 | `blind` | 15 s: the screen fades to black (1.2 s in, 1 s out, never flashing) but for a small pinhole round the crosshair |
| 10 | `holo_legendary` | A Legendary (purple) holocron, the same |
| 11 | `drunk` | 30 s: the view sways and rolls slowly (9 degrees of roll at most) and the aim drifts a little (7 degrees, slowly), easing in and out over 1.5 s |
| 12 | `tiny_fov` | 20 s: field of view 40 |
| 13 | `nothing` | "Nothing happens..." |
| 14 | `huge_fov` | 20 s: field of view 150 |
| 15 | `no_saber` | 20 s: a lit saber is put out (the saber's own toggle, `sv_saberswitch`'s `generic_cmd`, every 0.75 s while it is lit) and cannot be lit (attacks, which light it, and the toggle are dropped while it is out) |
| 16 | `quack` | 30 s: a quack on every attack press, and every 0.45 s while one is held (SJK's own synthesized quack, heard by the roller alone) |
| 17 | `walk` | 30 s: forced walk (`BUTTON_WALKING`, moves at the walk's 46) |
| 18 | `bunny` | 20 s: jump held all the while |
| 19 | `curse` | "CURSED... Something bad comes within a minute": one of the twelve effects above (not mythical, nothing, explode or curse) strikes at a random moment within 60 s ("The curse strikes: ...!") |

A small line under the top of the HUD, in the wheel's red, names the effect running and its
seconds left ("WHEEL OF DOOM · MIRRORED · 23 s"), or "CURSED..." while a curse waits. One
effect runs at a time. Every effect ends on its timer, on a map change or another server, on
a disconnect and on the player's death (a curse waiting too). An outcome a newer hub knows
and this client does not is told by its name with "a newer hub's outcome", and does nothing.

Mirror flips the finished frame, the HUD included: once it is drawn, the window's image is
copied into a texture of its size and drawn back mirrored (a mirrored projection would turn
the culling inside out). The texture is made the first frame it is needed and let go when
the effect ends. A surface that cannot be copied from (no `COPY_SRC`) keeps the picture as
it is; the controls are still mirrored.

## Sounds

The Quack effect's quack is `sound/sjk/wheel_of_doom/quack.wav`, SJK's own, synthesized by
[wheel_of_doom_assets.py](../scripts/wheel_of_doom_assets.py) and bundled in
[assets/wheel_of_doom](../crates/sjk-viewer/assets/wheel_of_doom/README.md) with the toy's
icon; it plays in the roller's ears. Settings > Sound > Toys, "WHEEL OF DOOM sounds"
(`s_toyWheelOfDoomSound`, on by default), silences it. The roll's own sounds are the
Force box's ([mystery-boxes.md](mystery-boxes.md)).

## Limits

- The effects are the roller's own client's: a modified client could ignore them. They
  cost a few comparisons a frame and allocate nothing; the HUD line is written into a
  fixed string.
- No saber uses the saber's own toggle in the user command; a mod that ignores it, or a
  staff or dual saber half lit, may keep a blade on.
- Mirror flips the HUD too, and needs a surface the client can copy from.
- Bunny holds the jump even while the player types in the chat.
- The coin roll needs the hub's `POST /v1/mystery-box/doom` (hub branch `force-box`, not
  merged or deployed on 23/11/2026); against a hub without it the box says "This SJK hub
  does not roll DOOM coins yet."
- Verified: unit tests (the 20 outcomes and their order, the effects' input transforms and
  timers, the curse, the effects ending; on the identity side the coins read from the
  hub's full answer, a coin roll's answer, its feed event and the refusals). Not tried in
  a game.
