//! The notification banner: a banner that comes down from the top of the screen when
//! something comes to the player ([`Unlock`]), rests a few seconds and goes back up, as a
//! phone's notifications do (16/11/2026, Sol's request; its look and motion are
//! [`banner`]). It plays the single-player game's secret-area sound (`audio/ui_cues.rs`,
//! off with `cg_achievementSound 0`) for a reward. What it announces:
//!
//! - an achievement unlocked (`achievements_frame.rs`, `toy_master.rs`), with the board's
//!   medallion (`achievements/medallion.rs`), its name, what it asked and the dormant kyber
//!   it brings;
//! - a new medal during a match (`medal_popup.rs`), with the medallion's picture, saying
//!   that the game menu shows it: the medal's ceremony waits for the game menu, and takes
//!   the medal's banners away as it opens ([`UnlockToast::withdraw`]);
//! - a new holocron or dormant kyber (`holocron_banners.rs`), anywhere, with the tier's
//!   picture (its gem without one) in the tier's colour, where it came from and where it
//!   waits (the Collection's Holocrons page); it is the only thing that tells of one, as
//!   the large pop-up that waited for the game menu is gone (16/11/2026);
//! - a contributor reward waiting (`official.rs`), a saber kyber on the wishlist in the
//!   shop today (`wishlist.rs`), a rare catch of the Fishing or Mining toy
//!   (`catch_book.rs`), a kyber glint they found and a daily catch goal done
//!   (`fishing_sell.rs`; centre prints until 16/11/2026), a request from the SJK team for
//!   the game's logs (`log_request_card.rs`, whose consent card still waits for a menu
//!   and takes the banner back as it opens);
//! - a newer SJK release the update check found (`update.rs`), once a session, with SJK's
//!   emblem, the version and "Click to install", in the SJK UI's holo blue and without the
//!   sound; the Update page opening takes it away;
//! - a content collection's automatic update that changed files
//!   (`console_jof_content.rs`), with SJK's emblem and what came, in holo blue and without
//!   the sound.
//!
//! One banner shows at a time, the others queue. Several of one kind waiting together
//! (achievements, medals, holocrons, dormant kybers: [`GROUP_AT`] or more) fold into one
//! banner that counts them ([`Unlock::Many`]: "5 new holocrons", in the rarest one's
//! colour), so a profile read for the first time (a reinstall finds up to twenty
//! holocrons) is one banner, not a minute and a half of them.
//!
//! It is not modal: it pauses nothing, over play as over the menus (the main menu too), and
//! takes no keys. The one input it takes is a click on an update's banner while the pointer
//! is free ([`crate::GpuState::update_card_pointer`]), which starts the download and opens
//! the Update page.
//!
//! The top centre is free in play: the HUDs' gauges sit in the bottom corners, timers at
//! the top right, notify lines at the top left, centre prints and the crosshair lower
//! (`version_overlay.rs`), so the banner covers neither the aim nor the chat; the top
//! centre's vote and warmup lines and the feats' medals (`hud/feats.rs`) are a moment
//! under it. It waits while the console is open, a large pop-up shows or the window is
//! away (alt-tabbed out or minimised), and its clock runs only while it is drawn, so a
//! banner is never spent unseen: one hidden half way resumes where it was, and a long
//! frame (a hitch, a map loading, a minimised window drawing nothing) moves it on by at
//! most [`MAX_STEP`]. It draws nothing, nor allocates, while no banner waits; its words are
//! written once as it starts, into strings it keeps.

pub(crate) mod banner;

use crate::achievements::Kind;
use crate::achievements::medallion::tint;
use crate::audio::ui_cues::{self, Cue};
use crate::holocrons::{Tier, icons};
use crate::medals::Medal;
use crate::menu::sjk::color;
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextStyle, TextVertex, UiFont};
use crate::update::Version;
use banner::Icon;
use sjk_ui::{Color, InputEvent, Rect, Vec2};
use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

/// The cvar that plays the sound with each reward's banner (1, the default) or not (0).
pub(crate) const SOUND_CVAR: &str = "cg_achievementSound";

/// From how many of one kind waiting together they share one banner ([`Unlock::Many`]).
pub(crate) const GROUP_AT: u32 = 3;
/// Where a medal's banner says it is shown.
pub(crate) const MENU_HINT: &str = "Open the game menu to see it";
/// The same for several medals.
const MENU_HINT_MANY: &str = "Open the game menu to see them";
/// Where a holocron's banner says it waits, and a dormant kyber's; then for several.
const HOLOCRON_HINT: &str = "See it in Collection > Holocrons";
const HOLOCRONS_HINT: &str = "See them in Collection > Holocrons";
const KYBER_HINT: &str = "Attune it in Collection > Holocrons";
const KYBERS_HINT: &str = "Attune them in Collection > Holocrons";
/// Where several achievements' banner says they are.
const ACHIEVEMENTS_HINT: &str = "See them in Collection > Achievements";
/// What a click on the update banner does ([`UnlockToast::update_card_at`]).
pub(crate) const UPDATE_HINT: &str = "Click to install";
/// The same for a folder that cannot update itself: the click opens the release page.
pub(crate) const UPDATE_MANUAL_HINT: &str = "Click for its page";
/// The console command that shows the update banner for a made-up release.
pub(crate) const UPDATE_COMMAND: &str = "debug_update";
/// Its line in the command list and browser.
pub(crate) const UPDATE_HELP: &str =
    "Rehearse the update banner, nothing checked or installed: debug_update [version] [manual]";
/// The release `debug_update` names when it is given none.
const REHEARSED_VERSION: &str = "2026.1231.1";

/// Pause between two banners, once the last has gone back up.
const GAP: Duration = Duration::from_millis(250);
/// The most one frame moves a banner's clock, in seconds.
pub(crate) const MAX_STEP: f32 = 0.1;

/// Where a banner is in its life.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    Entering,
    Shown,
    Leaving,
    Gone,
}

impl Phase {
    /// The phase `t` seconds after the banner began; `reduced` with `ui_rarityEffects 0`.
    pub(crate) fn at(t: f32, reduced: bool) -> Self {
        let enter = if reduced {
            banner::FADE_IN
        } else {
            banner::ENTER
        };
        if t < enter {
            Self::Entering
        } else if t < banner::leaves_at(reduced) {
            Self::Shown
        } else if t < banner::lifetime(reduced) {
            Self::Leaving
        } else {
            Self::Gone
        }
    }
}

/// What a banner announces.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Unlock {
    /// An achievement unlocked; `crystal` when the hub unlocked it, so its kyber crystal
    /// comes (without a hub it unlocks on this client and brings none).
    Achievement { kind: &'static Kind, crystal: bool },
    /// The SJK team gave a medal, `count` times in all for a repeatable one; `promoted`
    /// when it was held already and its rank rose to this one.
    Medal {
        medal: Medal,
        count: u32,
        promoted: Option<crate::medals::Rank>,
    },
    /// A holocron dropped, numbered `id` at the hub; `gift` when staff gave it. A kyber
    /// crystal ([`crate::kyber`]) arrives the same way, `earned` saying from what.
    /// `rehearsal` when `debug_holocron` made it up: it is never counted as seen.
    Holocron {
        tier: &'static Tier,
        id: u64,
        gift: bool,
        earned: Option<crate::kyber::Earned>,
        rehearsal: bool,
    },
    /// A newer SJK release is out; `manual` when this folder cannot install it itself
    /// (the Update page then offers the release page).
    Update { version: Version, manual: bool },
    /// A contributor reward (`official.rs`) waits for the player, numbered `id` at the
    /// hub: the claim window waits for the game menu.
    Reward { id: u64 },
    /// A content collection (`title`: JoF content, the Sith cloud) updated itself:
    /// `files` downloaded, `bytes` in all, and `removed` the server no longer lists.
    Content {
        title: &'static str,
        files: u32,
        bytes: u64,
        removed: u32,
    },
    /// A saber shader on the player's wishlist is among the shop's offers today
    /// (`wishlist.rs`, 10/11/2026), told once a shop day.
    Wished {
        skin: &'static crate::unlockables::Unlockable,
    },
    /// A rare fish landed or a rare ore dug (`mining`) by the player's toy
    /// ([`crate::catch_book`]): the catch's `name` and how many the log now holds.
    RareCatch {
        name: &'static str,
        count: u32,
        mining: bool,
    },
    /// The Fishing or Mining (`mining`) toy found a kyber glint: `shards` kyber shards,
    /// the player holding `balance` after (`fishing_sell.rs`).
    KyberGlint {
        shards: u32,
        balance: i64,
        mining: bool,
    },
    /// The day's Fishing or Mining (`mining`) goal is done: it won an Echo.
    CatchGoal { mining: bool },
    /// The SJK team asks for the game's logs (request `id` at the hub): the consent card
    /// waits for a menu (`log_request_card.rs`).
    LogRequest { id: i64 },
    /// `count` of one kind that waited together, on one banner ([`GROUP_AT`]).
    Many { many: Many, count: u32 },
}

