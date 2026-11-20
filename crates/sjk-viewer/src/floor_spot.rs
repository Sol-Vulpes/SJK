//! Where a small thing set down in the world rests: what a small box meets between two
//! points of the map's brushes, and the floor under a spot told by the relay. The
//! Boombox stands on it ([`crate::boombox::place_spot`]); the Banana peel brought it
//! (29/10/2026) and was removed on 20/11/2026.

use super::*;
use glam::Vec3;

/// `CONTENTS_SOLID`: only the map's brushes stop it.
const MASK: u32 = 1;
/// Half the box's width, in units.
const HALF: f32 = 2.0;

/// What a trace met: where it stopped, the surface's normal, and whether it was the sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ground {
    pub(crate) stop: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) sky: bool,
}

impl GpuState {
    /// What a small box meets between two points of the map's brushes, if anything.
    pub(crate) fn floor_trace(&mut self, from: Vec3, to: Vec3) -> Option<Ground> {
        let bounds = sjk_bsp::Aabb::new([-HALF, -HALF, 0.0], [HALF, HALF, 1.0])
            .expect("the box's bounds are valid");
        let trace = self.bsp.trace_box_with(
            &mut self.trace_scratch,
            from.to_array(),
            to.to_array(),
            bounds,
            MASK,
        );
        if trace.start_solid || trace.fraction >= 1.0 {
            return None;
        }
        Some(Ground {
            stop: Vec3::from_array(trace.end_position),
            normal: trace
                .plane
                .as_ref()
                .map_or(Vec3::Z, |plane| Vec3::from_array(plane.normal)),
            sky: trace.surface_flags & weather::cover::SURF_SKY != 0,
        })
    }

    /// Where a thing relayed to rest at `at` rests here, and the floor's normal there:
    /// the floor found just under it (whole units carry it half a unit off), else as told.
    pub(crate) fn floor_rest(&mut self, at: [i32; 3]) -> (Vec3, Vec3) {
        let at = Vec3::from_array(at.map(|c| c as f32));
        self.floor_trace(at + Vec3::Z * 8.0, at - Vec3::Z * 16.0)
            .filter(|floor| !floor.sky)
            .map_or((at, Vec3::Z), |floor| {
                (floor.stop, floor.normal.normalize_or(Vec3::Z))
            })
    }
}
