# SJK chat and emotes

Status (08/10/2026): **planned**, design agreed with Sol the same day. Nothing below
is built yet; this page becomes the subsystem's documentation as it is.

SJK chat is one conversation for every SJK player, carried by the SJK hub
(`cl_hubUrl`, sjk.dfox.app) instead of the game server: it reaches SJK players on
any server, or in the menus with no server at all, and players on stock clients
never see it. Emotes ride the same hub connection so SJK players on one game server
see each other's emotes; the emotes themselves (animations, sounds, art) are a
later, separate piece of work, and this page only prepares the path for them.

The hub's side lives in Sol-Vulpes/SJK-hub (`PROTOCOL.md`, "Chat" and "Emotes"),
the client's in `sjk-identity` and the viewer. Both read
[identity.md](identity.md): chat and emotes are signed by the player's identity key
and need `cl_identity 1`.

## Decisions

| Question | Decision |
| --- | --- |
| Who may write | Any registered SJK key; everyone reading is registered too. |
| History | In the hub's memory only: the last 200 messages, gone on a restart. |
| Menus | A docked box on the SJK UI's main page and a full SJK Chat page. |
| Emote ids | Open: the hub relays any well-formed id; the catalogue is client data. |

## Hub protocol (additions to version 1)

Old clients ignore all of it, so it stays `/v1/`.

### Chat

`POST /v1/chat`, signed, a registered key (403 `not_registered`), body exactly
`{"text":"..","name":".."}` (`name` optional: the in-game name worn, checked like a
claim's and recorded in worn names). Answer `{"id":<n>}`.

The text's rules (`chat.rs`, word for word in the client's `sjk-identity` and the
hub, as `bio.rs` is): runs of spaces and tabs become one space and the ends are
trimmed; then it must be one line (400 `chat_lines`), 1 to 150 characters (400
`chat_length`), only the bio's alphabet and colour codes (400 `chat_characters`)
and no run of one character longer than 8 (400 `chat_noise`). JoF's emoji names
(`:poop:`, `#>:D`) fit the alphabet.

Limits: a key sends at most one message every 2 seconds and 15 a minute (429
`chat_quota`); the same text from the same key within 30 seconds is refused (409
`chat_duplicate`); a key the operator or staff muted is refused (403 `chat_muted`).
Per address, a `Chat` allowance of 20 a minute beside the general one.

A message as the feed carries it:

    {"id":12,"at":<unix s>,"key_id":"..","name":"^2Sol","verified":true,
     "staff":false,"text":".."}

`name` is the name the key wore when it sent the message (the hub's display name
when it sent none). The hub keeps the last 200 messages in memory, never on disk.

### Emotes

`POST /v1/emote`, signed, body exactly `{"server":"ip:port","emote":".."}`. The key
needs a live claim on `server` (403 `not_on_server`); the hub takes the slot and
name from that claim, so a key emotes only for the slot it claims. `emote` is 1 to
32 of `a`-`z`, `0`-`9`, `_` (400 `bad_emote`); the hub knows no catalogue. At most
one every 2 seconds per key (429 `emote_quota`). Answer `{"id":<n>}`. An emote lives
10 seconds at the hub:

    {"id":13,"at":<unix s>,"slot":3,"claimed_name":"^2Sol","key_id":"..","emote":"wave"}

### The feed

`GET /v1/feed?after=<id>&server=<ip:port>&wait=<0..25>`, signed (the path and query
are signed as for any request). `server` is optional. Chat messages and emotes share
one id sequence. The hub answers as soon as it holds anything newer than `after`,
else after `wait` seconds:

    {"next":14,"chat":[Message],"emotes":[Emote],"deleted":[<id>],"online":7}

- `chat`: messages newer than `after`, at most 50 (the newest 50 for `after=0`).
- `emotes`: emotes newer than `after` on `server`, less than 10 seconds old.
- `deleted`: ids of messages staff deleted since `after` (a client drops them).
- `online`: keys that read the feed in the last 60 seconds.
- `next`: the `after` of the next request.