/// What several on one banner are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Many {
    /// Achievements; `crystals` when one of them brings a dormant kyber.
    Achievements { crystals: bool },
    /// Medals, new ones or promotions.
    Medals,
    /// Holocrons, or dormant kybers when `best` is a kyber's tier: the rarest tier among
    /// them, whether every one is of it, the highest hub number among them and whether
    /// `debug_holocron` made them up.
    Holocrons {
        best: &'static Tier,
        alike: bool,
        top: u64,
        rehearsal: bool,
    },
}

/// Which banners fold together.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fold {
    Achievements,
    Medals,
    Holocrons { kyber: bool, rehearsal: bool },
}

impl Unlock {
    /// The same arrival: the same achievement, medal and count, or holocron.
    fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Achievement { kind: a, .. }, Self::Achievement { kind: b, .. }) => a.id == b.id,
            (
                Self::Medal {
                    medal,
                    count,
                    promoted,
                },
                Self::Medal {
                    medal: other,
                    count: other_count,
                    promoted: other_promoted,
                },
            ) => medal == other && count == other_count && promoted == other_promoted,
            (Self::Holocron { id, .. }, Self::Holocron { id: other, .. }) => id == other,
            (Self::Update { version, .. }, Self::Update { version: other, .. }) => version == other,
            (Self::Reward { id }, Self::Reward { id: other }) => id == other,
            (Self::LogRequest { id }, Self::LogRequest { id: other }) => id == other,
            (Self::CatchGoal { mining }, Self::CatchGoal { mining: other }) => mining == other,
            (Self::Wished { skin }, Self::Wished { skin: other }) => skin.id == other.id,
            (
                Self::Content {
                    title,
                    files,
                    bytes,
                    removed,
                },
                Self::Content {
                    title: other_title,
                    files: other_files,
                    bytes: other_bytes,
                    removed: other_removed,
                },
            ) => {
                (title, files, bytes, removed)
                    == (other_title, other_files, other_bytes, other_removed)
            }
            _ => false,
        }
    }

    /// Which banners this one folds with, if any.
    fn fold(self) -> Option<Fold> {
        match self {
            Self::Achievement { .. }
            | Self::Many {
                many: Many::Achievements { .. },
                ..
            } => Some(Fold::Achievements),
            Self::Medal { .. }
            | Self::Many {
                many: Many::Medals, ..
            } => Some(Fold::Medals),
            Self::Holocron {
                tier, rehearsal, ..
            }
            | Self::Many {
                many:
                    Many::Holocrons {
                        best: tier,
                        rehearsal,
                        ..
                    },
                ..
            } => Some(Fold::Holocrons {
                kyber: tier.is_kyber(),
                rehearsal,
            }),
            _ => None,
        }
    }

    /// How many arrivals the banner tells of.
    pub(crate) const fn count(self) -> u32 {
        match self {
            Self::Many { count, .. } => count,
            _ => 1,
        }
    }

    /// This banner and `other`, which fold together ([`Self::fold`]), as one.
    fn folded(self, other: Self) -> Self {
        let count = self.count() + other.count();
        let many = match (self.as_many(), other.as_many()) {
            (Many::Achievements { crystals: a }, Many::Achievements { crystals: b }) => {
                Many::Achievements { crystals: a || b }
            }
            (
                Many::Holocrons {
                    best: a,
                    alike: a_alike,
                    top: a_top,
                    rehearsal,
                },
                Many::Holocrons {
                    best: b,
                    alike: b_alike,
                    top: b_top,
                    ..
                },
            ) => Many::Holocrons {
                best: if b.index > a.index { b } else { a },
                alike: a_alike && b_alike && a.id == b.id,
                top: a_top.max(b_top),
                rehearsal,
            },
            (many, _) => many,
        };
        Self::Many { many, count }
    }

    /// What this banner is as one of several.
    fn as_many(self) -> Many {
        match self {
            Self::Many { many, .. } => many,
            Self::Holocron {
                tier,
                id,
                rehearsal,
                ..
            } => Many::Holocrons {
                best: tier,
                alike: true,
                top: id,
                rehearsal,
            },
            Self::Achievement { kind, crystal } => Many::Achievements {
                crystals: crystal && crate::kyber::reward(kind.id).is_some(),
            },
            _ => Many::Medals,
        }
    }

    /// A holocron's or a dormant kyber's banner, alone or several.
    pub(crate) const fn is_holocron(self) -> bool {
        matches!(
            self,
            Self::Holocron { .. }
                | Self::Many {
                    many: Many::Holocrons { .. },
                    ..
                }
        )
    }

    /// A medal's banner, alone or several.
    pub(crate) const fn is_medal(self) -> bool {
        matches!(
            self,
            Self::Medal { .. }
                | Self::Many {
                    many: Many::Medals,
                    ..
                }
        )
    }

    /// The highest hub number of the real holocrons (not rehearsed) a banner tells of: the
    /// one counted as seen once it shows.
    pub(crate) const fn holocron_top(self) -> Option<u64> {
        match self {
            Self::Holocron {
                id,
                rehearsal: false,
                ..
            }
            | Self::Many {
                many:
                    Many::Holocrons {
                        top: id,
                        rehearsal: false,
                        ..
                    },
                ..
            } => Some(id),
            _ => None,
        }
    }

    /// The header's capitals.
    pub(crate) const fn kicker(self) -> &'static str {
        match self {
            Self::Achievement { .. } => "ACHIEVEMENT UNLOCKED",
            Self::Medal {
                promoted: Some(_), ..
            } => "MEDAL PROMOTED",
            Self::Medal { .. } => "NEW MEDAL",
            Self::Holocron { tier, .. } if tier.is_kyber() => "NEW DORMANT KYBER",
            Self::Holocron { .. } => "NEW HOLOCRON",
            Self::Update { .. } => "UPDATE AVAILABLE",
            Self::Reward { .. } => "CONTRIBUTOR REWARD",
            Self::Content { .. } => "CONTENT UPDATED",
            Self::Wished { .. } => "ON YOUR WISHLIST",
            Self::RareCatch { mining: false, .. } => "RARE CATCH",
            Self::RareCatch { mining: true, .. } => "RARE FIND",
            Self::KyberGlint { .. } => "KYBER GLINT",
            Self::CatchGoal { .. } => "DAILY GOAL",
            Self::LogRequest { .. } => "LOG REQUEST",
            Self::Many {
                many: Many::Achievements { .. },
                ..
            } => "ACHIEVEMENTS UNLOCKED",
            Self::Many {
                many: Many::Medals, ..
            } => "NEW MEDALS",
            Self::Many {
                many: Many::Holocrons { best, .. },
                ..
            } if best.is_kyber() => "NEW DORMANT KYBERS",
            Self::Many { .. } => "NEW HOLOCRONS",
        }
    }

    /// Whether the banner plays the chime: everything but an update or a content
    /// update, which are news, not rewards, a wishlist's offer and a request for logs,
    /// and the toys' rare catches, glints and goals, which were told without it (a rare
    /// catch's toy plays it among its own sounds, at the toys' volume).
    pub(crate) const fn chimes(self) -> bool {
        !matches!(
            self,
            Self::Update { .. }
                | Self::Content { .. }
                | Self::Wished { .. }
                | Self::RareCatch { .. }
                | Self::KyberGlint { .. }
                | Self::CatchGoal { .. }
                | Self::LogRequest { .. }
        )
    }

    /// The note under the line under the title: the kyber crystal an achievement brings
    /// when the hub unlocked it, else its category; where a medal or a holocron is shown;
    /// what a click on an update does.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Achievement {
                kind,
                crystal: true,
            } => crate::kyber::reward(kind.id)
                .map_or(kind.category.name(), crate::kyber::reward_line),
            Self::Achievement { kind, .. } => kind.category.name(),
            Self::Medal { .. } => MENU_HINT,
            Self::Holocron { tier, .. } if tier.is_kyber() => KYBER_HINT,
            Self::Holocron { .. } => HOLOCRON_HINT,
            Self::Update { manual: false, .. } => UPDATE_HINT,
            Self::Update { manual: true, .. } => UPDATE_MANUAL_HINT,
            Self::Reward { .. } => "From the SJK team",
            Self::Content { .. } => "Automatic update",
            Self::Wished { .. } => "Collection, Forge, Shop",
            Self::RareCatch { mining: false, .. } => "fishing_brag shares it",
            Self::RareCatch { mining: true, .. } => "mining_brag shares it",
            Self::KyberGlint { mining: false, .. } => "Found while fishing",
            Self::KyberGlint { mining: true, .. } => "Found while mining",
            Self::CatchGoal { .. } => "A new goal tomorrow",
            Self::LogRequest { .. } => "Open the menu to answer",
            Self::Many {
                many: Many::Achievements { .. },
                ..
            } => ACHIEVEMENTS_HINT,
            Self::Many {
                many: Many::Medals, ..
            } => MENU_HINT_MANY,
            Self::Many {
                many: Many::Holocrons { best, .. },
                ..
            } if best.is_kyber() => KYBERS_HINT,
            Self::Many { .. } => HOLOCRONS_HINT,
        }
    }

    /// The banner's colour: its accent, its icon's tile and glow, its header.
    pub(crate) fn hue(self) -> Color {
        match self {
            Self::Achievement { kind, .. } => tint(kind.category),
            Self::Medal { .. }
            | Self::Reward { .. }
            | Self::RareCatch { .. }
            | Self::CatchGoal { .. }
            | Self::Many {
                many: Many::Achievements { .. } | Many::Medals,
                ..
            } => color::GOLD,
            Self::Holocron { tier, .. } => tier.colour,
            Self::Many {
                many: Many::Holocrons { best, .. },
                ..
            } => best.colour,
            Self::Wished { skin } => skin.tier.colour(),
            Self::Update { .. }
            | Self::Content { .. }
            | Self::KyberGlint { .. }
            | Self::LogRequest { .. } => color::HOLO,
        }
    }

    /// The note's colour: the crystal an achievement brings in the crystal's colour,
    /// else quiet.
    fn note_colour(self) -> Color {
        match self {
            Self::Achievement { kind, .. } => crate::kyber::reward(kind.id)
                .map_or(color::MUTED, |tier| banner::light(tier.colour, 0.3)),
            _ => color::MUTED,
        }
    }

    /// What the icon's tile holds: an achievement's medallion, a medal's medallion
    /// picture, a holocron's or dormant kyber's picture (its gem where it is missing),
    /// the toy's for a rare catch, SJK's emblem for the rest.
    fn icon(self) -> Icon {
        let logo = Icon::Picture(crate::ui_renderer::LOGO_TEXTURE, 0.7);
        match self {
            Self::Achievement { kind, .. } => Icon::Medallion(kind),
            Self::Medal { medal, .. } => Icon::Picture(medal.icon(), 0.86),
            Self::Holocron { tier, .. }
            | Self::Many {
                many: Many::Holocrons { best: tier, .. },
                ..
            } => {
                if icons::is_loaded(tier.slot) {
                    Icon::Picture(icons::texture(tier.slot), 0.96)
                } else {
                    Icon::Gem(tier.colour)
                }
            }
            Self::RareCatch { mining, .. }
            | Self::KyberGlint { mining, .. }
            | Self::CatchGoal { mining } => {
                let toy = if mining { "mining" } else { "fishing" };
                crate::quick_wheel::catalog::icon_index(toy).map_or(logo, |index| {
                    Icon::Picture(crate::ui_renderer::wheel_icon(index), 0.74)
                })
            }
            _ => logo,
        }
    }

    /// The title, with a medal's count when it was given more than once.
    pub(crate) const fn name(self) -> Name {
        Name(self)
    }

    /// The line under the title.
    pub(crate) const fn description(self) -> Description {
        Description(self)
    }
}

