//! The `saberskin` console command (`docs/unlockables.md`, "Settings and commands"):
//! alone it lists the catalogue's blade skins ([`crate::unlockables`]), each owned or
//! locked, and which one is worn; `saberskin <id>` or `saberskin none` sets
//! `cg_saberSkin`. A locked skin is set all the same and shows once the player's own hub
//! profile lists it (the gate is `GpuState::local_saber_skin`'s).

use crate::console::ViewerConsole;
use crate::unlockables::{self, Holdings, SABER_SKIN_CVAR, Unlockable};

/// Console command name.
pub(crate) const COMMAND: &str = "saberskin";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "List your blade skins, or wear one: saberskin <id> | none";

const USAGE: &str = "saberskin [<id> | none]";

/// What `saberskin <word>` asks to wear.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Choice {
    /// The stock blade (`none`).
    Stock,
    Skin(&'static Unlockable),
}

impl Choice {
    /// The value `cg_saberSkin` takes.
    fn setting(self) -> &'static str {
        match self {
            Self::Stock => "",
            Self::Skin(unlockable) => unlockable.id,
        }
    }
}

/// Read the command's word.
pub(crate) fn parse(word: &str) -> Result<Choice, String> {
    if word.eq_ignore_ascii_case("none") {
        return Ok(Choice::Stock);
    }
    unlockables::blade_skin(word)
        .map(Choice::Skin)
        .ok_or_else(|| {
            format!("No blade skin \"{word}\": saberskin alone lists them (usage: {USAGE})")
        })
}

/// The lines `saberskin` alone prints: every blade skin with whether it is owned (and
/// since when) or locked (and how to get it), the worn one marked, and why ownership is
/// unknown when it is.
pub(crate) fn listing(setting: &str, holdings: Holdings<'_>) -> Vec<String> {
    let chosen = unlockables::blade_skin(setting);
    let mut lines =
        vec!["Blade skins (saberskin <id> wears one, saberskin none the stock blade):".to_owned()];
    for unlockable in unlockables::blade_skins() {
        let picked = chosen.is_some_and(|chosen| chosen.id == unlockable.id);
        let state = match holdings.unlock(unlockable.id) {
            Some(unlock) => {
                let since = crate::medals::date_text(unlock.granted);
                let owned = if since.is_empty() {
                    "^2owned".to_owned()
                } else {
                    format!("^2owned since {since}")
                };
                if picked {
                    format!("{owned}^7, worn")
                } else {
                    format!("{owned}^7")
                }
            }
            None if picked => format!(
                "^3locked^7, chosen: shows once unlocked ({})",
                unlockable.how_to_get
            ),
            None => format!("^3locked^7: {}", unlockable.how_to_get),
        };
        lines.push(format!("  {}  {}  {state}", unlockable.id, unlockable.name));
    }
    let setting = setting.trim();
    if chosen.is_none() && !setting.is_empty() {
        lines.push(format!(
            "cg_saberSkin names \"{setting}\", which this client does not know: the stock blade shows."
        ));
    }
    if let Some(reason) = holdings.reason() {
        lines.push(format!("^3{reason}"));
    }
    lines
}

/// What `saberskin <word>` answers once `cg_saberSkin` is set to `choice`.
pub(crate) fn answer(choice: Choice, holdings: Holdings<'_>) -> String {
    match choice {
        Choice::Stock => "You wear the stock blade.".to_owned(),
        Choice::Skin(unlockable) if holdings.unlock(unlockable.id).is_some() => {
            format!("You wear the {}.", unlockable.name)
        }
        Choice::Skin(unlockable) => match holdings.reason() {
            Some(reason) => format!(
                "Chose the {}; it shows only once your SJK profile holds it. {reason}",
                unlockable.name
            ),
            None => format!(
                "Chose the {}; it shows only once unlocked ({})",
                unlockable.name, unlockable.how_to_get
            ),
        },
    }
}

