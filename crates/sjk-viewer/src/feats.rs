//! Feats (`cg_feats`): multi-kills, sprees and a few special kills, called out by an
//! announcer and shown as medals under the top centre ([`crate::hud::feats`]).
//!
//! Everything is worked out on the client from the obituaries it already decodes
//! ([`sjk_client::ObituaryTracker`], `EV_OBITUARY`), so feats need nothing from the
//! server and work on any server and in demos. Every client's sprees and chains are
//! followed (a victim's spree decides [`Feat::SpreeEnded`]); the viewed player's feats
//! are the ones shown and heard.
//!
//! The rules ([`Rules::kill`]):
//! - a multi-kill is a chain of kills each at most [`MULTI_KILL_MS`] after the one before;
//! - a spree counts kills since the player last died (killed, suicide or the world),
//!   with a feat every [`SPREE_STEP`] kills up to [`Feat::Godlike`];
//! - [`Feat::FirstBlood`] is the level's first kill, only when the client saw the level
//!   start (`CS_LEVEL_START_TIME`), so joining a match never claims it;
//! - [`Feat::Revenge`] kills the player who last killed you, [`Feat::SpreeEnded`] a
//!   player on a spree;
//! - [`Feat::Hooked`] is a kill by JA+'s grapple hook, see [`HOOK_MEANS`].

pub(crate) mod announcer;

use sjk_client::ObituaryEvent;

/// Mount the bundled medal pictures (`gfx/sjk/feats/<id>.png`, made by
/// `scripts/feat_assets.py`) and SJK's announcer pack, below all game data.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK feats", PICTURES.iter().copied())?;
    announcer::mount(vfs)
}

/// A bundled file's bytes, by its name in `assets/feats`.
macro_rules! picture {
    ($name:literal) => {
        (
            concat!("gfx/sjk/feats/", $name, ".png"),
            include_bytes!(concat!("../assets/feats/", $name, ".png")).as_slice(),
        )
    };
}

/// The medal pictures at their game paths, one per [`Feat`].
const PICTURES: [(&str, &[u8]); 14] = [
    picture!("hooked"),
    picture!("first_blood"),
    picture!("double_kill"),
    picture!("triple_kill"),
    picture!("quad_kill"),
    picture!("penta_kill"),
    picture!("massacre"),
    picture!("killing_spree"),
    picture!("rampage"),
    picture!("dominating"),
    picture!("unstoppable"),
    picture!("godlike"),
    picture!("spree_ended"),
    picture!("revenge"),
];

/// Longest gap between two kills of one multi-kill (server milliseconds).
pub(crate) const MULTI_KILL_MS: i32 = 4_000;
/// Kills between two spree feats, and the first spree feat's count.
pub(crate) const SPREE_STEP: u16 = 5;
/// A level that started less than this long before the client's first look at it
/// (server milliseconds) was seen starting: its first kill is first blood.
const FIRST_BLOOD_GRACE_MS: i32 = 10_000;
/// `CS_LEVEL_START_TIME`.
const CS_LEVEL_START_TIME: usize = 21;
/// Client slots (`MAX_CLIENTS`).
const CLIENTS: usize = 64;

/// JA+'s grapple hook kills as `MOD_STUN_BATON` (1) whatever weapon its owner holds: the
/// hook is fired as a stun baton missile with `damage = 1` and `methodOfDeath =
/// MOD_STUN_BATON` (Blackwolf's VisionJK rebuild of JA+ 2.4 B7,
/// `codemp/game/japlus/g_japlus_hook.c:146-147`). A stun baton kill whose killer did
/// not hold the baton (`WP_STUN_BATON`, [`BATON_WEAPON`]) was the hook. No server is
/// checked by name: a mod that kills with the baton's means some other way gets the
/// same feat.
pub(crate) const HOOK_MEANS: u8 = 1;
/// `WP_STUN_BATON`.
pub(crate) const BATON_WEAPON: u8 = 1;

