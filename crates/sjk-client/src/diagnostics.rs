//! Where the session's network diagnostics go.
//!
//! The freeze and gamestate probes ([`crate::freeze_probe`], [`crate::gamestate_probe`])
//! name which server gate stopped a player's commands. A release build has no console,
//! so their lines printed to stderr were lost exactly when a player's report needed
//! them (hub report #324). The host sets a sink once, its session log, and the lines
//! reach the logs a player sends; without a sink they still go to stderr.

use std::fmt;
use std::sync::OnceLock;

static SINK: OnceLock<fn(&str)> = OnceLock::new();

/// Send every later diagnostic line to `sink`. The first call wins.
pub fn set_sink(sink: fn(&str)) {
    let _ = SINK.set(sink);
}

/// Write one diagnostic line to the sink, or to stderr without one.
pub(crate) fn line(arguments: fmt::Arguments<'_>) {
    match SINK.get() {
        Some(sink) => sink(&arguments.to_string()),
        None => eprintln!("{arguments}"),
    }
}
