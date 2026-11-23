//! The WHEEL OF DOOM's hub calls (the hub's `PROTOCOL.md`, "WHEEL OF DOOM", since
//! 07/11/2026). Until 20/11/2026 wheels stood on maps and were spun for a DOOM coin; the
//! coin now rolls at a Force box (`POST /v1/mystery-box/doom`), and the wheels, their
//! spin and their feed event left the client on 23/11/2026. This crate reads the
//! player's coins and a coin roll's answer; the viewer plays what it gave
//! (`sjk-viewer`'s `wheel_of_doom.rs` and `mystery_box.rs`, `docs/wheel-of-doom.md`).

use crate::hub::HubError;
use crate::mystery_box::BoxHolocron;
use crate::wire::Profile;
use serde::Deserialize;

/// `GET /v1/wheel-of-doom`, of which the client reads the player's DOOM coins (the hub
/// still sends the placed wheels, the cooldown and the segments older clients spin).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Wheel {
    /// The DOOM coins the player holds.
    #[serde(default)]
    pub coins: u64,
}

/// One DOOM coin roll at a Force box (a mystery box, since 20/11/2026: the WHEEL OF DOOM
/// merged into it), as the roller is answered (`POST /v1/mystery-box/doom`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Doom {
    /// The box rolled at.
    #[serde(rename = "box")]
    pub box_id: i64,
    /// What it gave: one of the wheel's outcomes (`holo_rare`, `mirror`, ...).
    pub outcome: String,
    /// The holocron a holocron outcome gave.
    #[serde(default)]
    pub holocron: Option<BoxHolocron>,
    /// The DOOM coins the roller holds after.
    #[serde(default)]
    pub coins: Option<u64>,
}

/// `POST /v1/mystery-box/doom`'s answer.
#[derive(Clone, Debug, Deserialize)]
pub struct DoomAnswer {
    /// What the roll gave.
    pub doom: Doom,
    /// The roller's profile after (a holocron won).
    pub profile: Profile,
}

/// What a feed event of the mystery box's toy says of a coin roll: box `box` gave
/// `outcome` (`doom_<box>_<outcome>`, since 20/11/2026).
pub fn parse_doom_event(event: &str) -> Option<(i64, &str)> {
    let (id, outcome) = event.strip_prefix("doom_")?.split_once('_')?;
    Some((id.parse().ok()?, outcome)).filter(|(_, outcome)| !outcome.is_empty())
}

/// A refused coin roll: why, in the player's words, and for the cooldown, the whole
/// seconds left before the next roll (read from the hub's message, which says it as
/// "again in N s").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refused {
    /// Why, in the player's words.
    pub words: String,
    /// The seconds left of the cooldown (`wheel_cooldown`).
    pub left: Option<u64>,
}

/// A refused DOOM coin roll at a Force box (`POST /v1/mystery-box/doom`, which answers
/// the wheel's codes) in the player's words, with the cooldown's seconds left.
pub fn box_refusal(error: &HubError) -> Refused {
    let HubError::Rejected { code, message, .. } = error else {
        return refusal(error);
    };
    let seconds = message
        .split(|c: char| !c.is_ascii_digit())
        .rfind(|digits| !digits.is_empty())
        .and_then(|digits| digits.parse::<u64>().ok());
    let (words, left) = match (code.as_str(), seconds) {
        ("wheel_busy", Some(left)) => (
            format!("Someone is rolling a DOOM coin at this Force box: {left} s left"),
            None,
        ),
        ("wheel_cooldown", Some(left)) => {
            (format!("Your next DOOM coin roll in {left} s"), Some(left))
        }
        ("wheel_busy", None) => (
            "Someone is rolling a DOOM coin at this Force box.".to_owned(),
            None,
        ),
        ("wheel_cooldown", None) => ("Roll your next DOOM coin a little later.".to_owned(), None),
        ("no_doom_coin", _) => (
            "No DOOM coin: the teddy bear in a Force box brings one.".to_owned(),
            None,
        ),
        ("wheel_off", _) => ("DOOM coin rolls are off for now.".to_owned(), None),
        ("no_such_box", _) => ("That Force box is not here any more.".to_owned(), None),
        ("box_server", _) => ("Force boxes do not open on this server.".to_owned(), None),
        ("client_update_required", _) => ("Update SJK to roll a DOOM coin.".to_owned(), None),
        _ => return refusal(error),
    };
    Refused { words, left }
}

