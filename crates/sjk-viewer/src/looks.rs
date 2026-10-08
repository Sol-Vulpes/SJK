//! Looks (`docs/unlockables.md`, "Receiving"): what each player on the game server
//! wears that SJK draws, the blade skin and whether their Illuminate holocron is lit.
//! Other players' looks come through the SJK hub ([`sjk_identity::Look`]): from the
//! presence roster when it changes and from the feed's look events as they come. The
//! local player's own look comes from its own settings (`cg_saberSkin`, gated by the
//! unlocks of its hub profile, and its Illuminate), never from the hub.
//!
//! A hub look counts only while the game shows, in its slot, the name the claim was
//! made under (the badges' and emotes' rule, [`sjk_identity::names_match`]). The
//! table is rebuilt when a roster or an event comes and twice a second, so a player
//! who leaves a slot loses the look at once for the renderer; a frame only reads the
//! fixed [`Worn`] table.
//!
//! The renderer reads [`Looks::saber_skin_id`], [`Looks::illuminated`] and
//! [`Looks::own_saber_skin`] on `GpuState::looks`.

use sjk_identity::Look;

/// Slots the table holds, as the emotes' do.
pub(crate) const SLOTS: usize = 64;
/// Archived: the blade-skin unlock id the player wears, empty for the stock blade.
pub(crate) const SABER_SKIN_CVAR: &str = "cg_saberSkin";
/// The blade skins this client knows (`docs/unlockables.md`, "Catalogue"); a look
/// naming any other id draws the stock blade.
pub(crate) const BLADE_SKINS: [&str; 1] = ["saber_sun"];

/// The blade skin `id` names, if this client knows it.
pub(crate) fn blade_skin(id: &str) -> Option<&'static str> {
    BLADE_SKINS.iter().copied().find(|known| *known == id)
}

/// What one player wears, as this client draws it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Worn {
    /// The blade skin, one of [`BLADE_SKINS`]; `None` for the stock blade.
    pub(crate) saber_skin: Option<&'static str>,
    /// The Illuminate holocron is lit.
    pub(crate) illuminate: bool,
}

impl Worn {
    /// What `look` draws as here: an unknown skin is the stock blade.
    pub(crate) fn of(look: &Look) -> Self {
        Self {
            saber_skin: blade_skin(&look.saber),
            illuminate: look.illuminate,
        }
    }

    /// The look the hub is told the local player wears: the skin only when the own
    /// profile lists it (`owned`), and the local holocron's lit state.
    pub(crate) fn own(skin_setting: &str, owned: impl Fn(&str) -> bool, lit: bool) -> Self {
        let skin_setting = skin_setting.trim();
        Self {
            saber_skin: blade_skin(skin_setting).filter(|skin| owned(skin)),
            illuminate: lit,
        }
    }

    /// The wire form, for `Service::set_look`.
    pub(crate) fn to_look(self) -> Look {
        Look {
            saber: self.saber_skin.unwrap_or_default().to_owned(),
            illuminate: self.illuminate,
        }
    }
}

/// A look the hub relayed for a slot, with the name its claim was made under.
#[derive(Clone, Debug, PartialEq)]
struct Claimed {
    claimed_name: String,
    worn: Worn,
}

/// Every slot's look on the current game server.
pub(crate) struct Looks {
    /// What the hub said each slot wears.
    claimed: [Option<Claimed>; SLOTS],
    /// What each slot is drawn wearing: the claimed looks whose names match, and the
    /// local player's own.
    worn: [Worn; SLOTS],
    /// The local player's slot on the server, if in a game.
    own_slot: Option<u8>,
    /// The local player's own look, gated.
    own: Worn,
    /// The roster's revision last taken, so it is read again only when it changes.
    pub(crate) roster_revision: Option<u64>,
    /// The server the hub looks are for.
    pub(crate) server: Option<std::net::SocketAddr>,
}

impl Default for Looks {
    fn default() -> Self {
        Self {
            claimed: std::array::from_fn(|_| None),
            worn: [Worn::default(); SLOTS],
            own_slot: None,
            own: Worn::default(),
            roster_revision: None,
            server: None,
        }
    }
}

