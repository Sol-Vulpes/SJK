# SJK chat and emotes

SJK chat is one conversation for every SJK player, carried by the SJK hub
(`cl_hubUrl`, sjk.dfox.app) instead of the game server: it reaches SJK players on
any server, or in the menus with no server at all, and players on stock clients
never see it. Emotes ride the same hub connection so SJK players on one game server
see each other's emotes; the emotes themselves (animations, sounds, art) are
separate work, and SJK only has the path for them so far.

Status (08/10/2026): built on `personal/sjk-chat` (client) and `feat/chat-emotes`
(Sol-Vulpes/SJK-hub), designed with Sol the same day; the hub's side is not deployed.
See [Verification](#verification) for what was checked.

The hub's side is described in Sol-Vulpes/SJK-hub (`PROTOCOL.md`, "Chat", "Emotes" and
"The feed"). Chat and emotes are signed by the player's identity key and need
`cl_identity 1` ([identity.md](identity.md)).

## Decisions

| Question | Decision |
| --- | --- |
| Who may write | Any registered SJK key; reading the feed needs a registered key too. |
| History | In the hub's memory only: the last 200 messages, gone on a restart. |
| Menus | A docked box on the SJK UI's main page and a full SJK chat page. |
| Emote ids | Open: the hub relays any well-formed id; the catalogue is client data. |

## Pieces