/// An unlock's name, written without allocating.
pub(crate) struct Name(Unlock);

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Unlock::Achievement { kind, .. } => formatter.write_str(&crate::terms::say(kind.name)),
            Unlock::Medal { medal, count, .. } if count > 1 => {
                write!(formatter, "{} x{count}", medal.name())
            }
            Unlock::Medal { medal, .. } => formatter.write_str(medal.name()),
            Unlock::Holocron { tier, .. } => formatter.write_str(&tier.title()),
            Unlock::Update { version, .. } => write!(formatter, "SJK {}", version.as_str()),
            Unlock::Reward { .. } => formatter.write_str("Contributor reward waiting"),
            Unlock::Content { title, .. } => write!(formatter, "{title} updated"),
            Unlock::Wished { skin } => formatter.write_str(skin.name),
            Unlock::RareCatch { name, .. } => {
                formatter.write_str(crate::catch_book::bare_name(name))
            }
            Unlock::KyberGlint { shards: 1, .. } => formatter.write_str("+1 kyber shard"),
            Unlock::KyberGlint { shards, .. } => write!(formatter, "+{shards} kyber shards"),
            Unlock::CatchGoal { mining: false } => {
                formatter.write_str("Today's fishing goal is done")
            }
            Unlock::CatchGoal { mining: true } => {
                formatter.write_str("Today's mining goal is done")
            }
            Unlock::LogRequest { .. } => formatter.write_str("The SJK team asks for your logs"),
            Unlock::Many {
                many: Many::Achievements { .. },
                count,
            } => write!(formatter, "{count} achievements"),
            Unlock::Many {
                many: Many::Medals,
                count,
            } => write!(formatter, "{count} new medals"),
            Unlock::Many {
                many: Many::Holocrons { best, .. },
                count,
            } if best.is_kyber() => write!(formatter, "{count} new dormant kybers"),
            Unlock::Many { count, .. } => write!(formatter, "{count} new holocrons"),
        }
    }
}

/// The line under an unlock's name, written without allocating: what an achievement
/// asked, what a medal is for, where a holocron came from and how rare it is.
pub(crate) struct Description(Unlock);

impl fmt::Display for Description {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Unlock::Achievement { kind, .. } => {
                formatter.write_str(&crate::terms::say(kind.description))
            }
            Unlock::Medal {
                promoted: Some(rank),
                ..
            } => write!(formatter, "Promoted to {}", rank.title()),
            Unlock::Medal { medal, .. } => formatter.write_str(medal.description()),
            Unlock::Holocron { gift: true, .. } => formatter.write_str("A gift from the SJK team"),
            Unlock::Holocron {
                earned: Some(earned),
                ..
            } => write!(formatter, "{earned}"),
            Unlock::Holocron { tier, .. } if tier.is_kyber() => {
                formatter.write_str(crate::kyber::ABOUT)
            }
            Unlock::Holocron { tier, .. } => write!(
                formatter,
                "Found while playing: {} of drops are {}",
                tier.odds(),
                tier.label
            ),
            Unlock::Update { manual: false, .. } => {
                formatter.write_str("Downloads now; it starts when SJK restarts")
            }
            Unlock::Update { manual: true, .. } => {
                formatter.write_str("This folder cannot update itself: see its page")
            }
            Unlock::Reward { .. } => formatter.write_str("Open the game menu to claim it"),
            Unlock::Wished { .. } => {
                formatter.write_str("A kyber on your wishlist is in the shop today")
            }
            Unlock::RareCatch { count: 0 | 1, .. } => {
                formatter.write_str("Your first! New in your log book")
            }
            Unlock::RareCatch { count, .. } => write!(
                formatter,
                "Your {} in the log book",
                crate::catch_book::ordinal(count)
            ),
            Unlock::Content {
                files,
                bytes,
                removed,
                ..
            } => {
                if files > 0 {
                    let plural = if files == 1 { "" } else { "s" };
                    write!(formatter, "{files} file{plural}, ")?;
                    write_size(formatter, bytes)?;
                    if removed > 0 {
                        write!(formatter, ", {removed} removed")?;
                    }
                } else {
                    let plural = if removed == 1 { "" } else { "s" };
                    write!(formatter, "{removed} old file{plural} removed")?;
                }
                formatter.write_str("  ·  from the next map")
            }
            Unlock::KyberGlint {
                balance, mining, ..
            } => write!(
                formatter,
                "A kyber glint in the {}! You hold {balance} now",
                if mining { "rock" } else { "water" }
            ),
            Unlock::CatchGoal { .. } => formatter.write_str("An Echo is yours!"),
            Unlock::LogRequest { .. } => {
                formatter.write_str("Nothing leaves your PC unless you send them")
            }
            Unlock::Many {
                many: Many::Achievements { crystals: true },
                ..
            } => formatter.write_str("Each brings a dormant kyber"),
            Unlock::Many {
                many: Many::Achievements { crystals: false },
                ..
            } => formatter.write_str("Unlocked together just now"),
            Unlock::Many {
                many: Many::Medals, ..
            } => formatter.write_str("From the SJK team"),
            Unlock::Many {
                many: Many::Holocrons {
                    best, alike: true, ..
                },
                ..
            } => write!(formatter, "Every one {} {}", best.article(), best.name),
            Unlock::Many {
                many: Many::Holocrons { best, .. },
                ..
            } => write!(formatter, "The rarest: {} {}", best.article(), best.name),
        }
    }
}

/// "1.4 GB", "75 MB", "820 KB" (decimal units, as the content card counts),
/// written without allocating.
fn write_size(formatter: &mut fmt::Formatter<'_>, bytes: u64) -> fmt::Result {
    const KB: f64 = 1000.0;
    let value = bytes as f64;
    if value >= KB * KB * KB {
        write!(formatter, "{:.1} GB", value / (KB * KB * KB))
    } else if value >= KB * KB {
        write!(formatter, "{:.0} MB", value / (KB * KB))
    } else if value >= KB {
        write!(formatter, "{:.0} KB", value / KB)
    } else {
        write!(formatter, "{bytes} B")
    }
}

/// The words of the banner showing, written once as it starts into strings kept from one
/// banner to the next, in the player's terms ([`crate::terms`]).
#[derive(Default)]
struct Words {
    header: String,
    title: String,
    body: String,
    note: String,
    /// How many, for several of one kind; empty for one.
    badge: String,
}

impl Words {
    fn write(&mut self, unlock: Unlock) {
        use std::fmt::Write as _;
        for text in [
            &mut self.header,
            &mut self.title,
            &mut self.body,
            &mut self.note,
            &mut self.badge,
        ] {
            text.clear();
        }
        let _ = write!(self.header, "{}", unlock.kicker());
        let _ = write!(self.title, "{}", unlock.name());
        let _ = write!(self.body, "{}", unlock.description());
        self.note.push_str(unlock.tag());
        if let Unlock::Many { count, .. } = unlock {
            let _ = write!(self.badge, "{count}");
        }
        if crate::terms::renamed() {
            for text in [
                &mut self.header,
                &mut self.title,
                &mut self.body,
                &mut self.note,
            ] {
                *text = crate::terms::say_owned(std::mem::take(text));
            }
        }
    }
}

