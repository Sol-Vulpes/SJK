//! Turning `cl_noCommandLimits` on from Settings asks twice
//! ([`crate::server_caps::LIMITS_OFF_CVAR`]): the first press warns that Heimdall,
//! JoF's anti-cheat, may kick for time nudge, the second says what changes, and only
//! the third turns it on. Since it applies at the next start, turning it on then asks
//! "Restart SJK?" on a card of its own (`crate::graphics_reload::card::offer_restart`).
//! Any other row, key or click starts the warnings again; turning it off asks nothing.

use std::cell::Cell;

thread_local! {
    /// Presses so far towards turning it on: 0, then 1 or 2 while a warning shows.
    /// The menus run on one thread; each test thread gets its own.
    static STEP: Cell<u8> = const { Cell::new(0) };
}

/// The first warning, in the setting's description lines (2 of 68 characters).
pub(super) const FIRST: &str =
    "Warning 1 of 2: Heimdall (JoF's anti-cheat) may kick you for time nudge usage. Press again.";
/// The second warning.
pub(super) const SECOND: &str = "Warning 2 of 2: commands go out unlimited, as in older builds. Applies after a restart. Press again.";

/// What a press on the row does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Press {
    /// Show the next warning; nothing changes.
    Warn,
    /// Turn it on (both warnings were shown); "Restart SJK?" is asked next.
    TurnOn,
    /// Turn it off, asking nothing.
    TurnOff,
}

/// A press on the row while the setting is `on`.
pub(super) fn press(on: bool) -> Press {
    let step = STEP.get();
    if on {
        STEP.set(0);
        return Press::TurnOff;
    }
    if step >= 2 {
        STEP.set(0);
        Press::TurnOn
    } else {
        STEP.set(step + 1);
        Press::Warn
    }
}

/// The warning the description shows instead of its help, while one is pending.
pub(super) fn warning() -> Option<&'static str> {
    match STEP.get() {
        1 => Some(FIRST),
        2 => Some(SECOND),
        _ => None,
    }
}

/// Anything but the next press: the warnings start again.
pub(super) fn cancel() {
    STEP.set(0);
}

/// Whether a warning is pending.
pub(super) fn pending() -> bool {
    STEP.get() != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_warnings_then_on_and_anything_else_starts_again() {
        cancel();
        assert_eq!(warning(), None);
        assert_eq!(press(false), Press::Warn);
        assert_eq!(warning(), Some(FIRST));
        assert_eq!(press(false), Press::Warn);
        assert_eq!(warning(), Some(SECOND));
        assert_eq!(press(false), Press::TurnOn, "the third press turns it on");
        assert_eq!(warning(), None);
        assert!(!pending());
        assert_eq!(press(true), Press::TurnOff, "one press turns it off");

        // A cancelled warning starts from the first again.
        assert_eq!(press(false), Press::Warn);
        cancel();
        assert_eq!(press(false), Press::Warn);
        assert_eq!(warning(), Some(FIRST));
        cancel();

        // Each fits the description's lines whole (the last line takes the rest).
        for text in [FIRST, SECOND] {
            let lines = super::super::help::lines(text);
            assert!(
                lines
                    .iter()
                    .all(|line| line.chars().count() <= super::super::help::HELP_LINE_CHARS),
                "{text}"
            );
            assert_eq!(
                lines.concat().replace(' ', ""),
                text.replace(' ', ""),
                "{text}"
            );
        }
    }
}
