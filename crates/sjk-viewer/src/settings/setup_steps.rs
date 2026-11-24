//! First setup's steps (`docs/sjk-ui.md`, First setup): the pop-up asks one thing at a
//! time, and [`STEPS`] is what it asks. Each step has a title, a sentence, its picture
//! buttons and the few plain rows under them; the pop-up (`sjk_popup.rs`) draws whatever
//! the table holds and knows no step by name, so a step is added, moved or changed here.
//! A step may also hold a text field ([`Field`], the player's name) over its buttons, and
//! close on a recap of what the steps before it hold ([`Step::recap`]).
//!
//! The rows are First setup's own (`quick.rs`), named by cvar. A picture button stands
//! for a row too ([`Act::stands_for`]): the two looks (each a picture of the same
//! moment of a match, `setup_pictures.rs`) are the EJK mode switch, the
//! themes the Theme row, the presets the Graphics preset row, Import the import row. A
//! row of `quick.rs` that no step shows is listed in [`LEFT_OUT`] with the reason; a test
//! keeps the three in step, so no row is lost by accident.

use crate::console::ViewerConsole;
use crate::graphics_quality::{self, Level, Rating};
use crate::menu::sjk::theme::{self, Theme};
use crate::setup_pictures::Picture;

/// One step of the pop-up.
pub(super) struct Step {
    /// Its name under its number at the top of the card.
    pub(super) name: &'static str,
    /// What it asks.
    pub(super) title: &'static str,
    /// One sentence under the title.
    pub(super) sentence: &'static str,
    /// Its picture buttons, side by side; none for a step of plain rows.
    pub(super) buttons: &'static [Button],
    /// Its rows, by cvar, in order.
    pub(super) rows: &'static [&'static str],
    /// Each row has what it does written under it, rather than only the focused one's
    /// under them all.
    pub(super) explained: bool,
    /// A sentence under the buttons.
    pub(super) aside: Option<Aside>,
    /// A text field over the buttons.
    pub(super) field: Option<Field>,
    /// Under its rows, what the steps before it hold now, each a way back to its step.
    pub(super) recap: bool,
}

/// A text field of a step: a text setting typed in place.
pub(super) struct Field {
    pub(super) label: &'static str,
    /// The text cvar it writes, as it is typed.
    pub(super) cvar: &'static str,
    /// What the cvar holds on a new profile, kept when the field is left empty.
    pub(super) empty: &'static str,
    /// The text is a player's name: `^` and a digit colour what follows, so the field
    /// shows what is typed, the colours as chips to add, and beside it the name as the
    /// game draws it.
    pub(super) colours: bool,
}

/// A sentence a step shows under its buttons.
pub(super) struct Aside {
    /// Shown only while this on/off setting is on; always when `None`.
    pub(super) when: Option<&'static str>,
    pub(super) text: Words,
}

/// An aside's words: written here, or made for this PC.
pub(super) enum Words {
    Fixed(&'static str),
    Made(fn() -> String),
}

/// One picture button.
pub(super) struct Button {
    pub(super) name: &'static str,
    /// What choosing it means, a short line each.
    pub(super) lines: &'static [&'static str],
    pub(super) art: Art,
    pub(super) act: Act,
}

/// What a button shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Art {
    /// One of the bundled renders ([`crate::setup_pictures`]).
    Picture(Picture),
    /// The render of the graphics preset the button would set.
    Preset,
    /// The theme's own colours, drawn.
    Theme(Theme),
    /// One of the settings' pictures (`settings_icons`), large.
    Icon(&'static str),
}

/// What choosing a button does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Act {
    /// Set `cvar` to `value`; the button is in use while it holds it.
    Set {
        cvar: &'static str,
        value: &'static str,
    },
    /// Wear a theme.
    Theme(Theme),
    /// Set a graphics preset: one, or one of several offered under the button, each
    /// with its short name.
    Preset(&'static [(Level, &'static str)]),
    /// Open the Import page.
    Import,
    /// Open the Character page; closing it comes back to the step.
    Character,
    /// Nothing to set: go on to the next step.
    Next,
}

