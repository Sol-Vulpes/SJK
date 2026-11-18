//! Eyes: the joints a rig model's manifest names on its `eyes` line turn towards what
//! the player looks at. The game turns a player's head, neck and chest towards its look
//! target (`hasLookTarget`, `cg_players.c:4279-4289`), but only in yaw and only so far
//! (`bg_pmove.c:8804-8863`); a stock model has no eyes to move, so the look ends there.
//! A rig's eyes look at the target's eyes, up and down too, as far as [`LIMIT`] past
//! where the head points, and come back to rest when there is none. They ease there
//! ([`RATE`]) rather than snap. Only the eye joint turns, about itself: the joints under
//! it stay where the head holds them, as a rig may hang its lids from the eye (an Unreal
//! face rig's `eye_lid_upper_mid` is under `eye`), and lids turning with the eye would
//! drag the skin round it.
//!
//! It all happens in the model's space, after the animation and any clip and before the
//! dynamic chains; [`super::Scratch::look_at`] says where the target is.

use super::Joint;
use glam::{Affine3A, Mat3A, Quat, Vec3A};

/// The furthest an eye turns from where it rests in the head, degrees.
pub(super) const LIMIT: f32 = 30.0;
/// How fast an eye takes up its turn, per second (about a tenth of a second to most of it).
pub(super) const RATE: f32 = 14.0;
/// A step longer than this (a hitch, the first pose) sets the eyes at once.
const MAX_STEP: f32 = 0.1;

/// A rig's eyes, read once at load.
#[derive(Clone, Debug, Default)]
pub(super) struct Eyes {
    eyes: Vec<Eye>,
}

#[derive(Clone, Debug)]
struct Eye {
    joint: usize,
    /// Where it looks, in its own bind frame: the model's front (-Y) in the bind pose.
    forward: Vec3A,
}

/// One actor's eye turns, each eased towards its aim.
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    turns: Vec<Quat>,
}

