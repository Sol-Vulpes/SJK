//! Who the SJK chat page's side window lists (`docs/hub-chat.md`, "Who is online"):
//! the SJK players reading the chat now and the ones seen most recently.
//!
//! A hub that lists them (the feed's `people`, [`sjk_identity::People`]) gives both:
//! online is a key that read the feed in the last minute (the keys the online count
//! counts), playing one of them holding a live claim marked active; recent are the keys
//! that read it before, most recently seen first. A hub that does not list them gives
//! only the count, and the window falls back on the chat itself: the last senders,
//! newest first, with when they last spoke.

use sjk_identity::{ChatState, normal_form};

/// Rows of each part.
pub(super) const ONLINE_ROWS: usize = 5;
pub(super) const RECENT_ROWS: usize = 4;

/// One player in the window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Row {
    pub(super) key_id: String,
    /// The name as the hub or the message gives it, colour codes kept.
    pub(super) name: String,
    pub(super) verified: bool,
    pub(super) staff: bool,
    /// Their picture's version as the hub listed it (`""` for none); `None` when the row
    /// comes from a message, which does not say.
    pub(super) avatar: Option<String>,
    /// Playing a match now (online rows only).
    pub(super) playing: bool,
    /// When they were last seen (recent rows), unix seconds.
    pub(super) seen: u64,
    /// Muted on this PC.
    pub(super) muted: bool,
}

/// What the window shows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Listed {
    /// The hub lists who is online; else only `count` is known and `recent` comes
    /// from the chat's senders.
    pub(super) listed: bool,
    /// How many read the chat now, as the hub says.
    pub(super) count: u32,
    /// The online players on show: playing first, then by name.
    pub(super) online: Vec<Row>,
    /// Online players left out for room.
    pub(super) more: usize,
    /// The recently seen players, most recent first.
    pub(super) recent: Vec<Row>,
}