| Piece | Where |
| --- | --- |
| Text rules (word for word with the hub's `src/chat.rs`) | [chat.rs](../crates/sjk-identity/src/chat.rs) |
| Wire types (`ChatMessage`, `Emote`, `Feed`) | [wire.rs](../crates/sjk-identity/src/wire.rs) |
| Requests (`chat`, `emote`, `feed`) | [hub.rs](../crates/sjk-identity/src/hub.rs) |
| The feed thread and `ChatState` | [feed.rs](../crates/sjk-identity/src/feed.rs), [service.rs](../crates/sjk-identity/src/service.rs) |
| Viewer glue, local mutes | [player_identity.rs](../crates/sjk-viewer/src/player_identity.rs) |
| In-game SJK channel | [chat/sjk.rs](../crates/sjk-viewer/src/chat/sjk.rs), [sjk_chat_frame.rs](../crates/sjk-viewer/src/sjk_chat_frame.rs) |
| Main page dock | [home.rs](../crates/sjk-viewer/src/menu/sjk/home.rs), [chat_dock.rs](../crates/sjk-viewer/src/menu/sjk/chat_dock.rs) |
| SJK chat page | [sjk_chat_panel.rs](../crates/sjk-viewer/src/sjk_chat_panel.rs), [sjk_chat_panel_view.rs](../crates/sjk-viewer/src/sjk_chat_panel_view.rs), [console_sjk_chat_page.rs](../crates/sjk-viewer/src/console_sjk_chat_page.rs) |
| Emotes | [emotes.rs](../crates/sjk-viewer/src/emotes.rs), [emotes_frame.rs](../crates/sjk-viewer/src/emotes_frame.rs) |

## Hub protocol (additions to version 1)

Old clients ignore all of it, so it stays `/v1/`.

- `POST /v1/chat` `{"text":"..","name":".."}` (`name`, the in-game name worn, is
  optional and joins the key's worn names). The text: one line (`chat_lines`), runs
  of spaces made one and the ends trimmed, 1 to 150 characters (`chat_length`) of the
  bio's alphabet with `^` only before a digit (`chat_characters`), no run of one
  character longer than 8 (`chat_noise`). A key sends at most one message every 2
  seconds and 15 a minute (`chat_quota`), not the same text twice within 30 seconds
  (`chat_duplicate`); a muted key is refused (`chat_muted`).
- `POST /v1/emote` `{"server":"ip:port","emote":".."}`: the key needs a live claim on
  the server (`not_on_server`), and the hub takes the slot and claimed name from it,
  so a key emotes only for the slot it claims. Ids are 1 to 32 of `a`-`z`, `0`-`9`,
  `_` (`bad_emote`); at most one every 2 seconds (`emote_quota`). An emote lives 10
  seconds.
- `GET /v1/feed?after=<id>&server=<ip:port>&wait=<0..25>`, signed. Messages, emotes
  and deletions share one id sequence. The hub answers as soon as it holds something
  newer than `after` for this reader, else after `wait` seconds:
  `{"next","chat":[..],"emotes":[..],"deleted":[..],"online"}`. `after` 0, or above
  the newest id (the hub restarted), gives the newest 50 messages and no emotes.
  A key reads at most 40 times a minute (`feed_quota`); the feed (300 a minute) and chat
  and emotes (60 a minute) have per-address allowances of their own, so several players
  sharing an address fit and talking never spends what claims need.
- Moderation: a `chat_muted` flag on keys (the operator's `chat-mute` and
  `chat-unmute`, `{"chat_muted":true}` in the operator API) and two staff requests,
  `/v1/staff/chat-delete` `{"id"}` and `/v1/staff/chat-mute` `{"key_id","muted"}`,
  logged in the staff log (a deleted message's text with it).

## Client service

- Sending (`Service::chat`, `Service::emote`) goes through the identity worker; the
  rules are checked before anything is sent, as a bio's are. What became of the last
  message or emote is `ChatState::outcome`.
- `Service::start_with_feed` starts a second thread, `sjk-hub-feed`, with its own HTTP
  client (timeout 40 seconds), so the worker's claims and reports never wait behind
  the long poll. It reads only while the identity is on, the hub answered the
  registration and `cl_sjkChat` is on (`Service::set_chat`), polls at most every 2
  seconds and backs off (2 to 60 seconds) when the hub fails. It keeps the last 200
  messages, the online count and whether the last poll reached the hub
  (`ChatState`; `loaded` says when the hub being read first answered, so the game's
  feed marks that backlog instead of replaying it), and queues up to 64 received emotes (`Service::take_emotes`). A new
  hub, or ids that go backwards (the hub restarted), start from the backlog.
- Shutting down tells the feed thread to stop; it ends after its poll without being
  waited for.

## In the game

- The chat overlay has a fourth channel, SJK. `messagemode5` (I by default; a profile
  gets I only when it is free and nothing is bound to `messagemode5`) opens the
  composer on it; Tab cycles All, Team and SJK. Enter on SJK hands the text to the hub,
  never to the game server.
- Hub messages join the chat feed tagged SJK in the SJK UI's gold (`#E8B84A`, apart
  from the game's blues), SJK VERIFIED for a verified sender. Names and texts go
  through `chat::for_display`. Messages staff delete leave the feed, and muting a key
  on the page hides its lines already there. Joining a game
  does not replay the hub's backlog in the feed (the dock and the page show it). A
  refusal (quota, rules) shows as an `SJK chat:` line.
- `cl_sjkChat` (default 1, Settings > Network > SJK chat) shows SJK chat and runs the
  feed; 0 hides it and stops the reading.

## In the menus

- SJK UI main page: the chat is docked under Recent servers. Its last five lines (a
  gold dot for a verified sender), cut to one row each, sit on the field, with the
  online count by its name. Down past the last server (or a click) reaches the field;
  Enter types (every key goes to the field), Enter sends, Escape stops. Open chat opens
  the page. A line under the field says why a message could not go.
- The SJK chat page (a console page in the SJK UI's look in every menu style): the
  history, newest at the bottom, each with its name, Staff and Verified, and how long
  ago ("3 minutes ago", as the main page's servers say, so no time zone is needed),
  wrapped; the field (the keyboard is there when the page opens), Send and the
  character count. Up from the field chooses the newest message, Up and Down move,
  Page Up and Page Down scroll, Tab walks every control. A chosen message offers Mute on
  this PC (for the session); for staff, Delete for everyone, Mute at the hub and Unmute
  at the hub (the client does not know a key's mute flag); under them, what the request
  came to (sending, done, or the hub's refusal). It opens from the dock's
  Open chat, `sjkchat`, `messagemode5` outside a game, and the in-game SJK menu's
  SJK chat.

## Emotes in the client (groundwork)

- `sjkemote <id>` sends an emote while on a server (named apart from the `emote`
  command some server mods have); `sjkemote` alone lists the catalogue.
- The catalogue is data: `emotes/<id>.emote` files in the game's file system (pk3s or
  the folder), each a JSON object: `name` (required, at most 32 characters), `torso`
  and `legs` animation names, `length_ms` (100 to 60000, required unless `loop` is
  true), `loop`, and `sound`. Unknown fields and bad values are refused with the file
  named in the log. None ship yet.
- A received emote counts only when the game shows, in its slot, the name its claim
  was made under (the badges' rule). It then plays in `emotes::ActiveEmotes`: per slot,
  the emote, when it started and when it ends (its length, 3 seconds for an id the
  catalogue lacks, never for a loop), and it ends early when the player moves 24 units
  or leaves. `emotes::active(slot)` is where the animations and sound plug in; until
  they exist, a received emote is written to the console (`* Sol: Wave`).

## Privacy

With `cl_identity`, `cl_hubUrl` and `cl_sjkChat` on, the hub sees the player's key
and IP address on every feed request (which it counts for `online`), and each message
or emote they send with the name they wear. Messages are public to every SJK player,
kept in the hub's memory only (the last 200), and gone when the hub restarts; a
message staff delete stays in the staff log. Emotes are kept 10 seconds. The mute
flag is the only new thing on the hub's disk. Local mutes are not saved.

## Verification

08/10/2026, Ubuntu 24.04, Rust 1.99:

- Hub: unit tests for the rules and the feed (ids, the 200 kept, quotas, duplicates,
  batches of 50, a restart's `after`, emotes per server and their 10 seconds,
  deletions, the online count) and API tests run in memory, including the long poll
  under tokio's paused clock (answered when a message comes, empty after 25 seconds),
  emotes needing a claim, IPv6 servers, the feed's and the chat's own allowances (four
  readers behind one address, 40 reads a minute per key), staff delete and
  mute with the log, the operator's API and commands. `cargo test` and `cargo clippy
  --all-targets` pass.
- Client: unit tests for the rules (identical to the hub's), `for_display`, the feed
  thread against a scripted hub (order, repeats, 200 kept, deletions, a new hub, a
  restarted hub, turning the chat off, the 2 second gap, back-off), the worker
  (rules before sending, emotes needing a server, the feed only when registered with
  the chat on), the overlay's channel, the dock (fits under four servers and above the
  version at 1080p, 4K, 21:9, 4:3, 5:4, 1024x768; keys, typing, pointer), the page
  (every focus fits at 1080p, 4K, 4:3 and 21:9 in the families and Inter; keys,
  scrolling, mutes, staff only for staff), emotes (catalogue files, lengths, moving,
  trust), and the key's default and migration.
- End to end against a hub built and run on this PC (`hub_e2e`): a reader waiting at
  the hub received a message about 2 seconds after it was said, refusals came back
  with the hub's codes, an emote needed a claim and reached the server's reader, and
  the service's feed thread read its own message and stopped when the chat was turned
  off.
- The dock and the page were rendered off screen over a plain backdrop in the UI's
  families and looked at.

Not verified: no client was started, so nothing was seen in a game or over the map;
nothing went through Cloudflare's tunnel or the deployed hub (which does not have the
routes yet); Windows; how the feed behaves with many players at once.
