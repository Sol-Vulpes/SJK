# Cards: what a holocron will hold

**Status: planned, nothing is built.** This page is the design agreed with Sol on
10/10/2026, so the work can start from it. [Holocron drops](holocrons.md) already
give players holocrons and the hub already stores them; they cannot be opened
yet. Opening is where cards come in. Until a section below says built, no player
text may promise cards.

A card is a collectible with a picture and a few lines of lore: a place, a
personality, something from Jedi Academy itself, a player of the community.
Collecting them grants nothing: no power, setting, cosmetic or right, as with
medals ([identity.md](identity.md#medals)). There is no trading and never any
real money in the system.

## Decisions

| Question | Decision |
| --- | --- |
| What opening gives | One card per holocron opened. |
| What the holocron's tier does | A higher tier raises the odds of rarer cards and of better finishes ([Odds](#odds)). |
| Finishes | Normal, Holo, Solar (the rarest: a holo with an animated solar-effects shader, in the spirit of Sol JK). |
| Editions | An edition is a stamp apart from the finish: **Alpha Edition** for a card opened while the alpha lasts, so a card can be Alpha and Holo. |
| Duplicates | Kept and shown as a count ("x3"), per card, finish and edition. A use for them (crafting toward better finishes) is a later decision. |
| Drop rates | Start stingy; buffing later is easy on the hub and nerfing is not. |
| Where the data lives | The hub holds the catalogue for rolling; names, lore and art come in asset packs, so a new set needs no client release ([Data](#data)). |
| Money, trading | None. Free, non-commercial, no trading in the first version. |

## Card data

### Data

The hub needs to know which cards exist and how likely each is, to roll one: a
catalogue file loaded at start (`cards/<set>.json` beside the hub's assets), not
code, so adding a set is a data change and a deploy. Per card: `id` (1 to 32 of
`a` to `z`, `0` to `9`, `_`; never changes meaning), `set`, `rarity`
(`common`, `uncommon`, `rare`, `legendary`) and the optional `community` flag.

The client reads the card's face from an asset pack
([identity.md](identity.md#asset-packs)), `sjk_cards`: `cards/<id>.card`, a JSON
object parsed strictly like a [blade-skin
file](unlockables.md#blade-skin-files) (an unknown or missing field refuses the
file and the log names it), holding `name` (at most 40 characters), `kind`
(`place`, `personality`, `game`, `community`), `lore` (at most 400 characters of
plain text in the bio's alphabet) and `art` (a path to the picture in the pack,
PNG or JPEG, at most 1024 on a side). A card whose file is not in a pack yet shows
as "Unknown card" with its id, never an error.

### Sets

A set is a group of cards with a completion count ("Places 7 of 12"). Suggested
first sets, about 20 to 30 cards in all for a launch:

- **Places**: planets, the Jedi Academy, Yavin 4, and the game's own maps.
- **Personalities**: figures of the lore.
- **Jedi Academy**: Force powers, saber forms, hilts and moments of the game.
- **Community**: players and servers of the community, with their consent.

### Rarity, finish, edition

- A card's *rarity* is its place in the catalogue (common to legendary) and
  decides how often it is picked.
- Its *finish* is rolled at opening: Normal, Holo (a rainbow sheen over the face,
  tilting with the pointer) or Solar (an animated sunlit shimmer and a warm
  corona). Finish is a property of the copy, not of the card.
- Its *edition* is the stamp of the era it was opened in. The hub reads the
  current edition from its configuration; the operator changes it when the alpha
  ends, and copies already opened keep theirs.

A copy is identified by card, finish and edition; a player holds a count of each.

## Odds

Starting numbers, all in the hub and changed without a client release. The
holocron's tier picks the row.

Rarity of the card:

| Holocron | Common | Uncommon | Rare | Legendary |
| --- | --- | --- | --- | --- |
| Uncommon | 70 % | 24 % | 5.5 % | 0.5 % |
| Rare | 50 % | 33 % | 15 % | 2 % |
| Legendary | 30 % | 38 % | 26 % | 6 % |
| Mythical | 10 % | 30 % | 45 % | 15 % |

Finish of the copy:

| Holocron | Normal | Holo | Solar |
| --- | --- | --- | --- |
| Uncommon | 97 % | 3 % | 0 % |
| Rare | 90 % | 10 % | 0 % |
| Legendary | 70 % | 29 % | 1 % |
| Mythical | 40 % | 55 % | 5 % |

A card is picked among those of the rolled rarity, equally; a rarity with no card
in the catalogue falls to the next one down. The hub rolls with its operating
system's entropy, as holocron drops do, so a client can only open what it holds.

## Opening

Planned flow, all decided by the hub:

1. `POST /v1/holocrons/<id>/open` (signed, own key). The hub checks the holocron
   is the key's and unopened, rolls card, finish and edition, stores the copy and
   marks the holocron opened (the `opened` column of `holocrons` is already
   reserved), in one transaction, so a repeat or a crash never opens it twice or
   loses the card. It answers the copy and the new count.
2. The profile lists the key's cards (`cards`: id, finish, edition, count, first
   obtained) and `GET /v1/cards` the catalogue's ids with their set and rarity, so
   a client can show what is missing without knowing more than the ids.
3. The client shows an opening ceremony in the holocron's tier colour, then the
   card (the tab's look below).

Opening one holocron at a time keeps the ceremony simple; "open all" is a later
convenience that would play a shorter one.

## In the client

- The Collection screen gets a Cards tab: a binder by set with progress, owned
  cards shown by their best finish with the count, missing ones as dark outlines
  with their rarity. A card's detail view shows the face large with its lore, its
  finishes owned and its editions.
- Holo and Solar are drawn by the UI renderer from the card's art with a tilt
  from the pointer; Solar adds a slow animated sunlit pass. They are shaders, not
  extra art. One picture per card serves every finish.
- The Holocrons tab gets an Open button for the holocron kinds held, once opening
  exists. Its text still promises nothing it does not do.
- Set completion can feed achievements the hub counts itself
  ([identity.md](identity.md#achievements)).

## The art and the rights

Sol makes the pictures, with AI tools, in one shared style so a set looks like a
set. Cards of the lore draw on Lucasfilm and Disney properties and Jedi Academy
itself, so the system stays strictly non-commercial. AI-made art is generally not
protected by copyright, so these packs cannot carry the "all rights reserved"
claim the blade skins' art does; the credits say the pictures are AI-made.

Community cards use a player's name and picture, or a stylised emblem of their
name, only with their consent, never an AI likeness of a real person's face. The
Staff page will need a way to remove a community card at the player's request.

## Privacy

A key's cards would be public like its medals: anyone can read a profile. The
client sends nothing about cards. The hub stores which copies a key holds and when
each was opened.

## Order of work

1. Hub: card catalogue loading, the `cards` table, `open`, profile fields and
   `GET /v1/cards`, tests for the odds and idempotence; a staff gift of a card.
2. Client: wire types, the card file format and pack, the Cards tab, the opening
   ceremony and `debug_card` rehearsal.
3. First set's art and lore as the `sjk_cards` pack.
4. Holo and Solar shaders in the UI renderer.