impl Eyes {
    /// The eyes on `eyes` (joint indices).
    pub(super) fn new(joints: &[Joint], eyes: &[usize]) -> Self {
        Self {
            eyes: eyes
                .iter()
                .map(|&joint| Eye {
                    joint,
                    forward: (joints[joint].bind.matrix3.inverse() * Vec3A::NEG_Y)
                        .try_normalize()
                        .unwrap_or(Vec3A::NEG_Y),
                })
                .collect(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.eyes.is_empty()
    }

    /// Turn the eyes in `pose` (model space) towards `target`, a point in the same space,
    /// `step` seconds after the last pose (`None`: the first), each at most [`LIMIT`] from
    /// where it rests in the head; with no target, back to rest.
    pub(super) fn turn(
        &self,
        pose: &mut [Affine3A],
        state: &mut State,
        target: Option<Vec3A>,
        step: Option<f32>,
    ) {
        let share = match step {
            Some(step)
                if (0.0..=MAX_STEP).contains(&step) && state.turns.len() == self.eyes.len() =>
            {
                1.0 - (-step * RATE).exp()
            }
            _ => 1.0,
        };
        state.turns.resize(self.eyes.len(), Quat::IDENTITY);
        for (eye, turn) in self.eyes.iter().zip(state.turns.iter_mut()) {
            let at = pose[eye.joint].translation;
            let rest = (pose[eye.joint].matrix3 * eye.forward).normalize_or_zero();
            let wanted = target
                .and_then(|target| (target - at).try_normalize())
                .filter(|_| rest != Vec3A::ZERO)
                .map_or(Quat::IDENTITY, |towards| {
                    let full = Quat::from_rotation_arc(rest.into(), towards.into());
                    let angle = full.angle_between(Quat::IDENTITY);
                    let limit = LIMIT.to_radians();
                    if angle > limit {
                        Quat::IDENTITY.slerp(full, limit / angle)
                    } else {
                        full
                    }
                });
            *turn = turn.slerp(wanted, share).normalize();
            if turn.angle_between(Quat::IDENTITY) < 1e-5 {
                continue;
            }
            let joint = &mut pose[eye.joint];
            joint.matrix3 = Mat3A::from_quat(*turn) * joint.matrix3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::clips::Rest;
    use super::*;

    fn joint(parent: Option<usize>, above: Vec3A, at: Vec3A) -> Joint {
        let bind = Affine3A::from_translation(at.into());
        let local = Affine3A::from_translation((at - above).into());
        Joint {
            name: String::new(),
            parent,
            bind,
            inverse_bind: bind.inverse(),
            local,
            rest: Rest::of(&local),
            above: Affine3A::IDENTITY,
        }
    }

    /// A head 60 units up, an eye in front of it, and a lid hanging from the eye.
    fn head_and_eye() -> (Vec<Joint>, Eyes) {
        let head = Vec3A::new(0.0, 0.0, 60.0);
        let eye = Vec3A::new(1.5, -3.0, 62.0);
        let joints = vec![
            joint(None, Vec3A::ZERO, head),
            joint(Some(0), head, eye),
            joint(Some(1), eye, eye + Vec3A::new(0.0, -0.5, 0.0)),
        ];
        let eyes = Eyes::new(&joints, &[1]);
        (joints, eyes)
    }

    /// Where the eye looks after `seconds` of looking at `target`, and where its
    /// highlight stands.
    fn look(target: Option<Vec3A>, seconds: usize) -> (Vec3A, Vec3A) {
        let (joints, eyes) = head_and_eye();
        let rest: Vec<Affine3A> = joints.iter().map(|joint| joint.bind).collect();
        let mut state = State::default();
        let mut pose = rest.clone();
        for frame in 0..=seconds * 60 {
            pose.clone_from(&rest);
            let step = (frame > 0).then_some(1.0 / 60.0);
            eyes.turn(&mut pose, &mut state, target, step);
        }
        let looking = pose[1].matrix3 * eyes.eyes[0].forward;
        (looking, pose[2].translation)
    }

    #[test]
    fn an_eye_looks_at_a_target_in_front_up_and_down_too() {
        let eye = Vec3A::new(1.5, -3.0, 62.0);
        // A player's eyes 100 units ahead and 20 above.
        let target = Vec3A::new(1.5, -103.0, 82.0);
        let (looking, lid) = look(Some(target), 1);
        let wanted = (target - eye).normalize();
        assert!(looking.dot(wanted) > 0.9999, "{looking:?}");
        // The eye turns where it stands; the lid under it stays.
        assert!(
            (lid - Vec3A::new(1.5, -3.5, 62.0)).length() < 1e-4,
            "{lid:?}"
        );
    }

    #[test]
    fn an_eye_turns_no_further_than_its_limit_and_rests_without_a_target() {
        // Straight to the side: 90 degrees off, so it stops at the limit.
        let (looking, _) = look(Some(Vec3A::new(200.0, -3.0, 62.0)), 1);
        let off = looking.angle_between(Vec3A::NEG_Y).to_degrees();
        assert!((off - LIMIT).abs() < 0.01, "{off}");
        assert!(looking.x > 0.0, "{looking:?}");
        let (looking, _) = look(None, 1);
        assert!(looking.dot(Vec3A::NEG_Y) > 0.9999, "{looking:?}");
    }

    #[test]
    fn an_eye_eases_towards_its_target() {
        let (joints, eyes) = head_and_eye();
        let rest: Vec<Affine3A> = joints.iter().map(|joint| joint.bind).collect();
        let mut state = State::default();
        let target = Some(Vec3A::new(60.0, -100.0, 62.0));
        let mut pose = rest.clone();
        // The first pose sets it at once; then the target jumps to the other side.
        eyes.turn(&mut pose, &mut state, target, None);
        let other = Some(Vec3A::new(-60.0, -100.0, 62.0));
        let mut pose = rest.clone();
        eyes.turn(&mut pose, &mut state, other, Some(1.0 / 60.0));
        let looking = pose[1].matrix3 * eyes.eyes[0].forward;
        // A sixtieth of a second takes it a little way back, not across.
        assert!(looking.x > 0.0, "{looking:?}");
    }
}
