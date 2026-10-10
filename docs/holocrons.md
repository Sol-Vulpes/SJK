# Holocron drops

A holocron is loot a player earns by playing SJK. The SJK hub rolls and stores them and
the client shows them. They cannot be opened yet, grant nothing and are only collected
and shown (Sol's decision, 10/10/2026). This page is the design, the decisions and what
is checked; the wire is the hub's `PROTOCOL.md` ("Holocrons") in Sol-Vulpes/SJK-hub,
decided together with the client and built in parallel. Players read the pop-up, the chat
lines and, once built, the Profile screen's Holocrons tab.

## Pieces

| Piece | Where |
| --- | --- |
| Wire types, the `active` claim flag, `GET /v1/holocrons`, drop events, staff requests | [wire.rs](../crates/sjk-identity/src/wire.rs), [hub.rs](../crates/sjk-identity/src/hub.rs), [staff.rs](../crates/sjk-identity/src/staff.rs) |
| Progress in the snapshot, `Service::set_active`, the worker's early profile reading | [service.rs](../crates/sjk-identity/src/service.rs) |
| Drops joined to the chat's messages, told to the worker when they are the player's own | [feed.rs](../crates/sjk-identity/src/feed.rs) |
| The one client list of tiers, the player's counts, recent holocrons, progress | [holocrons.rs](../crates/sjk-viewer/src/holocrons.rs) |
| What counts as actively playing, the input clock | [holocrons/activity.rs](../crates/sjk-viewer/src/holocrons/activity.rs), [holocrons_frame.rs](../crates/sjk-viewer/src/holocrons_frame.rs), input hooks in [viewer_app.rs](../crates/sjk-viewer/src/viewer_app.rs) |
| The drop pop-up, its ceremony and look | [holocron_popup.rs](../crates/sjk-viewer/src/holocron_popup.rs), [holocron_popup/view.rs](../crates/sjk-viewer/src/holocron_popup/view.rs) |
| `holocrons_seen.txt` | [holocrons/seen.rs](../crates/sjk-viewer/src/holocrons/seen.rs) |
| The chat lines' words and the tier gem | [holocrons/line.rs](../crates/sjk-viewer/src/holocrons/line.rs), [holocrons/gem.rs](../crates/sjk-viewer/src/holocrons/gem.rs), [sjk_chat_look.rs](../crates/sjk-viewer/src/sjk_chat_look.rs) |
| The tiers' pictures in the UI atlas | [holocrons/icons.rs](../crates/sjk-viewer/src/holocrons/icons.rs), [ui_renderer/icons.rs](../crates/sjk-viewer/src/ui_renderer/icons.rs), [hud/icons.rs](../crates/sjk-viewer/src/hud/icons.rs) |
| `debug_holocron` | [holocrons/rehearsal.rs](../crates/sjk-viewer/src/holocrons/rehearsal.rs) |
| Give holocron and Remove on the Staff page | [staff_panel.rs](../crates/sjk-viewer/src/staff_panel.rs), [staff_panel_view.rs](../crates/sjk-viewer/src/staff_panel_view.rs) |
| The hub's rolls, caps and storage | repository Sol-Vulpes/SJK-hub (`PROTOCOL.md`, `src/holocrons.rs`) |

## Tiers

Four tiers, the same ids, names and order in the hub's `src/holocrons.rs` and in the
client's [holocrons.rs](../crates/sjk-viewer/src/holocrons.rs); a test on each side pins
the list. A client shows only the tiers it knows.

| Id | Name | Colour | Odds |
| --- | --- | --- | --- |
| `uncommon` | Uncommon Holocron | green `#2FC77A` | 60% (600 per mille) |
| `rare` | Rare Holocron | blue `#2E7BFF` | 28% (280) |
| `legendary` | Legendary Holocron | purple `#A64DFF` | 10.5% (105) |
| `mythical` | Mythical Holocron | gold `#FFC933` | 1.5% (15) |

The colours are none of the game's `^n` codes, in the game's palette or the SJK UI's
lifted one (at least 0.28 away in RGB, tested), and the four are at least 0.4 apart from
one another for players who tell hues apart poorly. Mythical gold is near the SJK chat's
own gold (`#F5C756`, 0.14 away) because both are gold; a drop line stands apart from a
message by its gem, its wording and having no sender column. Each tier's picture is
`gfx/sjk/holocron_<tier>.png`, bundled with the client and mounted below all game data
(the list in [illuminate.rs](../crates/sjk-viewer/src/illuminate.rs)); where it is missing
the tier draws as a gem made of shapes ([holocrons/gem.rs](../crates/sjk-viewer/src/holocrons/gem.rs)).

## Earning

- Every claim to the hub carries `"active": true` only when the player is actively
  playing. The client says so when it is in a live, non-local game of someone else's, in
  the player's own view (not spectating or following), mid-match (not at the intermission
  or the scoreboard freeze), out of the menus (the game menu, the console, a dialog, a
  pop-up or the main menu) and had input (a key or mouse button pressed, the wheel, the
  mouse moving) in the last two minutes. A claim without the flag, which an older client
  sends, earns nothing; `active: false` is left out of the body altogether.
