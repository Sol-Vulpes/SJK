//! A player's jetpack, as `CG_Player` draws it (`codemp/cgame/cg_players.c`, JoF EJK
//! 11843-11935).
//!
//! Stock bolts `models/weapons2/jetpack/model.glm` into the player's Ghoul2 instance
//! at slot 3, on the body's third bolt, `*chestg` (`CG_InitJetpackGhoul2`,
//! `cg_players.c:676-678,10230-10250`). While `EF_JETPACK_ACTIVE` is set each of the
//! pack's `torso_ljet` and `torso_rjet` bolts plays `effects/boba/jet.efx` (twice
//! with `EF_JETPACK_FLAMING`), pushed back from the bolt along its game-facing axes;
//! the first frame plays `jetpackOnSound` and the first frame after plays
//! `jetpackOffSound`. The hover and fire loops are `sjk_client`'s loop adapter's.

use crate::bolt::{self, BoltMatrix};
use crate::saber::{self, Attachment};
use glam::{Quat, Vec3};
use sjk_vfs::VirtualFileSystem;
use std::cell::Cell;

/// `JETPACK_MODEL`.
pub(crate) const MODEL: &str = "models/weapons2/jetpack/model.glm";
/// `cgs.effects.mBobaJet`.
pub(crate) const EFFECT: &str = "boba/jet";
/// `cgs.media.jetpackOnSound` (`cg_jetpackOnSound 1`) and `jetpackOffSound`.
pub(crate) const SOUNDS: [&str; 2] = [
    "sound/chars/boba/bf_land.mp3",
    "sound/chars/boba/bf_blast-off.mp3",
];
/// The body bolt the pack hangs on.
const CHEST_BOLT: &str = "*chestg";
/// The pack's jet bolts, in `G2API_AddBolt` order.
const JET_BOLTS: [&str; 2] = ["torso_ljet", "torso_rjet"];
/// `MAX_GENTITIES`.
const MAX_ENTITIES: usize = 1_024;

/// The pack's mesh and its jet bolts in pack space, read once per world.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Catalog {
    pub(crate) mesh: Option<usize>,
    jets: [Option<BoltMatrix>; 2],
}

impl Catalog {
    /// Find the loaded pack and read its bolts in the bind pose, as weapon
    /// `*flash` bolts are (`object_meshes::load_one`).
    pub(crate) fn build(vfs: &VirtualFileSystem, meshes: &[crate::StaticModelMesh]) -> Self {
        let mesh = meshes
            .iter()
            .position(|mesh| mesh.appearance.variant.is_empty() && mesh.appearance.model == MODEL);
        let jets = vfs
            .read(MODEL)
            .ok()
            .flatten()
            .and_then(|asset| sjk_model::Glm::parse(&asset.bytes).ok())
            .map(|model| {
                let pose = vec![
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0]
                    ];
                    model.bone_count
                ];
                JET_BOLTS.map(|name| model.surface_bolt_matrix(name, 0, &pose).ok().flatten())
            })
            .unwrap_or_default();
        Self { mesh, jets }
    }

    /// The two jet flames in world space for a pack at `origin`/`rotation`: origin
    /// and direction, as `CG_Player` moves them off the bolts.
    pub(crate) fn flames(&self, origin: Vec3, rotation: Quat) -> [Option<(Vec3, Vec3)>; 2] {
        std::array::from_fn(|jet| {
            let game = bolt::game_facing(self.jets[jet]?);
            let world = |column| rotation * Vec3::from_array(bolt::column(&game, column));
            let position = origin + world(3);
            // `NEGATIVE_Y` and `POSITIVE_X`; the left jet steps back along -Y first.
            let negative_y = -world(1).normalize_or_zero();
            let positive_x = world(0).normalize_or_zero();
            Some(if jet == 0 {
                (position - 9.5 * negative_y - 13.5 * positive_x, positive_x)
            } else {
                (position - 9.5 * positive_x - 13.5 * negative_y, negative_y)
            })
        })
    }
}

/// One actor's `*chestg` bolt, evaluated only while it wears a pack.
#[derive(Clone, Debug, Default)]
pub(crate) struct Worn {
    /// The actor wore a pack when last drawn; set by the submission.
    pub(crate) wanted: Cell<bool>,
    /// `*chestg` in the latest pose, model space.
    pub(crate) chest: Option<Attachment>,
}

impl Worn {
    /// Refresh the chest bolt from one evaluated pose; nothing is computed for an
    /// actor without a pack.
    pub(crate) fn update(&mut self, preview: &crate::PlayerPreview, matrices: &[[[f32; 4]; 3]]) {
        self.chest = if self.wanted.get() {
            saber::attachment_from_matrices(preview, matrices, CHEST_BOLT)
        } else {
            None
        };
    }
}

/// `centity_t::hasPlayedJetpackSounds`, per entity.
pub(crate) struct Sounds {
    played: Box<[bool]>,
}

impl Default for Sounds {
    fn default() -> Self {
        Self {
            played: vec![false; MAX_ENTITIES].into_boxed_slice(),
        }
    }
}

/// A jetpack one-shot to start.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Cue {
    On,
    Off,
}

impl Cue {
    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::On => SOUNDS[0],
            Self::Off => SOUNDS[1],
        }
    }
}

impl Sounds {
    /// Advance one entity's pack for a drawn frame: the on sound the first frame its
    /// jets burn, the off sound the first frame after. A pack hidden by a mind trick
    /// stays silent but still keeps its state.
    pub(crate) fn advance(&mut self, entity: u16, active: bool, audible: bool) -> Option<Cue> {
        let played = self.played.get_mut(usize::from(entity))?;
        let cue = match (active, *played) {
            (true, false) => Some(Cue::On),
            (false, true) => Some(Cue::Off),
            _ => None,
        };
        *played = active;
        cue.filter(|_| audible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_then_off_once_each() {
        let mut sounds = Sounds::default();
        assert_eq!(sounds.advance(7, true, true), Some(Cue::On));
        assert_eq!(sounds.advance(7, true, true), None);
        assert_eq!(sounds.advance(7, false, true), Some(Cue::Off));
        assert_eq!(sounds.advance(7, false, true), None);
    }

    #[test]
    fn a_tricked_pack_is_silent_but_tracked() {
        let mut sounds = Sounds::default();
        assert_eq!(sounds.advance(3, true, false), None);
        assert_eq!(sounds.advance(3, true, true), None);
        assert_eq!(sounds.advance(3, false, true), Some(Cue::Off));
    }

    #[test]
    fn flames_step_back_from_their_bolts() {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let catalog = Catalog {
            mesh: None,
            jets: [Some(identity); 2],
        };
        // Game facing turns raw -Y into +X and raw +X into +Y: POSITIVE_X is -Y here and
        // NEGATIVE_Y is -X.
        let [left, right] = catalog.flames(Vec3::ZERO, Quat::IDENTITY);
        let (origin, direction) = left.unwrap();
        assert!(direction.abs_diff_eq(-Vec3::Y, 1e-6));
        assert!(origin.abs_diff_eq(Vec3::new(9.5, 13.5, 0.0), 1e-5));
        let (origin, direction) = right.unwrap();
        assert!(direction.abs_diff_eq(-Vec3::X, 1e-6));
        assert!(origin.abs_diff_eq(Vec3::new(13.5, 9.5, 0.0), 1e-5));
    }
}