impl Looks {
    /// The blade skin the player in `client` wears, one of [`BLADE_SKINS`]; `None` for
    /// the stock blade (and for a slot out of range).
    // The seam the blade skins' drawing plugs into (`saber_skins.rs`, separate work).
    #[allow(dead_code)]
    pub(crate) fn saber_skin_id(&self, client: usize) -> Option<&'static str> {
        self.worn.get(client)?.saber_skin
    }

    /// Whether the player in `client` has their Illuminate holocron lit.
    pub(crate) fn illuminated(&self, client: usize) -> bool {
        self.worn.get(client).is_some_and(|worn| worn.illuminate)
    }

    /// The local player's own blade skin, gated by its profile's unlocks: for the
    /// first-person blade and the Character page's preview, in a game or not.
    // The seam the blade skins' drawing plugs into (`saber_skins.rs`, separate work).
    #[allow(dead_code)]
    pub(crate) fn own_saber_skin(&self) -> Option<&'static str> {
        self.own.saber_skin
    }

    /// The local player's own look.
    pub(crate) fn own(&self) -> Worn {
        self.own
    }

    /// Forget every hub look: another server, or none.
    pub(crate) fn clear(&mut self, server: Option<std::net::SocketAddr>) {
        self.claimed = std::array::from_fn(|_| None);
        self.roster_revision = None;
        self.server = server;
        self.rebuild(|_| None);
    }

    /// Take the presence roster's looks in place of every hub look: `(slot, claimed
    /// name, look)` for each live claim, a claim without a look wearing none.
    pub(crate) fn replace_roster<'a>(
        &mut self,
        roster: impl IntoIterator<Item = (u8, &'a str, Option<&'a Look>)>,
    ) {
        self.claimed = std::array::from_fn(|_| None);
        for (slot, claimed_name, look) in roster {
            if let Some(look) = look {
                self.set(slot, claimed_name, look);
            }
        }
    }

    /// Take a look event from the feed: newer than the roster, so it wins until the
    /// roster changes again.
    pub(crate) fn apply_event(&mut self, slot: u8, claimed_name: &str, look: &Look) {
        self.set(slot, claimed_name, look);
    }

    fn set(&mut self, slot: u8, claimed_name: &str, look: &Look) {
        if let Some(entry) = self.claimed.get_mut(usize::from(slot)) {
            *entry = Some(Claimed {
                claimed_name: claimed_name.to_owned(),
                worn: Worn::of(look),
            });
        }
    }

    /// The local player's slot on the server, if in a game.
    pub(crate) fn own_slot(&self) -> Option<u8> {
        self.own_slot
    }

    /// The local player's slot (`None` out of a game) and own look. A slot it left
    /// wears nothing until the next [`Looks::rebuild`].
    pub(crate) fn set_own(&mut self, slot: Option<u8>, own: Worn) {
        if slot != self.own_slot
            && let Some(left) = self
                .own_slot
                .and_then(|left| self.worn.get_mut(usize::from(left)))
        {
            *left = Worn::default();
        }
        self.own_slot = slot;
        self.own = own;
        if let Some(entry) = slot.and_then(|slot| self.worn.get_mut(usize::from(slot))) {
            *entry = own;
        }
    }

    /// Rebuild what each slot is drawn wearing; `shown` is the name the game shows in
    /// a slot. Only slots with a hub look ask for it.
    pub(crate) fn rebuild(&mut self, shown: impl Fn(u8) -> Option<String>) {
        for (slot, (worn, claimed)) in self.worn.iter_mut().zip(&self.claimed).enumerate() {
            *worn = claimed
                .as_ref()
                .filter(|claimed| {
                    u8::try_from(slot)
                        .ok()
                        .and_then(&shown)
                        .is_some_and(|shown| {
                            sjk_identity::names_match(&claimed.claimed_name, &shown)
                        })
                })
                .map_or_else(Worn::default, |claimed| claimed.worn);
        }
        let (slot, own) = (self.own_slot, self.own);
        self.set_own(slot, own);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(saber: &str, illuminate: bool) -> Look {
        Look {
            saber: saber.to_owned(),
            illuminate,
        }
    }

    /// The game shows `^2Sol` in slot 3 and `Fox` in slot 5.
    fn shown(slot: u8) -> Option<String> {
        match slot {
            3 => Some("^2Sol".to_owned()),
            5 => Some("Fox".to_owned()),
            _ => None,
        }
    }

    #[test]
    fn a_hub_look_counts_only_under_the_name_the_game_shows() {
        let mut looks = Looks::default();
        let sun = look("saber_sun", true);
        looks.replace_roster([
            (3, "sol", Some(&sun)),
            (5, "Wolf", Some(&sun)),
            (7, "Kit", Some(&sun)),
            (9, "Ren", None),
        ]);
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), Some("saber_sun"));
        assert!(looks.illuminated(3));
        assert_eq!(looks.saber_skin_id(5), None, "another name in the slot");
        assert!(!looks.illuminated(5));
        assert!(!looks.illuminated(7), "nobody in the slot");
        assert!(!looks.illuminated(9));
        assert!(!looks.illuminated(200), "no such slot");
        // Someone else takes slot 3: the look goes with the next rebuild.
        looks.rebuild(|slot| (slot == 3).then(|| "Fox".to_owned()));
        assert!(!looks.illuminated(3));
    }

    #[test]
    fn an_unknown_blade_skin_is_the_stock_blade() {
        let mut looks = Looks::default();
        looks.apply_event(3, "Sol", &look("saber_from_the_future", true));
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), None);
        assert!(looks.illuminated(3), "the rest of the look still counts");
    }

    #[test]
    fn feed_events_win_until_the_roster_changes() {
        let mut looks = Looks::default();
        looks.replace_roster([(3, "Sol", Some(&look("", false)))]);
        looks.rebuild(shown);
        assert!(!looks.illuminated(3));
        // The feed's event is newer than the roster.
        looks.apply_event(3, "Sol", &look("saber_sun", true));
        looks.apply_event(5, "Fox", &look("", true));
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), Some("saber_sun"));
        assert!(looks.illuminated(5));
        // A changed roster replaces every hub look: slot 5's claim has none now.
        looks.replace_roster([(3, "Sol", Some(&look("", true)))]);
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), None);
        assert!(looks.illuminated(3));
        assert!(!looks.illuminated(5));
    }

    #[test]
    fn another_server_clears_the_hub_looks_but_not_the_own_one() {
        let mut looks = Looks::default();
        looks.apply_event(3, "Sol", &look("saber_sun", true));
        looks.roster_revision = Some(4);
        let own = Worn {
            saber_skin: Some("saber_sun"),
            illuminate: true,
        };
        looks.set_own(Some(1), own);
        looks.rebuild(shown);
        assert!(looks.illuminated(3));
        let elsewhere = "5.6.7.8:29070".parse().ok();
        looks.clear(elsewhere);
        assert!(!looks.illuminated(3));
        assert_eq!(looks.roster_revision, None);
        assert_eq!(looks.server, elsewhere);
        assert_eq!(looks.own(), own);
        assert!(looks.illuminated(1), "the local player's own slot");
    }

    #[test]
    fn the_own_look_comes_from_the_settings_not_the_hub() {
        let mut looks = Looks::default();
        // The hub's roster lists the local player's own claim too.
        looks.replace_roster([(3, "Sol", Some(&look("saber_sun", true)))]);
        looks.set_own(Some(3), Worn::default());
        looks.rebuild(shown);
        assert_eq!(looks.saber_skin_id(3), None);
        assert!(!looks.illuminated(3));
        // Out of a game the own look still holds, for the Character page.
        let own = Worn::own("saber_sun", |_| true, false);
        looks.set_own(None, own);
        assert_eq!(looks.own_saber_skin(), Some("saber_sun"));
    }

    #[test]
    fn the_own_blade_skin_needs_the_unlock_and_a_known_id() {
        let owned = |id: &str| id == "saber_sun";
        assert_eq!(
            Worn::own(" saber_sun ", owned, true),
            Worn {
                saber_skin: Some("saber_sun"),
                illuminate: true,
            }
        );
        assert_eq!(Worn::own("saber_sun", |_| false, false).saber_skin, None);
        assert_eq!(Worn::own("saber_moon", |_| true, false).saber_skin, None);
        assert_eq!(Worn::own("", owned, true).to_look(), look("", true));
        assert_eq!(
            Worn::own("saber_sun", owned, false).to_look(),
            look("saber_sun", false)
        );
    }
}