- On each claim renewal with `active: true` the hub adds the time since its last claim of
  the key, at most 60 seconds, to the key's active time, and `last_tick` moves on with every
  claim. The client renews every 45 seconds, so a renewal counts in full and sending claims
  faster counts only real elapsed time. At 1800 seconds the hub drops a holocron and keeps
  the rest. Progress persists in the hub's database across sessions. A hub started with
  `--holocron-every <secs>` drops sooner, for tests; the caps stay.
- The tier is rolled by the hub from a system random source.
- Caps, all the hub's: at most 8 drops in a rolling 24 hours per key (staff gifts excluded);
  at most 1 mythical per key in 24 hours (another becomes legendary); at most 16 per
  address in 24 hours across all keys. A drop a cap holds back waits (the active time stays
  at 1800) and happens at the first renewal after the cap frees; nothing is lost.
- A staff gift (`source: "staff"`) counts toward no cap and is not announced.

The client reads the progress with `GET /v1/holocrons` (`progress_secs`, `every_secs`, the
day's `today` and `daily_cap`) at the registration, every five minutes, soon after a drop
of its own and when a screen asks ([`holocrons::refresh`](../crates/sjk-viewer/src/holocrons.rs),
at most once every 30 seconds). A hub from before holocrons refuses it; it is asked again
in an hour and the identity's status never shows it. The remaining time is as the hub last
said and does not count down by itself.

## What the hub stores, and privacy

The hub stores each key's active play time (a total carried toward the next holocron) and
the time of its last claim, and every holocron dropped for a key: its tier, time, source
(`play` or `staff`), a staff note and the address (or a hash of it) it dropped from, for
the per-address cap. The profile lists a key's counts and its recent holocrons publicly,
as it does its medals, and legendary and mythical drops are told to every SJK client in
the feed with the key's hub name. Uncommon and rare ones reach only their owner. Old
clients ignore all of it. [identity.md](identity.md#privacy) says what a default install
sends.

## In the client

### The pop-up

A holocron new to the client shows once, large, on a darkened screen in its tier's colour:
"New holocron" (and "1 of 3"), where it came from (dropped while playing, or a gift from
the SJK team), its name, when it was found (`dd/mm/yyyy HH:MM`, UTC), how rare it is and,
for a gift, the team's note. It follows the medal pop-up
([identity.md](identity.md#medals), [medal_popup.rs](../crates/sjk-viewer/src/medal_popup.rs)):

- It opens on the main menu, or when the game menu opens in a match, never over play. A
  holocron that arrives during a match is announced once with a centre print, "New SJK
  holocron: Rare Holocron / Open the game menu to see it". While it shows, the menu under it
  is neither drawn nor given input. A medal on show or waiting goes first.
- Each arrives in a ceremony of the medal's timing and light in the tier's colour (it comes
  down into place, rings and sparks run out, a ring of ticks turns, a glow breathes while it
  waits, the words fade up one group after another) with the multiplayer game's fanfare
  (`music/goodsmall.mp3`, the medals' cue; silent without game data). Enter, Space, Right,
  Escape or a click finish the entrance, then take Next (Close on the last). The look is the
  SJK UI's in its families, Inter until they load, and the same under the classic menus.
- Everything is draw-list shapes of a fixed number: no allocation a frame, and a canvas of
  320 commands holds the busiest moment.
- `holocrons_seen.txt` in the settings folder, beside `identity.key`, holds the key id and
  the highest holocron number shown, written when a holocron's button is taken. A number
  above it is new. With no file for the key (a reinstall, a new PC) everything the profile
  holds is new, and the newest 20 show once, oldest first. A holocron of a tier the client
  does not know is left out and never counted as shown. Holocron numbers only rise at the
  hub, so a staff removal never brings an old one back.
- The identity worker reads the own profile again soon (at most once every 30 seconds) when
  a drop for the player's own key arrives in the feed, and the pop-up sees the new holocron
  within half a second of the profile arriving.

### Chat lines

A drop is one SJK chat line in the tier's colour with a small gem where the verified tick
goes, in all three places the SJK chat shows ([hub-chat.md](hub-chat.md#how-a-line-looks)):

- The game's chat feed and the docked chat: `Sol found a Legendary Holocron!` (the hub
  name without colour codes, "Someone" when it has none), and for the player's own
  `You found an Uncommon Holocron.`; a `!` ends the two grand tiers' lines, a `.` the others.
  There is no sender column, no colon and no tick.
- The chat page: the name in its column, the gem, then `found a Legendary Holocron!`, with
  no colon; a tier the page does not know reads `found a holocron.`.
- A muted player's drops are not shown, and a drop of an unknown tier is left out of the
  feed and the dock. A drop is not a message, though it shares the feed's numbers and is
  kept with the chat's last 200.
- Legendary and mythical drops are delivered to every reader and uncommon and rare ones to
  their owner only, by the hub; a fresh reader gets only the drops of the last 60 seconds,
  so a joiner is not replayed history. With `cl_sjkChat 0` no line is kept, but the
  player's own drop still makes the worker read the profile.

### Staff

The Staff page ([identity.md](identity.md#staff)) has a Holocrons section under the
players: the chosen player's counts, the four tiers to choose (the choice ringed in its
colour), Give (with the note field, `StaffRequest::HolocronGive`, `POST
/v1/staff/holocron-give`) and the player's four newest holocrons with Remove
(`StaffRequest::HolocronRemove`, `POST /v1/staff/holocron-remove`). The hub's answer is the
target's profile and replaces the chosen player's, and the player's own when it is theirs.
To make room the player list shows 10 players instead of 14.

### `debug_holocron`

`debug_holocron <tier|all> [x<count>]` (alone it lists the tiers and the odds) rehearses
drops offline: `all` queues one of each tier, `rare x3` three of one (1 to 20). They go
through the same queue, centre print, pop-up, ceremony and sound, and the first one's chat
line goes into the game's feed as the player's own. The console closes so the pop-up shows
at once on a menu. Nothing is sent to the hub, and rehearsed holocrons (numbered from
4,000,000,000) are never written to `holocrons_seen.txt`.

## For other screens: the client's list

[holocrons.rs](../crates/sjk-viewer/src/holocrons.rs) is the one list the rest of the
client reads; the Profile screen's Holocrons tab is built on it.

- `TIERS: [Tier; COUNT]`, `tiers()`, `Tier::from_id` / `by_id(id)`, `colour(id)`, `name(id)`;
  a `Tier` has `id`, `name`, `colour` (`colour_alpha(a)`), `per_mille`, `odds` text, `index`,
  `icon_path()`, `article()` and `is_announced()`.
- `counts() -> Option<[u32; COUNT]>` (the own profile's counts in tier order),
  `recent() -> Option<Vec<Entry>>` (the own recent holocrons, newest first, known tiers
  only; an `Entry` has `id`, `tier`, `dropped`, `gift`, `note` and `when()`), both `None`
  until the hub answered; `entries(list)` and `counts_of(counts)` for a profile read some
  other way.
- `progress() -> Option<HolocronProgress>`, `refresh()` (ask the hub again when a tab opens)
  and `next_text(&progress)` ("Next holocron in about 20 minutes of play", or "Daily limit
  reached: 8 of 8 holocrons today").
- `icons::texture(index)` is the atlas cell of a tier's picture, uploaded with the HUD's
  pictures; `hud.holocron_icons` says which loaded. `gem::draw(list, centre, half, colour,
  alpha, rows)` draws the fallback.
- The 3D stage is the Illuminate holocron's model (`models/sjk/holocron.md3`), tinted by
  the tab.

## Checked and not checked

Verified (10/10/2026, Windows 11, `cargo test --locked --workspace`): the wire types parse
(profiles with and without the new fields, drop events, the progress answer); claims carry
`active` only when true; the worker reads progress after registering and every five
minutes, treats a hub without holocrons as cosmetic and reads the profile and progress
soon at most twice a minute after an own drop; the feed merges drops with messages in id
order and tells the worker once per own drop; staff give and remove send the protocol's
fields; the catalogue, its colours and the pop-up's queue, order, first-read cap, seen file,
rehearsal, ceremony and layout (every tier, with and without a picture, a note, in five
window shapes, in the families and in Inter); the three chat views' lines; the staff page's
requests; the activity rules against player states. Not tried: any of it against a running
hub (`crates/sjk-identity/tests/hub_e2e.rs` has an ignored test that needs one), in a game,
or with the art of the tiers (the pop-up draws the gem until the pictures are mounted); the
input hooks and the 500 ms comparison are not covered by a test of their own.

## Planned, not built

Opening holocrons, anything a holocron grants, and the Profile screen's Holocrons tab (a
3D holocron on the menu stage that cycles by tier, the counts, the list of recent drops,
the progress and the odds).
