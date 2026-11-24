//! Search over every setting, in both of the SJK UI's modes and the classic+
//! panels: the rows of each of Advanced's categories in the rail's order (the
//! renderer's tabs under Graphics), each with its category's name, so a search
//! shows its results under their categories' headings and the detail box can
//! say where a setting lives.
//!
//! What players type is not always what a setting is called. The hub's settings
//! usage (2,438 searches up to 21/11/2026) shows three ways a search comes back
//! empty though the setting exists, and each has its answer here:
//!
//! - another word for it ("afk" for away, "pov" for the field of view, "msaa" for
//!   the edge smoothing): [`SYNONYMS`], one entry per word seen;
//! - a slip of the fingers ("caht", "crosshar", "namepalte", "voulme"): when
//!   nothing matches, names within a letter or two of what was typed ([`near`]);
//! - the keyboard left on another layout ("\u{446}\u{440}\u{443}\u{443}\u{434}"
//!   is "wheel" typed on a Russian one): when nothing matches, the same keys read
//!   as Latin ([`latin_keys`]).
//!
//! A search with many results ("hud" finds 69, "chat" 46) leads with the settings
//! whose own name carries the word, under [`BEST`], before the groups that only
//! mention it.

use super::catalog::*;
use super::{Advanced, Group, help, settings};
use std::sync::OnceLock;

/// Every searchable setting, in the OPTIONS groups' order, and its group.
struct Searchable {
    rows: Vec<Setting>,
    groups: Vec<&'static str>,
}

fn searchable() -> &'static Searchable {
    static SEARCHABLE: OnceLock<Searchable> = OnceLock::new();
    SEARCHABLE.get_or_init(|| {
        let tab =
            |caption: &str| settings(TABS.iter().position(|tab| *tab == caption).unwrap_or(0));
        let advanced = |group| Group::Advanced(group).rows();
        let sources: [(&'static str, &'static [Setting]); 16] = [
            ("Display", Group::Display.rows()),
            // The renderer and the Video rows Display leaves to Graphics.
            ("Graphics", tab("VIDEO")),
            ("Graphics: image", RENDER_IMAGE),
            ("Graphics: lighting", RENDER_LIGHTING),
            ("Graphics: shadows", RENDER_SHADOWS),
            ("Graphics: weather", RENDER_WEATHER),
            ("Sound", advanced(Advanced::Sound)),
            ("Controls", tab("CONTROLS")),
            ("Gameplay", advanced(Advanced::Gameplay)),
            ("Interface", advanced(Advanced::Interface)),
            ("HUD", advanced(Advanced::Hud)),
            ("Chat", advanced(Advanced::Chat)),
            // The quick wheel's pages, the one row of Interface left.
            ("Quick wheel", Group::Interface.rows()),
            ("Mods", advanced(Advanced::Mods)),
            ("Network", Group::Network.rows()),
            // Its rows are the others' but for its own last one.
            ("First setup", Group::Quick.rows()),
        ];
        let mut found = Searchable {
            rows: Vec::new(),
            groups: Vec::new(),
        };
        for (group, rows) in sources {
            for setting in rows {
                if !found.rows.iter().any(|seen| seen.cvar == setting.cvar) {
                    found.rows.push(*setting);
                    found.groups.push(group);
                }
            }
        }
        found
    })
}

/// The rows a search section shows (all of them; the panel filters).
pub(super) fn rows() -> &'static [Setting] {
    &searchable().rows
}

/// The OPTIONS group of searchable row `row`.
pub(super) fn group(row: usize) -> Option<&'static str> {
    searchable().groups.get(row).copied()
}

/// The heading of the results whose own name carries the word searched, shown
/// first when a search finds more than [`BEST_OVER`] settings.
pub(super) const BEST: &str = "Best matches";
/// A search finding this many settings or fewer lists them by group alone.
const BEST_OVER: usize = 8;