/// What [`banner::draw`] draws: the words, colours and icon of the banner showing.
pub(crate) struct Look<'a> {
    pub(crate) header: &'a str,
    pub(crate) title: &'a str,
    pub(crate) body: &'a str,
    pub(crate) note: &'a str,
    pub(crate) badge: &'a str,
    pub(crate) hue: Color,
    pub(crate) note_colour: Color,
    pub(crate) icon: Icon,
}

/// The banner showing and how long it has been seen.
#[derive(Clone, Copy)]
struct Showing {
    unlock: Unlock,
    /// Seconds it has been drawn.
    shown: f32,
    /// The frame that last moved it on.
    last: Instant,
    /// It fades in place rather than sliding (`ui_rarityEffects 0` as it began).
    reduced: bool,
}

/// The banners' state: those waiting, the one showing and its canvas.
pub(crate) struct UnlockToast {
    canvas: MenuCanvas,
    queue: VecDeque<Unlock>,
    current: Option<Showing>,
    words: Words,
    /// When the last banner ended, for the pause before the next.
    ended: Option<Instant>,
    /// The banner that began since [`Self::take_started`] was last asked.
    started: Option<Unlock>,
    /// Banners begun now fade in place (`ui_rarityEffects 0`, [`Self::set_reduced`]).
    reduced: bool,
    /// The update state's generation last read (`update::generation`), and the release
    /// whose banner was given this session.
    update_seen: u32,
    update_given: Option<Version>,
    /// Where the banner showing was laid out this frame, for a click on an update's
    /// banner; `None` while none draws.
    card: Option<Rect>,
    /// A moment held still (seconds after the start), for the off-screen shots.
    #[cfg(test)]
    held: Option<f32>,
}

impl Default for UnlockToast {
    fn default() -> Self {
        Self {
            // Six text runs and about forty shapes a frame, a hundred with a gem.
            canvas: MenuCanvas::with_capacities(10, 128, 160),
            queue: VecDeque::with_capacity(4),
            current: None,
            words: Words::default(),
            ended: None,
            started: None,
            reduced: false,
            update_seen: 0,
            update_given: None,
            card: None,
            #[cfg(test)]
            held: None,
        }
    }
}

impl UnlockToast {
    /// `unlock` came: its banner waits for those before it. One already waiting or showing
    /// is not queued twice, and one that folds with others waiting
    /// ([`GROUP_AT`] or more of one kind) joins them on one banner.
    pub(crate) fn push(&mut self, unlock: Unlock) {
        let showing = self
            .current
            .is_some_and(|current| current.unlock.same(unlock));
        if showing || self.queue.iter().any(|waiting| waiting.same(unlock)) {
            return;
        }
        if let Some(fold) = unlock.fold() {
            let together: u32 = self
                .queue
                .iter()
                .filter(|waiting| waiting.fold() == Some(fold))
                .map(|waiting| waiting.count())
                .sum();
            if together > 0 && together + unlock.count() >= GROUP_AT {
                // The first of them takes the others in, where it waited.
                let mut first: Option<(usize, Unlock)> = None;
                let mut index = 0;
                self.queue.retain(|waiting| {
                    let at = index;
                    index += 1;
                    if waiting.fold() != Some(fold) {
                        return true;
                    }
                    match &mut first {
                        None => {
                            first = Some((at, *waiting));
                            true
                        }
                        Some((_, group)) => {
                            *group = group.folded(*waiting);
                            false
                        }
                    }
                });
                if let Some((at, group)) = first {
                    self.queue[at] = group.folded(unlock);
                }
                return;
            }
        }
        self.queue.push_back(unlock);
    }

    /// Take back the banners `which` picks, waiting or showing, at `now`.
    pub(crate) fn withdraw(&mut self, now: Instant, which: impl Fn(Unlock) -> bool) {
        self.queue.retain(|waiting| !which(*waiting));
        if self.current.is_some_and(|showing| which(showing.unlock)) {
            self.current = None;
            self.ended = Some(now);
        }
    }

    /// The update state moved to `generation`: when `available` (read only then) names a
    /// newer release, give its banner, once a session for each release, unless the Update
    /// page shows it (`page_open`), which also takes a banner still waiting or showing
    /// back.
    pub(crate) fn offer_update(
        &mut self,
        now: Instant,
        generation: u32,
        page_open: bool,
        available: impl FnOnce() -> Option<(String, bool)>,
    ) {
        if page_open && self.pending() > 0 {
            self.withdraw(now, |unlock| matches!(unlock, Unlock::Update { .. }));
        }
        if generation == self.update_seen {
            return;
        }
        self.update_seen = generation;
        let Some((text, manual)) = available() else {
            return;
        };
        let Some(version) = Version::new(&text) else {
            return;
        };
        if self.update_given == Some(version) {
            return;
        }
        self.update_given = Some(version);
        if !page_open {
            self.push(Unlock::Update { version, manual });
        }
    }

    /// Banners waiting, the one showing included.
    pub(crate) fn pending(&self) -> usize {
        self.queue.len() + usize::from(self.current.is_some())
    }

    /// Whether banners begun from now on fade in place rather than slide
    /// (`ui_rarityEffects 0`).
    pub(crate) fn set_reduced(&mut self, reduced: bool) {
        self.reduced = reduced;
    }

    /// The banner that began since this was last asked, once.
    pub(crate) fn take_started(&mut self) -> Option<Unlock> {
        self.started.take()
    }

    /// Seconds the banner showing has been seen.
    fn elapsed(&self, showing: Showing) -> f32 {
        #[cfg(test)]
        if let Some(held) = self.held {
            return held;
        }
        showing.shown
    }

    /// [`Self::update_holding`] with no banner held, for the tests and off-screen shots.
    #[cfg(test)]
    pub(crate) fn update(&mut self, now: Instant, may_show: bool, sound: bool) -> bool {
        self.update_holding(now, may_show, |_| false, sound)
    }

    /// Move the banners on to `now`: the one showing by the frame's time while it
    /// `may_show` (not under the console or a large pop-up, the window not away), at most
    /// [`MAX_STEP`]; end it once its time is up, and start the next one waiting when it
    /// may show, with its `sound` where it chimes. Returns whether a banner draws this
    /// frame. The banners `held` picks also wait (a private duel's, [`held_in_duel`]): one
    /// showing stands still unseen, and the first waiting banner not held starts before
    /// those held ahead of it, which keep their place.
    pub(crate) fn update_holding(
        &mut self,
        now: Instant,
        may_show: bool,
        held: impl Fn(Unlock) -> bool,
        sound: bool,
    ) -> bool {
        if let Some(showing) = &mut self.current {
            let step = now
                .saturating_duration_since(showing.last)
                .as_secs_f32()
                .min(MAX_STEP);
            showing.last = now;
            if may_show && !held(showing.unlock) {
                showing.shown += step;
            }
        }
        if let Some(showing) = self.current
            && Phase::at(self.elapsed(showing), showing.reduced) == Phase::Gone
        {
            self.current = None;
            self.ended = Some(now);
        }
        if self.current.is_none()
            && may_show
            && let Some(next) = self.queue.iter().position(|waiting| !held(*waiting))
        {
            let rested = self
                .ended
                .is_none_or(|ended| now.saturating_duration_since(ended) >= GAP);
            if rested && let Some(unlock) = self.queue.remove(next) {
                self.current = Some(Showing {
                    unlock,
                    shown: 0.0,
                    last: now,
                    reduced: self.reduced,
                });
                self.words.write(unlock);
                self.started = Some(unlock);
                if sound && unlock.chimes() {
                    ui_cues::post(Cue::Achievement);
                }
            }
        }
        let draws = may_show && self.current.is_some_and(|showing| !held(showing.unlock));
        if !draws {
            self.card = None;
        }
        draws
    }

    /// The update banner under `position` (window pixels) as last laid out: whether this
    /// folder must take the release from its page. `None` off the banner, while it goes
    /// back up, or while no update banner draws.
    pub(crate) fn update_card_at(&self, position: Vec2) -> Option<bool> {
        let showing = self.current?;
        let Unlock::Update { manual, .. } = showing.unlock else {
            return None;
        };
        let leaving = Phase::at(self.elapsed(showing), showing.reduced) == Phase::Leaving;
        (!leaving && self.card?.contains(position)).then_some(manual)
    }

    /// Take the update's banner away at `now`: it was clicked.
    pub(crate) fn dismiss_update(&mut self, now: Instant) {
        self.withdraw(now, |unlock| matches!(unlock, Unlock::Update { .. }));
        self.card = None;
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.canvas.draw_list()
    }

    /// Whether a banner was drawn last frame: it takes the top centre, where the build
    /// label (`version_overlay.rs`) is left out meanwhile, as text would draw over it.
    pub(crate) fn on_screen(&self) -> bool {
        self.card.is_some()
    }

