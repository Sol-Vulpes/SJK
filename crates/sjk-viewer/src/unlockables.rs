//! The unlockables this client knows (`docs/unlockables.md`, "Catalogue"): the one
//! catalogue the looks, the blade skins, the `saberskin` command, the Unlockables page
//! and the Staff page read. The hub keeps its own copy (`src/unlocks.rs`); a new
//! unlockable needs both, and an id never changes meaning. An id this client does not
//! know is ignored wherever it comes from (a profile, a look, `cg_saberSkin`).
//!
//! The catalogue is public metadata only: what an unlockable looks and sounds like is
//! art in the SJK hub's packs (`sjk_packs.rs`), a blade skin's in its blade-skin file.

/// Archived: the blade-skin unlock id the player wears, empty for the stock blade. It
/// shows (and is sent to the hub) only while the player's own hub profile lists it.
pub(crate) const SABER_SKIN_CVAR: &str = "cg_saberSkin";

/// What an unlockable is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// A look a player's blades wear instead of their colour, drawn from its blade-skin
    /// file (`skins/blades/<id>.bladeskin` in a pack).
    BladeSkin,
}

/// One unlockable of the catalogue.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Unlockable {
    /// The id the hub, a look and `cg_saberSkin` name it by.
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    /// What it is, in a line.
    pub(crate) description: &'static str,
    /// How a player gets it, in a line.
    pub(crate) how_to_get: &'static str,
}

impl Unlockable {
    /// Whether it is a blade skin.
    pub(crate) const fn is_blade_skin(&self) -> bool {
        matches!(self.kind, Kind::BladeSkin)
    }
}

/// Every unlockable, in the hub's catalogue order (the order profiles list them in).
pub(crate) const ALL: [Unlockable; 5] = [
    Unlockable {
        id: "saber_sun",
        name: "Sun blade",
        kind: Kind::BladeSkin,
        description: "A white-gold core in an orange corona, flares running along it, and its own sounds.",
        how_to_get: "Given by the SJK team.",
    },
    Unlockable {
        id: "saber_storm",
        name: "Storm blade",
        kind: Kind::BladeSkin,
        description: "A white-blue core crackling with lightning that leaps off it, sparks at its tip.",
        how_to_get: "Given by the SJK team.",
    },
    Unlockable {
        id: "saber_void",
        name: "Void blade",
        kind: Kind::BladeSkin,
        description: "A dark core in a violet rim, star specks drifting into it, and a deep hum.",
        how_to_get: "Given by the SJK team.",
    },
    Unlockable {
        id: "saber_frost",
        name: "Frost blade",
        kind: Kind::BladeSkin,
        description: "A pale ice core with a pointed tip, frost shards trailing it, and a chiming hum.",
        how_to_get: "Given by the SJK team.",
    },
    Unlockable {
        id: "saber_prism",
        name: "Prism blade",
        kind: Kind::BladeSkin,
        description: "A white core in a corona whose colours run through the rainbow, slowly turning.",
        how_to_get: "Given by the SJK team.",
    },
];

/// The unlockable `id` names (spaces round it and case ignored), if this client knows it.
pub(crate) fn find(id: &str) -> Option<&'static Unlockable> {
    let id = id.trim();
    ALL.iter()
        .find(|unlockable| unlockable.id.eq_ignore_ascii_case(id))
}

/// The blade skin `id` names, if this client knows one by it.
pub(crate) fn blade_skin(id: &str) -> Option<&'static Unlockable> {
    find(id).filter(|unlockable| unlockable.is_blade_skin())
}

/// The blade skins, in catalogue order.
pub(crate) fn blade_skins() -> impl Iterator<Item = &'static Unlockable> {
    ALL.iter().filter(|unlockable| unlockable.is_blade_skin())
}

/// What this client knows of the unlocks the player's own hub profile lists, by the
/// same rule as the gate on what they wear (`player_identity::owns_unlock`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Holdings<'a> {
    /// The identity is off: unlockables need it.
    IdentityOff,
    /// No hub is set (`cl_hubUrl`).
    NoHub,
    /// The hub has not answered yet, or cannot be reached.
    Waiting,
    /// The own profile's unlocks.
    Known(&'a [sjk_identity::Unlock]),
}

impl<'a> Holdings<'a> {
    /// From `cl_identity` (`enabled`) and the identity service's snapshot.
    pub(crate) fn of(enabled: bool, snapshot: Option<&'a sjk_identity::Snapshot>) -> Self {
        use sjk_identity::Status;
        let Some(snapshot) = snapshot.filter(|_| enabled) else {
            return Self::IdentityOff;
        };
        match (&snapshot.status, &snapshot.me) {
            (Status::Disabled, _) => Self::IdentityOff,
            (Status::NoHub, _) => Self::NoHub,
            (_, Some(me)) => Self::Known(&me.unlocks),
            (_, None) => Self::Waiting,
        }
    }