/// Words players type for a setting called otherwise: `(typed, names)`, each name a
/// part of the label of a setting meant. An entry answers a search that is the word,
/// begins with it ("quite mode") or, from three letters, begins it ("anti"). Each
/// was typed by players and found nothing, or not what it meant (the hub's settings
/// usage, 21/11/2026).
const SYNONYMS: &[(&str, &[&str])] = &[
    ("afk", &["away"]),
    ("idle", &["away"]),
    ("idling", &["away"]),
    ("pov", &["field of view", "weapon fov"]),
    ("msaa", &["supersampling", "fxaa"]),
    ("antialiasing", &["supersampling", "fxaa"]),
    ("anti-aliasing", &["supersampling", "fxaa"]),
    ("aliasing", &["supersampling", "fxaa"]),
    ("smoothing", &["supersampling", "fxaa"]),
    ("vsync", &["vertical sync"]),
    ("fullscreen", &["display mode"]),
    ("windowed", &["display mode"]),
    ("borderless", &["display mode"]),
    ("gamma", &["brightness"]),
    ("dark", &["brightness"]),
    ("hp", &["health"]),
    ("dance", &["emote"]),
    ("taunt", &["emote"]),
    ("quiet", &["do not disturb"]),
    ("quite", &["do not disturb"]),
    ("dnd", &["do not disturb"]),
    ("transparent", &["opacity", "backdrop"]),
    ("transparency", &["opacity", "backdrop"]),
    ("alpha", &["opacity", "backdrop"]),
    ("lightning", &["lighting"]),
    ("netgraph", &["lagometer"]),
    ("reticle", &["crosshair"]),
    ("reticule", &["crosshair"]),
    ("aim", &["crosshair"]),
    ("loud", &["volume"]),
    ("mute", &["volume"]),
    ("framerate", &["fps"]),
    ("frame rate", &["fps"]),
    ("smoothness", &["fps cap", "vertical sync"]),
    // EJK mode (`ejk_mode.rs`), by the names players give the client it follows and
    // the look they ask for.
    ("eternal", &["ejk mode"]),
    ("jof", &["ejk mode"]),
    ("openjk", &["ejk mode"]),
    ("ojk", &["ejk mode"]),
    ("classic", &["ejk mode", "chat style"]),
    ("retail", &["ejk mode"]),
    // The chat's style (`chat/style.rs`), by what players call the field they type in.
    ("composer", &["chat style"]),
    ("messagemode", &["chat style"]),
    ("say", &["chat style"]),
    ("cel", &["toon"]),
    ("cartoon", &["toon"]),
];

/// What a row is called and found by, lower case.
struct Names {
    label: String,
    cvar: String,
    group: String,
    help: String,
}

impl Names {
    fn of(setting: &Setting, group: &str) -> Self {
        Self {
            label: setting.label.to_ascii_lowercase(),
            cvar: setting.cvar.to_ascii_lowercase(),
            group: group.to_ascii_lowercase(),
            help: help::help(setting.cvar)
                .unwrap_or_default()
                .to_ascii_lowercase(),
        }
    }

    /// The row matches `query` (lower case): its label, console name,
    /// description or group holds it.
    fn holds(&self, query: &str) -> bool {
        self.label.contains(query)
            || self.cvar.contains(query)
            || self.group.contains(query)
            || self.help.contains(query)
    }

    /// The row's own name carries `query`: a word of its label begins with it.
    fn named(&self, query: &str) -> bool {
        words(&self.label).any(|word| word.starts_with(query))
            || (query.contains(' ') && self.label.contains(query))
    }
}

/// The words of `text`: its runs of letters and digits.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// The names [`SYNONYMS`] gives `query` (lower case).
fn synonyms(query: &str) -> impl Iterator<Item = &'static str> + '_ {
    SYNONYMS
        .iter()
        .filter(move |(typed, _)| {
            query == *typed
                || query.starts_with(typed)
                || (query.chars().count() >= 3 && typed.starts_with(query))
        })
        .flat_map(|(_, names)| names.iter().copied())
}