    /// Lay the banner showing out for `viewport` as it stands.
    pub(crate) fn build(&mut self, viewport: [f32; 2]) {
        let Some(showing) = self.current else {
            return;
        };
        let t = self.elapsed(showing);
        self.card = Some(banner::rect(viewport, t, showing.reduced));
        let unlock = showing.unlock;
        let words = &self.words;
        let look = Look {
            header: &words.header,
            title: &words.title,
            body: &words.body,
            note: &words.note,
            badge: &words.badge,
            hue: unlock.hue(),
            note_colour: unlock.note_colour(),
            icon: unlock.icon(),
        };
        banner::draw(&mut self.canvas, &look, t, showing.reduced, viewport);
    }

    /// Append the banner's text: in the SJK UI's families when `fonts` has them, else in
    /// Inter (`font`).
    pub(crate) fn append_text(
        &self,
        fonts: Option<crate::game_font::SjkFonts<'_>>,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        match fonts {
            Some(fonts) => self
                .canvas
                .append_text_families(fonts, viewport, TextStyle::NEUTRAL),
            None => self
                .canvas
                .append_text_styled(vertices, font, viewport, TextStyle::NEUTRAL),
        }
    }
}

impl crate::GpuState {
    /// `debug_update [version] [manual]`: show the update banner for a made-up release, as
    /// the update check would on finding it (`manual`: a folder that cannot update itself),
    /// without checking or installing anything. The console closes so it shows at once.
    pub(crate) fn debug_update_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let text = args.first().map_or(REHEARSED_VERSION, String::as_str);
        let version = Version::new(text)
            .ok_or_else(|| format!("{text} is not a release version, such as 2026.1011.1"))?;
        let manual = args
            .get(1)
            .is_some_and(|word| word.eq_ignore_ascii_case("manual"));
        self.unlock_toast.push(Unlock::Update { version, manual });
        if let Some(console) = &mut self.console {
            console.set_open(false);
        }
        self.sync_cursor_policy();
        Ok(vec![format!(
            "The update banner for SJK {} is queued; nothing is checked or installed",
            version.as_str()
        )])
    }
}

#[cfg(test)]
impl UnlockToast {
    /// Show `unlocks` one after another (folded as they would be), the first held `at`
    /// seconds after it began, sliding (or fading when `reduced`), for the off-screen
    /// shots.
    pub(crate) fn preview_with(unlocks: &[Unlock], at: f32, reduced: bool) -> Self {
        let mut toast = Self {
            reduced,
            ..Self::default()
        };
        for unlock in unlocks {
            toast.push(*unlock);
        }
        toast.held = Some(at);
        let _ = toast.update(Instant::now(), true, false);
        toast
    }

    /// [`Self::preview_with`], sliding.
    pub(crate) fn preview(unlocks: &[Unlock], at: f32) -> Self {
        Self::preview_with(unlocks, at, false)
    }
}

/// Whether a banner may show: not under the console or a large pop-up (`covered`) and
/// the window not away.
fn may_show(covered: bool, console_open: bool, away: bool) -> bool {
    !covered && !console_open && !away
}

/// Whether a private duel (`in_duel`) holds `unlock` back: a reward there is a
/// distraction (hub report #81, Milamber), so the banner waits and plays once the duel
/// ends. Holocron banners show anyway when `holocrons_in_duel` (`cl_holocronNotifications`
/// 2, [`crate::holocron_banners::NOTIFICATIONS_CVAR`]).
fn held_in_duel(unlock: Unlock, in_duel: bool, holocrons_in_duel: bool) -> bool {
    in_duel && !(holocrons_in_duel && unlock.is_holocron())
}

impl crate::GpuState {
    /// A click on the update banner while the pointer is free (over the menus): it starts
    /// the download, as Install does, and opens the Update page with its progress; for a
    /// folder that cannot update itself it opens the release page. Returns whether the
    /// banner took the event.
    pub(crate) fn update_card_pointer(&mut self, event: InputEvent) -> bool {
        let InputEvent::PointerPress {
            position,
            button: sjk_ui::PointerButton::Primary,
        } = event
        else {
            return false;
        };
        if self.pointer_captured {
            return false;
        }
        let Some(manual) = self.unlock_toast.update_card_at(position) else {
            return false;
        };
        self.unlock_toast.dismiss_update(Instant::now());
        let _ = crate::update::click_notice();
        if !manual && let Some(console) = &mut self.console {
            console.open_update_panel();
        }
        self.sync_cursor_policy();
        true
    }

