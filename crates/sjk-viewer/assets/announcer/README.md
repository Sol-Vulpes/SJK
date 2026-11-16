# Announcer voices

The feats' announcer (`cg_feats`, [client.md](../../../../docs/client.md#feats)) speaks with
a voice pack: a folder `sound/sjk/announcer/<pack>/` with one sound per feat, `.wav`, `.mp3`
or `.ogg`. `cg_announcer` (Settings > HUD > "Announcer voice") names the pack. `sjk` is
SJK's own, bundled in `sjk/` here and mounted below all game data, so a PK3 can replace its
lines one by one with the same paths, or bring a whole new pack in a folder of its own. A
line a pack lacks is taken from `sjk`; a line neither has stays silent.

To make a pack, put any of these files in `sound/sjk/announcer/<your pack>/` in a PK3:

| File (any of the three extensions) | Line |
| --- | --- |
| `hooked` | "Hooked!" (a kill with JA+'s grapple hook) |
| `first_blood` | "First blood!" |
| `double_kill` | "Double kill!" |
| `triple_kill` | "Triple kill!" |
| `quad_kill` | "Quad kill!" |
| `penta_kill` | "Penta kill!" |
| `massacre` | "Massacre!" (six kills or more in a row) |
| `killing_spree` | "Killing spree!" (5 kills without dying) |
| `rampage` | "Rampage!" (10) |
| `dominating` | "Dominating!" (15) |
| `unstoppable` | "Unstoppable!" (20) |
| `godlike` | "Godlike!" (25) |
| `spree_ended` | "Spree ended!" (you ended someone's spree) |
| `revenge` | "Revenge!" (you killed your last killer) |

Keep each line short (under about 1.3 s: lines are spaced that far apart) and mono.

SJK's own lines are not recorded yet: until they are, the `sjk` pack is empty and only
packs from PK3s speak.
