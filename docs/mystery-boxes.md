# Mystery boxes

**Since 23/11/2026 the box is an ancient Sith tomb** (Sol: "more of an ancient sith tomb
thing, dark, and with red effects, stone door sound when it opens"; it keeps the name Force
box in game, and mystery box in the code, commands and hub). SJK's own, made by
[mystery_box_model.py](../scripts/mystery_box_model.py) from noise and shapes, nothing
taken from a game or a film (its glyphs are random strokes of no alphabet):

- a sarcophagus of black basalt, 54 x 28 at its foot on two steps (80 x 41 on the floor),
  its walls leaning in to a rim 27 units up, a horn of stone leaning in over each corner
  to 47 units, a pilaster a quarter in from each end of the long sides;
- cut into it and glowing red, pulsing slowly (and in the dynamic glow): a sigil in a
  sunk disc on each side and end, glyphs either side of it and on the lid, lines of
  light, a notched slit down each horn and pilaster, a few hot cracks; the stone takes
  the world's light through a normal map of its joints and cuts;
- a ridged lid in two slabs that meet along the ridge, a thin line of light on the seam.
  Opening, the slabs lift 2.5 units off the rim, slide 10 apart and tip 5 degrees
  outwards over 2.3 seconds, and shut the same way (`lid_slabs`; one model,
  `mystery_box_lid.md3`, drawn twice, the second turned half round);
- inside, stone lit from below, and while the lid is parted a sheet of embers and two
  sleeves of red fire that rise with the slabs (`mystery_box_fire.md3`, stretched up by
  how far the lid has parted); what the roll shows waits 0.9 s inside for the lid, then
  rises to 58 units; the words stand 84 up;
- a faint red halo rising 64 units over it, as before, so a box is found from afar.

Its sounds are still Jedi Academy's own, by path: `movers/doors/door_stone_start` as the
seal breaks, `door_stone_move` (2.4 s of stone grinding) while the slabs part and again
while they shut, `door_stone_stop` as they come to rest either way; Force speed's hum
still runs through the roll, and the take, win, jackpot and empty sounds are those of
20/11/2026. Force pull and push no longer play.

**From 20/11/2026 to 23/11/2026 the Force box** (Sol: "Star Wars themed, not a wooden box, more like an
ancient Jedi Force box, with its own sounds from Star Wars"; the code, commands and hub keep
the name mystery box). Its look was a placeholder: the same chest,
repainted in temple stone and old gold, a glowing blue rune of SJK's own on each half of its
long sides, grooves of blue light along them and a blue halo. A model had only to keep
the files' contract ([the assets' README](../crates/sjk-viewer/assets/mystery_box/README.md):
the chest 60 x 26 x 24 at its bottom's middle, front +X, the halo a second surface; the lid
on its hinge reaching +X; shaders `models/sjk/mystery_box` and `_halo`). Its sounds were Jedi
Academy's own, by path: Force pull opened it, Force speed's hum runs through the roll (5.14 s,
the roll 5.2), a Force push shuts it, `protecthit` takes the prize, `interface/secret_area`
greets the jackpot and `noforce` nothing; the bundled music box tune and the extras pack's
opening, roll and closing are no longer played (the extras laugh is). It also takes the
WHEEL OF DOOM's DOOM coin: [The DOOM coin roll](#the-doom-coin-roll).

