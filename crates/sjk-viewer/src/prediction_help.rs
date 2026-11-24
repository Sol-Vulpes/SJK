//! `help_prediction`: every setting and command about how your own movement is
//! predicted, how others are smoothed and how commands reach the server, printed to the
//! console with this session's value, the default and what each does
//! (`docs/networking.md`). Sol asked for it on 24/11/2026, after a player found the
//! prediction "different" in the 2026.1123 betas (nothing in prediction had changed:
//! the ping display had, `snapshot_ping.rs`). Values and descriptions come from the
//! cvar registry, so they never drift from what the client registers.

/// Console command that prints the list.
pub(crate) const COMMAND: &str = "help_prediction";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "List the prediction, smoothing and command-timing settings with their values";

/// The sections and their cvars, in reading order.
const SECTIONS: &[(&str, &[&str])] = &[
    (
        "Your movement (prediction)",
        &[
            "cg_noPredict",
            "cg_predictLegacy",
            "cg_errorDecay",
            "cg_showMiss",
            "cg_predictItems",
        ],
    ),
    (
        "Commands and packets",
        &[
            "gameplay_fps",
            "gameplay_fps_nocap",
            "com_maxfps",
            "cl_freshStamps",
            "cl_maxpackets",
            "cl_packetdup",
        ],
    ),
    (
        "Other players and timing",
        &[
            "cg_smoothClients",
            "cl_timeNudge",
            "snaps",
            "rate",
            "cl_showTimeDelta",
            "cg_lagometer",
        ],
    ),
    (
        "The server's movement rules (read only, from the server)",
        &["pmove_fixed", "pmove_msec", "pmove_float", "g_stepSlideFix"],
    ),
];

/// Commands about the same subject, and what each does.
const COMMANDS: &[(&str, &str)] = &[
    (
        "help_gameplay_fps",
        "How gameplay FPS (the command step) works.",
    ),
    (
        "help_tickrate",
        "Frames, commands and snapshots: what tick rate means in SJK.",
    ),
    (
        "fakenoclip / freecam",
        "Local flight, predicted on your PC only.",
    ),
];

/// One cvar as the registry knows it: current value, default, description.
pub(crate) struct Row<'a> {
    pub(crate) name: &'a str,
    pub(crate) value: String,
    pub(crate) default: String,
    pub(crate) description: String,
}

/// The console lines, `row` reading each cvar (`None`: not registered, left out).
pub(crate) fn lines<'a>(mut row: impl FnMut(&'a str) -> Option<Row<'a>>) -> Vec<String> {
    let mut lines = vec![
        "^5Prediction: how your movement, the others and your commands are timed".to_owned(),
        "Your client moves you at once (prediction) and corrects you when the server \
         disagrees; others are drawn from the server's snapshots. Value, then default \
         in brackets."
            .to_owned(),
    ];
    for (title, cvars) in SECTIONS {
        lines.push(format!("^3{title}"));
        for name in *cvars {
            let Some(row) = row(name) else { continue };
            let changed = if row.value == row.default { "" } else { "^2" };
            lines.push(format!(
                "^7{} {changed}{}^7 [{}]: {}",
                row.name, row.value, row.default, row.description
            ));
        }
    }
    lines.push("^3Commands".to_owned());
    for (command, what) in COMMANDS {
        lines.push(format!("^7{command}: {what}"));
    }
    lines
}

impl crate::GpuState {
    /// `help_prediction`: the list, with this session's values, to the console.
    pub(crate) fn prediction_help_command(&self) -> Result<Vec<String>, String> {
        let console = self.console.as_ref().ok_or("Console unavailable")?;
        Ok(lines(|name| {
            console.cvar_entry(name).map(|cvar| Row {
                name: &cvar.name,
                value: cvar.shown_value(),
                default: cvar.shown_default(),
                description: cvar.description.clone(),
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_cvar_is_registered_and_shown() {
        let directory = tempfile::tempdir().unwrap();
        let console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut missing = Vec::new();
        for (_, cvars) in SECTIONS {
            for name in *cvars {
                if console.cvar_entry(name).is_none() {
                    missing.push(*name);
                }
            }
        }
        assert!(missing.is_empty(), "not registered: {missing:?}");
        let text = lines(|name| {
            console.cvar_entry(name).map(|cvar| Row {
                name: &cvar.name,
                value: cvar.shown_value(),
                default: cvar.shown_default(),
                description: cvar.description.clone(),
            })
        })
        .join("\n");
        for needle in [
            "cg_errorDecay",
            "cl_timeNudge",
            "pmove_fixed",
            "help_tickrate",
        ] {
            assert!(text.contains(needle), "missing {needle}");
        }
    }

    #[test]
    fn a_changed_value_is_marked() {
        let lines = lines(|name| {
            Some(Row {
                name,
                value: if name == "cl_timeNudge" { "-10" } else { "0" }.to_owned(),
                default: "0".to_owned(),
                description: "d".to_owned(),
            })
        });
        assert!(
            lines
                .iter()
                .any(|line| line.contains("cl_timeNudge ^2-10^7 [0]"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("cg_noPredict 0^7 [0]"))
        );
    }
}
