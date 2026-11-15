//! The Collection's hall on the menu map's stage (`docs/sjk-ui.md`, Collection): the
//! backdrop camera turns until the player's model stands in the hall's column on the
//! window's right ([`crate::console::collection_panel::hall`]), eased in as the hall shows
//! and out as a kind is opened, as the Saber Lab's builder turns it
//! ([`crate::menu_backdrop::framed_yaw`]).

use super::*;

/// Seconds the camera takes to turn to the hall's framing, and back.
const EASE: f32 = 0.45;
/// How far over the model's origin the camera aims: about its chest, as the lab's.
const TARGET_ABOVE_ORIGIN: f32 = 8.0;

/// How far the camera has turned to the hall's framing (0 to 1), what it turns to and
/// when it last eased.
#[derive(Default)]
pub(super) struct HallTurn {
    share: f32,
    target: Option<Vec3>,
    at: Option<Instant>,
}

impl MenuStage {
    /// Ease the camera's turn towards the hall's framing while `wanted` (the hall shows
    /// and the model stands on the stage), and back once it is not.
    pub(crate) fn ease_hall_turn(&mut self, wanted: bool, now: Instant) {
        if let Some(actor) = self.actor.as_ref().filter(|_| wanted) {
            self.hall_turn.target = Some(actor.origin + Vec3::Z * TARGET_ABOVE_ORIGIN);
        }
        let turn = &mut self.hall_turn;
        let step = turn.at.replace(now).map_or(0.0, |at| {
            now.saturating_duration_since(at).as_secs_f32() / EASE
        });
        turn.share = if wanted {
            (turn.share + step).min(1.0)
        } else {
            (turn.share - step).max(0.0)
        };
    }

    /// What the camera turns to put in the hall's column, and how much of the way
    /// (smoothed), while it turns or stays turned.
    pub(crate) fn hall_turn(&self) -> Option<(Vec3, f32)> {
        let turn = &self.hall_turn;
        let target = turn.target.filter(|_| turn.share > 0.0)?;
        let share = turn.share;
        Some((target, share * share * (3.0 - 2.0 * share)))
    }
}