impl Act {
    /// The row of `quick.rs` the button stands for, by cvar: what the test that
    /// keeps the steps and `quick.rs` in step reads.
    #[cfg(test)]
    pub(super) fn stands_for(self) -> Option<&'static str> {
        match self {
            Self::Set { cvar, .. } => Some(cvar),
            Self::Theme(_) => Some(theme::CVAR),
            Self::Preset(_) => Some(graphics_quality::ROW_NAME),
            Self::Import => Some(super::catalog::IMPORT_ROW),
            Self::Character | Self::Next => None,
        }
    }

    /// How many choices the button holds: a preset button's levels, else one.
    pub(super) fn options(self) -> usize {
        match self {
            Self::Preset(levels) => levels.len().max(1),
            _ => 1,
        }
    }

    /// Whether choice `option` of the button is what `console` holds.
    pub(super) fn in_use(self, option: usize, console: &ViewerConsole) -> bool {
        match self {
            Self::Set { cvar, value } => console.cvar(cvar).is_some_and(|held| {
                let held = held.as_text();
                held.trim().eq_ignore_ascii_case(value)
                    || matches!(
                        (held.trim(), value),
                        ("false", "0") | ("true", "1") | ("0", "false") | ("1", "true")
                    )
            }),
            Self::Theme(theme) => Theme::from_cvar(console.text_value(theme::CVAR)) == theme,
            Self::Preset(levels) => levels
                .get(option)
                .is_some_and(|(level, _)| Level::current(console) == Some(*level)),
            Self::Import | Self::Character | Self::Next => false,
        }
    }

    /// The choice a click on the button itself takes when none of its choices is in
    /// use: of several presets, the richest that runs smoothly on a card of `mib` MiB
    /// (the first when none does).
    pub(super) fn usual_option(self, mib: Option<u64>) -> usize {
        match self {
            Self::Preset(levels) => levels
                .iter()
                .rposition(|(level, _)| level.rating(mib) == Rating::Smooth)
                .unwrap_or(0),
            _ => 0,
        }
    }

    /// The preset choice `option` sets, if the button sets one.
    pub(super) fn level(self, option: usize) -> Option<Level> {
        match self {
            Self::Preset(levels) => levels.get(option).map(|(level, _)| *level),
            _ => None,
        }
    }
}

/// One thing the keyboard can rest on among a step's buttons: choice `option` of
/// button `button`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Target {
    pub(super) button: usize,
    pub(super) option: usize,
}

impl Step {
    /// Every choice of every button, left to right: what Left and Right move through.
    pub(super) fn targets(&self) -> Vec<Target> {
        self.buttons
            .iter()
            .enumerate()
            .flat_map(|(button, entry)| {
                (0..entry.act.options()).map(move |option| Target { button, option })
            })
            .collect()
    }

    /// The first target of button `button`.
    pub(super) fn first_target(&self, button: usize) -> usize {
        self.buttons[..button.min(self.buttons.len())]
            .iter()
            .map(|entry| entry.act.options())
            .sum()
    }
}

const EJK: &str = crate::ejk_mode::CVAR;