Boxes SJK's creator places on maps, which any SJK player opens for credits (Sol's idea,
27/10/2026). The hub keeps where each box stands, takes the price, rolls what is inside
and pays it; the client draws the boxes on the map file they were placed on, opens the one
the player faces with `+use`, and shows what came out to the opener and to the SJK players
around. Code: [mystery_box.rs](../crates/sjk-viewer/src/mystery_box.rs), the hub calls in
[sjk-identity's mystery_box.rs](../crates/sjk-identity/src/mystery_box.rs).

No game server knows of a box. It is drawn by SJK clients only, has no collision (players
walk through it), and players on other clients see someone pressing Use at nothing.

The first box was SJK's own too (bundled, `assets/mystery_box`, made by
[mystery_box_model.py](../scripts/mystery_box_model.py)): a long wooden chest bound in
iron, 60 x 26 x 24 units, a pale question mark glowing (pulsing) on each half of its two
long sides, and a lid hinged at the back that swings open while it rolls and closes once
what came out is taken. Since 30/10/2026 (Sol: "not just that rectangle") it stands on
four iron feet under a plinth, its three iron bands and four corner posts stand out of the
planks, an iron rim runs round its top, a lock plate hangs on its front and a handle on
each end, and its lid arches, banded too. Since 31/10/2026 it is open under its lid, dark
walls and a floor inside (Sol: the top looked sealed with wood; it was closed with
planks and the bands' tops crossed it), and a faint halo of warm light rises 52 units
round its top, its rays drifting slowly, so a box is found from afar (Sol's idea; a second
surface of the chest's model, `models/sjk/mystery_box_halo`, added and two-sided).

Words stand in the world over every box, in the headline font (Fira Sans Condensed, as
centre prints), for every SJK player who sees it (hidden behind
walls, fading out past 1000 units, gone past 1400): "MYSTERY BOX" and its price (or "Free
roll: press Use" for a player holding one), "Rolling..." while it rolls, then what came
out (the credits, a holocron's or dormant kyber's tier...; the opener's also says
to press Use to take it).

## Opening one

Standing at a box (within 80 units across the floor and 96 up or down) and facing it
(within 60 degrees), the player sees the [Use prompt](client.md#use-prompt) by the
crosshair: the key bound to `+use` in a cap, then "Open mystery box" over its price or
"Free roll" (since 30/10/2026; a centre print said it once before). `+use` asks the hub to
open it; the hub refuses without the price, within the cooldown or when the box is gone,
and the refusal shows in the centre of the screen. What the client knows already, the
prompt says instead, its cap greyed: "Your turn again in N s", "100 credits: not
enough", "Opening..." while the hub answers, "Someone is opening it" at another player's
roll; it says "Take it" over the player's own prize, and nothing while their own roll
runs or at a box that blows up. While nobody opens it, a "Possible drops" panel fades in
under the prompt (since 15/11/2026): each thing the hub's table holds with its chance
("A holocron, Uncommon or better 45.5 %", "50 credits 15 %", "The teddy bear: a DOOM coin
12 %"...), likeliest first ([client.md](client.md#use-prompt)).

**The cooldown** (Sol, 30/10/2026): a player opens one box every 15 seconds (3 before,
the hub's `cooldown`), so a roll and its taking, about 8 seconds, leave the box free a
while for another player. The client knows its own last opening and says so on the Use
prompt without asking the hub.

An opened box plays its opening sound and its roll while it rattles and things rise out of
it and roll over it, changing fast, then slower, like a wound-down spring. At 5.2 seconds
(5.5 before 30/10/2026), on the roll's last note, it stops on what came out and plays a
chime, or a fanfare (the jackpot, the Mystery blade, a dormant kyber). As its lid closes
(taken, or after 10 seconds) it plays its closing sound.

Until 20/11/2026 (the Force box's sounds since, above): the opening, the roll and the
closing were Sol's sounds of 30/10/2026, in the optional SJK
extras pack (`unofficial_sjk_extras.pk3`, the Sith cloud's `extras`:
`sound/sjk/mystery_box/extras_open.mp3`, `extras_roll.mp3`, `extras_close.mp3`), not
bundled. A player without the pack hears a crate sliding open (retail) and a little music
box tune (SJK's own, bundled: `assets/mystery_box`, made by
[mystery_box_assets.py](../scripts/mystery_box_assets.py), its last chord at 5.2 s), and
no closing sound. Both rolls end together, so every client stops at the same time.

| Outcome | Stops on |
| --- | --- |
| `coins` | a small heap of gold coins |
| `double` | a pile of gold coins |
| `jackpot` | a big pile of gold coins |
| `explode` | a teddy bear (a thermal detonator before 30/10/2026) |
| `holocron` | the tier's own holocron, every part of it, for everyone (since 11/11/2026 the hub's event names the tier; SJK's placeholder holocron for others before) |
| `kyber` | the tier's own dormant kyber, for everyone |
| `shards` | three cyan kyber crystals; seven, bigger, for the opener's pile of more than 15 shards |
| `blade` | a saber hilt; for an opener who holds the blade already, the shards it pays instead |

Before 11/11/2026 (Sol: "replace the crystals for credits with actually a pile of coins")
`empty` stopped on a handful of bolts, `refund` on a blue crystal, `double` on a green
one, `jackpot` on a red one and `again` on a little mystery box; an older hub's `empty`
and `again` still do. The coins are SJK's own, bundled, made by
[box_loot_model.py](../scripts/box_loot_model.py). An older hub's `raygun` stops on the
bolts: the Ray Gun went on 20/11/2026.

What rolls before it stops is only these prizes (Sol, 29/10/2026: the game's weapons
rolled with them before, which no box gave), but the teddy bear, which only comes at the
end (30/10/2026): the three piles of coins, both piles of shards, the hilt, the four tiers' holocrons, Uncommon to Mythical, and the Rare and Mythical dormant kybers,
each drawn as the Holocrons tab draws it, its parts posed and turning
(`mystery_box::HOLOCRONS`, `KYBERS`; Sol, 11/11/2026: "we should see the kyber models and
holos too", no placeholder). `world_shot::mystery_box_shots::duel6_mystery_box_holocrons`
shows the four holocrons, `duel6_mystery_box_kybers` the four dormant kybers,
`duel6_mystery_box_rework` the coins and the hilt, and `duel6_mystery_box_loot` the
shards, the little box and the hilt.

The opener is told what came out in the centre of the screen. It floats over the box,
turning slowly, until they take it with `+use` at the box (a sparkle; a chat line gives
the credits left) or for 10 seconds, when it is taken for them. Meanwhile the box is busy:
another player's Use prompt there says someone is opening it. The hub pays at the opening
itself, so walking away loses nothing. The other SJK players on the server see and hear
the same roll through the hub's toy event (`mystery_box`, `open_<box>_<outcome>`). When
the opener takes what came out, their client tells each other SJK player on the server
through the toy relay (`mystery_box`, `take_<box>`, one player every 50 ms), and their box
closes too, with the take and closing sounds (30/10/2026; before, they saw it float for
the 10 seconds). Only the opener's take closes it; a player who misses it sees it go at
the 10 seconds.

Every sound plays at the box on a sound source of the box's own (`0x5200_0000` and its id),
never an entity's: the audio moves an entity's sounds to where it is, so box sounds once
followed the player on slot 0 (bug report #185).

Settings > Sound > Toys, "Mystery box sounds" (`s_toyMysteryBoxSound`), silences them.

The price and the odds are the hub's (`GET /v1/mystery-boxes`), PROVISIONAL, Sol's to
tune. Since 11/11/2026 (Sol: never empty loot, never the 100 credits back, no free
rerolls, much more likely at least a green holocron since a box costs about one, a very
rare dormant kyber):

| Outcome | Chance | Pays |
| --- | --- | --- |
| `holocron` | 52.5 % | a holocron, Uncommon (green) or better |
| `coins` | 15 % | 50 credits |
| `double` | 6 % | 200 credits |
| `jackpot` | 1 % | 500 credits |
| `explode` | 12 % | a DOOM coin; the price goes to a hunter |
| `shards` | 12 % | 10 to 25 kyber shards |
| `blade` | 1 % | the Mystery blade (60 shards if you hold it) |
| `kyber` | 0.5 % | a dormant kyber: 60 % Uncommon, 30 % Rare, 9 % Legendary, 1 % Mythical |

On average a box pays back about 25 of its 100 credits in coins, beside a holocron nearly
one time in two. Before 11/11/2026: `empty` 36 %, `refund` (the price back) 24 %,
`double` 5 %, `jackpot` 1 %, `explode` 12 %, `holocron` 10 %, `shards` 6 %, `again` (a
free roll) 5 %, `blade` 1 %, paying back 39 of the 100 credits (Sol,
29/10/2026 in the evening: 79 before, which a player rolling a hundred times barely felt;
double was 15%, jackpot 5%, empty 29% and explode 5%). A holocron came one time in fifty
before 29/10/2026, with empty 43% and refund 30%. The last three came the same day (Sol: "loot you
can land on"), their odds taken from empty (39% before) and refund (26%). A
newer hub's outcome the client does not know is shown by its credits alone.

**It blows up** (`explode`, Sol's idea of 28/10/2026, after Call of Duty zombies' boxes):
the roll stops on a teddy bear (since 30/10/2026; a thermal detonator before; SJK's own,
bundled, made by [teddy_bear_model.py](../scripts/teddy_bear_model.py)), and the box
laughs, "a ghost having fun" (`LAUGH`, Sol's laugh of 29/10/2026,
`sound/sjk/mystery_box/laugh.mp3`, in the optional SJK extras pack; a player without the
pack hears the retail gloat, `sound/chars/alora/misc/gloat1.mp3`). 1.5 seconds later, with
a detpack's explosion (`detpack/explosion`, `sound/weapons/detpack/explode.mp3`) where
the opener stands, it kills them: their own client sends the game's `kill`, which the
server carries out, so every player sees them die (Sol, 30/10/2026; before, their client
threw them down as a [ragdoll](client.md#ragdoll) and they got up). Not in a duel, with
the flag, or wherever the Ragdoll could not strike them (`Ragdoll::strikable`). The
laugh plays at the box for every SJK player who sees the roll (before 30/10/2026, to
every hunter wherever they were, at the opening). The hub hands what they paid (nothing
for a free roll) to another player on the server with the Box hunt toy on, at random, one
in play when there is one (since 03/11/2026, hub reports #189 and #204: before, a hunter
who had just left or sat in a menu could get it unseen, and the credits looked lost);
with nobody else hunting there, it is lost. The opener reads where it went as it blows
up; the one who got the credits reads it in the centre of the screen and the other
hunters in the chat once the roll stops on the bear (the hub's box hunt event
`laugh_<opener>_<to>_<credits>`, which arrives at the opening, held for the roll).

**The teddy bear brings a DOOM coin** (Sol, 11/11/2026): the opener of a box that blows up
gets a DOOM coin besides, the only way to spin a [WHEEL OF DOOM](wheel-of-doom.md); the
centre says "+1 DOOM coin".

**A holocron** (`holocron`, 28/10/2026): the roll stops on the tier's holocron, and the hub
gives one of any tier, rolled on the holocrons' own odds ([holocrons.md](holocrons.md));
it counts against no daily cap and cannot be traded (source `box`). The opener reads its
tier over the box and in the centre, and finds it in the Collection; since 11/11/2026 the
other players see the tier's own holocron too (the hub's event `open_<box>_holocron_<tier>`).

**A dormant kyber** (`kyber`, 11/11/2026, very rare): the hub gives a dormant kyber of a tier
rolled on the box's own odds (source `box`), attuned in the Collection as any dormant kyber
([holocrons.md](holocrons.md#dormant-kybers)); everyone sees the tier's dormant kyber over the
box (`open_<box>_kyber_<tier>`), with the jackpot's fanfare.

**The Ray Gun** (`raygun`, 11/11/2026 to 20/11/2026, after Call of Duty zombies' box) was
a prize the box handed its opener for 30 seconds, held and fired as a Prop gun; it went
with the Prop gun toy (Sol). Its 7 % went to the holocron (hub branch `no-raygun`,
deployed with the release that removed it); until then a hub may still roll it, and the
client stops on the bolts with nothing to give.

**Shards, a free roll, the Mystery blade** (`shards`, `again`, `blade`, 29/10/2026): a
pile of 10 to 25 kyber shards to spend in the shop; a free roll, spent on the next box
the player opens like one won in the box hunt (the hub's answer says the free rolls left,
`free`, and the client keeps that count; no box gives one since 11/11/2026, Sol: "it's
just waste of time", but the box hunt still does); or the boxes' own kyber, the Mystery
blade (`saber_mystery`, blade 169, Legendary), which no holocron drops, the shop never
sells and nobody trades. A player who holds it already gets 60 shards instead, what
shattering a Legendary kyber gives. The blade fanfares like the jackpot.

## The DOOM coin roll

Since 20/11/2026 (Sol: the WHEEL OF DOOM merged into the box, "you can just spend a special
coin to have better loot or worse punishment"; then "more loot still"), a player holding a
DOOM coin may roll it at an idle box instead of credits: a tap of Use opens the box for its
price as before (on the release), Use held 1 second rolls the coin. Without a coin, Use acts
on the press as it always did. The Use prompt says both ("Tap: 100 credits · Hold Use: DOOM
coin roll (2 coins)", then "Keep holding: DOOM coin roll" while held, or "DOOM coin roll
again in N s").

The hub (`POST /v1/mystery-box/doom`, the open's body; hub branch `force-box`) spends the
coin and rolls the WHEEL's outcomes at the box's own odds, per mille: Mythical 24, Legendary
72, Rare 180, Uncommon 324 (a holocron 60 %), the coin back 60, nothing 40, the blast 44,
the curse 28 and each of the twelve other punishments 19 (30 %). It pays as the WHEEL did (a
holocron of the tier, kept out of the caps and the trades; the coin back), under the
WHEEL's switch, one coin roll a box every 6 seconds and one a player every 15. The box
opens and rolls as for credits and stops on the tier's holocron, a heap of coins for the
coin back, the teddy bear for the blast (which blows the roller up as a box does) or the
bolts; the punishment then strikes the roller through the WHEEL's own effects
(`wheel_of_doom_strike`). Nothing waits to be taken: the outcome stays 3 seconds over the
box. The other SJK players see the roll from the hub's toy event `doom_<box>_<outcome>`
(older clients ignore it), their prompt saying "Someone rolls a DOOM coin".

## Placing one (SJK's creator)

| Command | Does |
| --- | --- |
| `mysterybox_place [server]` | Place a box where you aim, facing you: on every server running this map file, or with `server` on this one only |
| `mysterybox_remove [id]` | Remove the box nearest you (within 256 units), or the one with that id |
| `mysterybox_list` | The boxes on this map, their ids and distance, and the price (anyone) |

The map editor's Place page (`mapedit_panel`), under Things (09/11/2026; before, a Mystery
box and a Remove box button beside Place model), and the browser map editor's World page
([map-edits.md](map-edits.md#the-browser-editor-creator-only-for-now)) list the boxes and
place one where a click meets the floor (following the editor's scope, All servers or This
server), or remove the chosen one, through the same code (`mystery_box_place_at`, `mysterybox_remove <id>`); the hub has no move. Placing needs the map's BSP checksum (as map edits do, `sv_mapChecksum`): the box
stands in that exact map file only. The hub takes the creator's request only; every
placing and removal goes to the staff log. The client reads the hub's list once registered,
every 10 minutes, and at once after a placing, a removal or a box that is gone.

Some of its sounds are retail (the crate sliding open, the chime, the fanfare, the locked
door, the box hunt's alarm, the gloat); the tune and the take are SJK's own; the opening,
roll, closing and laugh of the extras pack are Sol's.

## Box hunt

Players opt in with the Box hunt toy (`toy_boxhunt`, Collection > Toys, the quick wheel;
look toy `box_hunt`); with it off a player is never the mark and is told nothing of the
hunt. Among the players of a server with it on and in play (at least 3), the hub picks a
mark at random for 3 minutes, one mark at most every 10 minutes (Sol, 29/10/2026: from
one pick to the next; the hunt rests in between, also when the mark is killed or leaves).

**Nobody away is tagged** (Sol, 23/11/2026: "avoid ever getting someone afk or typing
getting tagged for the hunt"). Three holes let it happen, and each is closed:

- idle meant two minutes without input, so a player who had walked off a minute ago was
  picked. For the hunt a player is away after 15 seconds without a key or the mouse
  (`hunt::IDLE`; holocrons keep their two minutes);
- a player who opened the chat as the hub made its pick was picked on a read a moment
  old. The mark's client reads the hunt at once on being told and says it is away; the
  hub (branch `hunt-in-play`) hands a mark found away within 20 seconds of the pick on
  at once to another hunter in play, for a whole 3 minutes, or calls the mark off when
  fewer than 3 others are in play and tries again a minute later;
- a client that had stopped answering was picked on what it said up to a minute before.
  The hub's pick now wants a read 30 seconds old at most (reads come every 20).

The second and third need the hub's `hunt-in-play`; without it the first alone applies.

**In play** (Sol, 31/10/2026; before, 2 players with the toy on, away or not): every read
of the hunt says whether the player is away: typing in the chat, in a menu, the console
or a dialog, spectating or following someone, at the intermission, or idle two minutes
(what holocrons count as playing, [holocrons.md](holocrons.md), and the chat besides). A
change is read at once (2 seconds apart at least). The hub picks nobody away, and needs 3
hunters in play; a hunter whose reads stopped for a minute counts as away. A mark away for
a minute since being picked hands it on: another hunter in play becomes the mark at once,
for a whole 3 minutes, told as any new mark is, when 3 others are in play; else they stay
the mark.

**The Sith look** (Sol, 23/11/2026: "give a new look to the hunt, the alert ... sith
themed", the box being a Sith tomb): the tomb marks a player. The card at the top centre
is a plate of black basalt pointed at both ends, edged in glowing red, a line of light
along its top and bottom and the tomb's split diamond at each end; it says "THE TOMB'S
MARK" (with the hunters left) over the name, or, blood red, "THE TOMB HAS CHOSEN" over
"YOU ARE THE MARK". It arrives as an alert: 1.9 times its size and 120 pixels lower for
2.6 seconds, then it shrinks up into its place in half a second, its edge flashing for
the first 2; on the mark's own screen the four edges flush red meanwhile. The alarm is
Force rage's roar (`sound/weapons/force/rage.mp3`, retail; `mpalarm` before). Over the
mark's head the word is "MARKED", and the arrow to them is a barbed blade. The centre
print reads "The tomb has marked you!", the hunters' chat line "the tomb has marked
<name>". The toy keeps its name, Box hunt. What follows describes the hunt as it was
first drawn; the rules are unchanged.

The mark is told in the centre of the screen ("You are the mark!"),
the other hunters by a chat line, and they see "MARK" in red over the mark's head
(hidden behind walls) and the mark's body ringed in red where they see it, the team
outlines' enemy ring (`hud::team_outline`, 29/10/2026), in every game type and whatever
`cg_teamOutline` says. While the game server sends the mark (they are in this player's
snapshot), a red arrow points at them through walls (Sol, 31/10/2026): over their head,
under the label, when they are in view, else on a ring round the crosshair pointing
their way. Every hunter, the mark too, hears the game's alarm
(`sound/effects/mpalarm.wav`, Sol's pick, 28/10/2026; with "Mystery box sounds") as a new
mark is picked, and a small card at the top centre of the screen names the mark (in
the headline font, "BOX HUNT · THE MARK" over the name) or, red, tells the player "YOU
ARE THE MARK"; its words take the menus' lifted colours (`text::CodePalette::Legible`), so
a black name reads on the dark card, and their shadow is the name without its colour
codes; its red edge flashes for 2 seconds as it arrives, and it stays until the
mark is hunted or the hub has no mark or another one. The card and the alarm are
[hunt.rs](../crates/sjk-viewer/src/mystery_box/hunt.rs) and
[label.rs](../crates/sjk-viewer/src/mystery_box/label.rs); the card draws with the words
in the world, so it is hidden with them (menus, console, scoreboard, intermission). A hunter who kills the mark wins a free mystery box roll: the
next box they open costs nothing (the centre says "press Use for your free roll" at a
box). The hunt then rests until 10 minutes after that mark was picked (a minute at
least). A hunter who wins nothing (they hold 3 unspent or won 5 that day) reads why in
the centre too. The day begins at 09:00 in Paris, as the holocrons' caps' (since
03/11/2026, hub report #228: a rolling 24 hours counted the evening before against
"today", so a player stopped after 3 or 4 kills one morning) (31/10/2026; before, in the chat only, so it looked like a kill that won
nothing).

**One life** (Sol, 01/11/2026): the hunters of a mark are its roster: those in play when
it is picked, and any who join in play while it lasts. Each has one life. A hunter the
mark kills is out of that hunt: their own client tells the hub (`POST /v1/box-hunt/out`,
`dead`, from the obituary: only a kill by the mark counts, not a fall, `/kill` or another
player, so the mark has to earn it), the mark no longer shows for them (no label, ring or
arrow), the card says "YOU'RE OUT", and killing the mark after they respawn wins nothing.
When every hunter still there is out, one at least killed by the mark, the mark wins 100
credits per hunter counted (ledger reason `box_hunt`, note `survived`; History says "Box
hunt: you outlasted every hunter"), at most 3 times a day (from 09:00 in Paris; after that, nothing),
and the hunt rests as after a kill. A hunter who left the server, took the toy off or was
away a minute stands in nobody's way and is not counted. The card counts the hunters
left ("2 OF 3 HUNTERS LEFT", from the hub's read). Hunters out only for cheating hand the
mark nothing: one must have been killed.

A player's report that they are out (killed by the mark, a cheat, or the mark's chat) is
kept by their client until the hub shows it counted: a read of the hunt at least 5 seconds
after it that still has them in the same chase (the hub's `out` empty, the same mark) sends
it again, 3 times at most, a read apart (17/11/2026, hub report #304: a hunter's teleport
report never reached the hub, so they were out on their screen and in the chase on the
hub's and the mark's, ringed in red and counted, and the mark's kills of them and its win
never counted). The hub counts an out once, so a report sent again pays nothing twice. The
hub pays the mark's win the moment it is told; an event that paid the player (their
`survived_` win, a blown box's price handed to them) asks for their profile within half a
minute, so the credits count follows (hub report #314: before, the next read of the
profile, up to 10 minutes later, and the win looked lost).

**Cheating** (Sol, 01/11/2026): a player in a hunt, mark or hunter, who teleports
themselves or flies in noclip is out of it at once. The hub sees neither, so the player's
own client watches its own player state and its own commands
([hunt_watch.rs](../crates/sjk-viewer/src/mystery_box/hunt_watch.rs)). Only the player's
own doing counts (since 03/11/2026, hub reports #235 and #237: a mark an admin brought to
them blew up, and map teleporters took hunters out): the game's `EF_TELEPORT_BIT` flipping
while they stay alive is a cheat only within 5 seconds of a command of theirs that moves
them (the `OWN_MOVES` table: JA+'s `amtele` in the forms that move the sender, which
`japlus.guntele`, `japlus.guntelewall`, `japlus.goto` and `japlus.teleoffset` send, and
`setviewpos`, which `japlus.recall` sends; 5 seconds for JA+'s Star Trek teleport, which
beams the player about 2 seconds late). The client keeps the last 16 reliable commands it
released to the server with when each left (`sjk_client::sent_commands`). `PM_NOCLIP`
counts after the player's own `noclip` on that map, and SJK's own fake noclip
(`fakenoclip`, the free camera) always. So does JA+'s `amghost` on the player themselves, sent
during the chase (hub report #240; a ghost shows in nothing the client receives, so a player
ghosted before the chase began is not caught). The map's teleporters, a spawn after dying,
spectating or following someone, and an admin moving the player (`amtele <player> ...`, a
bring) are not cheats; nor is the player's own `amtele <slot> ...` moving someone else. A
hunter caught is out as one killed is (`cheat`; the others read "is out for cheating").
Out of a hunt (resting, no mark), teleports are free. Before 03/11/2026 any flip of the bit
counted unless the player landed within 128 units of a point the map names for its
teleporters or a spawn point, which missed teleporters a server adds and every admin
teleport.

**The mark blows up** (Sol, 02/11/2026): a mark caught cheating, or typing in the chat
for 30 seconds straight (warned "Leave the chat!" at 20; their client watches it and says
`chat`), blows up and the hunt ends. A hunter still in the chase picked at random by the
hub (another hunter there when none is) wins the free mystery box roll killing the mark
would have, within the same limits (3 unspent, 5 a day); the mark loses no credits.
Everyone taking part
sees the box's explosion where the mark stands (when their game server sends the mark)
and hears it; the mark's own client sends the game's `kill`, as a box that blows up on
its opener does. The mark reads "BOOM!" and who gets the roll, the hunter picked "Free
mystery box roll" (or why not), the others a chat line. The card telling a player they
are the mark says so. Other away (a menu, the console, idle) still hands the mark on
after a minute, with nothing lost.

**The mark sees the hunters** (Sol, 02/11/2026): the mark's read of the hunt carries the
slots of the hunters still in the chase (`chasers`), and the mark's client rings them in
red as the hunters' clients ring the mark ([team_outline.rs](../crates/sjk-viewer/src/hud/team_outline.rs)).
A hunter out (the hub's `out_` event) loses the ring at once; one who joins the chase
gets it by the mark's next read (20 seconds at most). No label or arrow over them.

The hub never sees kills, so the mark's own client says who killed it
(`POST /v1/box-hunt/killed`, from the game's obituary): a hunter cannot claim a kill it
did not make, only the mark could fail to tell. It tells it once its player is the mark
by the hub's event, before its next read (31/10/2026; a kill in those first seconds went
untold), and tries twice. The hub counts a kill only before the mark's end, so a client
stops showing the mark a second before it, by the seconds left the hub's read says
(`left`; by the end time and its own clock for an older hub), and reads again then: it
showed the mark up to 20 seconds after their end before 31/10/2026, and a hunter who
killed them then won nothing (a player's report). A hunter keeps at most 3 free rolls and
wins at most 5 a day. The client reads the hunt (`POST /v1/box-hunt`) every 20 seconds
while the toy is on or a box stands on the map, and at once after the hub's toy events
(`box_hunt`: `mark_<slot>`, `hunted_<slot>_<won>`, and since 01/11/2026 `out_<slot>_dead`,
`out_<slot>_cheat`, `survived_<slot>_<credits>_<hunters>`, and since 02/11/2026
`blown_<slot>_<cheat|chat>_<to>_<won>` for the mark, `to` 255 for nobody).
All the numbers are the hub's, PROVISIONAL.

## Limits

- The hub cannot tell where a player stands: the reach is the client's.
- An outcome pays credits, blows the opener up or gives a holocron; other effects need a
  client release for each new kind.
- The explosion is SJK's only: no game server knows of it, so it hurts nobody else, and
  players on other clients see the opener die (the server's `kill`) with no box or
  explosion. A server that refuses `kill`, or counts it against the score, does so.
- Three friends with the toy on could take turns being killed for free rolls; the
  3-minute mark, the pause and the 5 a day cap how much that pays. The same friends could
  let the mark kill them for its credits: only the mark's kills count, and 3 wins a day
  cap it (at most 300 x hunters a day). A mark blowing up on purpose hands a friend a
  free roll, as letting them kill it would: the same 3 unspent and 5 a day cap it.
- The chat watch is the mark's own client, as the cheat watch is: a modified client could
  keep quiet.
- Teleports and noclip are seen by the player's own client: a modified client could keep
  quiet. A teleport during a long lag spike can be missed (two flips of the bit between
  snapshots). A map teleporter taken within 5 seconds of the player's own teleport command
  counts as a cheat, and a teleport command a mod names otherwise than `OWN_MOVES` does not
  (one more entry each).
- Away is what the player's own client says: a modified client could say it is in play.
  The client of 31/10/2026 sends `away`, which a hub before that day refuses (its read
  takes no other field): the hub goes first.
- The arrow shows the mark only while the game server sends them, which it does by what
  the player could see (the PVS), not across the whole map.
- Verified: unit tests (the pick of the faced box, which boxes stand on a map file and
  server, the roll slowing to its prize, the stop, the take and the take after 10
  seconds, the lid, the words over a box, the hunt's words and reads, the result lines,
  the mark by the event before a read, the cheat watch (the player's own teleport, not the map's or an admin's, the `amtele` forms, noclip asked or forced, fake noclip, a spawn, a new map), the out and outlasted words and card, the blown words, the chat warning and limit, the hunters ringed for the mark, the mark's end by the hub's seconds left or its
  clock; on the hub, the mark picked among hunters in play only and 3 of them, the away
  mark handed on after a minute (or kept with too few others), the kill counted only
  from the mark and for a hunter still in the chase, a hunter out by the mark's kill or a cheat, the mark's win once all are out (not for cheaters alone, nor past 3 a day, nor held up by one who left), a hunter's 5 wins a day counted from 09:00 in Paris, a box's price going to a hunter in play first, the mark's cheat or chat blowing it up and a hunter in the chase winning a free roll (within the limits), the mark alone reading its hunters, the pause, the free roll spent; on the hub, the odds, placing and
  removing as the creator only, the price, the roll, the cooldown and another server's
  box). Not tried in a game; the hub side is deployed (27/10/2026).
