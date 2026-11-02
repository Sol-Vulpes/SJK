# Casino

The `casino` command opens one SJK page in every menu style: multiplayer Texas
Hold'em, European roulette, slots and blackjack. It wagers the player's existing
SJK credits. It requires identity and a hub with the matching casino API; older
hubs refuse the request without changing credits. No game-server messages are sent.

## Rules

- **Poker:** sixteen six-seat tables, 100-credit buy-in, blinds 1/2, no rake.
  Each seated player presses **Ready for hand** before every deal. No-limit raises
  use the wager control as the total for the current street. Short all-ins do not
  reopen a completed action until their cumulative increase reaches a full raise. Side pots are settled separately; tied pots split,
  with odd chips going clockwise from the button. An absent player checks when
  free or folds after 30 seconds; a later read advances the expired turn. Cash out
  between hands. Closing the page retains the seat and stack; it does not consent
  to another hand. Tables are shared across the hub, independent of game servers.
- **Roulette:** one zero, numbers 0..36. Red/black, odd/even and low/high return
  twice the wager on a win; a single number returns 36 times. Zero loses the
  even-money bets. Theoretical return is 36/37 (97.30%).
- **Slots:** three independent reels with six equally likely symbols (0..5).
  Matching triples return 5/10/20/30/50/90 times the wager, respectively.
  All other combinations return zero. Theoretical return is 205/216 (94.91%).
- **Blackjack:** one shuffled 52-card deck per round, dealer stands on soft 17.
  A natural returns 2.5 times the wager (3:2 profit); a normal win returns twice
  the wager, a push returns it, and a loss returns zero. Bets must be even.
  Hit and stand are supported; split, double and insurance are not implemented.

All returns include the stake. House-game wagers are 1..500 whole credits; the
page starts at ten, with whole-credit steps (twos for blackjack), +10 and +100
shortcuts. Poker also has an All in button; raises are bounded by the stack.
These are initial rules and limits.

## Authority and recovery

The hub shuffles, rolls, validates turns and owns escrow. It persists blackjack
rounds, poker tables and the credit ledger in the same transaction. A client
receives only its own hole cards during a hand and non-folded hands at a contested
showdown; winning by folds does not expose the winner's cards;
the dealer's hole card stays hidden until blackjack ends. A signed mutation has
a random 128-bit request token, a player revision and, for poker, a table revision.
Retries reuse the token and cannot charge or pay again. A changed table requires
a refresh. No protocol-26 messages, movement or server game rules change.

After a network error, **Retry same request** repeats that operation safely.
**Read current state** retrieves the latest state without another wager. Reopening
the page resumes an unfinished blackjack round or poker seat. Profile merges
require finishing blackjack and cashing out of poker first.

This needs its paired hub change. Verification and remaining limits are recorded
in [status.md](status.md); implementation is not evidence of deployment.

Bet reopening and odd-chip order follow the [Poker TDA rules](https://www.pokertda.com/view-poker-tda-rules/),
with the explicit credit-game limits above. This is not a claim of tournament certification.
