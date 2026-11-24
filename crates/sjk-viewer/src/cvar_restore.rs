//! Settings put aside while a mode holds them.
//!
//! A mode that sets a group of settings (`renderer vanilla` in `graphics_quality.rs`,
//! Tactical in `tactical.rs`, EJK mode in `ejk_mode.rs`, listed in [`MODES`]) first keeps
//! the player's own value of each in an archived cvar of its own (its record), as
//! `cvar=value` pairs joined by `;`, so leaving the mode gives them back, after a restart
//! too. This is that text's one format, and the one place that writes and reads it
//! ([`put_aside`], [`give_back`]).
//!
//! The modes share settings (all three set the Vanilla graphics), so they are told of
//! one another here rather than each keeping "what the setting held when I came on",
//! which for the second mode on is the first mode's value, not the player's:
//!
//! - A record only ever holds the **player's own** value. A mode coming on over a
//!   setting another applied mode already holds takes that mode's record of it, not the
//!   value on show.
//! - A mode going off gives a setting **to the mode that still holds it** (that mode's
//!   value) when one does, else back to the player.
//! - Each mode keeps its own way of going off ([`Mode::keeps_hand_changes`]): EJK mode
//!   leaves alone a setting the player changed by hand while it was on, Tactical gives
//!   everything back. A setting counts as changed by hand when it holds the value of
//!   none of the modes that hold it; a mode that leaves such a setting alone has let go
//!   of it.
//!
//! The format is the one every release wrote, so a profile saved with modes on reads as
//! it is, and a build from before this file told the modes of one another still reads
//! what this one saves. Records that such a build crossed (the second mode's record
//! holding the first mode's values) are told apart where they can be ([`own`]): of two
//! records that differ, the one that is not a mode's value is the player's.

use crate::console::ViewerConsole;
use crate::graphics_quality::holds;

/// A mode that holds a group of settings while it is on.
pub(crate) struct Mode {
    /// Its name, for the console.
    pub(crate) name: &'static str,
    /// The archived cvar its record is kept in; not empty while the mode is applied.
    pub(crate) record: &'static str,
    /// The settings it holds now and the value it holds each at (for a mode with
    /// levels, those of the level its record was taken for).
    pub(crate) values: fn(&ViewerConsole) -> Vec<(&'static str, &'static str)>,
    /// Going off, it leaves alone a setting changed by hand while it was on (the
    /// player's choice since); a mode without this gives every setting back.
    pub(crate) keeps_hand_changes: bool,
}

/// Every mode that puts settings aside. A new one is added here, and only here, to be
/// told of the others.
pub(crate) const MODES: &[&Mode] = &[
    &crate::graphics_quality::VANILLA_MODE,
    &crate::tactical::MODE,
    &crate::ejk_mode::MODE,
];

/// The `(cvar, value)` pairs of a record; anything else in it is skipped.
pub(crate) fn pairs(kept: &str) -> impl Iterator<Item = (&str, &str)> {
    kept.split(';').filter_map(|pair| pair.split_once('='))
}

/// `mode`'s record as it is saved.
fn record(console: &ViewerConsole, mode: &Mode) -> Vec<(String, String)> {
    let kept = console
        .cvar(mode.record)
        .map(sjk_shell::CvarValue::as_text)
        .unwrap_or_default();
    pairs(&kept)
        .map(|(cvar, value)| (cvar.to_owned(), value.to_owned()))
        .collect()
}

/// A record as the text its cvar holds.
fn text(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(cvar, value)| format!("{cvar}={value}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// Whether `mode` is applied: it has something put aside.
pub(crate) fn applied(console: &ViewerConsole, mode: &Mode) -> bool {
    matches!(console.cvar(mode.record), Some(sjk_shell::CvarValue::Text(kept)) if !kept.is_empty())
}

/// Whether `mode`'s record names `cvar`.
#[cfg(test)]
pub(crate) fn claims(console: &ViewerConsole, mode: &Mode, cvar: &str) -> bool {
    record(console, mode)
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(cvar))
}

/// Another applied mode than `mode`, read once for a change.
struct Other {
    mode: &'static Mode,
    record: Vec<(String, String)>,
    values: Vec<(&'static str, &'static str)>,
    /// Its record was changed here and is to be saved.
    changed: bool,
}

impl Other {
    fn at(&self, cvar: &str) -> Option<usize> {
        self.record
            .iter()
            .position(|(name, _)| name.eq_ignore_ascii_case(cvar))
    }

    /// The value the mode holds `cvar` at; `None` when its table no longer names it.
    fn value(&self, cvar: &str) -> Option<&'static str> {
        value_of(&self.values, cvar)
    }

    /// Whether `cvar` is on the mode's value (or the mode no longer says which).
    fn holds(&self, console: &ViewerConsole, cvar: &str) -> bool {
        self.value(cvar)
            .is_none_or(|value| holds(console, cvar, value))
    }

    fn forget(&mut self, cvar: &str) {
        if let Some(at) = self.at(cvar) {
            self.record.remove(at);
            self.changed = true;
        }
    }

    fn keep(&mut self, cvar: &str, own: &str) {
        if let Some(at) = self.at(cvar)
            && self.record[at].1 != own
        {
            self.record[at].1 = own.to_owned();
            self.changed = true;
        }
    }
}

