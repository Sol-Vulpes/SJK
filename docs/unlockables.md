# Unlockables and looks

Unlockables are cosmetic things an SJK player owns at the hub (`cl_hubUrl`,
sjk.dfox.app): the first is the **Sun blade**, a saber blade skin with its own
look and sounds. A player's *look* (the blade skin they wear and whether their
Illuminate holocron is lit) travels through the hub, so every SJK player on the
same game server sees it. Players on stock clients see nothing, and no game
server knows of any of it.

Status (08/10/2026): designed with Sol the same day and being built on
`personal/saber-skins` (client, stacked on `personal/sjk-chat`) and
`feat/unlocks-looks` (Sol-Vulpes/SJK-hub, stacked on `feat/chat-emotes`). The
hub's side is not deployed. Sections below say what is built and what is
verified as each part lands.

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
| Sounds | Synthesized by `scripts/saber_skin_sounds.py`, bundled in the client and mounted below the game data, so a PK3 with the same paths replaces them. |

## Catalogue

Fixed, in the hub (`src/unlocks.rs`) and in the client
([unlockables.rs](../crates/sjk-viewer/src/unlockables.rs)); a new unlockable
needs both. Ids never change meaning.

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

  A batch with looks is not empty. `after` 0 or above the newest id gives no looks.
- A client applies a look, as emotes and badges, only when the name its game shows in
  `slot` matches `claimed_name`.

## Client

### Settings and commands

- `cg_saberSkin` (archived, default empty): the blade-skin unlock id the player
  wears. It applies only while the player's own hub profile lists that unlock; with
  no identity, no hub or the unlock missing, the stock blade shows and nothing is
  sent. Built so far: the cvar, read once a frame in one place
  (`GpuState::local_saber_skin`, [saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs)),
  which for now applies it without the profile check.
- `unlockables` opens the Unlockables page; `saberskin` alone lists the blade skins
  with owned or locked, `saberskin <id>` or `saberskin none` sets `cg_saberSkin`.

### Sending

The identity worker keeps the look the viewer last gave it (`Service::set_look`).
After its claim on a server is accepted, and whenever the look changes, it sends
`POST /v1/look`, at most one a second (the latest wins); a new claim (another
server or slot) sends the look again. A `not_unlocked` answer stops sending that
skin until the profile changes.

### Receiving

`looks` in the viewer keep one look per slot: from the presence roster when it
changes and from feed events as they come (the feed's are newer). A look counts only
under the badges' name rule. The local player's own look comes from its settings,
not the hub.

### The Sun blade

Built on `personal/saber-skins-blade` (renderer and sounds); who wears it comes from
the per-client table the hub's looks are to fill.

- Drawn as its own saber material (slot 7, after the neutral RGB pair) with
  engine-generated textures: a white-hot core into gold, a corona from gold through
  orange to a red rim with drifting granulation and flame tongues, and flares running
  from hilt to tip now and then, animated in `saber.wgsl` from per-instance time and a
  per-blade seed; the dynamic glow pass gets the same animation
  ([rendering.md](rendering.md#saber-blade-skins)). Its trail is amber gold and its
  light warm orange with a slow flicker. Retail and RGB blades are unchanged.
- Shown for every client whose entry in the viewer's `SaberSkins` table is set
  (`SaberSkins::set(client, Some(BladeSkin::Sun))`): in the game, in first person,
  thrown, and on the menu stage (the Character page) for the local player.
- Sounds, per player wearing it: ignition and switching off, the hum loop and three
  swings replace the stock ones (`sound/sjk/sabers/sun/*`, synthesized by
  `scripts/saber_skin_sounds.py`, bundled and mounted below the game data). The
  viewer passes the table to the audio adapter every frame
  (`LegacySoundAdapter::set_saber_sound_overrides`, [client.md](client.md#blade-skins)).
  A thrown saber's hum stays the hilt's own.
- Verified: unit tests (material slot, colours, the table, the cvar, the sound
  overrides per client, the bundled WAVs and the hum's loop point) and the
  `duel6_sun_blade` world shot. Not verified: the sounds by ear, a live match.

### Illuminate for others

Every lit look puts a holocron by that player's left shoulder with its warm light,
placed from their entity's position and view yaw, with the same fade, bob and spin
as one's own.

### Unlockables page

An SJK page (SJK UI look in every menu style, as the Staff page) opened from
Profile and `unlockables`: a card per unlockable with a live swatch, its name,
owned (with the date and the team's note) or locked (with how to get it), and Equip
or Unequip for owned blade skins. The Staff page gains Unlock and Relock for the
player found.

## Planned, not built

Achievements and medals granting unlockables, more blade skins, other kinds of
unlockable (holocron skins, trails, emotes).
