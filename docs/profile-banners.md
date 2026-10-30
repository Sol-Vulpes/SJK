# Profile banners (proposal)

Proposed cosmetic reward, requested 30/10/2026. This page describes a design for
review; banners, drops and the protocol below are not implemented by this PR.

A banner is a wide cover behind a player's profile picture and name, like a
profile cover on Discord. It is separate from a nameplate, avatar or saber skin.
Players earn banners from the existing Echo/holocron rewards and equip one from
Collection > Banners. Nothing changes gameplay.

## Where it appears

- **Full profile:** a roughly 3.6:1 cover across the top, with the avatar overlapping
  its lower left edge. Name, verified mark and status remain readable over a dark
  gradient. Bio, medals and achievements stay below it. On smaller windows the
  profile scrolls; the banner never squeezes the existing text into smaller type.
- **Player cards:** a shallow crop of the same banner above the existing details.
  The avatar and actions retain their normal hit targets. The banner's important
  art stays inside a central safe area so different card widths can crop it.
- **Collection:** a Banners tab with owned covers, tier, duplicate count, a large
  profile preview, Equip and Unequip. Locked covers can be previewed using the
  collection's existing reveal rules. A neutral SJK cover is always available.

Every menu style uses the SJK version of these controls and covers. The banner
does not replace a player's name colours, trust mark, medal or avatar.

## First set

| Banner | Suggested tier | Look |
| --- | --- | --- |
| Jedi Archives | Uncommon | Navy panels, warm gold lines, faint archival lettering |
| Binary Sunset | Rare | Two suns above a warm horizon, a restrained heat shimmer |
| Sith Eclipse | Rare | A dark disc with a red rim and drifting embers |
| Hyperspace | Legendary | Blue star trails moving slowly towards the centre |
| Living Nebula | Mythical | Violet and teal clouds with occasional distant stars |

These are starting concepts, not final rarity or drop-balance decisions. Art
must be original or licensed for distribution. No screenshots or extracted assets
from other commercial games are needed. Static covers come first; motion is an
optional later layer, never flashing or required to recognise a banner. When
motion is disabled the same cover shows an attractive still frame.

## Earning and ownership

Banners should be another reward kind in the existing opening flow, using the
same identity-owned inventory, tier eligibility and authoritative reward roll.
The client must not roll a reward or claim ownership itself. The proposed PR
does not choose new drop percentages: the banner-kind weight and the resulting
change to existing reward odds need review together in the hub change.

An opening reveals a banner card with its name, tier and full cover, and it stays
in Collection until equipped or managed through the supported inventory actions.
Duplicate banners should follow the existing duplicate and trading rules for the
chosen reward kind; do not introduce a second currency, duplicate rerolls or
automatic conversion solely for banners. Any inventory action not supported for
that kind must remain unavailable rather than appearing to work only locally.

## Protocol and implementation outline

The client and hub need matching, additive PRs before this can ship:

1. Add a named banner catalogue and reward kind, with permanent ids, tier,
   edition, image path and optional motion metadata. Keep client and hub
   catalogue checks aligned as for other rewards.
2. Grant owned banner copies through the normal opening transaction. Include
   them in the normal profile/inventory response, with existing count semantics.
3. Add an optional equipped banner id to the public profile. A signed update
   accepts only an owned, supported id; an empty id unequips it. The hub checks
   ownership again after inventory changes so a traded-away last copy cannot
   remain equipped.
4. Let old clients ignore the added fields. A new client seeing an unknown id,
   unavailable art or an older hub draws the neutral cover and keeps the profile
   usable. Ship the hub support before enabling rewards on the live service.
5. Build the Collection picker and both profile sizes using a shared banner
   renderer. Load and cache art outside the frame path; motion must not add
   per-frame asset reads or new allocations.

Use a profile field, not the game-server look or protocol-26 player snapshot:
the banner belongs to the SJK identity and must also appear in menus and across
game servers. No server-specific banner rules or arbitrary remote image URLs.

## Acceptance before implementation is called verified

- Inventory/opening tests cover tier eligibility, ownership, duplicates, atomic
  failure and a transferred-away last copy.
- Contract tests cover old clients, old hubs, missing fields and unknown ids.
- UI checks cover equip/unequip, keyboard and pointer use, avatar/name contrast,
  long names, narrow windows, larger text and disabled motion.
- Attach full-profile, player-card and Collection screenshots to the feature PR.
  Measure animated covers in a release build before choosing their complexity.

No hub rollout, art production or runtime verification is claimed by this design
proposal. The next review should settle the visual direction, reward weighting
and whether the first implementation includes motion.
