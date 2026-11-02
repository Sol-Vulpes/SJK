# Beso credit rain

`toy_beso` spends **100 of the owner's existing SJK credits** on ten packets that
nearby SJK players collect by walking over them after opting in to **See and collect
credit rain** in Collection > Toys > Beso (also in Settings). This is off by default
(`cg_toyBesoShow`), preserving the stock look until chosen; the donor always sees its
own paid rain. The Collection's Toys tab has a
**Rain 100** button and a bindable rain key, in the SJK UI in every menu style.
There is no recurring spend or automatic rain. The console command accepts a
different whole amount: `toy_beso 1000`. Amounts are 100..100000, in multiples of
ten; every packet holds one tenth of the amount. The owner cannot collect it.

The donor must be alive on a game server, with identity enabled and at least one
other identified SJK player within 256 units. The nearby roster is captured when
funding the burst. Only those people, still claiming that server, may collect it.
Packets fall in a ring around the donor and can be collected after two seconds,
by walking within 56 units. Their gold credit marks are presentation only, with
no collision, movement or protocol-26 changes. The rain lasts thirty seconds.

The hub debits the total before showing a burst and atomically awards each packet
once, even if two clients collect together. It returns uncollected credits on the
next successful credit-rain request after expiry; this can be delayed while no
clients are reading rain. A disconnected donor keeps the refund. A profile merge
waits for the donor's active rain to expire, then settles refunds first.

After an uncertain funding error, `toy_beso retry` sends the same saved request
token. It cannot debit the burst twice. The current process retains that request
until success (the Rain 100 button also retries it); `toy_beso forget` explicitly
discards an uncertain token without cancelling existing rain or refunds. Restarting the client loses its retry token, but live bursts and
the ledger remain at the hub. Rejoining discovers the current rain and balance.

This uses SJK identity claims and client-observed positions, like other hub toys.
Legacy servers do not attest positions: the hub checks eligibility, server, map
and the reported pickup distance, but cannot prove a client stood there. A donor
can deliberately give credits to a remote player by altering its client; a
collector cannot add itself to the donor's eligible roster or mint credits.

The hub feed announces new rain; clients read every ten seconds while idle and
once a second while a request or burst is active. Drawing uses retained data and
the existing bounded world-marker list. It is hidden with ordinary HUD markers
while menus, console, scoreboard or intermission cover the game.

This requires its matching hub update. Implementation, verification and remaining
limits are recorded in [status.md](status.md); it has not been deployed by this change.