fn value_of(values: &[(&'static str, &'static str)], cvar: &str) -> Option<&'static str> {
    values
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(cvar))
        .map(|(_, value)| *value)
}

/// The applied modes other than `mode`.
fn others(console: &ViewerConsole, mode: &Mode) -> Vec<Other> {
    MODES
        .iter()
        .filter(|other| other.record != mode.record)
        .filter(|other| applied(console, other))
        .map(|other| Other {
            mode: other,
            record: record(console, other),
            values: (other.values)(console),
            changed: false,
        })
        .collect()
}

/// Whether two values of a setting are the same: numbers as numbers ("1.0" is "1").
fn same(left: &str, right: &str) -> bool {
    match (left.trim().parse::<f64>(), right.trim().parse::<f64>()) {
        (Ok(left), Ok(right)) => (left - right).abs() < 1e-6,
        _ => left.trim().eq_ignore_ascii_case(right.trim()),
    }
}

/// The player's own value of a setting, from what the records of the modes holding it
/// say (`recorded`) and the values those modes hold it at (`held`). The records agree
/// unless a build that did not tell the modes of one another wrote them: then the
/// second mode's record holds the first mode's value, so the record that is no mode's
/// value is the player's (the first, when that does not tell them apart).
fn own<'a>(recorded: &[&'a str], held: &[&str]) -> Option<&'a str> {
    recorded
        .iter()
        .copied()
        .find(|value| !held.iter().any(|mode| same(value, mode)))
        .or(recorded.first().copied())
}

/// `mode` comes on: put the player's own value of each of `values`' settings aside in
/// its record and set the mode's. Settings this client does not have (a build without a
/// toy) are skipped; what the record held before is replaced. One save.
pub(crate) fn put_aside(
    console: &mut ViewerConsole,
    mode: &Mode,
    values: &[(&'static str, &'static str)],
) {
    let mut others = others(console, mode);
    let mut kept: Vec<(String, String)> = Vec::with_capacity(values.len());
    for &(cvar, value) in values {
        let Some(current) = console.cvar(cvar).map(sjk_shell::CvarValue::as_text) else {
            continue;
        };
        let holding = |others: &[Other]| -> Vec<usize> {
            (0..others.len())
                .filter(|index| others[*index].at(cvar).is_some())
                .collect()
        };
        let mut claimed = holding(&others);
        // On none of their values: the player set it by hand since. The modes that
        // leave such a setting alone have let go of it.
        let by_hand = !claimed.is_empty()
            && !claimed
                .iter()
                .any(|index| others[*index].holds(console, cvar));
        if by_hand {
            for index in &claimed {
                if others[*index].mode.keeps_hand_changes {
                    others[*index].forget(cvar);
                }
            }
            claimed = holding(&others);
        }
        let recorded: Vec<&str> = claimed
            .iter()
            .filter_map(|index| {
                let other = &others[*index];
                other.at(cvar).map(|at| other.record[at].1.as_str())
            })
            .collect();
        let held: Vec<&str> = claimed
            .iter()
            .filter_map(|index| others[*index].value(cvar))
            .chain([value])
            .collect();
        let own = own(&recorded, &held).map_or(current, str::to_owned);
        for index in claimed {
            others[index].keep(cvar, &own);
        }
        kept.push((cvar.to_owned(), own));
    }
    let record = text(&kept);
    let records: Vec<(&str, String)> = others
        .iter()
        .filter(|other| other.changed)
        .map(|other| (other.mode.record, text(&other.record)))
        .collect();
    console.set_cvars(
        kept.iter()
            .filter_map(|(cvar, _)| Some((cvar.as_str(), value_of(values, cvar)?)))
            .chain(
                records
                    .iter()
                    .map(|(record, text)| (*record, text.as_str())),
            )
            .chain([(mode.record, record.as_str())]),
    );
}