/// The window's players from `chat`, `muted` being the keys muted on this PC.
pub(super) fn derive(chat: &ChatState, muted: &[String]) -> Listed {
    let is_muted = |key: &str| muted.iter().any(|muted| muted == key);
    let Some(people) = &chat.people else {
        // No list: the last senders, newest first, a holocron drop being no message.
        let mut recent: Vec<Row> = Vec::with_capacity(RECENT_ROWS);
        for message in chat.messages.iter().rev() {
            if recent.len() == RECENT_ROWS {
                break;
            }
            if message.holocron.is_some()
                || message.key_id.is_empty()
                || recent.iter().any(|row| row.key_id == message.key_id)
            {
                continue;
            }
            recent.push(Row {
                key_id: message.key_id.clone(),
                name: message.name.clone(),
                verified: message.verified,
                staff: message.staff,
                avatar: None,
                playing: false,
                seen: u64::try_from(message.at).unwrap_or(0),
                muted: is_muted(&message.key_id),
            });
        }
        return Listed {
            listed: false,
            count: chat.online,
            online: Vec::new(),
            more: 0,
            recent,
        };
    };
    let row = |person: &sjk_identity::Person| Row {
        key_id: person.key_id.clone(),
        name: person.name.clone(),
        verified: person.verified,
        staff: person.staff,
        avatar: Some(person.avatar.clone()),
        playing: person.playing,
        seen: u64::try_from(person.seen).unwrap_or(0),
        muted: is_muted(&person.key_id),
    };
    // Online: each key once, playing first, then by name as players read it.
    let mut online: Vec<&sjk_identity::Person> = Vec::with_capacity(people.online.len());
    for person in &people.online {
        if !person.key_id.is_empty() && !online.iter().any(|seen| seen.key_id == person.key_id) {
            online.push(person);
        }
    }
    online.sort_by_cached_key(|person| {
        (
            !person.playing,
            normal_form(&person.name),
            person.key_id.clone(),
        )
    });
    // A full window keeps its last row to say how many more there are.
    let shown = if online.len() > ONLINE_ROWS {
        ONLINE_ROWS - 1
    } else {
        online.len()
    };
    let more = online.len() - shown;
    // Recent: not online now, most recently seen first, each key once.
    let mut recent: Vec<&sjk_identity::Person> = people
        .recent
        .iter()
        .filter(|person| {
            !person.key_id.is_empty() && !online.iter().any(|on| on.key_id == person.key_id)
        })
        .collect();
    recent.sort_by_key(|person| std::cmp::Reverse(person.seen));
    let mut kept: Vec<Row> = Vec::with_capacity(RECENT_ROWS);
    for person in recent {
        if kept.len() == RECENT_ROWS {
            break;
        }
        if !kept.iter().any(|row| row.key_id == person.key_id) {
            kept.push(row(person));
        }
    }
    Listed {
        listed: true,
        count: chat.online,
        online: online[..shown].iter().map(|person| row(person)).collect(),
        more,
        recent: kept,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::{ChatMessage, DropMark, People, Person};

    fn person(key: &str, name: &str, seen: i64, playing: bool) -> Person {
        Person {
            key_id: key.repeat(16),
            name: name.to_owned(),
            seen,
            playing,
            ..Person::default()
        }
    }

    fn said(id: u64, key: &str, name: &str) -> ChatMessage {
        ChatMessage {
            id,
            at: 1_000 + id as i64,
            key_id: key.repeat(16),
            name: name.to_owned(),
            verified: key == "a",
            staff: false,
            text: "hi".to_owned(),
            holocron: None,
        }
    }

    fn names(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn online_lists_players_playing_first_then_by_name_and_counts_the_rest() {
        let chat = ChatState {
            online: 8,
            people: Some(People {
                online: vec![
                    person("a", "^2zed", 10, false),
                    person("b", "Fox", 10, true),
                    person("c", "^1Alpha", 10, false),
                    person("a", "^2zed", 10, false),
                    person("d", "kyle", 10, false),
                    person("e", "Bastila", 10, true),
                    person("f", "Mara", 10, false),
                ],
                recent: Vec::new(),
            }),
            ..ChatState::default()
        };
        let listed = derive(&chat, &[]);
        assert!(listed.listed);
        assert_eq!(listed.count, 8);
        // Six keys (one listed twice): four rows and a fifth that says two more.
        assert_eq!(names(&listed.online), ["Bastila", "Fox", "^1Alpha", "kyle"]);
        assert_eq!(listed.more, 2);
        assert!(listed.online[0].playing && !listed.online[2].playing);
        // Five fit without a "more" row.
        let mut few = chat.clone();
        few.people.as_mut().unwrap().online.truncate(5);
        let listed = derive(&few, &[]);
        assert_eq!((listed.online.len(), listed.more), (4, 0), "{listed:?}");
    }

    #[test]
    fn recent_leaves_out_who_is_online_and_comes_newest_first() {
        let chat = ChatState {
            people: Some(People {
                online: vec![person("a", "Sol", 100, false)],
                recent: vec![
                    person("b", "Old", 10, false),
                    person("a", "Sol", 50, false),
                    person("c", "Newest", 90, false),
                    person("d", "Middle", 60, false),
                    person("e", "Older", 20, false),
                    person("f", "Oldest", 5, false),
                ],
            }),
            ..ChatState::default()
        };
        let muted = ["d".repeat(16)];
        let listed = derive(&chat, &muted);
        assert_eq!(names(&listed.recent), ["Newest", "Middle", "Older", "Old"]);
        assert_eq!(listed.recent[0].seen, 90);
        assert!(listed.recent[1].muted && !listed.recent[0].muted);
        assert_eq!(listed.recent[0].avatar.as_deref(), Some(""));
    }

    #[test]
    fn a_hub_that_lists_nobody_falls_back_on_the_last_senders() {
        let mut drop = said(9, "z", "Lucky");
        drop.holocron = Some(DropMark {
            tier: "legendary".to_owned(),
            own: false,
        });
        let chat = ChatState {
            online: 4,
            messages: [
                said(1, "e", "Earliest"),
                said(2, "a", "^2Sol"),
                said(3, "b", "Fox"),
                said(4, "c", "Kyle"),
                said(5, "a", "^2Sol"),
                said(6, "d", "Mara"),
                drop,
            ]
            .into_iter()
            .collect(),
            ..ChatState::default()
        };
        let listed = derive(&chat, &[]);
        assert!(!listed.listed);
        assert_eq!(listed.count, 4);
        assert!(listed.online.is_empty());
        // Newest first, each sender once, the drop left out, four of them.
        assert_eq!(names(&listed.recent), ["Mara", "^2Sol", "Kyle", "Fox"]);
        assert_eq!(listed.recent[1].seen, 1_005, "when they last spoke");
        assert!(listed.recent[1].verified);
        assert_eq!(listed.recent[0].avatar, None, "a message does not say");
        // Nothing said: nobody.
        assert_eq!(derive(&ChatState::default(), &[]), Listed::default());
    }
}
