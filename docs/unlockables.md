# Unlockables and looks

Unlockables are cosmetic things an SJK player owns at the hub (`cl_hubUrl`,
sjk.dfox.app): saber blade skins with their own look and sounds, the **Sun**,
**Storm**, **Void**, **Frost** and **Prism** blades. Their art is SJK's own and not in
this repository: the hub delivers it
to SJK clients in packs ([Packs](#packs)), and the client draws it with open, generic
code from data ([Blade-skin files](#blade-skin-files)). A player's *look* (the blade skin they wear and whether their
Illuminate holocron is lit) travels through the hub, so every SJK player on the
same game server sees it. Players on stock clients see nothing, and no game
server knows of any of it.

Status (08/10/2026): designed with Sol the same day and built on
`personal/saber-skins` (client, stacked on `personal/sjk-chat`) and
`feat/unlocks-looks` (Sol-Vulpes/SJK-hub, stacked on `feat/chat-emotes`). The
hub's side is not deployed, so nothing has been seen with real players yet. Each
section below says what is built and how it was verified.

09/10/2026, branch `feat/more-blade-skins` (stacked on `personal/blade-skins`, with the
hub's `feat/more-blade-skins`): a skinned blade's tip is round instead of square
([Blade skins](#blade-skins)); the generic shading gained lightning arcs, drifting motes
and a turning hue, and the catalogue the Storm, Void, Frost and Prism blades. Checked
by unit tests and a CPU copy of the shader's maths rendered to images; not yet seen on a
GPU or heard.

10/10/2026, branch `personal/saber-shaders` (with the hub's `feat/saber-shaders`): nine
more saber shaders (Unstable, Molten, Spectral, Glitch, Hologram, Runic, Chameleon,
Banner, Heartbeat) and the generic sections they use ([Blade-skin
files](#blade-skin-files), [rendering.md](rendering.md#saber-blade-skins)); up to 16
skins load at once. The Collection's rack and the Saber tab's blade row scroll, as
fifteen choices no longer fit. Seen in off-screen GPU renders on duel6 (world shot
`duel6_second_blades`); not in a live match, not heard.

## Decisions

| Question | Decision |
| --- | --- |
| Who sees a look | Every SJK player on the same game server, through the hub. |
| Who may wear an unlockable | Only a key the hub lists it for. The hub refuses to relay a look whose blade skin the key does not own (`not_unlocked`). |
| How one is unlocked | The hub operator or staff grant it by hand (Staff page, operator command), and since 10/10/2026 a medal brings one while held: Bug Hunter the Glitch, Early Tester the Hologram, Early Contributor the Runic blade ([Medal shaders](#medal-shaders)). Achievements granting them are an idea for later, with harder goals than today's; holocrons dropping them are parked until Sol decides the pool and odds. |
| How a look travels | Stored on the key's live claim, listed in presence for players who arrive later, and sent through the feed as a live event when it changes. |
| Illuminate | Free for everyone; its lit state is part of the look so others see the holocron. |
| Feed with chat off | The feed reads while the player is on a game server whatever `cl_sjkChat` says (chat stays hidden); in the menus it reads only with chat on. |
| Choosing | The Collection's Shaders tab (SJK UI look; the Unlockables page until 10/10/2026) lists every blade skin, owned or locked, previews the chosen one on the player's own saber and equips owned ones; the Profile screen's Saber tab offers the owned blade skins too (10/10/2026); `cg_saberSkin` holds the choice. |
| Art (looks, sounds) | SJK's own, all rights reserved, not under the code's GPLv2: kept in the hub's private repository and served by the hub to registered SJK keys as packs. The code that draws it stays open and generic: no skin's values, sounds or pictures are in this repository or its history to come. |
| Delivery | The identity service downloads the hub's packs into `assets/` beside `identity.key` (at start once registered, then every 6 hours, only when a pack's SHA-256 changed); the viewer mounts every cached pack at start and a new one at once, below the game data, so a PK3 with the same paths replaces their files. A cached pack mounts with the identity off too. |
| Format | A blade skin is a JSON file, `skins/blades/<unlock id>.bladeskin`, holding every parameter of the generic shading, its trail, light and sound paths. The format is documented here; the files are art. |

## Catalogue

Fixed, in the hub (`src/unlocks.rs`) and in the client
([unlockables.rs](../crates/sjk-viewer/src/unlockables.rs), the one client list of
public metadata: id, name, kind, a line on what it is and how to get it; the looks, the
blade skins, `saberskin`, the Collection and the Staff page all read it); a new
unlockable needs both, and a blade skin also its file in a pack. Ids never change
meaning.

| Id | Name | Kind | How to get it |
| --- | --- | --- | --- |
| `saber_sun` | Sun blade | blade skin | Given by the SJK team. |
| `saber_storm` | Storm blade | blade skin | Given by the SJK team. |
| `saber_void` | Void blade | blade skin | Given by the SJK team. |
| `saber_frost` | Frost blade | blade skin | Given by the SJK team. |
| `saber_prism` | Prism blade | blade skin | Given by the SJK team. |
| `saber_unstable` | Unstable blade | blade skin | Given by the SJK team. |
| `saber_molten` | Molten blade | blade skin | Given by the SJK team. |
| `saber_spectral` | Spectral blade | blade skin | Given by the SJK team. |
| `saber_glitch` | Glitch blade | blade skin | Comes with the Bug Hunter medal, for as long as you hold it. |
| `saber_hologram` | Hologram blade | blade skin | Comes with the Early Tester medal, for as long as you hold it. |
| `saber_runic` | Runic blade | blade skin | Comes with the Early Contributor medal, for as long as you hold it. |
| `saber_chameleon` | Chameleon blade | blade skin | Given by the SJK team. |
| `saber_banner` | Banner blade | blade skin | Given by the SJK team. |
| `saber_heartbeat` | Heartbeat blade | blade skin | Given by the SJK team. |

A client ignores an id it does not know. Both catalogues' tests pin this exact list
(ids, names, order), so a change to one fails until the other follows.

What each looks and sounds like, in words (the values and the sounds themselves are
the hub's, described in its `scripts/blade_skin_sounds.py` and `saber_skin_sounds.py`):

- **Sun**: a white-gold core in an orange corona graded from red at its rim to gold
  inside, granules drifting along it, flame tongues licking out of its edge and, now
  and then, a bright flare running from the hilt to the tip; a warm flickering light;
  a solar roar.
- **Storm**: a white-blue core in a violet-to-electric-blue corona that crackles; a few
  thin, jagged white-blue lightning arcs at a time, struck again and again at random
  places, leaving the blade, bulging out to one side and coming back further along,
  crawling toward the tip and re-shaping as they fade; some leap from the tip into the
  air instead; white sparks spray from round the tip; a fast-flickering light; an
  electric buzz with sizzle and crackle.
- **Void**: a hollow blade: the inside of its glow is black (nothing is added there, so
  the world shows through), its rim violet, with a faint deep-violet line for a core
  and a dark round mouth at the tip; slow tendrils licking inward and rare slow magenta
  surges; pale star specks drift in from outside and fade as they reach it; a low,
  slowly beating drone.
- **Frost**: a pale ice-white core ending in a long pointed icicle tip, in an icy blue
  corona with a soft cold haze; faint crystalline veins by the core that re-form now
  and then; thin frost shards glinting as they trail toward the hilt and round the tip;
  a soft hum under shimmering bell tones and tiny ice tinkles.
- **Prism**: a white core whose fringe and corona run through the rainbow along the
  blade, the spectrum sliding slowly with time and the rim a different hue from the
  inside (refraction bands), sharp bright bands drifting along it and coloured
  sparkles; its light turns through the colours with the blade's middle; a chord in
  detuned voices under a slowly sweeping resonance.
- **Unstable**: a raw red blade whose glow's edge is frayed by fast noise and whose
  length now and then falters (cut short for an instant), sparks spitting off it, a
  hard-flickering light; a torn, stuttering buzz under dense crackle.
- **Molten**: a dim, dark-red core split by bright orange lava cracks (veins) running
  through the core and the inside of the glow, embers dripping off it and falling as
  gravity pulls them, whichever way the blade is held; a bubbling, sizzling rumble.
- **Spectral**: a pale, see-through blue-white blade with a wide soft glow and faint
  wisps; swinging, it leaves up to four fading afterimages of its glow where it was;
  an airy pad under a whisper, with echoes.
- **Glitch**: a teal blade whose red and blue part from its green; on random draws
  blocks of it jump sideways and flash, now and then the whole blade; a gated buzz with
  digital blips.
- **Hologram**: a hollow blue projection drawn as a wireframe (two side lines and rings
  round the blade) with scan lines running along it, jittering sideways now and then;
  a clean tone, a projector whine and a scan tick.
- **Runic**: a dim gold core with glyphs scrolling toward the tip that spell its
  wearer's name (SJK's own stroke glyphs, one per letter); a chord of fifths with bell
  glints.
- **Chameleon**: a pale blade that takes the colour of the light where it is (the map's
  light grid, saturation raised), so it changes from place to place and map to map; a
  body whose resonance slowly morphs.
- **Banner**: its wearer's team colour in team games (red, blue), silver outside a team,
  rippling like a flag; a warm buzz and fluttering cloth.
- **Heartbeat**: a warm crimson blade that beats with a slow double heartbeat (lub-dub,
  once a second), brightening and swelling with each beat, its light beating with it;
  a drone with a heartbeat.

## Hub protocol (additions to version 1)

Old clients ignore all of it, so it stays `/v1/`. The hub's `PROTOCOL.md` gains an
"Unlocks" and a "Looks" section with exactly this.

### Unlocks

- Profile gains `"unlocks":[{"id":"saber_sun","granted":<unix s>,"note":".."}]`, in
  the catalogue's order; `note` is 0 to 200 characters of plain text from the team,
  often empty. Absent from older hubs.
- A key holds each unlock at most once. Only the operator (`unlock <key_id> <id>
  [note]` and `relock <key_id> <id>` commands, and the operator API beside medals)
  and staff can grant or take one back:
  - `POST /v1/staff/unlock` (signed, staff) `{"key_id":"..","unlock":"..","note":".."}`
    answers the target's Profile. An unknown id is 400 `bad_unlock`, a key already
    holding it 409 `already_unlocked`, an unknown key 404 `not_found`.
  - `POST /v1/staff/relock` (signed, staff) `{"key_id":"..","unlock":".."}` answers the
    Profile; a key not holding it is 404 `not_unlocked`.
  - Both are written to the staff log and count against the staff limits.
- Taking a blade skin back clears it from that key's live looks at once (and sends a
  look event, below), so it disappears for everyone.
- Removing an identity removes its unlocks.

### Looks

A look is what a player wears that others draw:

    {"saber":"saber_sun","illuminate":true}

`saber` is a blade-skin unlock id or `""` for the stock blade; `illuminate` is
whether the Illuminate holocron is lit. Both are required in a request.

- `POST /v1/look` (signed) `{"server":"ip:port","saber":"..","illuminate":false}`.
  Exactly those fields (400 `bad_body` otherwise). `server` as in claims (400
  `bad_server`). The key must hold a live claim on `server` (403 `not_on_server`);
  the hub takes the slot and claimed name from it. A `saber` that is not `""` must be
  a blade skin in the catalogue (400 `bad_look`) that the key holds (403
  `not_unlocked`). At most one look a second per key and 30 a minute (429
  `look_quota`; a refused one does not count). Looks share the chat and emotes
  per-address allowance. Answers `{"id":<n>}`, the feed id of its event.
- The look is kept on the claim. Renewing the claim for the same server, slot and
  name keeps it; a claim for another server, slot or name, a release or the claim's
  expiry drops it (a new claim starts with no look).
- Presence entries gain `"look":{"saber":"..","illuminate":false}` when the claim
  has one (absent otherwise).
- The feed gains `looks`, sharing the id sequence: the look events on the reader's
  `server` with an id above `after`, oldest first, kept for 60 seconds. Each:

      {"id":15,"at":<unix s>,"slot":3,"claimed_name":"^2Sol","key_id":"..",
       "saber":"saber_sun","illuminate":true}

  A batch with looks is not empty. `after` 0 or above the newest id (a new
  reader, or a hub that restarted) gives the server's looks of the last 60 seconds:
  looks are state, so a reader's first answer must not miss one, and applied oldest
  first they leave each slot with its latest look.
- A client applies a look, as emotes and badges, only when the name its game shows in
  `slot` matches `claimed_name`.

## Client

### Settings and commands

Built.

- `cg_saberSkin` (archived, default empty; one registration, the constant
  `unlockables::SABER_SKIN_CVAR`): the blade-skin unlock id the player wears. It
  applies only while the player's own hub profile lists that unlock; with no
  identity, no hub, the hub not answered yet or the unlock missing, the stock blade
  shows and nothing is sent. The looks read it twice a second and gate it
  (`Looks::own_saber_skin`); the renderer takes the gated skin from there
  (`GpuState::local_saber_skin`, [saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs)),
  for the hand, first person, a thrown saber, the sounds and the Character page's
  preview.
- `saberskin` ([saber_skin_command.rs](../crates/sjk-viewer/src/saber_skin_command.rs))
  alone lists the blade skins: id, name, owned (since when) or locked (how to get
  it), which one is worn or chosen, and why ownership is not known when it is not
  (identity off, no hub, waiting). `saberskin <id>` (case ignored, stored as the
  catalogue's id) or `saberskin none` sets `cg_saberSkin`; a locked skin is set all
  the same and the answer says it shows only once unlocked. An unknown id is refused.
  Tab completes the command's name; its argument has no completion (the console
  completes command and cvar names only).
- `unlockables` opens (or closes) the Collection on its Shaders tab: in the SJK UI
  the Collection screen ([sjk-ui.md](sjk-ui.md#collection)).

Verified by unit tests (the listing, the answers, the cvar set even when locked, the
gate); not yet used against the deployed hub.

### Sending

Built ([service.rs](../crates/sjk-identity/src/service.rs),
[looks_frame.rs](../crates/sjk-viewer/src/looks_frame.rs)). Twice a second the viewer
computes the own look: `cg_saberSkin` when the client knows that blade skin and the own
profile lists it (else `""`), and whether the local Illuminate is lit; it hands it to
`Service::set_look` only when it changed. The identity worker keeps the latest. After
its claim on a server is accepted, and whenever the look changes, it sends
`POST /v1/look`, at most one a second (changes in between are coalesced, the latest
wins); a new claim (another server, slot or name) sends it again, and renewing the
same claim does not. What the hub holds is known (none for a new claim or after a
release, else the last look it took) or unknown: at the worker's start (a client that
stopped without releasing may have left a live claim with a look), after a failed claim
(it may have lapsed, or still be live with the old look) and after a failed release.
A look equal to none (stock blade, holocron out) is not sent on a claim known to hold
none; while unknown, the next accepted claim gets the look whatever it is. A
`not_unlocked` (or `bad_look`) answer leaves that skin out, the look going on with
`saber:""` so Illuminate still syncs, until the profile's unlocks change. The own
profile is read every ten minutes, and sooner (at most once every 30 seconds) after a
`not_unlocked`, or when the feed relays a look of the player's own key, newer than the
last one sent, with another blade skin (staff took it back; `Service::take_looks`
hands it to the worker), so a relocked skin leaves the player's own blade within
seconds; too many
(429: `look_quota`, or the per-address `rate_limited` the chat and emotes share), a
refused signature (401) and failures wait 10 seconds and go again; `not_on_server`
waits for the next accepted claim;
another refusal (an older hub) is not repeated until the look or the claim changes.
Leaving the server sends nothing (the release drops the look).
`Snapshot::look_outcome` holds what became of the last one. Unit tests against a fake
hub, and end to end (`hub_e2e`) against the hub's unlocks work in progress run on this
PC: a look needs a claim, shows in presence and the feed, survives the claim's renewal
but not another slot, a skin the key lacks is refused until granted, and the service
wears its look (without a skin it lacks) and reads looks with the chat off.

### Receiving

Built ([feed.rs](../crates/sjk-identity/src/feed.rs),
[looks.rs](../crates/sjk-viewer/src/looks.rs)). The feed reads on a game server
whatever `cl_sjkChat` says, keeping no message with the chat off, and queues the
looks it gets (`Service::take_looks`, the newest 64). Another game server is read
from `after` 0, so its first answer brings that server's looks of the last minute.
Each queued look keeps the server it was read for and the reading's generation, which
changes with the hub, the server or the identity going off or on; the viewer is
handed only the looks read for the server it is on under the current generation, and
clears every hub look (reading the roster again) when the generation changes, so a
poll still under way at a change cannot bring an old look back. `GpuState::looks` keeps one look
per slot (64): the presence roster's when its revision changes (it replaces every hub
look) and feed events as they come (newer, so they win until the roster changes
again). A look counts only while the game shows the claimed name in its slot (the
badges' rule); the names are compared when a roster or an event comes and twice a
second, so a frame only reads a fixed table. Another server, leaving or a new feed
generation clears them.
An unknown skin id draws the stock blade. A player muted on this PC
([hub-chat.md](hub-chat.md#muting-a-player)) wears nothing while muted: the stock
blade without the skin's sounds, no holocron (`Looks::set_muted`, which never mutes
the own slot). The local player's own look comes from its
settings, not the hub.

What the renderer reads, on `GpuState::looks`:

- `saber_skin_id(client) -> Option<&'static str>`: the blade skin the player in that
  slot wears (a catalogue id), `None` for the stock blade; the local player's slot
  holds its own gated skin.
- `revision()`: changes whenever what a slot wears (or the own look) changes, so the
  blade skins' table is filled again only then.
- `illuminated(client) -> bool`: their holocron is lit.
- `own_saber_skin() -> Option<&'static str>`: the local player's gated skin, in a game
  or not (first person, the Character page's preview).

The player card ([client.md](client.md#player-card)) names the shader a player wears,
with its live swatch, by the same rule: the looks' skin for another slot, the own gated
skin for the local player's, and only when its pack is loaded, so the card never names
a shader their blade does not show here (10/10/2026).

### Blade skins

Built (renderer and sounds); who wears which is the viewer's per-client table, filled
from the looks.

- A blade skin's look is its blade-skin file, loaded from the packs
  ([saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs) `LoadedSkins`, at most
  16): drawn as its own saber material after the neutral RGB pair, with its glow/core
  pair (the pack's images or generated from the file's profiles), coloured and animated
  in `saber.wgsl` from per-instance time, a per-blade seed and the file's parameters
  (core, corona gradient, granulation, flame tongues, shimmer, flares and, when the
  file has them, lightning arcs, drifting motes and a turning hue); the dynamic glow
  pass gets the same animation ([rendering.md](rendering.md#saber-blade-skins)). Its
  trail and light colours and the light's flicker (and hue, when it turns) are the
  file's. Retail and RGB blades are unchanged. A skin whose pack is not loaded (no hub
  yet, a client offline that never had it) is the stock blade, sounds too.
- A skinned blade ends round (09/10/2026; Sol saw the Sun's tip square). The core line
  is a flat quad from behind the hilt to the tip, and the generated core's fringe is
  still about a fifth of its brightness at the quad's edges and end, so its end and
  corners showed as a bright square, plainest with a wide, bright fringe like the Sun's
  over a tip glow half as bright as the shaft's (stock blades draw the same flat line,
  but their white core is hidden in their glow). Past the tip, too, the corona's
  grading and its flame tongues were taken from the distance across only, so they ran
  on straight beyond the tip as a column, and a widening corona widened only sideways.
  Now, for skins only: over its last `core.tip` half-widths the core line narrows on a
  quarter circle to a rounded point and is cut (softly, over a pixel) at that edge;
  past the tip the corona widens about the tip as about the shaft, its distance out is
  measured from the tip, and the tongues run on round it. Along the shaft nothing
  changed. Pinned by a test of a CPU copy of the tip's maths (the shaft unchanged, the
  quarter circle, the corners cut, the glow round past the tip) that also checks the
  shader holds the same expressions.
- Shown for every client whose entry in the viewer's `SaberSkins` table is set: in the
  game, in first person, thrown, and on the menu stage (the Character page) for the
  local player. Once a frame (`GpuState::sync_saber_skins`) the table takes every
  other slot's skin from `Looks::saber_skin_id` when the looks' revision or the loaded
  skins changed (no allocation; 32 lookups only then), and the local player's own from
  the gated `Looks::own_saber_skin` in its slot (also with no session, for the
  preview). That slot is the game state's `client_num`, not the snapshot's player
  state's: following (spectating) someone, the state is theirs (`PMF_FOLLOW`), and they
  keep their own hub look, blades, thrown saber and sounds alike (`looks::ViewSlots`).
- Sounds, per player wearing it: ignition and switching off, the hum loop and three
  swings play over the stock ones, which keep playing below (each on a channel of its
  own, so the skin's does not cut them), at the paths the file names in its pack. The viewer
  passes the table to the audio adapter every frame
  (`LegacySoundAdapter::set_saber_sound_overrides`, [client.md](client.md#blade-skins)),
  so other players' sounds follow their looks too; a pack arriving mid-session has its
  sounds registered at once. A thrown saber hums the skin's hum as well: over the saber
  entity's own `loopSound`, while its owner (`genericenemyindex`) wears a skin.
- Verified: unit tests against a made-up test skin (material slots, the uniform's
  layout and values, colours and flicker, the table following the looks, the gated own
  skin and the loaded skins, loading from a pack with images and sounds, a runtime
  mount, the sound overrides per client and registered mid-session, a thrown saber's
  hum), `saber.wgsl` validated by naga, and the `duel6_sun_blade` world shot with the
  hub's pack built locally (`SJK_TEST_PACKS`), compared with shots of the earlier
  built-in Sun. Not verified: the sounds by ear, a live match, another player's skin
  from a real hub, a pack downloaded from the deployed hub.

### Packs

Built ([sjk_packs.rs](../crates/sjk-viewer/src/sjk_packs.rs); the download is the
identity service's, [identity.md](identity.md#asset-packs)). A pack is a PK3 of game
paths the hub serves (`GET /v1/assets`, `GET /v1/assets/<name>`); today one,
`sjk_skins`, with the Sun blade's file and sounds.

- The identity service keeps them in `assets/<name>.pk3` beside `identity.key`,
  checked against the hub's size and SHA-256 and written in one rename.
- At start the viewer mounts every cached pack (`sjk_packs::mount_at_start`), whatever
  `cl_identity` says: the content is SJK's own and already on the machine. Its
  blade-skin files load into `LoadedSkins` under a new generation; each frame
  `GpuState` compares one atomic counter and, when it changed, uploads the skins'
  parameters and pairs and registers their sounds.
- Twice a second the viewer compares the service's count of packs written; when it
  changes the folder is mounted again, so a pack that arrives mid-session shows (and
  sounds) at once, for the local player and everyone wearing it. What the last check
  did is written to the log.
- Every game file system built for a world mounts the cached packs below all game data,
  as the holocron's files are.

### Blade-skin files

`skins/blades/<unlock id>.bladeskin` in a pack, a JSON object
([blade_skin_file.rs](../crates/sjk-viewer/src/blade_skin_file.rs)). Parsing is strict:
an unknown or missing field, a wrong type or a value outside its range refuses the file,
and the log names the file and the field. A file whose id is not a blade skin of the
client's catalogue is left out (a newer pack may hold skins an older client does not
know). `grain` below is the granulation in [0, 1], `flare` the flares' sum at a point.
`arcs`, `motes` and `hue` are optional sections (09/10/2026), and so are `sputter`,
`glitch`, `scan`, `pulse`, `ghosts`, `embers`, `veins`, `team`, `ambient` and `glyphs`
(10/10/2026, [blade_skin_effects.rs](../crates/sjk-viewer/src/blade_skin_effects.rs)): a
file without one draws none of it (its lanes of the uniform are zeros), so files written
before them draw as they did; when present, every field of the section is required.
`team` and `ambient` may not both be given.

| Field | Meaning | Range |
| --- | --- | --- |
| `version` | Format version | 1 |
| `glow_profile` | Generated glow: `width`, `peak`, `tail_width`, `tail_peak` (a Gaussian and an optional wider tail, in half-widths); default the neutral pair's | 0.05-1, 0-1, 0.05-2, 0-1 |
| `core_profile` | Generated core: `width` (hot centre, red), `fringe_width` (green); default the neutral pair's | 0.05-1, 0.05-1.5 |
| `glow_image`, `core_image` | Optional images in the pack (PNG, TGA, JPEG, at most 1024 a side) instead of the profiles | game paths |
| `core.white` | Hot centre colour; `white_flare` how much a flare brightens it | 0-4; 0-10 |
| `core.fringe_cool`, `fringe_hot` | Fringe colours, mixed by `fringe_heat` `base + grain × grain` | 0-4 |
| `core.fringe_brightness` | `base + grain × grain + flare × flare` | base 0-10 |
| `core.breathe` | The core's width breathing: `amount × sin(time × rate + along × along + seed × seed)` | amount 0-0.5 |
| `core.tip` | Optional: the rounded tip's length in core half-widths, over which the line narrows on a quarter circle to a point; default 1.5 | 0.5-8 |
| `corona.rim_cool`, `rim_hot`, `inner` | Rim colours (mixed by `rim_heat` `grain × grain + flare × flare`) and the inside colour | 0-4 |
| `corona.inner_width` | How far out the inside colour reaches, in capsule radii | 0.01-1 |
| `corona.inner_mix` | The inside colour's share: `inside² × (base + grain × grain) + flare × flare` | |
| `corona.brightness` | `base + grain × grain + flare × flare` | base 0-10 |
| `corona.reach` | Widest the glow gets, in stock capsules | 1-2 |
| `corona.swell` | How much `grain` and `flare` widen the glow | -2-2 |
| `granulation.octaves` | 1 or 2 octaves: noise at `(along × scale − time × speed, time × evolve + seed × offset)`, summed by `weight` | weight 0-1 |
| `granulation.low`, `high` | The sum's contrast (smoothstep) | 0-1, low < high |
| `flares` | `count` tracks; track n's rate `rate + rate_step × n + rate_seed × seed`, phase offsets `phase_seed`, `phase_step`; lit when its draw ≥ `threshold`; runs from `overshoot` units before the hilt to past the tip; `size + size_jitter × random` long | count 0-8, threshold 0-1 |
| `shimmer` | Up to 2 waves (as `breathe`) widening the corona | amount 0-0.5 |
| `tongues` | Noise at `(along × along + seed × offset, out × out − time × speed)`, from `edge_low` to `edge_high` capsule radii out, brightness `low + range × noise`; past the tip `out` is the distance from the tip and `along` runs on round it | edge 0-2, low < high |
| `arcs.count`, `color`, `brightness` | Lightning arcs at once, their colour and brightness | 0-4, 0-4, 0-10 |
| `arcs.width`, `halo` | A filament's half-width (units; one thinner than a pixel widens to it and dims) and its soft halo's share (four widths wide) | 0.02-1, 0-2 |
| `arcs.reach`, `jag`, `kinks` | How far an arc bulges out and how far its corners zigzag (capsule radii), and corners a unit along: piecewise-linear noise of two octaves | 0-2, 0-1, 0.05-4 |
| `arcs.span` | `low`, `high`: a strike's length, a share of the blade | 0.05-1, low ≤ high |
| `arcs.rate`, `threshold`, `jitter` | Strikes a second per arc, each at a new random place and side (a strike shows when its draw ≥ `threshold`), re-shaped `jitter` times a second | 0.05-60, 0-1, 0-120 |
| `arcs.tip` | The share of strikes that leap from just below the tip into the air (dying out on the way) instead of coming back to the blade | 0-1 |
| `arcs.crawl`, `decay` | Units a second a strike crawls along the blade (negative: hiltward); its light `(1 − age)^decay` over the strike (0 holds it) | -100-100, 0-8 |
| `motes.color`, `brightness` | Motes' colour and brightness | 0-4, 0-10 |
| `motes.density`, `cells`, `rings` | A field of cells, `cells` a unit along and `rings` a capsule radius out (each column staggered), `density` of them holding a mote | 0-1, 0.05-8, 0.5-16 |
| `motes.size`, `stretch` | A mote's half-width as a share of its cell, and how many times longer it is along the blade (`size × stretch` at most 0.5) | 0.02-0.5, 1-8 |
| `motes.drift` | `along` units a second toward the tip, `out` capsule radii a second outward (negative: inward); round the tip they flow round it | -100-100, -10-10 |
| `motes.twinkle` | About how many times a second each twinkles (0: steady) | 0-60 |
| `motes.inner`, `outer`, `focus` | Where (radii out) they appear and are gone; `focus` 1 shows them only toward and round the tip | 0-2, 0-2.5, 0-1 |
| `hue` | `rate` turns a second, `along` turns a unit along the blade, `out` turns a capsule radius out: the glow's and the core fringe's colours turned round the grey axis (brightness and saturation kept), and the light's with the blade's middle | -10-10, -1-1, -4-4 |
| `sputter.ragged`, `scale`, `speed` | How far noise frays the glow's edge (a share of its width), its cells a unit along and how fast it runs | 0-1, 0.05-8, -100-100 |
| `sputter.cut`, `rate`, `threshold` | The most of the blade a sputter cuts off (a share of its length); draws a second, a draw sputtering when its value is at least `threshold` | 0-0.5, 0-60, 0-1 |
| `glitch.split` | How far red and blue part from green, capsule radii (doubled in a glitch) | 0-1 |
| `glitch.blocks`, `rate`, `threshold` | A block's length (units); draws a second, a block glitching when its draw is at least `threshold` (the whole blade when a rarer draw passes halfway past it) | 0.5-40, 0.05-60, 0-1 |
| `glitch.shift`, `flash` | How far a glitched block jumps (radii) and how much it brightens | 0-1, 0-4 |
| `scan.color`, `brightness` | The hologram's wireframe colour and brightness | 0-4, 0-10 |
| `scan.lines`, `speed`, `depth` | Scan lines a unit along, how fast they run (units a second) and how deep they darken the glow and core | 0.05-8, -100-100, 0-1 |
| `scan.rings`, `edge`, `width`, `hollow` | Wireframe rings a unit along (0: none), where its side lines run and their half-width (radii), how much the inside is dimmed | 0-2, 0.1-1.5, 0.01-0.5, 0-1 |
| `scan.jitter`, `jitter_rate` | How far a jitter throws the projection sideways (radii) and draws a second (a quarter of them jitter) | 0-0.5, 0-60 |
| `pulse.rate`, `amount`, `second`, `gap`, `width`, `swell` | A heartbeat on the clock (no seed, so blades and their lights beat together): beats a second, how much a beat brightens, the second beat's share and seconds after the first (which peaks 0.1 s into a cycle), a beat's half-length (s) and how much it widens the glow | 0.1-4, 0-2, 0-1, 0.05-1, 0.01-0.5, 0-0.5 |
| `ghosts.count`, `spacing`, `fade` | Afterimages: the blade's pose kept every `spacing` ms, the last `count` drawn again as glow alone, each `fade` of the one before; a pose the blade has not moved from is skipped | 1-4, 10-250, 0-1 |
| `embers.color`, `brightness` | The embers' colour and brightness | 0-4, 0-10 |
| `embers.density`, `cells`, `fall`, `life` | The share of drip places dripping, places a unit along, how fast they fall (units a second, accelerating) and how long one lasts (s) | 0-1, 0.05-4, 1-400, 0.1-4 |
| `embers.size`, `spread` | An ember's radius (units) and how much its way strays sideways | 0.05-2, 0-1 |
| `veins.color`, `brightness` | The veins' colour and brightness in the glow's inside | 0-4, 0-10 |
| `veins.scale`, `speed`, `width`, `core` | Ridged noise cells a unit, how fast they run along, a vein's half-width (share of a cell) and how bright they show in the core | 0.05-8, -100-100, 0.01-0.5, 0-10 |
| `team.amount`, `red`, `blue`, `none` | The share of its colour the glow and core fringe take from the wearer's team: red, blue, or none outside a team | 0-1, 0-4 each |
| `ambient.amount`, `saturate`, `floor` | The share of its colour taken from the light where the blade is (the grid's ambient and half its directed light at its hilt end, at full brightness), how much its saturation is raised and each channel's least | 0-1, 0-4, 0-1 |
| `glyphs.color`, `brightness` | The glyphs' colour and brightness | 0-4, 0-10 |
| `glyphs.size`, `spacing`, `speed`, `width` | A glyph's height along the blade and the gap after it (units), how fast they scroll toward the tip and a stroke's width (a share of a glyph): the wearer's name, a letter each, a cell left empty between repeats | 1-20, 0-10, -100-100, 0.02-0.3 |
| `trail` | Blur trail colour | 0-1 |
| `light.color` | Dynamic light colour (stock gain) | 0-4 |
| `light.flicker` | `amount` and up to 2 `waves` (`rate`, `weight`, `phase` per hilt): brightness `1 − amount + amount × Σ weight × sin(t × rate + phase × hilt phase)` | amount 0-1 |
| `sounds` | `on`, `off`, `hum`, `swings` (three): game paths in the pack | 1-63 characters, relative, no `..` |

Other coefficients are any number from -1000 to 1000.

### Illuminate for others

Built ([illuminate.rs](../crates/sjk-viewer/src/illuminate.rs)). Every lit look puts a
holocron by that player's left shoulder, placed from their entity's interpolated
origin, eye height (from its box, so crouching lowers it) and view yaw, with the same
fade, bob, spin and trailing as one's own: the local holocron and one per other slot
step through the same code, each slot's bob a little out of step. The cube always
shows for another player while the game draws them; dead, hidden, cloaked or out of
the snapshot, it goes out where it was. Every cube shows, but only the four nearest
the camera add their warm light, so the frame's 32 lights stay for the weapons.
Following (spectating) another player, one's own holocron goes out and the followed
player's, if their look has it lit, floats by the view's eye as one's own does (their
entity is not in the snapshot), showing only its light in first person.
Unit tests and an off-screen shot on duel6 (two holocrons placed for remote players
without a session); not yet seen with real players, nor in follow mode.

### Shaders tab

Players read blade skins as saber **shaders** (10/10/2026, Sol's name, so body
shaders can follow): the player-facing words say "saber shader", the code and the hub
keep "blade skin" and the ids.

Built 08/10/2026 as the Unlockables page, the Profile screen's Collection tab in the
SJK UI; since 10/10/2026 the [Collection](sjk-ui.md#collection)'s Shaders tab
([collection_shaders.rs](../crates/sjk-viewer/src/collection_shaders.rs), its swatches
in [collection_swatch.rs](../crates/sjk-viewer/src/collection_swatch.rs)). An SJK page
in the SJK UI's look in every menu style; in the SJK UI the main page's and the
in-game menu's Collection open it, with the classic menus `unlockables` (and the
Profile page's See the collection) open it on its own:

- a rack, a row to each: the stock blade (in the player's `color1`), then every
  blade skin with a live swatch (drawn with the UI's shapes from its loaded file's
  colours, flicker and flares, moving: a breathing corona, flame loops and granules, a
  flare running hilt to tip, and the file's lightning arcs, motes and turning hue when
  it has them; grey and still under a padlock when locked; owned but with its pack not
  loaded yet, a neutral still blade), its name and its state (Worn, Yours, Locked);
- the player's model holding the chosen blade, on the menu map's stage or in the
  page's live preview over a match: a locked one too, as a preview this screen alone
  draws (`PreviewSkin`; nothing is set or sent);
- beside the model what the chosen one is, owned (`Yours since dd/mm/yyyy, from the
  SJK team` and the team's note, or `..., with your Bug Hunter medal` when a medal
  brings it) or how to get it, and Equip, Unequip or Wear the
  stock blade, which set `cg_saberSkin`;
- with the identity off, no hub or no answer yet, the tab says so and no skin is
  owned.

The Staff page lists every unlockable for the player found, with Unlock (sending the
note field's text) and Relock ([identity.md](identity.md#staff)), two to a row of 36
pixels (10/10/2026) so the whole catalogue (fourteen) fits above the keys, each with the
one button that applies.

Verified: unit tests (keys, pointer, the worn one chosen on opening, equip and unequip,
a locked one previewed but never equipped, every row and its words within the canvas
at 1080p, 4K, 4:3 and 21:9, the swatch moving and staying inside its frame, the Staff
page's Unlock and Relock) and the world shot `duel6_sjk_collection` (the Sun worn on
the stage, the Storm previewed, a 4:3 window, the in-game preview, the classic menus)
with the hub's pack; not tried in the game.

### Saber tab's blade choice

Built 10/10/2026 (Sol's request: choose an unlocked blade skin where the saber is
made), [player_menu/blade_skins.rs](../crates/sjk-viewer/src/player_menu/blade_skins.rs),
drawn by the Saber page ([sjk-ui.md](sjk-ui.md#character)). In the SJK UI the
Profile screen's Saber tab has a Blade row: the stock blade in the first blade's
colour, then every blade skin the player's own hub profile lists, in catalogue
order, each drawn as its Collection swatch shrunk (`swatch::small_blade`, the same
drawing scaled; the stock one `swatch::small_stock`). Locked skins are not listed
there, the Collection shows them; with the identity off, no hub, the hub not
answered yet or no skin owned, only the stock blade shows, with a line saying why.
What the player owns is read twice a second while the tab shows (and when the screen
opens), without copying the identity's state (`player_identity::with_snapshot`).

Picking a blade (a click, or Left and Right on the row) sets `cg_saberSkin` through
`unlockables::wear`, which the Collection's Equip and Unequip use too, by the same
rules (`Holdings::can_wear`, `Holdings::worn_blade_skin`, which the profile card
also reads); the stock blade sets it empty. A click on the blade already shown
chosen writes nothing, so a skin chosen before the hub answered stays chosen. The
stage model (or the live preview, opened from a game) wears the skin once the looks
read the setting again, within half a second, as after Equip. The classic menus are
unchanged.

Verified: unit tests (the stock blade and the owned skins offered in catalogue order,
locked and unknown ids left out, picking and stepping writing the setting, a pending
choice kept, the hint without skins or identity, a click on a swatch wearing it, the
page with every skin carrying every effect within the screen's canvas, Dual too) and
the world shot `duel6_sjk_saber_page` with the hub's pack (`SJK_TEST_PACKS`); not
tried in the game.

## Medal shaders

Built 10/10/2026, branch `personal/medal-blades` (with the hub's `feat/medal-blades`),
Sol's decision: Bug Hunter brings the Glitch blade, Early Tester the Hologram, Early
Contributor the Runic. The hub derives the unlock from the medal when it reads a profile
or a look (no grant row), lists it in the profile's `unlocks` with `medal` set to the
medal's id while no grant by hand covers it, and takes it back with the medal's last
award, clearing it off a live look with a look event. A grant by hand of the same blade
is separate and outlives the medal. The client ([wire.rs](../crates/sjk-identity/src/wire.rs)
`Unlock::medal`, absent from older hubs) wears it like any unlock (`Holdings::can_wear`
is unchanged); the Collection's Shaders tab says "Yours since dd/mm/yyyy, with your Bug
Hunter medal", its Medals tab adds "Comes with the Glitch blade, a saber shader." under
what the medal is for, and the three blades' "How to get it" names the medal
(`Medal::blade` in [medals.rs](../crates/sjk-viewer/src/medals.rs) pins the pairs, a
test checks the texts). Verified: unit tests (client and hub, the hub's covering award,
counted awards, revoke, a grant by hand surviving, relock and the look); the hub not
deployed when built, nothing tried in the game.

## Planned, not built

Achievements granting unlockables (with harder goals), holocrons dropping blade skins
(parked: pool, odds and duplicates undecided), other kinds of unlockable (holocron
skins, trails, emotes). Up to 16 blade skins load at once (the renderer's slots; the
uniform array is 13312 bytes of the 16 KiB WebGPU guarantees, so a seventeenth needs
fewer lanes per skin or a storage buffer first).