    /// Move the banners on and lay the one showing out over the frame unless `covered`
    /// (the console over the frame, a large pop-up) or the window is away; returns whether
    /// it draws this frame. A newer release the update check found gets its banner here,
    /// and a holocron's banner beginning counts it as seen
    /// ([`crate::holocron_banners::HolocronBanners::shown`]).
    pub(crate) fn append_unlock_toast(&mut self, viewport: [f32; 2], covered: bool) -> bool {
        let now = Instant::now();
        let page_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::update_page_open);
        self.unlock_toast.offer_update(
            now,
            crate::update::generation(),
            page_open,
            crate::update::available,
        );
        if self.unlock_toast.pending() == 0 {
            return false;
        }
        let console = self.console.as_ref();
        let console_open = console.is_some_and(crate::console::ViewerConsole::is_open);
        let away = console.is_some_and(crate::console::ViewerConsole::window_away);
        let sound = console
            .and_then(|console| console.bool_cvar(SOUND_CVAR))
            .unwrap_or(true);
        let in_duel = self
            .live_session
            .as_ref()
            .is_some_and(|session| session.latest_snapshot().player.duel_in_progress());
        let holocrons_in_duel =
            crate::holocron_banners::notification_mode(console) == crate::holocron_banners::ALWAYS;
        // Tactical mode holds every banner in a game until it is off (`tactical.rs`).
        let tactical = self.live_session.is_some() && crate::tactical::hides_fun();
        self.unlock_toast.set_reduced(!crate::rarity_fx::enabled());
        let draws = self.unlock_toast.update_holding(
            now,
            may_show(covered, console_open, away),
            |unlock| tactical || held_in_duel(unlock, in_duel, holocrons_in_duel),
            sound,
        );
        if let Some(started) = self.unlock_toast.take_started() {
            self.holocron_banners.shown(started);
        }
        if !draws {
            return false;
        }
        self.unlock_toast.build(viewport);
        self.unlock_toast.append_text(
            self.game_fonts.sjk(),
            &mut self.text_vertices,
            &self.ui_font,
            viewport,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::achievements::{self, ALL};
    use crate::holocrons::TIERS;

    const LIFETIME: f32 = banner::ENTER + banner::HOLD + banner::LEAVE;

    /// Hub report #81: a private duel holds the rewards back, like the console does;
    /// `cl_holocronNotifications` 2 lets the holocron banners through.
    #[test]
    fn a_private_duel_holds_the_rewards_back() {
        assert!(may_show(false, false, false));
        assert!(!may_show(true, false, false));
        assert!(!may_show(false, true, false));
        assert!(!may_show(false, false, true));
        for unlock in [kind("first_blood"), medal(Medal::BugHunter, 1)] {
            assert!(!held_in_duel(unlock, false, false));
            assert!(held_in_duel(unlock, true, false));
            assert!(held_in_duel(unlock, true, true));
        }
        let several = holocron(1, 0).folded(holocron(2, 1));
        for unlock in [holocron(1, 0), several] {
            assert!(!held_in_duel(unlock, false, false));
            assert!(held_in_duel(unlock, true, false));
            assert!(!held_in_duel(unlock, true, true));
        }
    }

    /// `cl_holocronNotifications` 1: a holocron banner that comes during a private duel
    /// waits unseen, its clock still, and plays in full once the duel ends.
    #[test]
    fn a_holocron_banner_withheld_during_a_duel_shows_after_it() {
        let mut toast = UnlockToast::default();
        toast.push(holocron(9, 1));
        let duel = |unlock| held_in_duel(unlock, true, false);
        let mut now = Instant::now();
        for _ in 0..200 {
            now += Duration::from_millis(50);
            assert!(!toast.update_holding(now, true, duel, true));
        }
        assert!(toast.current.is_none());
        assert!(toast.take_started().is_none(), "not begun");
        assert_eq!(toast.pending(), 1, "it waits through the duel");
        // The duel ends: it shows from its start.
        assert!(toast.update_holding(now, true, |unlock| held_in_duel(unlock, false, false), true));
        assert_eq!(shown(&toast), Some(holocron(9, 1).name().to_string()));
        assert_eq!(toast.current.map(|showing| showing.shown), Some(0.0));
        assert_eq!(toast.take_started().and_then(Unlock::holocron_top), Some(9));
        assert!(toast.take_started().is_none(), "told once");
    }

    /// `cl_holocronNotifications` 2: a holocron banner shows during a duel ahead of the
    /// rewards held before it, which play once the duel ends.
    #[test]
    fn a_holocron_banner_may_pass_the_held_rewards_during_a_duel() {
        let mut toast = UnlockToast::default();
        toast.push(kind("first_blood"));
        toast.push(holocron(9, 1));
        let duel = |unlock| held_in_duel(unlock, true, true);
        let mut now = Instant::now();
        assert!(toast.update_holding(now, true, duel, false));
        assert_eq!(shown(&toast), Some(holocron(9, 1).name().to_string()));
        for _ in 0..((LIFETIME + GAP.as_secs_f32()) / 0.05).ceil() as usize + 2 {
            now += Duration::from_millis(50);
            toast.update_holding(now, true, duel, false);
        }
        assert!(toast.current.is_none(), "the holocron banner ended");
        assert_eq!(toast.pending(), 1, "the achievement waits");
        assert!(toast.update_holding(now, true, |unlock| held_in_duel(unlock, false, true), false));
        assert_eq!(shown(&toast), Some(kind("first_blood").name().to_string()));
    }

    fn kind(id: &str) -> Unlock {
        Unlock::Achievement {
            kind: achievements::find(id).expect("an achievement"),
            crystal: true,
        }
    }

    fn medal(medal: Medal, count: u32) -> Unlock {
        Unlock::Medal {
            medal,
            count,
            promoted: None,
        }
    }

    fn holocron(id: u64, tier: usize) -> Unlock {
        Unlock::Holocron {
            tier: &TIERS[tier],
            id,
            gift: false,
            earned: None,
            rehearsal: false,
        }
    }

    fn rehearsed(id: u64, tier: usize) -> Unlock {
        Unlock::Holocron {
            tier: &TIERS[tier],
            id,
            gift: false,
            earned: None,
            rehearsal: true,
        }
    }

    fn crystal(id: u64, tier: usize) -> Unlock {
        Unlock::Holocron {
            tier: &crate::kyber::TIERS[tier],
            id,
            gift: false,
            earned: Some(crate::kyber::Earned::of("")),
            rehearsal: false,
        }
    }

    /// Frames 50 ms apart from `from` for `seconds`, each told whether the banner
    /// `may_show`; returns the last frame's time and whether it drew.
    fn frames(
        toast: &mut UnlockToast,
        from: Instant,
        seconds: f32,
        may_show: bool,
    ) -> (Instant, bool) {
        let step = Duration::from_millis(50);
        let mut now = from;
        let mut drew = false;
        for _ in 0..(seconds / 0.05).round() as usize {
            now += step;
            drew = toast.update(now, may_show, true);
        }
        (now, drew)
    }

    /// The name of the unlock showing.
    fn shown(toast: &UnlockToast) -> Option<String> {
        toast
            .current
            .map(|showing| showing.unlock.name().to_string())
    }

    /// Every unlock a banner can show: each achievement, each medal once and given many
    /// times, each tier dropped and given, several of each kind.
    fn every_unlock() -> Vec<Unlock> {
        let mut unlocks: Vec<Unlock> = ALL
            .iter()
            .flat_map(|kind| [true, false].map(|crystal| Unlock::Achievement { kind, crystal }))
            .collect();
        for each in Medal::ALL {
            unlocks.push(medal(each, 1));
            if each.repeatable() {
                unlocks.push(medal(each, 128));
            }
        }
        for (index, tier) in TIERS.iter().enumerate() {
            unlocks.push(holocron(index as u64, index));
            unlocks.push(Unlock::Holocron {
                tier,
                id: 100 + index as u64,
                gift: true,
                earned: None,
                rehearsal: false,
            });
        }
        // Each kyber crystal, earned from a named achievement and from an unnamed one.
        for (index, tier) in crate::kyber::TIERS.iter().enumerate() {
            for earned in [
                crate::kyber::Earned::of("kills_1000"),
                crate::kyber::Earned::of(""),
            ] {
                unlocks.push(Unlock::Holocron {
                    tier,
                    id: 200 + index as u64,
                    gift: false,
                    earned: Some(earned),
                    rehearsal: false,
                });
            }
        }
        for (files, removed) in [(312, 4), (0, 1)] {
            unlocks.push(Unlock::Content {
                title: "JoF content",
                files,
                bytes: 4_638_000_000,
                removed,
            });
        }
        for manual in [false, true] {
            unlocks.push(Unlock::Update {
                version: Version::new("2026.1231.12").unwrap(),
                manual,
            });
        }
        // The longest name of a saber shader on the wishlist's banner.
        let longest = crate::unlockables::blade_skins()
            .max_by_key(|skin| skin.name.len())
            .unwrap();
        unlocks.push(Unlock::Wished { skin: longest });
        // The longest catch of each toy, landed first and the 128th time.
        for mining in [false, true] {
            for count in [1, 128] {
                unlocks.push(Unlock::RareCatch {
                    name: "Blue-Ringed Moonwater Octopus",
                    count,
                    mining,
                });
            }
        }
        unlocks.push(Unlock::Reward { id: 4 });
        for mining in [false, true] {
            unlocks.push(Unlock::KyberGlint {
                shards: 12,
                balance: 1_234_567,
                mining,
            });
            unlocks.push(Unlock::CatchGoal { mining });
        }
        unlocks.push(Unlock::LogRequest { id: 7 });
        // Several of each kind, the counts as high as a reinstall brings.
        for crystals in [true, false] {
            unlocks.push(Unlock::Many {
                many: Many::Achievements { crystals },
                count: 120,
            });
        }
        unlocks.push(Unlock::Many {
            many: Many::Medals,
            count: 20,
        });
        for best in TIERS.iter().chain(&crate::kyber::TIERS) {
            for alike in [true, false] {
                unlocks.push(Unlock::Many {
                    many: Many::Holocrons {
                        best,
                        alike,
                        top: 30,
                        rehearsal: false,
                    },
                    count: 20,
                });
            }
        }
        unlocks
    }

    #[test]
    fn unlocks_queue_and_show_one_after_another_each_with_its_sound() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        toast.push(kind("first_blood"));
        toast.push(holocron(3, 1));
        toast.push(kind("first_blood"));
        assert_eq!(toast.pending(), 2, "an unlock waiting is not queued twice");
        let start = Instant::now();
        assert!(toast.update(start, true, true));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        toast.push(kind("first_blood"));
        assert_eq!(toast.pending(), 2, "nor the one showing");
        let first = shown(&toast);
        let (now, drew) = frames(&mut toast, start, LIFETIME - 0.1, true);
        assert!(drew);
        assert_eq!(shown(&toast), first);
        assert!(ui_cues::take_posted().is_empty(), "one sound per banner");
        // Gone, and the next waits out the pause.
        let (now, drew) = frames(&mut toast, now, 0.15, true);
        assert!(!drew && toast.current.is_none());
        let (now, drew) = frames(&mut toast, now, 0.1, true);
        assert!(!drew);
        let (now, drew) = frames(&mut toast, now, 0.2, true);
        assert!(drew);
        assert_eq!(shown(&toast), Some(holocron(3, 1).name().to_string()));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        let (_, drew) = frames(&mut toast, now, LIFETIME + 0.2, true);
        assert!(!drew);
        assert_eq!(toast.pending(), 0);
    }

    /// With `ui_rarityEffects 0` a banner fades in place, on its own clock: shorter than
    /// a slide, and it keeps the motion it began with.
    #[test]
    fn a_reduced_banner_keeps_its_motion_and_its_own_length() {
        let mut toast = UnlockToast::default();
        toast.set_reduced(true);
        toast.push(kind("first_blood"));
        let start = Instant::now();
        assert!(toast.update(start, true, false));
        toast.set_reduced(false);
        let reduced = banner::lifetime(true);
        let (now, drew) = frames(&mut toast, start, reduced - 0.1, true);
        assert!(drew && toast.current.is_some_and(|showing| showing.reduced));
        let (_, drew) = frames(&mut toast, now, 0.15, true);
        assert!(!drew && toast.current.is_none(), "{reduced}");
        assert_ne!(reduced, LIFETIME);
    }

    /// The banner's clock runs only while it is drawn: hidden (the window away, the
    /// console) it waits where it was, and a long frame counts as one short step.
    #[test]
    fn a_hidden_banner_waits_where_it_was() {
        let mut toast = UnlockToast::default();
        toast.push(kind("first_blood"));
        let start = Instant::now();
        assert!(toast.update(start, true, false));
        let (now, _) = frames(&mut toast, start, 1.0, true);
        let at = toast.current.map(|showing| showing.shown).unwrap();
        assert!((at - 1.0).abs() < 1e-3, "{at}");
        // Alt-tabbed out for a minute: not drawn, not moved on.
        let (now, drew) = frames(&mut toast, now, 60.0, false);
        assert!(!drew);
        assert_eq!(toast.current.map(|showing| showing.shown), Some(at));
        // Back: it goes on from there.
        let (now, drew) = frames(&mut toast, now, 0.5, true);
        assert!(drew);
        let at = toast.current.map(|showing| showing.shown).unwrap();
        assert!((at - 1.5).abs() < 1e-3, "{at}");
        // A minimised window drew nothing for ten seconds: one step.
        assert!(toast.update(now + Duration::from_secs(10), true, false));
        let later = toast.current.map(|showing| showing.shown).unwrap();
        assert!((later - at - MAX_STEP).abs() < 1e-4, "{later}");
        // Nothing waiting starts while hidden.
        let mut idle = UnlockToast::default();
        idle.push(kind("maps_10"));
        assert!(!idle.update(now, false, true));
        assert!(idle.current.is_none());
    }