    /// The grant of `id` the profile lists, if it lists it.
    pub(crate) fn unlock(&self, id: &str) -> Option<&'a sjk_identity::Unlock> {
        match self {
            Self::Known(unlocks) => unlocks.iter().find(|unlock| unlock.id == id),
            _ => None,
        }
    }

    /// Why what the player holds is not known, in a sentence; `None` once it is.
    pub(crate) fn reason(&self) -> Option<&'static str> {
        match self {
            Self::IdentityOff => {
                Some("Unlockables need the SJK identity: switch it on in Identity settings.")
            }
            Self::NoHub => Some("No hub is set: unlockables are kept on the SJK hub (cl_hubUrl)."),
            Self::Waiting => Some("Waiting for the SJK hub to answer."),
            Self::Known(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_every_blade_skin_has_one_entry() {
        for (index, unlockable) in ALL.iter().enumerate() {
            assert!(
                ALL[index + 1..]
                    .iter()
                    .all(|other| !other.id.eq_ignore_ascii_case(unlockable.id)),
                "{} is listed twice",
                unlockable.id
            );
            assert!(
                unlockable
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{} is a hub id",
                unlockable.id
            );
            assert!(!unlockable.name.is_empty() && !unlockable.description.is_empty());
            assert!(unlockable.how_to_get.ends_with('.'));
        }
        assert_eq!(
            blade_skins().count(),
            ALL.iter().filter(|u| u.kind == Kind::BladeSkin).count()
        );
    }

    #[test]
    fn holdings_follow_the_identity_and_the_own_profile() {
        use sjk_identity::{Profile, Snapshot, Status, Unlock};
        let unlock = Unlock {
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "Thanks".into(),
        };
        let me = Profile {
            key_id: "0123456789abcdef".into(),
            key: String::new(),
            name: "Sol".into(),
            bio: String::new(),
            verified: false,
            staff: false,
            created: 0,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
            unlocks: vec![unlock.clone()],
            avatar: String::new(),
        };
        let snapshot = |status, me| Snapshot {
            status,
            key_id: "0123456789abcdef".into(),
            me,
            server: None,
            players: Vec::new(),
            profiles: std::collections::HashMap::new(),
            notice: None,
            revision: 0,
            report: None,
            note: None,
            player_report: None,
            avatar: None,
            look_outcome: None,
            packs_revision: 0,
            assets_note: None,
        };
        let online = snapshot(Status::Online, Some(me));
        assert_eq!(Holdings::of(false, Some(&online)), Holdings::IdentityOff);
        assert_eq!(Holdings::of(true, None), Holdings::IdentityOff);
        let known = Holdings::of(true, Some(&online));
        assert_eq!(known.unlock("saber_sun"), Some(&unlock));
        assert_eq!(known.unlock("saber_moon"), None);
        assert_eq!(known.reason(), None);
        let waiting = snapshot(Status::Registering, None);
        assert_eq!(Holdings::of(true, Some(&waiting)), Holdings::Waiting);
        let no_hub = snapshot(Status::NoHub, None);
        assert_eq!(Holdings::of(true, Some(&no_hub)), Holdings::NoHub);
        assert!(
            Holdings::IdentityOff
                .reason()
                .unwrap()
                .contains("SJK identity")
        );
        assert_eq!(Holdings::Waiting.unlock("saber_sun"), None);
    }

    /// The hub's catalogue (`src/unlocks.rs` in Sol-Vulpes/SJK-hub) lists exactly these, in
    /// this order and with these names; its test pins the same list, so a change to one
    /// side fails until the other follows.
    #[test]
    fn the_catalogue_matches_the_hubs() {
        let listed: Vec<(&str, &str)> = ALL.iter().map(|u| (u.id, u.name)).collect();
        assert_eq!(
            listed,
            [
                ("saber_sun", "Sun blade"),
                ("saber_storm", "Storm blade"),
                ("saber_void", "Void blade"),
                ("saber_frost", "Frost blade"),
                ("saber_prism", "Prism blade"),
            ]
        );
        assert!(ALL.iter().all(Unlockable::is_blade_skin));
        // The blade skins fit the renderer's slots, every one loaded at once.
        assert!(blade_skins().count() <= crate::saber_skins::MAX_SKINS);
    }

    #[test]
    fn ids_are_found_whatever_the_case_and_spaces() {
        assert_eq!(find("saber_sun").map(|u| u.name), Some("Sun blade"));
        assert_eq!(blade_skin(" SABER_Sun ").map(|u| u.id), Some("saber_sun"));
        assert!(blade_skin("saber_sun").is_some_and(Unlockable::is_blade_skin));
        assert!(find("saber_moon").is_none());
        assert!(find("").is_none());
    }
}
