//! How the pieces bolted on a body (hat, cape, saber hilts held or holstered, gun,
//! jetpack) are drawn so they look like it. In JA they are bolt-ons of the body's Ghoul2
//! instance or refEntities that copy its `renderfx` and `customShader` (JoF EJK
//! `CG_DrawCosmeticOnPlayer`, `codemp/cgame/cg_players.c:11542-11543`), so they take
//! the same duel darkening (`RF_RGB_TINT`), JoF's blue duel figure
//! (`cg_duelGhost 1`, [`crate::duel_ghost`]) and mind-trick fade as the body. Hub #323:
//! a hat stayed solid on a duellist drawn as the figure.

use crate::actor_instance::ActorInstance;
use crate::entity_materials::{OverrideInstance, OverrideMesh};

/// The body's look, for its bolt-ons.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoltOnLook {
    /// `RF_RGB_TINT`: the duel darkening, as the body's instance colour.
    pub(crate) tint: Option<[f32; 4]>,
    /// The pass the body is drawn in.
    pub(crate) pass: Pass,
}

/// The pass a body is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Its own shaders.
    #[default]
    Plain,
    /// `RF_FORCE_ENT_ALPHA` at this alpha (a mind trick's fade).
    Faded(u8),
    /// JoF EJK's duel figure in place of the body: this material, or not drawn at all
    /// when it did not load (as the body).
    Ghost(Option<usize>),
}

/// Where one body's bolt-ons go this frame, in its look.
pub(crate) struct BoltOns<'a> {
    pub(crate) look: BoltOnLook,
    pub(crate) groups: &'a mut [Vec<ActorInstance>],
    pub(crate) overrides: &'a mut Vec<OverrideInstance>,
}

impl BoltOns<'_> {
    /// Draw rigid model `mesh` at `instance` in the body's look. The override list is
    /// preallocated: a full one drops the piece rather than grow.
    pub(crate) fn push(&mut self, mesh: usize, instance: ActorInstance) {
        let instance = match self.look.tint {
            Some(tint) => instance.with_rgb_tint(tint),
            None => instance,
        };
        let (material, forced_alpha, instance) = match self.look.pass {
            Pass::Plain => {
                if let Some(group) = self.groups.get_mut(mesh) {
                    group.push(instance);
                }
                return;
            }
            Pass::Faded(alpha) => (None, true, instance.with_forced_alpha(alpha)),
            Pass::Ghost(None) => return,
            Pass::Ghost(Some(material)) => (Some(material), false, instance),
        };
        if self.overrides.len() < self.overrides.capacity() {
            self.overrides.push(OverrideInstance {
                mesh: OverrideMesh::Object(mesh),
                material,
                instance,
                no_depth: false,
                forced_alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push(look: BoltOnLook) -> (Vec<Vec<ActorInstance>>, Vec<OverrideInstance>) {
        let mut groups = vec![Vec::with_capacity(4), Vec::with_capacity(4)];
        let mut overrides = Vec::with_capacity(4);
        BoltOns {
            look,
            groups: &mut groups,
            overrides: &mut overrides,
        }
        .push(
            1,
            ActorInstance::new([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3]),
        );
        (groups, overrides)
    }

    #[test]
    fn a_bolt_on_takes_the_duel_figure_tint_and_fade_of_its_body() {
        let (groups, overrides) = push(BoltOnLook::default());
        assert_eq!((groups[1].len(), overrides.len()), (1, 0));

        // Hub #323: the duellist's figure covers the hat as well.
        let (groups, overrides) = push(BoltOnLook {
            pass: Pass::Ghost(Some(7)),
            ..BoltOnLook::default()
        });
        assert!(groups.iter().all(Vec::is_empty));
        assert_eq!(overrides.len(), 1);
        assert_eq!(overrides[0].mesh, OverrideMesh::Object(1));
        assert_eq!(overrides[0].material, Some(7));
        assert!(!overrides[0].forced_alpha);

        // No figure shader: nothing of the duellist is drawn, the hat neither.
        let (groups, overrides) = push(BoltOnLook {
            pass: Pass::Ghost(None),
            ..BoltOnLook::default()
        });
        assert!(groups.iter().all(Vec::is_empty) && overrides.is_empty());

        let (_, overrides) = push(BoltOnLook {
            pass: Pass::Faded(100),
            ..BoltOnLook::default()
        });
        assert!(overrides[0].forced_alpha && overrides[0].material.is_none());

        let dark = [0.2, 0.2, 0.2, 1.0];
        let (groups, _) = push(BoltOnLook {
            tint: Some(dark),
            ..BoltOnLook::default()
        });
        let tinted =
            ActorInstance::new([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3]).with_rgb_tint(dark);
        assert_eq!(groups[1][0].entity_color, tinted.entity_color);
        assert_eq!(groups[1][0].entity_control, tinted.entity_control);
    }
}