/// The steps, in order.
pub(super) const STEPS: &[Step] = &[
    Step {
        name: "Look",
        title: "Choose your look",
        sentence: "The same moment of a match, drawn both ways. The game and the servers are the same, and you can change your mind at any time.",
        buttons: &[
            Button {
                name: "SJK",
                lines: &[
                    "The new look: SJK's own menus over the live map",
                    "A status bar, a kill feed and SJK's extras in a match",
                    "Vanilla Plus graphics, yours to raise in step 4",
                ],
                art: Art::Picture(Picture::LookSjk),
                act: Act::Set {
                    cvar: EJK,
                    value: "0",
                },
            },
            Button {
                name: "EJK",
                lines: &[
                    "JoF EJK's look: the classic menus, the game's fonts",
                    "EJK's HUD, scoreboard and console, Vanilla graphics",
                    "Your own settings come back when you return to SJK",
                ],
                art: Art::Picture(Picture::LookEjk),
                act: Act::Set {
                    cvar: EJK,
                    value: "1",
                },
            },
        ],
        rows: &[crate::menu::style::CVAR, crate::menu_hud::STYLE_CVAR],
        explained: false,
        aside: None,
        field: None,
        recap: false,
    },
    Step {
        name: "Colours",
        title: "Pick your colours",
        sentence: "The theme colours SJK's pages and pop-ups, this one first. Custom takes any hue.",
        buttons: &[
            Button {
                name: "Solar",
                lines: &["Amber and gold, the sun's"],
                art: Art::Theme(Theme::Warm),
                act: Act::Theme(Theme::Warm),
            },
            Button {
                name: "Sith",
                lines: &["Red and gold"],
                art: Art::Theme(Theme::Sith),
                act: Act::Theme(Theme::Sith),
            },
            Button {
                name: "Jedi",
                lines: &["Deep navy and holo blue"],
                art: Art::Theme(Theme::Cold),
                act: Act::Theme(Theme::Cold),
            },
            Button {
                name: "Custom",
                lines: &["Your own hue and saturation"],
                art: Art::Theme(Theme::Custom),
                act: Act::Theme(Theme::Custom),
            },
        ],
        rows: &[theme::HUE_CVAR, theme::SATURATION_CVAR],
        explained: false,
        aside: Some(Aside {
            when: Some(EJK),
            text: Words::Fixed(
                "EJK mode is on: the classic menus keep the game's own colours. The theme still colours SJK's pages, such as this one.",
            ),
        }),
        field: None,
        recap: false,
    },
    Step {
        name: "Player",
        title: "Your player",
        sentence: "Your name as other players see it, and your setup from another client if you have one.",
        buttons: &[
            Button {
                name: "Import my setup",
                lines: &[
                    "Name, model, keys and settings from base JKA, EJK, TaystJK or another client",
                    "You choose which parts come over",
                ],
                art: Art::Icon("import"),
                act: Act::Import,
            },
            Button {
                name: "Choose a character",
                lines: &[
                    "Your model, its colours and your saber, on the Character page",
                    "Closing it brings you back here",
                ],
                art: Art::Icon("game_options"),
                act: Act::Character,
            },
            Button {
                name: "Start fresh",
                lines: &[
                    "Keep SJK's defaults and set things up in the next steps",
                    "Import stays here, and in Settings, for later",
                ],
                art: Art::Icon("first_setup"),
                act: Act::Next,
            },
        ],
        rows: &[],
        explained: false,
        aside: None,
        field: Some(Field {
            label: "Name",
            cvar: "name",
            empty: "Padawan",
            colours: true,
        }),
        recap: false,
    },
    Step {
        name: "Graphics",
        title: "Graphics",
        sentence: "The same spot of the same map, drawn three ways. The mark says how each should run on this PC.",
        buttons: &[
            Button {
                name: "Vanilla",
                lines: &[
                    "The original game's look, at the least cost",
                    "Baked lighting as the maps were made, every SJK effect off",
                    "The graphics EJK mode sets",
                ],
                art: Art::Preset,
                act: Act::Preset(&[(Level::Vanilla, "")]),
            },
            Button {
                name: "Vanilla Plus",
                lines: &[
                    "SJK's image over the maps' own lighting: HDR, bloom, glow",
                    "Smoother edges, ambient occlusion and lit models",
                    "A new profile starts here",
                ],
                art: Art::Preset,
                act: Act::Preset(&[(Level::VanillaPlus, "")]),
            },
            Button {
                name: "Solar Renderer",
                lines: &["SJK's own renderer: a live sun, sky, lamps and shadows"],
                art: Art::Preset,
                act: Act::Preset(&[
                    (Level::SolarLow, "Low"),
                    (Level::SolarMedium, "Medium"),
                    (Level::SolarHigh, "High"),
                    (Level::SolarUltra, "Ultra"),
                ]),
            },
        ],
        rows: &["r_resolution", "r_fullscreen", "r_vsync"],
        explained: false,
        aside: Some(Aside {
            when: None,
            text: Words::Made(graphics_quality::card_words),
        }),
        field: None,
        recap: false,
    },
    Step {
        name: "Play",
        title: "How you play",
        sentence: "Movement, camera, aim and sound. Settings has each of these too, and many more.",
        buttons: &[],
        rows: &[
            crate::gameplay_fps::CVAR,
            crate::camera::STYLE_CVAR,
            "cg_fov",
            "sensitivity",
            "m_invert",
            "cl_run",
            "cg_crosshair",
            "cg_hudScale",
            "s_volume",
            "s_musicVolume",
        ],
        explained: false,
        aside: None,
        field: None,
        recap: false,
    },
    Step {
        name: "Online",
        title: "Online",
        sentence: "What SJK does over the network and shows over other players. Each is yours to turn off.",
        buttons: &[],
        rows: &["cl_identity", "cl_autoUpdate", "cg_nameplate"],
        explained: true,
        aside: Some(Aside {
            when: None,
            text: Words::Fixed(
                "Done closes First setup for good: it no longer opens at start. The firstsetup command opens it again.",
            ),
        }),
        field: None,
        recap: true,
    },
];