/// A refusal [`box_refusal`] has no words of its own for: the hub's message, or that
/// this hub has no coin roll.
fn refusal(error: &HubError) -> Refused {
    let words = match error {
        HubError::Rejected { code, .. } if code == "not_on_server" => {
            "The hub does not see you on this server yet.".to_owned()
        }
        HubError::Rejected { message, .. } if !message.is_empty() => message.clone(),
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub does not roll DOOM coins yet.".to_owned()
        }
        other => other.to_string(),
    };
    Refused { words, left: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_coins_read_from_what_the_hub_still_sends_of_the_wheels() {
        let wheel: Wheel = serde_json::from_str(
            r#"{"wheels":[{"id":2,"map":"mp/ffa3","checksum":-1234,"server":"",
                "origin":[1,2,3],"yaw":90,"placed":5}],"price":0,"coins_price":1,"coins":3,
                "cooldown":40,"spin":35,"open":false,
                "outcomes":[{"id":"mythical","per_mille":10}]}"#,
        )
        .unwrap();
        assert_eq!(wheel.coins, 3);
        assert_eq!(serde_json::from_str::<Wheel>("{}").unwrap().coins, 0);
    }

    #[test]
    fn other_refusals_say_the_hubs_message_or_that_it_has_no_roll() {
        let rejected = |code: &str, message: &str| HubError::Rejected {
            status: 409,
            code: code.to_owned(),
            message: message.to_owned(),
        };
        assert_eq!(
            box_refusal(&rejected("opening_staff_only", "raw")).words,
            "raw"
        );
        assert!(
            box_refusal(&rejected("not_on_server", "raw"))
                .words
                .contains("does not see you")
        );
        let missing = HubError::Protocol("status 404".to_owned());
        assert_eq!(
            box_refusal(&missing),
            Refused {
                words: "This SJK hub does not roll DOOM coins yet.".to_owned(),
                left: None
            }
        );
    }

    #[test]
    fn a_coin_roll_event_names_its_box_and_outcome() {
        assert_eq!(
            parse_doom_event("doom_12_holo_rare"),
            Some((12, "holo_rare"))
        );
        assert_eq!(parse_doom_event("doom_3_mirror"), Some((3, "mirror")));
        assert_eq!(parse_doom_event("open_3_coins"), None);
        assert_eq!(parse_doom_event("doom_x_mirror"), None);
        assert_eq!(parse_doom_event("doom_3_"), None);
        let doom: Doom = serde_json::from_value(serde_json::json!({
            "id": 9, "box": 4, "outcome": "holo_rare",
            "holocron": {"id": 2, "tier": "rare"}, "coins": 1,
        }))
        .unwrap();
        assert_eq!((doom.box_id, doom.coins), (4, Some(1)));
        assert_eq!(doom.holocron.unwrap().tier, "rare");
    }

    #[test]
    fn a_refused_coin_roll_speaks_of_the_force_box() {
        let rejected = |code: &str, message: &str| HubError::Rejected {
            status: 409,
            code: code.to_owned(),
            message: message.to_owned(),
        };
        let busy = box_refusal(&rejected("wheel_busy", "busy for 4 s"));
        assert_eq!(
            busy.words,
            "Someone is rolling a DOOM coin at this Force box: 4 s left"
        );
        assert_eq!(busy.left, None);
        let wait = box_refusal(&rejected("wheel_cooldown", "again in 9 s"));
        assert_eq!(
            (wait.words.as_str(), wait.left),
            ("Your next DOOM coin roll in 9 s", Some(9))
        );
        for code in ["no_doom_coin", "wheel_off", "no_such_box", "box_server"] {
            let words = box_refusal(&rejected(code, "")).words;
            assert!(!words.contains("WHEEL"), "{code}: {words}");
        }
    }
}