/// One feat, in the order the HUD and the announcer give them for one kill.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Feat {
    Hooked,
    FirstBlood,
    DoubleKill,
    TripleKill,
    QuadKill,
    PentaKill,
    Massacre,
    KillingSpree,
    Rampage,
    Dominating,
    Unstoppable,
    Godlike,
    SpreeEnded,
    Revenge,
}

impl Feat {
    /// Every feat.
    pub(crate) const ALL: [Self; 14] = [
        Self::Hooked,
        Self::FirstBlood,
        Self::DoubleKill,
        Self::TripleKill,
        Self::QuadKill,
        Self::PentaKill,
        Self::Massacre,
        Self::KillingSpree,
        Self::Rampage,
        Self::Dominating,
        Self::Unstoppable,
        Self::Godlike,
        Self::SpreeEnded,
        Self::Revenge,
    ];

    /// The feat's place in [`Self::ALL`].
    pub(crate) fn index(self) -> usize {
        self as usize
    }

    /// The name of its files: `sound/sjk/announcer/<pack>/<id>` and
    /// `gfx/sjk/feats/<id>.png`.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Hooked => "hooked",
            Self::FirstBlood => "first_blood",
            Self::DoubleKill => "double_kill",
            Self::TripleKill => "triple_kill",
            Self::QuadKill => "quad_kill",
            Self::PentaKill => "penta_kill",
            Self::Massacre => "massacre",
            Self::KillingSpree => "killing_spree",
            Self::Rampage => "rampage",
            Self::Dominating => "dominating",
            Self::Unstoppable => "unstoppable",
            Self::Godlike => "godlike",
            Self::SpreeEnded => "spree_ended",
            Self::Revenge => "revenge",
        }
    }

    /// What the medal says.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Hooked => "HOOKED!",
            Self::FirstBlood => "FIRST BLOOD",
            Self::DoubleKill => "DOUBLE KILL",
            Self::TripleKill => "TRIPLE KILL",
            Self::QuadKill => "QUAD KILL",
            Self::PentaKill => "PENTA KILL",
            Self::Massacre => "MASSACRE",
            Self::KillingSpree => "KILLING SPREE",
            Self::Rampage => "RAMPAGE",
            Self::Dominating => "DOMINATING",
            Self::Unstoppable => "UNSTOPPABLE",
            Self::Godlike => "GODLIKE",
            Self::SpreeEnded => "SPREE ENDED",
            Self::Revenge => "REVENGE",
        }
    }

    /// The medal's accent colour (linear RGB), by family: chains warm, sprees red to
    /// gold, the others their own.
    pub(crate) const fn accent(self) -> [f32; 3] {
        match self {
            Self::Hooked => [0.35, 0.85, 0.95],
            Self::FirstBlood => [0.85, 0.12, 0.12],
            Self::DoubleKill | Self::TripleKill => [1.0, 0.62, 0.18],
            Self::QuadKill | Self::PentaKill | Self::Massacre => [1.0, 0.35, 0.12],
            Self::KillingSpree | Self::Rampage => [0.95, 0.25, 0.3],
            Self::Dominating | Self::Unstoppable => [0.75, 0.35, 1.0],
            Self::Godlike => [1.0, 0.84, 0.3],
            Self::SpreeEnded => [0.6, 0.65, 0.72],
            Self::Revenge => [0.55, 0.9, 0.4],
        }
    }

    /// The chain feat of a chain of `kills`, none for a single kill.
    fn chain(kills: u8) -> Option<Self> {
        match kills {
            0 | 1 => None,
            2 => Some(Self::DoubleKill),
            3 => Some(Self::TripleKill),
            4 => Some(Self::QuadKill),
            5 => Some(Self::PentaKill),
            _ => Some(Self::Massacre),
        }
    }

    /// The spree feat a spree of `kills` reaches, if that kill reaches one.
    fn spree(kills: u16) -> Option<Self> {
        if kills == 0 || !kills.is_multiple_of(SPREE_STEP) {
            return None;
        }
        Some(match kills / SPREE_STEP {
            1 => Self::KillingSpree,
            2 => Self::Rampage,
            3 => Self::Dominating,
            4 => Self::Unstoppable,
            5 => Self::Godlike,
            _ => return None,
        })
    }
}

