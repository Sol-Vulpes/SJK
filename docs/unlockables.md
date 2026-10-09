# Unlockables and looks

Unlockables are cosmetic things an SJK player owns at the hub (`cl_hubUrl`,
sjk.dfox.app): the first is the **Sun blade**, a saber blade skin with its own
look and sounds. Their art is SJK's own and not in this repository: the hub delivers it
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

## Decisions

| Question | Decision |
| --- | --- |
| Who sees a look | Every SJK player on the same game server, through the hub. |
| Who may wear an unlockable | Only a key the hub lists it for. The hub refuses to relay a look whose blade skin the key does not own (`not_unlocked`). |
| How one is unlocked | The hub operator or staff grant it by hand for now (Staff page, operator command). Achievements or medals granting them are planned, not built. |
| How a look travels | Stored on the key's live claim, listed in presence for players who arrive later, and sent through the feed as a live event when it changes. |
| Illuminate | Free for everyone; its lit state is part of the look so others see the holocron. |
| Feed with chat off | The feed reads while the player is on a game server whatever `cl_sjkChat` says (chat stays hidden); in the menus it reads only with chat on. |
| Choosing | A new Unlockables page (SJK UI look) lists every unlockable, owned or locked, and equips them; `cg_saberSkin` holds the choice. |
| Art (looks, sounds) | SJK's own, all rights reserved, not under the code's GPLv2: kept in the hub's private repository and served by the hub to registered SJK keys as packs. The code that draws it stays open and generic: no skin's values, sounds or pictures are in this repository or its history to come. |
| Delivery | The identity service downloads the hub's packs into `assets/` beside `identity.key` (at start once registered, then every 6 hours, only when a pack's SHA-256 changed); the viewer mounts every cached pack at start and a new one at once, below the game data, so a PK3 with the same paths replaces their files. A cached pack mounts with the identity off too. |
| Format | A blade skin is a JSON file, `skins/blades/<unlock id>.bladeskin`, holding every parameter of the generic shading, its trail, light and sound paths. The format is documented here; the files are art. |

## Catalogue

Fixed, in the hub (`src/unlocks.rs`) and in the client
([unlockables.rs](../crates/sjk-viewer/src/unlockables.rs), the one client list of
public metadata: id, name, kind, a line on what it is and how to get it; the looks, the
blade skins, `saberskin`, the Unlockables page and the Staff page all read it); a new
unlockable needs both, and a blade skin also its file in a pack. Ids never change
meaning.

| Id | Name | Kind | How to get it |
| --- | --- | --- | --- |
| `saber_sun` | Sun blade | blade skin | Given by the SJK team. |

A client ignores an id it does not know.

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
- `unlockables` opens (or closes) the Unlockables page.

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
An unknown skin id draws the stock blade. The local player's own look comes from its
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

### Blade skins

Built (renderer and sounds); who wears which is the viewer's per-client table, filled
from the looks.

- A blade skin's look is its blade-skin file, loaded from the packs
  ([saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs) `LoadedSkins`, at most
  8): drawn as its own saber material after the neutral RGB pair, with its glow/core
  pair (the pack's images or generated from the file's profiles), coloured and animated
  in `saber.wgsl` from per-instance time, a per-blade seed and the file's parameters
  (core, corona gradient, granulation, flame tongues, shimmer, flares); the dynamic glow
  pass gets the same animation ([rendering.md](rendering.md#saber-blade-skins)). Its
  trail and light colours and the light's flicker are the file's. Retail and RGB blades
  are unchanged. A skin whose pack is not loaded (no hub yet, a client offline that never
  had it) is the stock blade, sounds too.
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
  swings replace the stock ones, at the paths the file names in its pack. The viewer
  passes the table to the audio adapter every frame
  (`LegacySoundAdapter::set_saber_sound_overrides`, [client.md](client.md#blade-skins)),
  so other players' sounds follow their looks too; a pack arriving mid-session has its
  sounds registered at once. A thrown saber hums the skin's hum as well: the saber
  entity's own `loopSound` is replaced while its owner (`genericenemyindex`) wears a
  skin.
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
| `tongues` | Noise at `(along × along + seed × offset, out × out − time × speed)`, from `edge_low` to `edge_high` capsule radii out, brightness `low + range × noise` | edge 0-2, low < high |
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

### Unlockables page

Built ([unlockables_panel.rs](../crates/sjk-viewer/src/unlockables_panel.rs), drawing
in [unlockables_panel_view.rs](../crates/sjk-viewer/src/unlockables_panel_view.rs) and
[unlockables_swatch.rs](../crates/sjk-viewer/src/unlockables_swatch.rs); layout in
[sjk-ui.md](sjk-ui.md#sjks-pages)). An SJK page in the SJK UI's look in every menu
style, as the Staff and Profile pages, opened by the Profile page's See unlockables
and the `unlockables` command:

- a card per catalogue entry: a live swatch (a blade skin drawn with the UI's shapes
  from its loaded file's colours, flicker and flares, moving: a breathing corona, flame
  loops and granules, a flare running hilt to tip; grey and still under a padlock when
  locked; owned but with its pack not loaded yet, a neutral still blade and "Its look
  downloads from the SJK hub"), its kind, name and what it is,
  owned (`Yours since dd/mm/yyyy, from the SJK team` and the team's note) or locked
  (how to get it), and Equip or Unequip for an owned blade skin, which sets
  `cg_saberSkin`;
- what the player wears and how unlockables work on the right;
- with the identity off, no hub or no answer yet, the page says so ("Unlockables need
  the SJK identity") and the cards are neither owned nor locked.

The Staff page lists every unlockable for the player found, with Unlock (sending the
note field's text) and Relock ([identity.md](identity.md#staff)).

Verified: unit tests (keys, pointer, focus, equip, every state fitting at 1080p, 4K,
4:3 and 21:9, the swatch moving and staying inside its frame, the Staff page's Unlock
and Relock) and the world shots `duel6_sjk_unlockables` (owned and worn, locked,
identity off, 4:3), `duel6_sjk_profile` and `duel6_sjk_staff`.

## Planned, not built

Achievements and medals granting unlockables, more blade skins, other kinds of
unlockable (holocron skins, trails, emotes).