    #[test]
    fn a_covered_screen_holds_the_queue_and_the_sound_can_be_off() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        toast.push(kind("maps_10"));
        let now = Instant::now();
        assert!(!toast.update(now, false, true));
        assert!(toast.current.is_none() && toast.pending() == 1);
        assert!(ui_cues::take_posted().is_empty());
        assert!(toast.update(now, true, false));
        assert!(ui_cues::take_posted().is_empty(), "cg_achievementSound 0");
    }

    /// Medals and holocrons queue with the achievements, each with the chime; a medal
    /// given again is a new arrival, the same holocron is not.
    #[test]
    fn medals_and_holocrons_queue_with_the_achievements() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        toast.push(medal(Medal::BugHunter, 1));
        toast.push(holocron(7, 2));
        toast.push(kind("first_blood"));
        toast.push(holocron(7, 2));
        toast.push(medal(Medal::BugHunter, 1));
        assert_eq!(toast.pending(), 3);
        let start = Instant::now();
        assert!(toast.update(start, true, true));
        assert_eq!(shown(&toast).as_deref(), Some("Bug Hunter"));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        let (now, drew) = frames(&mut toast, start, LIFETIME + 0.05, true);
        assert!(!drew);
        let (_, drew) = frames(&mut toast, now, 0.3, true);
        assert!(drew);
        assert_eq!(shown(&toast).as_deref(), Some("Legendary Holocron"));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
    }

    /// Two holocrons waiting are two banners; a third makes them one, which counts them,
    /// takes the rarest's colour and the highest number, and takes in those that come
    /// while it waits. Dormant kybers, rehearsals and other kinds fold apart, and the
    /// banner showing never takes any in.
    #[test]
    fn many_of_one_kind_fold_into_one_banner() {
        let mut toast = UnlockToast::default();
        toast.push(holocron(10, 0));
        toast.push(holocron(11, 3));
        assert_eq!(toast.queue.len(), 2, "two are still two");
        toast.push(kind("first_blood"));
        toast.push(holocron(12, 1));
        assert_eq!(
            toast.queue.len(),
            2,
            "three fold into one, where the first waited"
        );
        let Unlock::Many {
            many:
                Many::Holocrons {
                    best,
                    alike,
                    top,
                    rehearsal,
                },
            count: 3,
        } = toast.queue[0]
        else {
            panic!("{:?}", toast.queue[0]);
        };
        assert!(best.id == "mythical" && !alike && top == 12 && !rehearsal);
        assert!(matches!(toast.queue[1], Unlock::Achievement { .. }));
        toast.push(holocron(13, 2));
        assert_eq!(
            toast.queue[0].count(),
            4,
            "one more joins the banner waiting"
        );
        assert_eq!(toast.queue[0].holocron_top(), Some(13));
        // Dormant kybers and rehearsals keep to themselves.
        toast.push(crystal(14, 0));
        toast.push(rehearsed(15, 0));
        assert_eq!(toast.queue.len(), 4);
        assert_eq!(toast.queue[0].count(), 4);
        // The banner showing takes nothing in.
        let start = Instant::now();
        assert!(toast.update(start, true, false));
        assert_eq!(toast.current.map(|showing| showing.unlock.count()), Some(4));
        toast.push(holocron(16, 0));
        toast.push(holocron(17, 0));
        assert_eq!(toast.queue.len(), 5, "two after it are two again");
        // A reinstall's twenty: one banner.
        let mut fresh = UnlockToast::default();
        for id in 1..=20 {
            fresh.push(holocron(id, 0));
        }
        assert_eq!(fresh.pending(), 1);
        let only = fresh.queue[0];
        assert_eq!(only.count(), 20);
        assert_eq!(only.name().to_string(), "20 new holocrons");
        assert_eq!(
            only.description().to_string(),
            "Every one an Uncommon Holocron"
        );
        assert_eq!(only.kicker(), "NEW HOLOCRONS");
        assert_eq!(only.tag(), HOLOCRONS_HINT);
        assert_eq!(only.hue(), TIERS[0].colour);
        // Medals and achievements too.
        let mut medals = UnlockToast::default();
        for medal_kind in [Medal::BugHunter, Medal::Tester, Medal::Supporter] {
            medals.push(medal(medal_kind, 1));
        }
        assert_eq!(medals.pending(), 1);
        assert!(medals.queue[0].is_medal());
        assert_eq!(medals.queue[0].name().to_string(), "3 new medals");
        assert_eq!(medals.queue[0].tag(), MENU_HINT_MANY);
        let mut board = UnlockToast::default();
        for id in ["first_blood", "streak_5", "maps_10"] {
            board.push(kind(id));
        }
        assert_eq!(board.pending(), 1);
        assert_eq!(board.queue[0].name().to_string(), "3 achievements");
        assert_eq!(board.queue[0].kicker(), "ACHIEVEMENTS UNLOCKED");
    }

    /// Several dormant kybers say so, with the rarest of them; a rehearsal's banner never
    /// names a number to count as seen.
    #[test]
    fn several_dormant_kybers_and_rehearsals_say_so() {
        let several = crystal(1, 0).folded(crystal(2, 2)).folded(crystal(3, 1));
        assert_eq!(several.kicker(), "NEW DORMANT KYBERS");
        assert_eq!(several.name().to_string(), "3 new dormant kybers");
        assert_eq!(
            several.description().to_string(),
            "The rarest: a Legendary Dormant Kyber"
        );
        assert_eq!(several.tag(), KYBERS_HINT);
        assert_eq!(several.holocron_top(), Some(3));
        let rehearsed = rehearsed(4_000_000_001, 0);
        assert_eq!(rehearsed.holocron_top(), None);
        assert_eq!(rehearsed.folded(rehearsed).holocron_top(), None);
        assert_eq!(kind("first_blood").holocron_top(), None);
    }

    /// An update's banner comes once a session for each release the check finds, without
    /// the chime, and never over the Update page, which takes it back.
    #[test]
    fn an_update_found_has_one_quiet_banner() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        let now = Instant::now();
        let found = || Some(("2026.1010.2".to_owned(), false));
        // Nothing changed: the state is not even read.
        toast.offer_update(now, 0, false, || panic!("read without a change"));
        toast.offer_update(now, 1, false, || None);
        assert_eq!(toast.pending(), 0, "checking");
        toast.offer_update(now, 2, false, found);
        assert_eq!(toast.pending(), 1);
        assert!(toast.update(now, true, true));
        assert_eq!(shown(&toast).as_deref(), Some("SJK 2026.1010.2"));
        assert!(ui_cues::take_posted().is_empty(), "no chime for an update");
        // Checked again: the same release gives no second banner; a newer one does.
        toast.offer_update(now, 3, false, found);
        assert_eq!(toast.pending(), 1);
        toast.offer_update(now, 4, false, || Some(("2026.1010.3".to_owned(), true)));
        assert_eq!(toast.pending(), 2);
        // The Update page opens: both go, and what it finds then gets no banner.
        toast.offer_update(now, 4, true, || None);
        assert_eq!(toast.pending(), 0);
        toast.offer_update(now, 5, true, || Some(("2026.1011.1".to_owned(), false)));
        assert_eq!(toast.pending(), 0);
        toast.offer_update(now, 6, false, || Some(("2026.1011.1".to_owned(), false)));
        assert_eq!(toast.pending(), 0, "the page showed it");
    }

    /// Withdrawing takes back one kind's banners, the one showing too, and leaves the
    /// others: the medal ceremony opening takes the medals'.
    #[test]
    fn withdrawing_takes_a_kinds_banners_back() {
        let mut toast = UnlockToast::default();
        toast.push(holocron(3, 0));
        toast.push(medal(Medal::Tester, 1));
        toast.push(kind("first_blood"));
        toast.push(holocron(4, 1));
        let now = Instant::now();
        assert!(toast.update(now, true, false));
        toast.withdraw(now, Unlock::is_holocron);
        assert!(toast.current.is_none(), "the holocron showing went");
        assert_eq!(toast.pending(), 2);
        toast.withdraw(now, Unlock::is_medal);
        assert_eq!(toast.pending(), 1);
        assert!(toast.update(now + GAP, true, false));
        assert_eq!(shown(&toast), Some(kind("first_blood").name().to_string()));
    }

    /// A click lands on the update's banner while it shows, not on another banner, before
    /// it is laid out, nor once it goes back up.
    #[test]
    fn the_update_banner_answers_a_click_on_itself() {
        let viewport = [1920.0, 1080.0];
        let version = Version::new("2026.1020.1").unwrap();
        let mut toast = UnlockToast::default();
        toast.push(Unlock::Update {
            version,
            manual: false,
        });
        let now = Instant::now();
        assert!(toast.update(now, true, false));
        let middle = Vec2::new(960.0, banner::MARGIN + banner::HEIGHT * 0.5);
        assert_eq!(toast.update_card_at(middle), None, "not laid out yet");
        toast.held = Some(banner::ENTER + 1.0);
        toast.build(viewport);
        assert_eq!(toast.update_card_at(middle), Some(false));
        assert_eq!(toast.update_card_at(Vec2::new(4.0, 4.0)), None);
        toast.held = Some(banner::leaves_at(false) + 0.1);
        assert_eq!(toast.update_card_at(middle), None, "leaving");
        toast.held = None;
        toast.dismiss_update(now);
        assert_eq!(toast.pending(), 0);
        assert_eq!(toast.update_card_at(middle), None);

        let mut medal_banner = UnlockToast::preview(&[medal(Medal::BugHunter, 1)], 1.0);
        medal_banner.build(viewport);
        assert_eq!(medal_banner.update_card_at(middle), None);
    }

    #[test]
    fn the_words_of_medals_and_holocrons() {
        let update = Unlock::Update {
            version: Version::new("2026.1010.2").unwrap(),
            manual: false,
        };
        assert_eq!(update.kicker(), "UPDATE AVAILABLE");
        assert_eq!(update.tag(), UPDATE_HINT);
        assert_eq!(update.name().to_string(), "SJK 2026.1010.2");
        assert!(!update.chimes() && kind("streak_5").chimes());
        let content = Unlock::Content {
            title: "Sith cloud",
            files: 3,
            bytes: 120_400_000,
            removed: 0,
        };
        assert_eq!(content.kicker(), "CONTENT UPDATED");
        assert_eq!(content.name().to_string(), "Sith cloud updated");
        assert_eq!(
            content.description().to_string(),
            "3 files, 120 MB  ·  from the next map"
        );
        let removed = Unlock::Content {
            title: "Sith cloud",
            files: 0,
            bytes: 0,
            removed: 1,
        };
        assert_eq!(
            removed.description().to_string(),
            "1 old file removed  ·  from the next map"
        );
        assert!(!content.chimes() && !content.same(removed) && content.same(content));
        let bug_hunter = medal(Medal::BugHunter, 3);
        assert_eq!(bug_hunter.kicker(), "NEW MEDAL");
        let promoted = Unlock::Medal {
            medal: Medal::Tester,
            count: 1,
            promoted: Some(crate::medals::Rank::Holo),
        };
        assert_eq!(promoted.kicker(), "MEDAL PROMOTED");
        assert_eq!(promoted.name().to_string(), "Tester");
        assert_eq!(
            promoted.description().to_string(),
            "Promoted to Rank IV Holo"
        );
        assert!(!promoted.same(bug_hunter));
        assert_eq!(bug_hunter.name().to_string(), "Bug Hunter x3");
        assert_eq!(bug_hunter.tag(), MENU_HINT);
        assert_eq!(medal(Medal::Tester, 1).name().to_string(), "Tester");
        let mythical = holocron(9, 3);
        assert_eq!(mythical.kicker(), "NEW HOLOCRON");
        assert_eq!(
            mythical.description().to_string(),
            "Found while playing: 1.5% of drops are Mythical"
        );
        // Where it waits: the Collection's Holocrons page, not the game menu.
        assert_eq!(mythical.tag(), "See it in Collection > Holocrons");
        let gift = Unlock::Holocron {
            tier: &TIERS[0],
            id: 1,
            gift: true,
            earned: None,
            rehearsal: false,
        };
        assert_eq!(gift.description().to_string(), "A gift from the SJK team");
        // An achievement's note is the kyber crystal it brings (21/10/2026).
        assert_eq!(kind("streak_5").tag(), "Reward: Uncommon Dormant Kyber");
        assert_eq!(kind("kills_1000").tag(), "Reward: Mythical Dormant Kyber");
        // Unlocked without a hub (identity off), it brings no crystal: its category.
        let local = Unlock::Achievement {
            kind: achievements::find("streak_5").expect("Rampage"),
            crystal: false,
        };
        assert_eq!(local.tag(), "Combat");
        let crystal = crystal(2, 1);
        assert_eq!(crystal.kicker(), "NEW DORMANT KYBER");
        assert_eq!(crystal.name().to_string(), "Rare Dormant Kyber");
        assert_eq!(
            crystal.description().to_string(),
            "Earned from an achievement"
        );
        assert_eq!(crystal.tag(), "Attune it in Collection > Holocrons");
        let named = Unlock::Holocron {
            tier: &crate::kyber::TIERS[1],
            id: 3,
            gift: false,
            earned: Some(crate::kyber::Earned::of("kills_100")),
            rehearsal: false,
        };
        assert_eq!(named.description().to_string(), "Earned: Centurion");
        // The toys' news, once centre prints, and a request for logs: quiet banners.
        let glint = Unlock::KyberGlint {
            shards: 3,
            balance: 40,
            mining: true,
        };
        assert_eq!(glint.name().to_string(), "+3 kyber shards");
        assert_eq!(
            glint.description().to_string(),
            "A kyber glint in the rock! You hold 40 now"
        );
        let goal = Unlock::CatchGoal { mining: false };
        assert_eq!(goal.name().to_string(), "Today's fishing goal is done");
        let logs = Unlock::LogRequest { id: 3 };
        assert_eq!(logs.tag(), "Open the menu to answer");
        assert!(!glint.chimes() && !goal.chimes() && !logs.chimes());
        assert!(logs.same(Unlock::LogRequest { id: 3 }));
        assert!(!logs.same(Unlock::LogRequest { id: 4 }));
    }

    /// The words are written once as a banner starts, in the player's terms, and kept for
    /// the next one; several of a kind carry their count on a badge.
    #[test]
    fn a_banners_words_are_written_once_as_it_starts() {
        let mut words = Words::default();
        words.write(holocron(1, 1));
        assert_eq!(words.header, "NEW HOLOCRON");
        assert_eq!(words.title, "Rare Holocron");
        assert_eq!(words.note, HOLOCRON_HINT);
        assert!(words.badge.is_empty());
        let several = holocron(1, 1).folded(holocron(2, 2)).folded(holocron(3, 1));
        words.write(several);
        assert_eq!(words.title, "3 new holocrons");
        assert_eq!(words.badge, "3");
        assert_eq!(words.body, "The rarest: a Legendary Holocron");
    }

    #[test]
    fn nothing_is_drawn_while_idle() {
        let mut toast = UnlockToast::default();
        assert!(!toast.update(Instant::now(), true, true));
        toast.build([1920.0, 1080.0]);
        assert!(toast.draw_list().is_empty());
        assert!(toast.take_started().is_none());
    }

    #[test]
    fn the_banner_comes_in_holds_and_leaves() {
        for reduced in [false, true] {
            assert_eq!(Phase::at(0.0, reduced), Phase::Entering);
            assert_eq!(Phase::at(banner::ENTER + 1.0, reduced), Phase::Shown);
            assert_eq!(
                Phase::at(banner::leaves_at(reduced) + 0.1, reduced),
                Phase::Leaving
            );
            assert_eq!(Phase::at(banner::lifetime(reduced), reduced), Phase::Gone);
        }
    }

    /// Every unlock, at every moment, fits the canvas and the screen at 1080 lines, 4K,
    /// 4:3, 21:9 and 800x600, its words its column in the families and in Inter.
    #[test]
    fn every_unlock_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let display = load(&crate::text::DISPLAY);
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let width = |font: &UiFont, value: &str, size: f32, face, spacing| {
            crate::text::visible_text_width_style(font, value, size / font.height, face, spacing)
        };
        let semibold = crate::text::TextFace::Semibold;
        let regular = crate::text::TextFace::Regular;
        let unlocks = every_unlock();
        for (title, words) in [(&display.font, &body.font), (&inter.font, &inter.font)] {
            for unlock in &unlocks {
                let mut text = Words::default();
                text.write(*unlock);
                let header = width(
                    title,
                    &text.header,
                    banner::HEADER_SIZE,
                    semibold,
                    banner::HEADER_SPACING,
                );
                assert!(
                    header + banner::NOW_ROOM <= banner::TEXT_WIDTH,
                    "{unlock:?}: {header}"
                );
                for (line, size, face) in [
                    (&text.title, banner::TITLE_SIZE, semibold),
                    (&text.body, banner::BODY_SIZE, regular),
                    (&text.note, banner::NOTE_SIZE, regular),
                ] {
                    assert!(line.len() <= 128, "{line}: one text slot");
                    let line_width = width(words, line, size, face, 0.0);
                    assert!(
                        line_width <= banner::TEXT_WIDTH,
                        "{line}: {line_width} of {}",
                        banner::TEXT_WIDTH
                    );
                }
            }
        }
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
            [800.0, 600.0],
        ] {
            for unlock in &unlocks {
                for reduced in [false, true] {
                    for t in [0.05, 0.3, 0.6, 1.0, 3.0, banner::leaves_at(reduced) + 0.2] {
                        let mut toast = UnlockToast::preview_with(&[*unlock], t, reduced);
                        toast.build(viewport);
                        assert!(!toast.canvas.overflowed(), "{unlock:?} {t} {viewport:?}");
                        let card = banner::rect(viewport, t, reduced);
                        assert!(
                            card.x >= 0.0 && card.right() <= viewport[0],
                            "{viewport:?} {t}"
                        );
                        // Clear of the crosshair, in the screen's top fifth.
                        assert!(card.bottom() < viewport[1] / 5.0, "{viewport:?}");
                    }
                }
            }
        }
    }
}