/// `query` as the same keys give it on a Latin keyboard, when it was typed on a
/// Russian one (the layout players leave on from chat); `None` when no letter of it
/// is Cyrillic.
fn latin_keys(query: &str) -> Option<String> {
    const CYRILLIC: &str = "\u{439}\u{446}\u{443}\u{43a}\u{435}\u{43d}\u{433}\u{448}\u{449}\u{437}\u{444}\u{44b}\u{432}\u{430}\u{43f}\u{440}\u{43e}\u{43b}\u{434}\u{44f}\u{447}\u{441}\u{43c}\u{438}\u{442}\u{44c}";
    const LATIN: &str = "qwertyuiopasdfghjklzxcvbnm";
    let mut any = false;
    let latin: String = query
        .chars()
        .map(|character| {
            let lower = character.to_lowercase().next().unwrap_or(character);
            match CYRILLIC.chars().position(|key| key == lower) {
                Some(at) => {
                    any = true;
                    LATIN.as_bytes()[at] as char
                }
                None => character,
            }
        })
        .collect();
    any.then_some(latin)
}

/// How many single slips (a letter wrong, missing, extra, or two swapped) turn `a`
/// into `b`, counted up to `limit` (more reads as `limit + 1`).
fn slips(a: &str, b: &str, limit: usize) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > limit {
        return limit + 1;
    }
    // Optimal string alignment over three rolling rows.
    let mut before: Vec<usize> = vec![0; b.len() + 1];
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current: Vec<usize> = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        current[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (previous[j] + 1)
                .min(current[j - 1] + 1)
                .min(previous[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(before[j - 2] + 1);
            }
            current[j] = best;
        }
        std::mem::swap(&mut before, &mut previous);
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()].min(limit + 1)
}

/// Whether `label` (lower case) has a word that `query` is a slip or two away from,
/// or from the start of: one slip for a word of four or five letters, two from six.
/// Shorter words are too close to each other to guess at.
fn near(label: &str, query: &str) -> bool {
    let length = query.chars().count();
    let limit = match length {
        0..=3 => return false,
        4..=5 => 1,
        _ => 2,
    };
    words(label).any(|word| {
        // The word whole, or as much of it as was typed, give or take a letter.
        let letters = word.chars().count();
        (length.saturating_sub(1)..=length + 1)
            .filter(|take| *take <= letters)
            .chain((letters <= length + 1).then_some(letters))
            .any(|take| {
                let start: String = word.chars().take(take).collect();
                slips(&start, query, limit) <= limit
            })
    })
}

/// The searchable rows matching `query` (lower case, trimmed), in the catalogue's
/// order: every row that holds it or that a synonym of it names.
fn found(query: &str, names: &[Names]) -> Vec<usize> {
    let also: Vec<&str> = synonyms(query).collect();
    names
        .iter()
        .enumerate()
        .filter(|(_, row)| row.holds(query) || also.iter().any(|name| row.label.contains(name)))
        .map(|(row, _)| row)
        .collect()
}

/// The rows matching `query`: the ones whose own name carries it first, under
/// [`BEST`], when the search finds many, then each group's under its heading. A
/// search that finds nothing as typed is read on a Latin keyboard, then as a slip
/// of the fingers (see the module's words).
pub(super) fn lines(query: &str) -> Vec<super::Line> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let searchable = searchable();
    static NAMES: OnceLock<Vec<Names>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        searchable
            .rows
            .iter()
            .zip(&searchable.groups)
            .map(|(setting, group)| Names::of(setting, group))
            .collect()
    });
    let mut query = query;
    let mut rows = found(&query, names);
    if rows.is_empty()
        && let Some(latin) = latin_keys(&query)
    {
        rows = found(&latin, names);
        query = latin;
    }
    if rows.is_empty() {
        rows = (0..names.len())
            .filter(|row| near(&names[*row].label, &query))
            .collect();
    }
    let best: Vec<usize> = if rows.len() > BEST_OVER {
        rows.iter()
            .copied()
            .filter(|row| names[*row].named(&query))
            .collect()
    } else {
        Vec::new()
    };
    // Named by every result, the heading would say nothing.
    let best = if best.len() == rows.len() {
        Vec::new()
    } else {
        best
    };
    let mut lines = Vec::with_capacity(rows.len() + 8);
    if !best.is_empty() {
        lines.push(super::Line::Heading(BEST));
        lines.extend(best.iter().map(|row| super::Line::Row(*row)));
    }
    let mut last_group = None;
    for row in rows {
        if best.contains(&row) {
            continue;
        }
        let group = searchable.groups[row];
        if last_group != Some(group) {
            lines.push(super::Line::Heading(group));
            last_group = Some(group);
        }
        lines.push(super::Line::Row(row));
    }
    lines
}