The feed has its own per-address allowance (40 a minute) and does not use the
general one. The client waits at least 2 seconds between two feed requests.

### Moderation

- `chat_muted`, a flag on a key like verified: the operator's `chat-mute` and
  `chat-unmute` commands and `{"chat_muted":true}` in the operator API.
- Staff (`PROTOCOL.md`, "Staff", same gate, quota and log):
  `POST /v1/staff/chat-delete` `{"id":<n>}` removes a message from memory and lists
  its id in `deleted`; `POST /v1/staff/chat-mute` `{"key_id":"..","muted":true}`.

## Client service

`sjk-identity`:

- `chat.rs`: the text rules and the wire types (`ChatMessage`, `Emote`).
- Sending (`Service::chat`, `Service::emote`) goes through the existing worker as
  commands; the rules are checked before anything is sent, as a bio's are.
- A second thread, `sjk-hub-feed`, owns the long poll with its own HTTP client
  (timeout 40 seconds) so the worker's claims and reports never wait behind it. It
  runs only while the identity is on, the hub answered the registration and
  `cl_sjkChat` is on; it backs off on failures. It keeps `ChatState` (the last 200
  messages, a revision, the online count, the outcome of the last send) and a queue
  of received emotes (`Service::take_emotes`). It names the server the worker is
  claiming on in its requests.

## In the game

- The chat overlay gets a fourth channel, SJK. `messagemode5` (also `sjkchat`)
  opens the composer on it; its default key is `I`. Tab cycles Global, Team, SJK.
  Enter on SJK hands the text to the service, never to the game server.
- Hub messages join the chat feed with an `[SJK]` tag in holo blue; a verified
  sender's name is followed by the gold mark. Muting a sender mutes their key.
- `cl_sjkChat` (default 1, Settings > Network) shows SJK chat and runs the feed;
  0 hides it and stops the feed.

## In the menus

- SJK UI main page: a docked SJK chat box under Recent servers, on the right: the
  last messages that fit, the online count, a field (Enter or a click types, Enter
  sends) and Open chat, which opens the page.
- The SJK Chat page (console page, the Profile and Staff pages' pattern, in the SJK
  UI's look in every menu style): the history with names, badges and times, the
  composer, the online count. A chosen message offers Mute (on this PC); for staff,
  Delete and Mute at the hub. It opens from the box, the `sjkchat` command and
  the in-game SJK menu.

## Emotes in the client (groundwork)

- `emote <id>` sends an emote while on a server; `emote` alone lists the catalogue.
- The catalogue is data: `emotes/<id>.emote` files in the game's file system (pk3s
  or the folder), each naming the emote, its torso and legs animations, its length,
  whether it loops and an optional sound. None ship yet.
- A received emote counts only when the presence list ties its slot to its key
  under the name the game shows there (the badges' rule). It enters `ActiveEmotes`:
  per slot, the emote, when it started and when it ends (its length, or sooner when
  the player moves). That table is where the animation and sound plug in; until an
  emote has them, it is written to the console only ("* Sol: wave").

## Privacy

With `cl_identity`, `cl_hubUrl` and `cl_sjkChat` on, the hub sees the player's key
and IP address on every feed request (which it counts for `online`), and each
message or emote they send with the name they wear. Messages are public to every
SJK player, kept in the hub's memory only (the last 200), and gone when the hub
restarts. Emotes are kept 10 seconds. The mute flag is the only new thing on disk.

## Verification plan

Hub: unit tests of the rules and limits and in-memory API tests, the long poll
with tokio's paused clock. Client: the rules against the hub's cases, the worker
and feed against a fake hub, the overlay's SJK channel, the docked box and page at
1080p, 4K, 4:3 and 21:9. An end-to-end run against a hub built on this PC. No
deployment without Sol's word.