/// `mode` goes off: each setting of its record goes to the mode that still holds it,
/// else back to the player, and the record is emptied. A mode that
/// [keeps hand changes](Mode::keeps_hand_changes) leaves alone a setting changed by
/// hand; a setting its table no longer names (put aside by an earlier version) is
/// given back as it was. One save.
pub(crate) fn give_back(console: &mut ViewerConsole, mode: &Mode) {
    let kept = record(console, mode);
    if kept.is_empty() {
        return;
    }
    let mine = (mode.values)(console);
    let mut others = others(console, mode);
    let mut writes: Vec<(String, String)> = Vec::with_capacity(kept.len());
    for (cvar, aside) in &kept {
        if console.cvar(cvar).is_none() {
            continue;
        }
        let holding = |others: &[Other]| -> Vec<usize> {
            (0..others.len())
                .filter(|index| others[*index].at(cvar).is_some())
                .collect()
        };
        let mut claimed = holding(&others);
        // The player's own value, which every record of it holds from now on.
        let recorded: Vec<&str> = [aside.as_str()]
            .into_iter()
            .chain(claimed.iter().filter_map(|index| {
                let other = &others[*index];
                other.at(cvar).map(|at| other.record[at].1.as_str())
            }))
            .collect();
        let held: Vec<&str> = value_of(&mine, cvar)
            .into_iter()
            .chain(
                claimed
                    .iter()
                    .filter_map(|index| others[*index].value(cvar)),
            )
            .collect();
        let own = own(&recorded, &held).unwrap_or(aside).to_owned();
        for index in &claimed {
            others[*index].keep(cvar, &own);
        }
        let on_mine = value_of(&mine, cvar).is_none_or(|value| holds(console, cvar, value));
        let on_theirs = claimed.iter().any(|index| {
            others[*index].value(cvar).is_some() && others[*index].holds(console, cvar)
        });
        if !on_mine && !on_theirs {
            // Changed by hand since.
            if mode.keeps_hand_changes {
                continue;
            }
            for index in &claimed {
                if others[*index].mode.keeps_hand_changes {
                    others[*index].forget(cvar);
                }
            }
            claimed = holding(&others);
        }
        match claimed.first() {
            // Already on the value of a mode that holds it.
            Some(_) if on_theirs => {}
            Some(index) => {
                if let Some(value) = others[*index].value(cvar) {
                    writes.push((cvar.clone(), value.to_owned()));
                }
            }
            None => writes.push((cvar.clone(), own)),
        }
    }
    for other in others.iter().filter(|other| other.changed) {
        writes.push((other.mode.record.to_owned(), text(&other.record)));
    }
    writes.push((mode.record.to_owned(), String::new()));
    console.set_cvars(
        writes
            .iter()
            .map(|(cvar, value)| (cvar.as_str(), value.as_str())),
    );
}