/// Rows of `quick.rs` that no step shows, each with why. They stay in the Settings
/// screen's First setup category and under their own categories. Read by the test
/// that keeps the steps and `quick.rs` in step.
#[cfg(test)]
pub(super) const LEFT_OUT: &[(&str, &str)] = &[
    (
        "cg_nameplateRange",
        "Fine-tuning of nameplates, which step 6 turns on or off: Settings > Nameplates",
    ),
    (
        "cg_nameplateBars",
        "Fine-tuning of nameplates, as the range",
    ),
    (
        "cg_nameplateIcons",
        "Fine-tuning of nameplates, as the range",
    ),
    (
        "cg_nameplateDuel",
        "Fine-tuning of nameplates, as the range",
    ),
    (
        "cg_auraShell",
        "The Force shells' look is a taste few set on their first start: Settings > Gameplay",
    ),
    ("cg_spProtAbsColor", "The Force shells' look, as the aura"),
    (
        crate::duel_ghost::CVAR,
        "How other duels' players show matters only once in a duel: Settings > Gameplay",
    ),
];

#[cfg(test)]
mod tests {
    use super::super::quick;
    use super::*;

    /// Every cvar a step names: its rows, then what its buttons stand for.
    fn named(step: &Step) -> Vec<&'static str> {
        let mut named: Vec<&'static str> = step.rows.to_vec();
        for button in step.buttons {
            if let Some(cvar) = button.act.stands_for()
                && !named.contains(&cvar)
            {
                named.push(cvar);
            }
        }
        named
    }

    #[test]
    fn every_row_of_first_setup_is_in_one_step_or_left_out_on_purpose() {
        let mut seen: Vec<&'static str> = Vec::new();
        for step in STEPS {
            for cvar in named(step) {
                assert!(!seen.contains(&cvar), "{cvar} is in two steps");
                seen.push(cvar);
            }
        }
        for (cvar, why) in LEFT_OUT {
            assert!(!seen.contains(cvar), "{cvar} is shown and left out");
            assert!(!why.is_empty());
            seen.push(cvar);
        }
        let rows: Vec<&str> = quick::rows()
            .iter()
            .map(|row| row.cvar)
            .filter(|cvar| *cvar != quick::HIDE_CVAR)
            .collect();
        for cvar in &rows {
            assert!(
                seen.iter().any(|name| name.eq_ignore_ascii_case(cvar)),
                "{cvar} is in no step and not in LEFT_OUT"
            );
        }
        for cvar in &seen {
            assert!(
                rows.iter().any(|name| name.eq_ignore_ascii_case(cvar)),
                "{cvar} is not a row of First setup"
            );
        }
        assert_eq!(seen.len(), rows.len());
    }

    #[test]
    fn every_steps_cvars_exist_and_no_step_scrolls() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        for step in STEPS {
            assert!(!step.title.is_empty() && !step.sentence.is_empty());
            assert!(
                !step.name.is_empty() && step.name.len() <= 9,
                "{}",
                step.name
            );
            for cvar in step.rows {
                assert!(
                    quick::rows()
                        .iter()
                        .any(|row| row.cvar.eq_ignore_ascii_case(cvar)),
                    "{cvar} has no row"
                );
                // The preset, the import and the HUD rows are named by a command.
                if console.cvar(cvar).is_none() {
                    assert!(
                        [crate::menu_hud::STYLE_CVAR, "r_resolution"].contains(cvar),
                        "{cvar} is no cvar"
                    );
                }
            }
            for button in step.buttons {
                assert!(!button.name.is_empty() && !button.lines.is_empty());
                match button.act {
                    Act::Set { cvar, .. } => assert!(console.cvar(cvar).is_some(), "{cvar}"),
                    Act::Preset(levels) => assert!(!levels.is_empty()),
                    Act::Theme(theme) => assert_eq!(button.name, theme.label()),
                    Act::Import | Act::Character | Act::Next => {}
                }
                if let Art::Icon(name) = button.art {
                    assert!(crate::settings_icons::picture(name).is_some(), "{name}");
                }
            }
            // A step with pictures keeps to a few rows; one without fits the card.
            let most = if step.buttons.is_empty() { 10 } else { 3 };
            let lines = step.rows.len() * if step.explained { 2 } else { 1 };
            assert!(lines <= most, "{} has {lines} lines", step.title);
            if let Some(Aside {
                when: Some(cvar), ..
            }) = step.aside
            {
                assert!(console.cvar(cvar).is_some(), "{cvar}");
            }
            // A field writes a text cvar, and what it keeps when left empty is that
            // cvar's own default.
            if let Some(field) = &step.field {
                assert!(!field.label.is_empty());
                assert_eq!(
                    console.text_value(field.cvar),
                    Some(field.empty),
                    "{}",
                    field.cvar
                );
            }
        }
    }

    #[test]
    fn the_preset_buttons_cover_every_preset_once() {
        let mut levels = Vec::new();
        for step in STEPS {
            for button in step.buttons {
                if let Act::Preset(offered) = button.act {
                    assert_eq!(button.art, Art::Preset);
                    levels.extend(offered.iter().map(|(level, _)| *level));
                    // Several presets under one button each have a name.
                    assert!(offered.len() == 1 || offered.iter().all(|(_, name)| !name.is_empty()));
                }
            }
        }
        assert_eq!(levels, Level::ALL);
    }

    #[test]
    fn a_button_is_in_use_while_its_setting_holds_its_value() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let look = &STEPS[0];
        assert!(
            look.buttons[0].act.in_use(0, &console),
            "a new profile wears SJK"
        );
        assert!(!look.buttons[1].act.in_use(0, &console));
        console.set_cvar(EJK, "1");
        assert!(!look.buttons[0].act.in_use(0, &console));
        assert!(look.buttons[1].act.in_use(0, &console));
        console.set_cvar(theme::CVAR, "sith");
        assert!(Act::Theme(Theme::Sith).in_use(0, &console));
        assert!(!Act::Theme(Theme::Warm).in_use(0, &console));
        Level::SolarMedium.apply(&mut console);
        let solar = STEPS
            .iter()
            .flat_map(|step| step.buttons)
            .find(|button| button.act.options() > 1)
            .unwrap();
        assert!(solar.act.in_use(1, &console));
        assert!(!solar.act.in_use(0, &console) && !solar.act.in_use(2, &console));
        assert!(!Act::Import.in_use(0, &console) && !Act::Next.in_use(0, &console));
        assert!(!Act::Character.in_use(0, &console));
    }

    /// Left and Right go through every choice, a button's several presets included.
    #[test]
    fn the_targets_run_left_to_right_through_every_choice() {
        let graphics = STEPS
            .iter()
            .find(|step| step.buttons.iter().any(|button| button.act.options() > 1))
            .unwrap();
        let targets = graphics.targets();
        assert_eq!(targets.len(), Level::ALL.len());
        assert_eq!(
            targets[0],
            Target {
                button: 0,
                option: 0
            }
        );
        assert_eq!(
            targets[2],
            Target {
                button: 2,
                option: 0
            }
        );
        assert_eq!(
            targets[5],
            Target {
                button: 2,
                option: 3
            }
        );
        assert_eq!(graphics.first_target(2), 2);
        assert_eq!(STEPS[0].targets().len(), 2);
        // A card too small for the Solar Renderer still gets a choice from its button.
        let solar = graphics.buttons[2].act;
        assert_eq!(solar.usual_option(Some(512)), 0);
        assert_eq!(solar.usual_option(Some(64_000)), 3);
        assert_eq!(solar.level(3), Some(Level::SolarUltra));
    }
}