/// The feats of one kill, at most one of each family.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Earned {
    feats: [Option<Feat>; 6],
    len: usize,
}

impl Earned {
    fn push(&mut self, feat: Feat) {
        if self.len < self.feats.len() {
            self.feats[self.len] = Some(feat);
            self.len += 1;
        }
    }

    /// The feats, in [`Feat`]'s order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = Feat> + '_ {
        let mut feats = self.feats;
        feats[..self.len].sort_by_key(|feat| feat.map(Feat::index));
        feats.into_iter().take(self.len).flatten()
    }
}

/// One client's running counts.
#[derive(Clone, Copy, Debug, Default)]
struct Fighter {
    /// Kills since its last death.
    spree: u16,
    /// Kills of its current chain, and when the last one was.
    chain: u8,
    last_kill: i32,
    /// Who killed it last, until it takes its revenge.
    killed_by: Option<u16>,
}

/// Every client's sprees and chains for the level being played.
#[derive(Clone, Debug)]
pub(crate) struct Rules {
    fighters: [Fighter; CLIENTS],
    /// The level's `CS_LEVEL_START_TIME`, once one was seen.
    level_start: Option<i32>,
    /// The level's first kill is still to come and the client saw it start.
    first_blood_open: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            fighters: [Fighter::default(); CLIENTS],
            level_start: None,
            first_blood_open: false,
        }
    }
}

impl Rules {
    /// Follow the level in `game` at `server_time`: a new level (or a restart of this
    /// one) clears every count; first blood is open when the client saw it start.
    pub(crate) fn level(&mut self, game: &sjk_protocol::GameState, server_time: i32) {
        let start = game
            .config_string(CS_LEVEL_START_TIME)
            .and_then(|text| std::str::from_utf8(text).ok())
            .and_then(|text| text.trim().parse::<i32>().ok())
            .unwrap_or(0);
        self.start_level(start, server_time);
    }

    fn start_level(&mut self, start: i32, server_time: i32) {
        if self.level_start == Some(start) {
            return;
        }
        let first_look = self.level_start.is_none();
        *self = Self::default();
        self.level_start = Some(start);
        self.first_blood_open =
            !first_look || server_time.wrapping_sub(start) < FIRST_BLOOD_GRACE_MS;
    }

