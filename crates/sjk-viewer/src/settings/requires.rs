//! Settings that do nothing while another one is off.
//!
//! A row whose setting has no effect right now (a nameplate's size with nameplates
//! off, a shadow's softness with no sun shadows) is drawn dimmed and its detail says
//! what it needs, so a player does not move a slider and wonder why nothing changed.
//! The row still works: what it is set to applies as soon as its parent is on.
//!
//! One [`Rule`] per dependency, each checked against the code that reads the cvars
//! (the reference after it), not guessed from a description: a rule that is wrong
//! tells a player a working setting is dead. A setting with several rules needs all
//! of them; a rule naming several parents needs any one.

use crate::console::ViewerConsole;

/// What a parent setting must be for the child to have an effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum When {
    /// A switch that is on, or a number that is not zero.
    On,
    /// A switch that is off.
    Off,
    /// Any value but this one.
    Not(&'static str),
    /// This value.
    Is(&'static str),
}

/// One parent a setting can depend on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Need {
    pub(super) parent: &'static str,
    pub(super) when: When,
}

const fn on(parent: &'static str) -> Need {
    Need {
        parent,
        when: When::On,
    }
}

const fn off(parent: &'static str) -> Need {
    Need {
        parent,
        when: When::Off,
    }
}

const fn not(parent: &'static str, value: &'static str) -> Need {
    Need {
        parent,
        when: When::Not(value),
    }
}

const fn is(parent: &'static str, value: &'static str) -> Need {
    Need {
        parent,
        when: When::Is(value),
    }
}

/// Setting `cvar` has an effect only while one of `any` holds.
#[derive(Debug)]
pub(super) struct Rule {
    pub(super) cvar: &'static str,
    pub(super) any: &'static [Need],
}

const fn rule(cvar: &'static str, any: &'static [Need]) -> Rule {
    Rule { cvar, any }
}

include!("requires_rules.rs");

impl Need {
    /// Whether `console`'s value of the parent lets the child act. A parent that is
    /// not registered holds nothing back.
    fn holds(&self, console: &ViewerConsole) -> bool {
        let Some(value) = console.cvar(self.parent) else {
            return true;
        };
        let same = |wanted: &str| {
            let text = value.as_text();
            match (text.trim().parse::<f64>(), wanted.parse::<f64>()) {
                (Ok(value), Ok(wanted)) => (value - wanted).abs() < 1e-6,
                _ => text.trim().eq_ignore_ascii_case(wanted),
            }
        };
        match self.when {
            When::On => super::switch_on(value),
            When::Off => !super::switch_on(value),
            When::Not(wanted) => !same(wanted),
            When::Is(wanted) => same(wanted),
        }
    }
}

/// The first rule that keeps setting `cvar` from having an effect right now; `None`
/// when it acts (or depends on nothing).
pub(super) fn idle(console: &ViewerConsole, cvar: &str) -> Option<&'static Rule> {
    RULES
        .iter()
        .filter(|rule| rule.cvar.eq_ignore_ascii_case(cvar))
        .find(|rule| !rule.any.iter().any(|need| need.holds(console)))
}

/// What `rule` asks for, in the rows' own names: "Nameplates", "World sun shadows
/// or Character sun shadows", "Camera style on SJK", "Shield hit sphere off".
pub(super) fn wants(rule: &Rule) -> String {
    let mut text = String::new();
    for (index, need) in rule.any.iter().enumerate() {
        if index > 0 {
            text.push_str(" or ");
        }
        let label = super::row_label(need.parent).map_or(need.parent, |label| {
            super::help::classic_label(need.parent, label).0
        });
        text.push_str(label);
        match need.when {
            When::On | When::Not(_) => {}
            When::Off => text.push_str(" off"),
            When::Is(value) => {
                text.push_str(" on ");
                // The value as the parent's row names it: a stop, a choice.
                let shown = super::search::rows()
                    .iter()
                    .find(|setting| setting.cvar.eq_ignore_ascii_case(need.parent))
                    .map_or(std::borrow::Cow::Borrowed(value), |setting| {
                        super::present::value(setting, value)
                    });
                text.push_str(super::classic_view::choice_label(need.parent, &shown));
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    /// A rule about a setting or a parent that does not exist would never show, or
    /// never lift.
    #[test]
    fn every_rule_names_settings_with_a_row() {
        let (_directory, console) = console();
        for rule in RULES {
            assert!(super::super::has_row(rule.cvar), "{} has no row", rule.cvar);
            assert!(!rule.any.is_empty(), "{}", rule.cvar);
            for need in rule.any {
                assert!(
                    console.cvar(need.parent).is_some(),
                    "{}: {} is not registered",
                    rule.cvar,
                    need.parent
                );
                assert!(
                    super::super::row_label(need.parent).is_some(),
                    "{}: {} has no row to name",
                    rule.cvar,
                    need.parent
                );
                assert_ne!(need.parent, rule.cvar);
            }
        }
    }

    #[test]
    fn a_row_is_idle_until_its_parent_is_on_and_says_what_it_needs() {
        let (_directory, mut console) = console();
        console.set_cvar("cg_nameplate", "0");
        let rule = idle(&console, "cg_nameplateScale").expect("nameplates are off");
        assert_eq!(wants(rule), "See & share nameplates");
        console.set_cvar("cg_nameplate", "1");
        assert!(idle(&console, "cg_nameplateScale").is_none());
        // A setting with no rule always acts.
        assert!(idle(&console, "cg_fov").is_none());
    }

    #[test]
    fn a_rule_with_several_parents_needs_any_one() {
        let (_directory, mut console) = console();
        for parent in [
            "r_worldSunShadows",
            "r_actorSunShadows",
            "r_dayNight",
            "r_volumetrics",
        ] {
            console.set_cvar(parent, "0");
        }
        let rule = idle(&console, "r_sunShadowTaps").expect("no sun shadows");
        assert_eq!(
            wants(rule),
            "World sun shadows or Character sun shadows or Real-time lighting or Light shafts"
        );
        // A value a parent must be on reads as its row names it.
        console.set_cvar("com_maxfps", "125");
        let rule = idle(&console, crate::runtime_settings::MONITOR_CAP_CVAR).expect("a set cap");
        assert_eq!(wants(rule), "FPS cap on Auto");
        console.set_cvar("r_actorSunShadows", "1");
        assert!(idle(&console, "r_sunShadowTaps").is_none());
    }
}
