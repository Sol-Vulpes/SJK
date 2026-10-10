//! Staff requests (`PROTOCOL.md`, "Staff"): what a key the hub's operator made staff
//! may do from inside the game: find players, verify them, give and take back medals and
//! unlockables, give and take back holocrons, clear achievements, moderate the SJK chat
//! and players' pictures, merge two players into one and unlink a key from a player. The
//! hub refuses every one from a key that is not staff; the client only offers them to
//! a player whose own profile says `staff`.

use crate::wire::Profile;

/// One staff request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaffRequest {
    /// Find players: 16 hex digits for one key, a part of a name, or empty for the
    /// players seen most recently.
    Search(String),
    /// Give a medal.
    Award {
        /// The key that gets it.
        key_id: String,
        /// The medal's id.
        medal: String,
        /// Why, shown with the medal; may be empty.
        note: String,
    },
    /// Take one award of a medal back.
    Unaward {
        /// The key that holds it.
        key_id: String,
        /// The medal's id.
        medal: String,
    },
    /// Grant an unlockable (`PROTOCOL.md`, "Unlocks").
    Unlock {
        /// The key that gets it.
        key_id: String,
        /// The unlockable's id.
        unlock: String,
        /// Shown with it; may be empty.
        note: String,
    },
    /// Take an unlockable back.
    Relock {
        /// The key that holds it.
        key_id: String,
        /// The unlockable's id.
        unlock: String,
    },
    /// Clear achievements.
    ClearAchievements {
        /// The key whose achievements go.
        key_id: String,
        /// The achievement's id, or empty for every one.
        id: String,
    },
    /// Delete an SJK chat message for everyone.
    ChatDelete {
        /// The message's id.
        id: u64,
    },
    /// Mute or unmute a key in the SJK chat.
    ChatMute {
        /// The key.
        key_id: String,
        /// Mute (true) or unmute.
        muted: bool,
    },
    /// Take a key's picture down.
    AvatarRemove {
        /// The key.
        key_id: String,
    },
    /// Stop a key from uploading pictures (taking its picture down), or let it again.
    AvatarBlock {
        /// The key.
        key_id: String,
        /// Stop (true) or let again.
        blocked: bool,
    },
    /// Give a holocron of a tier (`PROTOCOL.md`, "Holocrons"). A gift counts toward no
    /// cap and is not announced.
    HolocronGive {
        /// The key that gets it.
        key_id: String,
        /// The tier's wire id (`uncommon`, `rare`, `legendary`, `mythical`).
        tier: String,
        /// Shown with it; may be empty.
        note: String,
    },
    /// Take one holocron back, by its number at the hub.
    HolocronRemove {
        /// The key that holds it.
        key_id: String,
        /// The holocron's number ([`crate::Holocron::id`]).
        id: u64,
    },
    /// Say whether the SJK team vouches for a player (the verified mark).
    Verify {
        /// The player's key.
        key_id: String,
        /// Vouch (true) or no longer.
        verified: bool,
    },
    /// Merge one player into another, for good: everything `from` holds goes to the
    /// player of `key_id`, and `from`'s keys become linked keys of that player. A staff
    /// player cannot be merged away.
    Merge {
        /// The player who is kept.
        key_id: String,
        /// The player merged away.
        from: String,
    },
    /// Detach a linked key from its player. The key starts afresh the next time it is
    /// used; nothing it brought moves back.
    Unlink {
        /// The linked key (never a player's main key).
        key_id: String,
    },
}

impl StaffRequest {
    /// The key the request changes, if it changes one.
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Search(_) | Self::ChatDelete { .. } => None,
            Self::Award { key_id, .. }
            | Self::Unaward { key_id, .. }
            | Self::Unlock { key_id, .. }
            | Self::Relock { key_id, .. }
            | Self::ClearAchievements { key_id, .. }
            | Self::ChatMute { key_id, .. }
            | Self::AvatarRemove { key_id }
            | Self::AvatarBlock { key_id, .. }
            | Self::HolocronGive { key_id, .. }
            | Self::HolocronRemove { key_id, .. }
            | Self::Verify { key_id, .. }
            | Self::Merge { key_id, .. }
            | Self::Unlink { key_id } => Some(key_id),
        }
    }
}

/// What the staff requests brought back.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StaffState {
    /// Counts answers, so a reader can tell a new one.
    pub serial: u64,
    /// The players the last search found, updated by later answers about them.
    pub players: Vec<Profile>,
    /// What the last request came to: done, or why not.
    pub message: String,
    /// Whether the last request failed.
    pub failed: bool,
    /// A request is on its way.
    pub busy: bool,
}

impl StaffState {
    /// Put `profile`, the hub's answer about a key, in place of that key's entry.
    pub fn replace(&mut self, profile: &Profile) {
        if let Some(entry) = self
            .players
            .iter_mut()
            .find(|entry| entry.key_id == profile.key_id)
        {
            *entry = profile.clone();
        } else {
            self.players.insert(0, profile.clone());
        }
    }

    /// Take the player who owns `key_id` (their main key or a linked one) out of the
    /// list, as a merge does with the player merged away; `kept` is never taken out.
    pub fn remove(&mut self, key_id: &str, kept: &str) {
        self.players
            .retain(|entry| entry.key_id == kept || !entry.has_key(key_id));
    }
}

/// What a request's success is called on the page.
pub(crate) fn done(request: &StaffRequest, count: usize) -> String {
    match request {
        StaffRequest::Search(query) if query.is_empty() => {
            format!("{count} players seen lately")
        }
        StaffRequest::Search(_) => match count {
            0 => "Nobody found".to_owned(),
            1 => "1 player found".to_owned(),
            count => format!("{count} players found"),
        },
        StaffRequest::Award { medal, .. } => format!("Gave {medal}"),
        StaffRequest::Unaward { medal, .. } => format!("Took back {medal}"),
        StaffRequest::Unlock { unlock, .. } => format!("Unlocked {unlock}"),
        StaffRequest::Relock { unlock, .. } => format!("Took back {unlock}"),
        StaffRequest::ClearAchievements { id, .. } if id.is_empty() => {
            "Cleared every achievement".to_owned()
        }
        StaffRequest::ClearAchievements { id, .. } => format!("Cleared {id}"),
        StaffRequest::ChatDelete { .. } => "Deleted the message".to_owned(),
        StaffRequest::ChatMute { muted: true, .. } => "Muted in the chat".to_owned(),
        StaffRequest::ChatMute { muted: false, .. } => "Unmuted in the chat".to_owned(),
        StaffRequest::AvatarRemove { .. } => "Took the picture down".to_owned(),
        StaffRequest::AvatarBlock { blocked: true, .. } => {
            "Stopped their pictures and took theirs down".to_owned()
        }
        StaffRequest::AvatarBlock { blocked: false, .. } => {
            "They may upload pictures again".to_owned()
        }
        StaffRequest::HolocronGive { tier, .. } => format!("Gave a {tier} holocron"),
        StaffRequest::HolocronRemove { id, .. } => format!("Took back holocron #{id}"),
        StaffRequest::Verify { verified: true, .. } => "Verified".to_owned(),
        StaffRequest::Verify {
            verified: false, ..
        } => "No longer verified".to_owned(),
        StaffRequest::Merge { from, .. } => format!("Merged {from} into this player"),
        StaffRequest::Unlink { key_id } => format!("Unlinked {key_id}"),
    }
}
