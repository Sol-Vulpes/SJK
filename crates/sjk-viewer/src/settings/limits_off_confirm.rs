//! Turning `cl_noCommandLimits` on from Settings asks twice
//! ([`crate::server_caps::LIMITS_OFF_CVAR`]): the first press warns that Heimdall,
//! JoF's anti-cheat, may kick for time nudge, the second says what changes, and only
//! the third turns it on. Since it applies at the next start, the description then
//! offers a restart, which a fourth press does (`crate::update::request_restart`).
//! Any other row, key or click starts again (an offered restart is declined, the
//! setting stays on); turning it off asks nothing.

use std::cell::Cell;

/// Presses so far: 1 or 2 while a warning shows, [`OFFER`] once it was turned on.
const OFFER: u8 = 3;

thread_local! {
    /// Presses so far towards turning it on (0: none), or [`OFFER`].
    /// The menus run on one thread; each test thread gets its own.
    static STEP: Cell<u8> = const { Cell::new(0) };
}

/// The first warning, in the setting's description lines (2 of 68 characters).
pub(super) const FIRST: &str =
    "Warning 1 of 2: Heimdall (JoF's anti-cheat) may kick you for time nudge usage. Press again.";
/// The second warning.
pub(super) const SECOND: &str = "Warning 2 of 2: commands go out unlimited, as in older builds. Applies after a restart. Press again.";
/// Shown once it is on: the restart it needs.
pub(super) const RESTART: &str =
    "On from the next start. Press again to restart SJK now, or leave it for later.";

/// What a press on the row does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Press {
    /// Show the next warning; nothing changes.
    Warn,
    /// Turn it on (both warnings were shown); the restart is offered next.
    TurnOn,
    /// Restart the client now (the offer after turning it on).
    Restart,
    /// Turn it off, asking nothing.
    TurnOff,
}

/// A press on the row while the setting is `on`.
pub(super) fn press(on: bool) -> Press {
    let step = STEP.get();
    if step == OFFER {
        STEP.set(0);
        return Press::Restart;
    }
    if on {
        STEP.set(0);
        return Press::TurnOff;
    }
    if step >= 2 {
        STEP.set(OFFER);
        Press::TurnOn
    } else {
        STEP.set(step + 1);
        Press::Warn
    }
}

/// The text the description shows instead of its help, while a warning or the
/// restart offer is pending.
pub(super) fn warning() -> Option<&'static str> {
    match STEP.get() {
        1 => Some(FIRST),
        2 => Some(SECOND),
        OFFER => Some(RESTART),
        _ => None,
    }
}

/// Anything but the next press: the warnings start again, or the restart waits.
pub(super) fn cancel() {
    STEP.set(0);
}

/// Whether a warning or the restart offer is pending.
pub(super) fn pending() -> bool {
    STEP.get() != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_warnings_then_on_then_the_restart_and_anything_else_starts_again() {
        cancel();
        assert_eq!(warning(), None);
        assert_eq!(press(false), Press::Warn);
        assert_eq!(warning(), Some(FIRST));
        assert_eq!(press(false), Press::Warn);
        assert_eq!(warning(), Some(SECOND));
        assert_eq!(press(false), Press::TurnOn, "the third press turns it on");
        assert_eq!(warning(), Some(RESTART));
        assert_eq!(press(true), Press::Restart, "the fourth restarts");
        assert_eq!(warning(), None);
        assert!(!pending());

        // A declined offer leaves it on; the next press turns it off.
        assert_eq!(press(false), Press::Warn);
        assert_eq!(press(false), Press::Warn);
        assert_eq!(press(false), Press::TurnOn);
        cancel();
        assert_eq!(press(true), Press::TurnOff);
        assert!(!pending());

        // A cancelled warning starts from the first again.
        assert_eq!(press(false), Press::Warn);
        cancel();
        assert_eq!(press(false), Press::Warn);
        assert_eq!(warning(), Some(FIRST));
        cancel();

        // Each fits the description's lines whole (the last line takes the rest).
        for text in [FIRST, SECOND, RESTART] {
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