/// Run `saberskin` with `args` on `console`, `snapshot` being the identity's.
pub(crate) fn run(
    console: &mut ViewerConsole,
    args: &[String],
    snapshot: Option<&sjk_identity::Snapshot>,
) -> Result<Vec<String>, String> {
    let enabled = console.bool_cvar("cl_identity") == Some(true);
    let holdings = Holdings::of(enabled, snapshot);
    match args {
        [] => Ok(listing(
            console.text_cvar(SABER_SKIN_CVAR).unwrap_or_default(),
            holdings,
        )),
        [word] => {
            let choice = parse(word)?;
            if !console.set_cvar(SABER_SKIN_CVAR, choice.setting()) {
                return Err("cg_saberSkin could not be set".to_owned());
            }
            Ok(vec![answer(choice, holdings)])
        }
        _ => Err(format!("usage: {USAGE}")),
    }
}

impl crate::GpuState {
    /// `saberskin [<id> | none]`.
    pub(crate) fn saber_skin_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let snapshot = crate::player_identity::snapshot();
        let Some(console) = self.console.as_mut() else {
            return Err("The console is not ready".to_owned());
        };
        run(console, args, snapshot.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::Unlock;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn sun() -> Unlock {
        Unlock {
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: String::new(),
        }
    }

    #[test]
    fn the_listing_says_owned_locked_and_worn() {
        let owned = [sun()];
        let lines = listing("saber_sun", Holdings::Known(&owned));
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("saber_sun  Sun blade"), "{}", lines[1]);
        assert!(
            lines[1].contains("owned since 07/10/2026^7, worn"),
            "{}",
            lines[1]
        );
        let lines = listing("", Holdings::Known(&owned));
        assert!(!lines[1].contains("worn"));
        let lines = listing(" SABER_SUN ", Holdings::Known(&[]));
        assert!(
            lines[1].contains("locked^7, chosen: shows once unlocked (Given by the SJK team.)"),
            "{}",
            lines[1]
        );
        let lines = listing("saber_moon", Holdings::IdentityOff);
        assert!(lines[1].contains("locked^7: Given by the SJK team."));
        assert!(lines[2].contains("\"saber_moon\", which this client does not know"));
        assert!(lines[3].contains("need the SJK identity"), "{}", lines[3]);
    }

    #[test]
    fn words_choose_a_skin_or_the_stock_blade() {
        assert_eq!(parse("none"), Ok(Choice::Stock));
        assert_eq!(parse("NONE"), Ok(Choice::Stock));
        assert_eq!(
            parse("Saber_Sun").map(Choice::setting),
            Ok("saber_sun"),
            "the catalogue's spelling is kept"
        );
        assert!(
            parse("saber_moon")
                .unwrap_err()
                .contains("saberskin alone lists them")
        );
        let owned = [sun()];
        let sun = parse("saber_sun").unwrap();
        assert_eq!(
            answer(sun, Holdings::Known(&owned)),
            "You wear the Sun blade."
        );
        assert!(answer(sun, Holdings::Known(&[])).contains("shows only once unlocked"));
        assert!(answer(sun, Holdings::IdentityOff).contains("need the SJK identity"));
        assert_eq!(
            answer(Choice::Stock, Holdings::IdentityOff),
            "You wear the stock blade."
        );
    }

    #[test]
    fn the_command_sets_cg_saber_skin_even_for_a_locked_skin() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            ViewerConsole::new(directory.path().join("config.cfg")).expect("a console");
        let lines = run(&mut console, &words("saber_SUN"), None).unwrap();
        assert_eq!(console.text_cvar(SABER_SKIN_CVAR).unwrap(), "saber_sun");
        assert!(lines[0].contains("only once your SJK profile holds it"));
        let lines = run(&mut console, &[], None).unwrap();
        assert!(lines[1].contains("chosen"));
        run(&mut console, &words("none"), None).unwrap();
        assert_eq!(console.text_cvar(SABER_SKIN_CVAR).unwrap(), "");
        assert!(run(&mut console, &words("saber_moon"), None).is_err());
        assert_eq!(console.text_cvar(SABER_SKIN_CVAR).unwrap(), "");
        assert!(run(&mut console, &words("a b"), None).is_err());
    }
}
