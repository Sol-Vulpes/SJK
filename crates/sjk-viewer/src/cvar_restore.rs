//! Settings put aside while a mode holds them.
//!
//! A mode that sets a group of settings (`renderer vanilla` in `graphics_quality.rs`,
//! Tactical in `tactical.rs`, EJK mode in `ejk_mode.rs`) first keeps the player's own
//! value of each in an archived cvar of its own, as `cvar=value` pairs joined by `;`, so
//! leaving the mode gives them back, after a restart too. This is that text's one format.

use crate::console::ViewerConsole;

/// The current value of each of `cvars` this client has, as the text a mode puts aside.
pub(crate) fn kept<'a>(
    console: &ViewerConsole,
    cvars: impl IntoIterator<Item = &'a str>,
) -> String {
    cvars
        .into_iter()
        .filter_map(|cvar| Some(format!("{cvar}={}", console.cvar(cvar)?.as_text())))
        .collect::<Vec<_>>()
        .join(";")
}

/// The `(cvar, value)` pairs of a text [`kept`] wrote; anything else in it is skipped.
pub(crate) fn pairs(kept: &str) -> impl Iterator<Item = (&str, &str)> {
    kept.split(';').filter_map(|pair| pair.split_once('='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_kept_reads_back_as_pairs() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("cg_fov", "100");
        let text = kept(&console, ["cg_fov", "no_such_cvar", "con_font"]);
        let read: Vec<_> = pairs(&text).collect();
        assert_eq!(read.len(), 2, "{text}");
        assert_eq!(read[0].0, "cg_fov");
        assert_eq!(read[0].1.parse::<f64>().ok(), Some(100.0));
        assert_eq!(read[1], ("con_font", ""));
        assert_eq!(pairs("").count(), 0);
    }
}