    /// Count `kill` and say what its killer earned.
    pub(crate) fn kill(&mut self, kill: &ObituaryEvent) -> Earned {
        let mut earned = Earned::default();
        let (target, attacker) = (usize::from(kill.target), usize::from(kill.attacker));
        let Some(victim) = self.fighters.get_mut(target) else {
            return earned;
        };
        let victim_spree = victim.spree;
        victim.spree = 0;
        victim.chain = 0;
        if attacker == target || attacker >= CLIENTS {
            // A suicide or a death to the world: it only ends the victim's run.
            return earned;
        }
        victim.killed_by = Some(kill.attacker);
        if std::mem::take(&mut self.first_blood_open) {
            earned.push(Feat::FirstBlood);
        }
        let killer = &mut self.fighters[attacker];
        killer.chain = if killer.chain > 0
            && kill.server_time.wrapping_sub(killer.last_kill) <= MULTI_KILL_MS
        {
            killer.chain.saturating_add(1)
        } else {
            1
        };
        killer.last_kill = kill.server_time;
        killer.spree = killer.spree.saturating_add(1);
        if let Some(feat) = Feat::chain(killer.chain) {
            earned.push(feat);
        }
        if let Some(feat) = Feat::spree(killer.spree) {
            earned.push(feat);
        }
        if killer.killed_by == Some(kill.target) {
            killer.killed_by = None;
            earned.push(Feat::Revenge);
        }
        if victim_spree >= SPREE_STEP {
            earned.push(Feat::SpreeEnded);
        }
        if kill.means_of_death == HOOK_MEANS && kill.attacker_weapon != BATON_WEAPON {
            earned.push(Feat::Hooked);
        }
        earned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SABER: u8 = 3;
    const WORLD: u16 = 1022;

    fn kill(attacker: u16, target: u16, time: i32) -> ObituaryEvent {
        ObituaryEvent {
            target,
            attacker,
            means_of_death: SABER,
            message: "",
            attacker_message: Some("KILLED_SABER"),
            local_fragged: false,
            local_was_killed: false,
            server_time: time,
            attacker_force: 0,
            attacker_weapon: 3,
        }
    }

    fn feats(rules: &mut Rules, event: ObituaryEvent) -> Vec<Feat> {
        rules.kill(&event).iter().collect()
    }

    fn joined_late() -> Rules {
        let mut rules = Rules::default();
        rules.start_level(0, 600_000);
        rules
    }

    #[test]
    fn kills_within_the_window_chain_and_a_gap_breaks_the_chain() {
        let mut rules = joined_late();
        assert!(feats(&mut rules, kill(1, 2, 1_000)).is_empty());
        assert_eq!(feats(&mut rules, kill(1, 3, 5_000)), [Feat::DoubleKill]);
        assert_eq!(feats(&mut rules, kill(1, 4, 9_000)), [Feat::TripleKill]);
        // 4001 ms: the chain starts again.
        assert!(feats(&mut rules, kill(1, 5, 13_001)).is_empty());
        // The fifth kill without dying is also a spree.
        assert_eq!(
            feats(&mut rules, kill(1, 6, 13_500)),
            [Feat::DoubleKill, Feat::KillingSpree]
        );
    }

    #[test]
    fn long_chains_climb_to_massacre_and_stay_there() {
        let mut rules = joined_late();
        let chain: Vec<Vec<Feat>> = (0..7)
            .map(|n| {
                feats(&mut rules, kill(1, 2 + n, 1_000 + i32::from(n) * 100))
                    .into_iter()
                    .filter(|feat| !matches!(feat, Feat::KillingSpree))
                    .collect()
            })
            .collect();
        assert_eq!(
            chain,
            [
                vec![],
                vec![Feat::DoubleKill],
                vec![Feat::TripleKill],
                vec![Feat::QuadKill],
                vec![Feat::PentaKill],
                vec![Feat::Massacre],
                vec![Feat::Massacre],
            ]
        );
    }

    #[test]
    fn sprees_call_every_fifth_kill_up_to_godlike() {
        let mut rules = joined_late();
        let mut sprees = Vec::new();
        for n in 1..=35u16 {
            // A minute apart: no chains.
            for feat in feats(&mut rules, kill(1, 2 + n % 20, i32::from(n) * 60_000)) {
                sprees.push((n, feat));
            }
        }
        assert_eq!(
            sprees,
            [
                (5, Feat::KillingSpree),
                (10, Feat::Rampage),
                (15, Feat::Dominating),
                (20, Feat::Unstoppable),
                (25, Feat::Godlike),
            ]
        );
    }

    #[test]
    fn any_death_ends_a_spree_and_its_end_is_a_feat() {
        let mut rules = joined_late();
        for n in 0..4 {
            rules.kill(&kill(1, 2, n * 60_000));
        }
        // A suicide, then a death to the world, each start the count again.
        rules.kill(&kill(1, 1, 300_000));
        for n in 0..4 {
            rules.kill(&kill(1, 2, 400_000 + n * 60_000));
        }
        rules.kill(&kill(WORLD, 1, 700_000));
        for n in 0..4 {
            assert!(feats(&mut rules, kill(1, 2, 800_000 + n * 60_000)).is_empty());
        }
        assert_eq!(
            feats(&mut rules, kill(1, 2, 1_100_000)),
            [Feat::KillingSpree]
        );
        // Ending a five-kill spree is a feat for whoever ends it.
        assert_eq!(feats(&mut rules, kill(3, 1, 1_200_000)), [Feat::SpreeEnded]);
    }

    #[test]
    fn killing_your_last_killer_is_revenge_once() {
        let mut rules = joined_late();
        rules.kill(&kill(2, 1, 1_000));
        assert_eq!(feats(&mut rules, kill(1, 2, 60_000)), [Feat::Revenge]);
        assert!(feats(&mut rules, kill(1, 2, 120_000)).is_empty());
    }

    #[test]
    fn first_blood_only_when_the_level_start_was_seen() {
        // Joined ten minutes in: nobody gets first blood.
        let mut rules = joined_late();
        assert!(feats(&mut rules, kill(1, 2, 600_100)).is_empty());
        // Joined 3 s after the start: the first kill is first blood, the second is not.
        let mut rules = Rules::default();
        rules.start_level(100_000, 103_000);
        assert_eq!(feats(&mut rules, kill(1, 2, 110_000)), [Feat::FirstBlood]);
        assert!(feats(&mut rules, kill(3, 4, 120_000)).is_empty());
        // A restart seen while connected opens it again and clears every count.
        rules.start_level(200_000, 900_000);
        assert_eq!(feats(&mut rules, kill(3, 4, 901_000)), [Feat::FirstBlood]);
        // A suicide is no first blood.
        rules.start_level(300_000, 300_100);
        assert!(feats(&mut rules, kill(5, 5, 300_200)).is_empty());
        assert_eq!(feats(&mut rules, kill(5, 6, 300_300)), [Feat::FirstBlood]);
    }

    #[test]
    fn a_stun_baton_kill_without_the_baton_is_the_hook() {
        let mut rules = joined_late();
        let mut hook = kill(1, 2, 1_000);
        hook.means_of_death = HOOK_MEANS;
        assert_eq!(feats(&mut rules, hook), [Feat::Hooked]);
        let mut baton = kill(1, 3, 60_000);
        baton.means_of_death = HOOK_MEANS;
        baton.attacker_weapon = BATON_WEAPON;
        assert!(feats(&mut rules, baton).is_empty());
    }

    #[test]
    fn one_kill_gives_its_feats_in_order() {
        let mut rules = Rules::default();
        rules.start_level(0, 0);
        rules.kill(&kill(2, 1, 100));
        for n in 0..4 {
            rules.kill(&kill(2, 3, 1_000 + n * 60_000));
        }
        rules.kill(&kill(1, 4, 400_000));
        let mut last = kill(1, 2, 401_000);
        last.means_of_death = HOOK_MEANS;
        assert_eq!(
            feats(&mut rules, last),
            [
                Feat::Hooked,
                Feat::DoubleKill,
                Feat::SpreeEnded,
                Feat::Revenge
            ]
        );
    }

    #[test]
    fn every_feat_has_its_bundled_picture() {
        for (feat, (path, bytes)) in Feat::ALL.iter().zip(PICTURES) {
            assert_eq!(path, format!("gfx/sjk/feats/{}.png", feat.id()));
            assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']), "{path}");
        }
    }

    #[test]
    fn every_feat_has_its_own_files_and_label() {
        let ids: std::collections::HashSet<_> = Feat::ALL.iter().map(|feat| feat.id()).collect();
        assert_eq!(ids.len(), Feat::ALL.len());
        for (index, feat) in Feat::ALL.iter().enumerate() {
            assert_eq!(feat.index(), index);
            assert!(!feat.label().is_empty());
        }
    }
}