/// How many settings match `query` (the number of rows [`lines`] would show).
pub(super) fn count(query: &str) -> usize {
    lines(query)
        .iter()
        .filter(|line| matches!(line, super::Line::Row(_)))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_option_is_searchable_once() {
        let rows = rows();
        for (index, setting) in rows.iter().enumerate() {
            assert!(
                rows[..index].iter().all(|seen| seen.cvar != setting.cvar),
                "{} twice",
                setting.cvar
            );
        }
        for caption in TABS {
            let tab = TABS.iter().position(|tab| *tab == caption).unwrap();
            for setting in settings(tab) {
                assert!(
                    rows.iter().any(|row| row.cvar == setting.cvar),
                    "{} ({caption}) cannot be found",
                    setting.cvar
                );
            }
        }
    }

    #[test]
    fn results_sit_under_their_groups() {
        let lines = lines("fov");
        assert_eq!(lines[0], super::super::Line::Heading("Display"));
        // The graphics preset is Graphics'.
        let preset = super::lines("preset");
        assert!(preset.contains(&super::super::Line::Heading("Graphics")));

        assert!(lines.iter().any(|line| matches!(
            line,
            super::super::Line::Row(row) if rows()[*row].cvar == "cg_fov"
        )));
        // A description finds its setting: "brightness" is in r_gamma's.
        let bright = super::lines("brightness");
        assert!(
            bright
                .iter()
                .any(|line| matches!(line, super::super::Line::Row(_)))
        );
        assert!(super::lines("no such setting at all").is_empty());
        assert_eq!(count("no such setting at all"), 0);
        assert!(count("fov") >= 1);
        // A group name lists the group; the scoreboard's rows are HUD's.
        let chat = super::lines("chat");
        assert!(chat.contains(&super::super::Line::Heading("Chat")));
        let scoreboard = super::lines("scoreboard");
        assert!(scoreboard.contains(&super::super::Line::Heading("HUD")));
    }

    fn finds(query: &str, cvar: &str) -> bool {
        lines(query).iter().any(|line| {
            matches!(
                line,
                super::super::Line::Row(row) if rows()[*row].cvar.eq_ignore_ascii_case(cvar)
            )
        })
    }

    /// Searches players typed that found nothing, or not what they meant (the hub's
    /// settings usage, 21/11/2026).
    #[test]
    fn another_word_for_a_setting_finds_it() {
        assert!(finds("afk", crate::away_emote::IDLE_CVAR));
        assert!(finds("pov", "cg_fov"));
        assert!(finds("msaa", "r_fxaa"));
        assert!(finds("anti", "r_superSample"));
        assert!(finds("quite mode", crate::chat::quiet::CVAR));
        assert!(finds("vsync", "r_vsync"));
    }

    #[test]
    fn ejk_mode_is_found_by_the_names_players_give_it() {
        for query in [
            "ejk",
            "ejk mode",
            "eternaljk",
            "jof",
            "openjk",
            "ojk",
            "classic",
            "retail",
        ] {
            assert!(finds(query, crate::ejk_mode::CVAR), "{query}");
        }
    }

    #[test]
    fn a_slip_of_the_fingers_still_finds_the_setting() {
        assert!(finds("crosshar", "cg_crosshair"));
        assert!(finds("namepalte", "cg_nameplate"));
        assert!(finds("nameplaye", "cg_nameplate"));
        assert!(finds("voulme", "s_volume"));
        assert!(finds("weater", crate::weather::CVAR));
        assert!(finds("shadw", "r_worldSunShadows"));
        // Short words are not guessed at, and a real result is never padded with
        // guesses.
        assert!(lines("xqz").is_empty());
        let fov = count("fov");
        assert!((1..12).contains(&fov), "{fov}");
    }

    #[test]
    fn a_search_typed_on_a_russian_keyboard_reads_as_its_keys() {
        // "wheel" and "camera" as their keys give them on a Russian layout.
        assert_eq!(
            latin_keys("\u{446}\u{440}\u{443}\u{443}\u{434}").as_deref(),
            Some("wheel")
        );
        assert_eq!(latin_keys("wheel"), None);
        assert_eq!(
            lines("\u{441}\u{444}\u{44c}\u{443}\u{43a}\u{444}"),
            lines("camera")
        );
        assert!(!lines("camera").is_empty());
    }

    #[test]
    fn slips_count_a_swap_as_one() {
        assert_eq!(slips("caht", "chat", 2), 1);
        assert_eq!(slips("crosshar", "crosshai", 2), 1);
        assert_eq!(slips("volume", "voulme", 2), 1);
        assert_eq!(slips("abc", "xyz", 1), 2);
        assert_eq!(slips("same", "same", 1), 0);
    }

    #[test]
    fn a_wide_search_leads_with_the_settings_named_for_it() {
        let hud = lines("hud");
        assert_eq!(hud[0], super::super::Line::Heading(BEST));
        let super::super::Line::Row(first) = hud[1] else {
            panic!("a row follows the heading");
        };
        assert!(rows()[first].label.to_ascii_lowercase().contains("hud"));
        // Each setting is listed once.
        let mut seen = std::collections::HashSet::new();
        for line in &hud {
            if let super::super::Line::Row(row) = line {
                assert!(seen.insert(*row), "{} twice", rows()[*row].cvar);
            }
        }
        // A narrow one keeps its groups alone.
        assert!(!lines("fov").contains(&super::super::Line::Heading(BEST)));
    }

    /// A search lists rows from every group at once, so each one's typed value
    /// must answer to its own token (hub report #203: past row 99 they did not).
    #[test]
    fn every_result_takes_a_typed_value() {
        use crate::menu_widgets::numeric::{VALUE_BASE, VALUE_ROWS, value_row};
        let rows = rows().len();
        assert!(
            rows > 100,
            "the search is expected to list more than 100 rows"
        );
        assert!(rows <= usize::from(VALUE_ROWS));
        for row in 0..rows {
            assert_eq!(value_row(VALUE_BASE + row as u16), Some(row));
        }
        assert_eq!(value_row(VALUE_BASE - 1), None);
    }

    /// The weak-PC settings are found by their names, console names and what they do.
    #[test]
    fn the_weak_pc_settings_are_found() {
        let found = |query: &str, cvar: &str, group: &str| {
            let lines = super::lines(query);
            let row = lines
                .iter()
                .find_map(|line| match line {
                    super::super::Line::Row(row) if rows()[*row].cvar == cvar => Some(*row),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{query:?} does not find {cvar}"));
            assert_eq!(super::group(row), Some(group), "{cvar}");
        };
        for query in ["stock blades", "cg_stockblades", "weak pc"] {
            found(query, crate::stock_blades::CVAR, "Gameplay");
        }
        for query in ["menu fps", "com_maxfpsmenu", "in the menus"] {
            found(query, crate::runtime_settings::MENU_CAP_CVAR, "Display");
        }
        for query in ["texture compression", "r_texturecompression", "compressed"] {
            found(
                query,
                crate::world_materials::filtering::mips::compress::CVAR,
                "Graphics: image",
            );
        }
    }
}