/// The names of the applied modes other than `mode` that hold one of `cvars`.
pub(crate) fn held_by_others<'a>(
    console: &ViewerConsole,
    mode: &Mode,
    cvars: impl IntoIterator<Item = &'a str> + Clone,
) -> Vec<&'static str> {
    others(console, mode)
        .iter()
        .filter(|other| {
            cvars
                .clone()
                .into_iter()
                .any(|cvar| other.at(cvar).is_some())
        })
        .map(|other| other.mode.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics_quality::{self, Level};
    use crate::{ejk_mode, tactical};

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    fn value(console: &ViewerConsole, cvar: &str) -> String {
        console
            .cvar(cvar)
            .map(sjk_shell::CvarValue::as_text)
            .unwrap_or_default()
    }

    /// Every setting one of the three modes sets.
    fn every_setting() -> Vec<&'static str> {
        let mut cvars: Vec<&'static str> = Vec::new();
        for (cvar, _) in ejk_mode::values()
            .into_iter()
            .chain(tactical::values(tactical::Level::Max))
        {
            if !cvars.contains(&cvar) {
                cvars.push(cvar);
            }
        }
        cvars
    }

    fn snapshot(console: &ViewerConsole) -> Vec<(&'static str, String)> {
        every_setting()
            .into_iter()
            .map(|cvar| (cvar, value(console, cvar)))
            .collect()
    }

    /// A profile with choices of its own: some a mode's values already, some not, and
    /// no graphics preset's exactly.
    fn players_profile(console: &mut ViewerConsole) {
        Level::SolarHigh.apply(console);
        console.set_cvar("r_sunShadowTaps", "12");
        console.set_cvar(crate::stock_blades::CVAR, "1");
        console.set_cvar(crate::menu_hud::STYLE_CVAR, "radial");
        console.set_cvar("cg_nameplate", "1");
        console.set_cvar(crate::status_balloons::CVAR, "1");
        console.set_cvar(crate::weather::settings::FOG_CVAR, "2");
        console.set_cvar(crate::tags::SHOW_CVAR, "0");
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Which {
        Renderer,
        Tactical(tactical::Level),
        Ejk,
    }

    fn turn(console: &mut ViewerConsole, which: Which, on: bool) {
        match which {
            Which::Renderer => graphics_quality::set_renderer(
                console,
                if on {
                    graphics_quality::VANILLA
                } else {
                    graphics_quality::SJK
                },
            ),
            Which::Tactical(level) => {
                tactical::set_for_test(console, if on { level } else { tactical::Level::Off });
            }
            Which::Ejk => {
                console.set_cvar(ejk_mode::CVAR, if on { "1" } else { "0" });
                ejk_mode::follow(console);
            }
        }
    }

    /// While `which` is on, every setting it sets is on its value.
    fn assert_held(console: &ViewerConsole, which: Which, what: &str) {
        let values = match which {
            Which::Renderer => graphics_quality::applied_cvars(Level::Vanilla),
            Which::Tactical(level) => tactical::values(level),
            Which::Ejk => ejk_mode::values(),
        };
        for (cvar, value) in values {
            assert!(
                holds(console, cvar, value),
                "{what}: {which:?} holds {cvar} at {value:?}, not {:?}",
                self::value(console, cvar)
            );
        }
    }

    fn orders<const N: usize>(items: [Which; N]) -> Vec<Vec<Which>> {
        fn permute(left: Vec<Which>, done: Vec<Which>, out: &mut Vec<Vec<Which>>) {
            if left.is_empty() {
                out.push(done);
                return;
            }
            for index in 0..left.len() {
                let mut rest = left.clone();
                let next = rest.remove(index);
                let mut done = done.clone();
                done.push(next);
                permute(rest, done, out);
            }
        }
        let mut out = Vec::new();
        permute(items.to_vec(), Vec::new(), &mut out);
        out
    }

    /// The three modes turned on in every order and off in every order, at each
    /// Tactical level: after each step the modes still on hold their values, and at
    /// the end the player has exactly their own settings and nothing is put aside.
    #[test]
    fn every_order_of_on_and_off_ends_on_the_players_own_settings() {
        for level in [
            tactical::Level::Minimal,
            tactical::Level::Medium,
            tactical::Level::Max,
        ] {
            let modes = [Which::Renderer, Which::Tactical(level), Which::Ejk];
            for on in orders(modes) {
                for off in orders(modes) {
                    let (_directory, mut console) = console();
                    players_profile(&mut console);
                    let before = snapshot(&console);
                    let what = format!("on {on:?}, off {off:?}");
                    let mut up: Vec<Which> = Vec::new();
                    for which in &on {
                        turn(&mut console, *which, true);
                        up.push(*which);
                        for held in &up {
                            assert_held(&console, *held, &what);
                        }
                    }
                    for which in &off {
                        turn(&mut console, *which, false);
                        up.retain(|other| other != which);
                        for held in &up {
                            assert_held(&console, *held, &what);
                        }
                    }
                    assert_eq!(snapshot(&console), before, "{what}");
                    for mode in MODES {
                        assert!(!applied(&console, mode), "{what}: {}", mode.name);
                    }
                }
            }
        }
    }

    /// Two of the three, in both orders, and one alone: the same.
    #[test]
    fn any_two_modes_and_each_alone_give_the_players_settings_back() {
        let all = [
            Which::Renderer,
            Which::Tactical(tactical::Level::Max),
            Which::Ejk,
        ];
        for first in all {
            for second in all {
                for first_off_first in [true, false] {
                    let (_directory, mut console) = console();
                    players_profile(&mut console);
                    let before = snapshot(&console);
                    turn(&mut console, first, true);
                    if second != first {
                        turn(&mut console, second, true);
                    }
                    assert_held(&console, first, "both on");
                    assert_held(&console, second, "both on");
                    let (one, two) = if first_off_first {
                        (first, second)
                    } else {
                        (second, first)
                    };
                    turn(&mut console, one, false);
                    if two != one {
                        assert_held(&console, two, "one off");
                        turn(&mut console, two, false);
                    }
                    assert_eq!(snapshot(&console), before, "{first:?} then {second:?}");
                }
            }
        }
    }

    /// A restart between any two steps changes nothing: the records are the profile's.
    #[test]
    fn the_records_survive_a_restart_between_the_steps() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.cfg");
        let mut console = ViewerConsole::new(path.clone()).unwrap();
        players_profile(&mut console);
        let before = snapshot(&console);
        let modes = [
            Which::Ejk,
            Which::Tactical(tactical::Level::Medium),
            Which::Renderer,
        ];
        for which in modes {
            turn(&mut console, which, true);
            drop(console);
            console = ViewerConsole::new(path.clone()).unwrap();
        }
        for which in modes {
            turn(&mut console, which, false);
            drop(console);
            console = ViewerConsole::new(path.clone()).unwrap();
        }
        assert_eq!(snapshot(&console), before);
    }

    /// EJK mode still leaves a hand-changed setting alone, and Tactical still gives
    /// everything back, with the other on.
    #[test]
    fn each_mode_keeps_its_own_way_of_going_off_beside_another() {
        // Changed by hand under both, EJK mode off first: left alone; Tactical then
        // gives the player's own back, as it does alone.
        let (_directory, mut console) = console();
        players_profile(&mut console);
        let tactical = Which::Tactical(tactical::Level::Medium);
        turn(&mut console, Which::Ejk, true);
        turn(&mut console, tactical, true);
        console.set_cvar(crate::stock_blades::CVAR, "0");
        turn(&mut console, Which::Ejk, false);
        assert_eq!(value(&console, crate::stock_blades::CVAR), "0");
        turn(&mut console, tactical, false);
        assert_eq!(value(&console, crate::stock_blades::CVAR), "1");

        // Tactical off first: it gives the setting back, and EJK mode, which had let
        // go of a setting changed by hand, leaves it.
        let (_directory, mut console) = self::console();
        players_profile(&mut console);
        turn(&mut console, Which::Ejk, true);
        turn(&mut console, tactical, true);
        console.set_cvar(crate::stock_blades::CVAR, "0");
        turn(&mut console, tactical, false);
        assert_eq!(value(&console, crate::stock_blades::CVAR), "1");
        assert!(!claims(
            &console,
            &ejk_mode::MODE,
            crate::stock_blades::CVAR
        ));
        turn(&mut console, Which::Ejk, false);
        assert_eq!(value(&console, crate::stock_blades::CVAR), "1");

        // Changed by hand under EJK mode alone, then Tactical on and off: the player's
        // choice comes back, and EJK mode going off leaves it.
        let (_directory, mut console) = self::console();
        players_profile(&mut console);
        turn(&mut console, Which::Ejk, true);
        console.set_cvar("r_sceneHdr", "1");
        turn(&mut console, tactical, true);
        assert!(holds(&console, "r_sceneHdr", "0"));
        turn(&mut console, tactical, false);
        assert!(holds(&console, "r_sceneHdr", "1"));
        turn(&mut console, Which::Ejk, false);
        assert!(holds(&console, "r_sceneHdr", "1"));
    }

    /// What a build before the modes were told of one another did: keep the values on
    /// show, whoever set them.
    fn old_turn_on(console: &mut ViewerConsole, record: &str, values: &[(&str, &str)]) {
        let kept = values
            .iter()
            .filter_map(|(cvar, _)| Some(format!("{cvar}={}", console.cvar(cvar)?.as_text())))
            .collect::<Vec<_>>()
            .join(";");
        console.set_cvars(values.iter().copied().chain([(record, kept.as_str())]));
    }

    /// A profile saved by an earlier build with one mode on reads as it is: the
    /// format is the same, and off gives the player's settings back.
    #[test]
    fn a_record_an_earlier_build_saved_is_read_as_it_is() {
        for which in [
            Which::Renderer,
            Which::Tactical(tactical::Level::Max),
            Which::Ejk,
        ] {
            let (_directory, mut console) = console();
            players_profile(&mut console);
            let before = snapshot(&console);
            match which {
                Which::Renderer => old_turn_on(
                    &mut console,
                    graphics_quality::RESTORE_CVAR,
                    &graphics_quality::applied_cvars(Level::Vanilla),
                ),
                Which::Tactical(level) => {
                    console.set_cvar(tactical::CVAR, "3");
                    old_turn_on(
                        &mut console,
                        tactical::RESTORE_CVAR,
                        &tactical::values(level),
                    );
                }
                Which::Ejk => {
                    console.set_cvar(ejk_mode::CVAR, "1");
                    old_turn_on(&mut console, ejk_mode::RESTORE_CVAR, &ejk_mode::values());
                }
            }
            assert_held(&console, which, "an earlier build's");
            turn(&mut console, which, false);
            assert_eq!(snapshot(&console), before, "{which:?}");
        }
    }

    /// A profile an earlier build saved with two modes on has the second's record
    /// crossed (it holds the first mode's values). Both records are read, the player's
    /// own values are told from the modes', and either order of off ends on them.
    #[test]
    fn records_an_earlier_build_crossed_still_give_the_players_settings_back() {
        let medium = tactical::Level::Medium;
        for ejk_first_on in [true, false] {
            for ejk_first_off in [true, false] {
                let (_directory, mut console) = console();
                players_profile(&mut console);
                let before = snapshot(&console);
                console.set_cvar(ejk_mode::CVAR, "1");
                console.set_cvar(tactical::CVAR, "2");
                let ejk = |console: &mut ViewerConsole| {
                    old_turn_on(console, ejk_mode::RESTORE_CVAR, &ejk_mode::values());
                };
                let tactical = |console: &mut ViewerConsole| {
                    old_turn_on(console, tactical::RESTORE_CVAR, &tactical::values(medium));
                };
                if ejk_first_on {
                    ejk(&mut console);
                    tactical(&mut console);
                    // Crossed: Tactical's record holds EJK mode's value, not the player's.
                    let crossed = value(&console, tactical::RESTORE_CVAR);
                    assert!(crossed.contains(&format!("{}=2", crate::stock_blades::CVAR)));
                } else {
                    tactical(&mut console);
                    ejk(&mut console);
                }
                let what = format!("EJK on first {ejk_first_on}, off first {ejk_first_off}");
                if ejk_first_off {
                    turn(&mut console, Which::Ejk, false);
                    assert_held(&console, Which::Tactical(medium), &what);
                    turn(&mut console, Which::Tactical(medium), false);
                } else {
                    turn(&mut console, Which::Tactical(medium), false);
                    assert_held(&console, Which::Ejk, &what);
                    turn(&mut console, Which::Ejk, false);
                }
                assert_eq!(snapshot(&console), before, "{what}");
            }
        }
    }

    #[test]
    fn a_record_reads_back_as_pairs_and_the_players_value_is_told_from_a_modes() {
        let read: Vec<_> = pairs("cg_fov=100;junk;con_font=;r_x=a=b").collect();
        assert_eq!(read, [("cg_fov", "100"), ("con_font", ""), ("r_x", "a=b")]);
        assert_eq!(pairs("").count(), 0);
        // The records agree: that value.
        assert_eq!(own(&["1", "1.0"], &["0"]), Some("1"));
        // They differ: the one that is no mode's value.
        assert_eq!(own(&["0", "4096"], &["0", "0"]), Some("4096"));
        assert_eq!(own(&["radial", "game"], &["game"]), Some("radial"));
        // Nothing tells them apart: the first.
        assert_eq!(own(&["0", "1"], &["0", "1"]), Some("0"));
        assert_eq!(own(&[], &["0"]), None);
        assert!(same(" 1.0", "1") && same("Game", "game") && !same("1", "2"));
    }

    /// Every mode has a record of its own, an archived text cvar empty by default.
    #[test]
    fn every_mode_has_its_own_record() {
        let (_directory, console) = console();
        for (index, mode) in MODES.iter().enumerate() {
            assert!(!mode.name.is_empty());
            assert!(
                matches!(console.cvar(mode.record), Some(sjk_shell::CvarValue::Text(kept)) if kept.is_empty()),
                "{}",
                mode.record
            );
            assert!(!applied(&console, mode));
            assert!(
                !MODES[..index]
                    .iter()
                    .any(|other| other.record == mode.record)
            );
            for (cvar, value) in (mode.values)(&console) {
                assert!(!cvar.contains([';', '=']) && !value.contains(';'), "{cvar}");
            }
        }
    }
}
