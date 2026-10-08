//! Movers (doors, lifts, `func_static` brushes) as occluders of the map's lamps.
//!
//! Lamp shadows come from a map-lifetime trace of the static world
//! (`lamp_visibility.rs`), which has no movers: a closed door let every lamp behind it
//! through. Each lamp a mover can reach gets a second tile in the same visibility
//! atlas, traced against the movers alone at their current pose; a receiver's lamp
//! visibility is the world tile's times the door tile's. A mover that changes pose
//! queues its lamps' door tiles again, traced on the GPU a few per frame
//! ([`gpu`]), so a lift moving all the time spreads its cost and the final pose is always
//! traced.
//!
//! Only movers the client knows block light: those in the current snapshot, at the pose
//! they are drawn with, and those of the map's baselines it has not seen yet, at their
//! spawn pose. A hidden or broken mover (not drawn) blocks nothing.
use glam::{Quat, Vec3};
use std::collections::VecDeque;

#[path = "mover_occlusion_gpu.rs"]
pub(crate) mod gpu;

/// One mover's opaque, shadow-casting triangles in its model space.
pub(crate) struct Occluder {
    /// The mover catalog's mesh (`movers::Catalog::meshes`).
    pub(crate) mesh: usize,
    pub(crate) triangles: Vec<[Vec3; 3]>,
    /// Model-space bounds of `triangles`.
    pub(crate) lower: Vec3,
    pub(crate) upper: Vec3,
    /// The world box it can move through ([`reach`]).
    pub(crate) reach: (Vec3, Vec3),
    /// The lamps the static world lets see `reach` ([`seen_by`]), sorted; `None` for all.
    pub(crate) seen_by: Option<Vec<u32>>,
}

impl Occluder {
    /// `None` without triangles.
    pub(crate) fn new(mesh: usize, triangles: Vec<[Vec3; 3]>, reach: (Vec3, Vec3)) -> Option<Self> {
        let mut lower = Vec3::splat(f32::INFINITY);
        let mut upper = Vec3::splat(f32::NEG_INFINITY);
        for point in triangles.iter().flatten() {
            lower = lower.min(*point);
            upper = upper.max(*point);
        }
        (!triangles.is_empty() && lower.is_finite() && upper.is_finite()).then_some(Self {
            mesh,
            triangles,
            lower,
            upper,
            reach,
            seen_by: None,
        })
    }
}

/// The lamps whose reach touches `reach` and that see into it past the static world
/// (`blocked`): a ray from the lamp to one of 27 points spread through the box gets
/// through. A mover in another room needs no door tile for the lamp.
pub(crate) fn seen_by(
    lamps: &[crate::lamp_lights::Lamp],
    reach: (Vec3, Vec3),
    blocked: impl Fn(Vec3, Vec3) -> bool + Sync,
) -> Vec<u32> {
    let (lower, upper) = reach;
    // Just inside the box, off the walls a door slides against.
    let inset = ((upper - lower) * 0.5).min(Vec3::ONE);
    let (lower, upper) = (lower + inset, upper - inset);
    let points: Vec<Vec3> = (0..27)
        .map(|i| {
            let t = Vec3::new((i % 3) as f32, (i / 3 % 3) as f32, (i / 9) as f32) * 0.5;
            lower + (upper - lower) * t
        })
        .collect();
    let sees = |lamp: &crate::lamp_lights::Lamp| {
        touches(lamp.position, lamp.radius, reach.0, reach.1) && {
            let from = lamp.position + lamp.normal * 0.5;
            points.iter().any(|&point| !blocked(from, point))
        }
    };
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .clamp(1, 4);
    let chunk = lamps.len().div_ceil(workers).max(1);
    let seen: Vec<Vec<u32>> = std::thread::scope(|scope| {
        let handles: Vec<_> = lamps
            .chunks(chunk)
            .enumerate()
            .map(|(part, chunk_lamps)| {
                let sees = &sees;
                scope.spawn(move || {
                    chunk_lamps
                        .iter()
                        .enumerate()
                        .filter(|(_, lamp)| sees(lamp))
                        .map(|(i, _)| (part * chunk + i) as u32)
                        .collect::<Vec<u32>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a lamp visibility worker"))
            .collect()
    });
    seen.concat()
}

/// The world box a brush model can move through, from its entity's spawn keys and its
/// model-space bounds. `func_door` and `func_button` slide between their spawn and
/// moved positions as `SP_func_door` computes them (`G_SetMovedir`, `lip`), `func_plat`
/// drops by its `height`; brushes that never move keep their spawn bounds. Anything
/// else (trains, rotating and bobbing movers, unknown classes) gets its bounds grown on
/// every side by its largest extent, which a train or a long swing can still leave.
pub(crate) fn reach(entity: Option<&sjk_entity::Entity>, lower: Vec3, upper: Vec3) -> (Vec3, Vec3) {
    let origin = entity
        .and_then(|e| e.vector("origin").ok().flatten())
        .map_or(Vec3::ZERO, Vec3::from_array);
    let (lower, upper) = (origin + lower, origin + upper);
    let size = upper - lower;
    let number = |key: &str, default: f32| {
        entity
            .and_then(|e| e.number(key).ok().flatten())
            .unwrap_or(default)
    };
    let slide = |travel: Vec3| (lower.min(lower + travel), upper.max(upper + travel));
    match entity.and_then(sjk_entity::Entity::classname) {
        Some("func_door" | "func_button") => {
            let angles = entity
                .and_then(|e| e.vector("angles").ok().flatten())
                .map(Vec3::from_array)
                .unwrap_or_else(|| Vec3::new(0., number("angle", 0.), 0.));
            let direction = if angles == Vec3::new(0., -1., 0.) {
                Vec3::Z
            } else if angles == Vec3::new(0., -2., 0.) {
                Vec3::NEG_Z
            } else {
                let (pitch, yaw) = (angles.x.to_radians(), angles.y.to_radians());
                Vec3::new(
                    pitch.cos() * yaw.cos(),
                    pitch.cos() * yaw.sin(),
                    -pitch.sin(),
                )
            };
            let lip = number(
                "lip",
                if entity.and_then(sjk_entity::Entity::classname) == Some("func_button") {
                    4.
                } else {
                    8.
                },
            );
            slide(direction * (direction.abs().dot(size) - lip))
        }
        Some("func_plat") => slide(Vec3::new(
            0.,
            0.,
            -number("height", size.z - number("lip", 8.)),
        )),
        Some("func_static" | "func_breakable" | "func_usable" | "func_glass" | "func_wall") => {
            (lower, upper)
        }
        _ => {
            let grow = Vec3::splat(size.max_element());
            (lower - grow, upper + grow)
        }
    }
}

/// Whether a lamp's sphere of influence touches a box.
fn touches(position: Vec3, radius: f32, lower: Vec3, upper: Vec3) -> bool {
    position.clamp(lower, upper).distance_squared(position) < radius * radius
}

/// One door tile: the lamp it belongs to and the occluders that can shadow that lamp.
#[derive(Debug, PartialEq)]
pub(crate) struct Tile {
    pub(crate) lamp: u32,
    pub(crate) occluders: Vec<u32>,
}

/// The lamps that get a door tile and, per occluder, the tiles it can shadow.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Doors {
    pub(crate) tiles: Vec<Tile>,
    pub(crate) by_occluder: Vec<Vec<u32>>,
}

impl Doors {
    /// Give a door tile to every lamp whose reach touches an occluder's, at most
    /// `capacity` of them: the most powerful when there are more.
    pub(crate) fn assign(
        lamps: &[crate::lamp_lights::Lamp],
        occluders: &[Occluder],
        capacity: usize,
    ) -> Self {
        let reaches: Vec<_> = occluders.iter().map(|occluder| occluder.reach).collect();
        let mut candidates: Vec<(u32, Vec<u32>)> = lamps
            .iter()
            .enumerate()
            .filter_map(|(lamp, light)| {
                let near: Vec<u32> = reaches
                    .iter()
                    .enumerate()
                    .filter(|(occluder, (lower, upper))| {
                        touches(light.position, light.radius, *lower, *upper)
                            && occluders[*occluder]
                                .seen_by
                                .as_ref()
                                .is_none_or(|seen| seen.binary_search(&(lamp as u32)).is_ok())
                    })
                    .map(|(occluder, _)| occluder as u32)
                    .collect();
                (!near.is_empty()).then_some((lamp as u32, near))
            })
            .collect();
        if candidates.len() > capacity {
            candidates.sort_by(|a, b| {
                lamps[b.0 as usize]
                    .power
                    .total_cmp(&lamps[a.0 as usize].power)
                    .then(a.0.cmp(&b.0))
            });
            candidates.truncate(capacity);
            candidates.sort_by_key(|candidate| candidate.0);
        }
        let mut by_occluder = vec![Vec::new(); occluders.len()];
        let tiles = candidates
            .into_iter()
            .enumerate()
            .map(|(tile, (lamp, occluders))| {
                for &occluder in &occluders {
                    by_occluder[occluder as usize].push(tile as u32);
                }
                Tile { lamp, occluders }
            })
            .collect();
        Self { tiles, by_occluder }
    }
}

/// Whether something inside the box `lower`..`upper` can stand between a lamp at
/// `lamp` and some point of the cell `cell_lower`..`cell_upper`: the cell meets the cone
/// from the lamp around the box's bounding sphere, beyond the sphere's near side.
/// Conservative: bounding spheres stand in for both boxes.
pub(crate) fn may_shadow(
    lamp: Vec3,
    lower: Vec3,
    upper: Vec3,
    cell_lower: Vec3,
    cell_upper: Vec3,
) -> bool {
    let centre = (lower + upper) * 0.5;
    let radius = (upper - lower).length() * 0.5;
    let to_box = centre - lamp;
    let distance = to_box.length();
    let cell_centre = (cell_lower + cell_upper) * 0.5;
    let cell_radius = (cell_upper - cell_lower).length() * 0.5;
    let to_cell = cell_centre - lamp;
    let reach = to_cell.length();
    if distance <= radius || reach <= cell_radius {
        return true;
    }
    if reach + cell_radius < distance - radius {
        return false;
    }
    let cone = (radius / distance).asin();
    let spread = (cell_radius / reach).asin();
    let angle = to_box.angle_between(to_cell);
    angle <= cone + spread
}

/// Per door tile, the lamp cache texels its movers can shadow: per layer, the union of
/// the texel rectangles (`[x0, y0, x1, y1)`, grown by two texels for the bake's filters)
/// of the surfaces in its lamp's reach that [`may_shadow`] finds behind them.
pub(crate) fn cache_regions(
    lamps: &[crate::lamp_lights::Lamp],
    doors: &Doors,
    occluders: &[Occluder],
    surfaces: &[Vec<crate::world_materials::lamp_cache::CachedSurface>],
    resolution: u32,
) -> Vec<Vec<(u32, [u32; 4])>> {
    doors
        .tiles
        .iter()
        .map(|tile| {
            let lamp = &lamps[tile.lamp as usize];
            let mut regions = Vec::new();
            for (layer, list) in surfaces.iter().enumerate() {
                let mut union: Option<[u32; 4]> = None;
                for surface in list {
                    if !touches(lamp.position, lamp.radius, surface.lower, surface.upper)
                        || !tile.occluders.iter().any(|&o| {
                            let (lower, upper) = occluders[o as usize].reach;
                            may_shadow(lamp.position, lower, upper, surface.lower, surface.upper)
                        })
                    {
                        continue;
                    }
                    let [x0, y0, x1, y1] = surface.texels;
                    let grown = [
                        x0.saturating_sub(2),
                        y0.saturating_sub(2),
                        (x1 + 2).min(resolution),
                        (y1 + 2).min(resolution),
                    ];
                    union = Some(union.map_or(grown, |u| merge(u, grown)));
                }
                if let Some(rect) = union {
                    regions.push((layer as u32, rect));
                }
            }
            regions
        })
        .collect()
}

/// The smallest rectangle holding both.
pub(crate) fn merge(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

/// Where an occluder is and whether it blocks light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pose {
    pub(crate) origin: Vec3,
    pub(crate) rotation: Quat,
    pub(crate) blocking: bool,
}

impl Pose {
    /// Unknown movers block nothing.
    pub(crate) const NONE: Self = Self {
        origin: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        blocking: false,
    };

    pub(crate) fn of(mover: &crate::movers::Presented) -> Self {
        Self {
            origin: Vec3::from_array(mover.origin),
            rotation: Quat::from_array(mover.rotation),
            blocking: mover.visible,
        }
    }

    /// A different pose for lighting: a hundredth of a unit, or about a hundredth of a
    /// degree, apart. A stopped mover evaluates to the same pose every frame.
    pub(crate) fn moved(&self, other: &Self) -> bool {
        self.blocking != other.blocking
            || (self.blocking
                && (self.origin.distance_squared(other.origin) > 1e-4
                    || turned(self.rotation, other.rotation)))
    }
}

/// Whether two rotations are more than about a hundredth of a degree apart, either sign
/// of the quaternion. Measured as the distance between the quaternions, which is zero for
/// equal ones: their dot product cannot tell, since `1 - 1e-8` is 1 in f32 and a
/// quaternion's dot with itself is one step below 1 at many angles (yaw 90 or 270 from
/// `legacy_angles_to_quaternion`), so a still mover looked as if it moved every frame.
fn turned(a: Quat, b: Quat) -> bool {
    let (a, b) = (glam::Vec4::from(a), glam::Vec4::from(b));
    // Unit quaternions θ apart are about θ/2 apart: (1e-4)² is θ near 0.0115°.
    (a - b).length_squared().min((a + b).length_squared()) > 1e-8
}

/// Door tiles waiting to be traced, oldest first, each at most once.
#[derive(Debug, Default)]
pub(crate) struct Queue {
    pending: VecDeque<u32>,
    queued: Vec<bool>,
}

impl Queue {
    pub(crate) fn new(tiles: usize) -> Self {
        Self {
            pending: VecDeque::with_capacity(tiles),
            queued: vec![false; tiles],
        }
    }

    pub(crate) fn push(&mut self, tile: u32) {
        if let Some(queued) = self.queued.get_mut(tile as usize)
            && !*queued
        {
            *queued = true;
            self.pending.push_back(tile);
        }
    }

    /// Move up to `budget` tiles into `batch`.
    pub(crate) fn take(&mut self, budget: usize, batch: &mut Vec<u32>) {
        batch.clear();
        while batch.len() < budget {
            let Some(tile) = self.pending.pop_front() else {
                break;
            };
            self.queued[tile as usize] = false;
            batch.push(tile);
        }
    }

    pub(crate) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.pending.len()
    }
}

/// Every occluder's current pose; queues the door tiles of those that change.
#[derive(Debug)]
pub(crate) struct Poses {
    pub(crate) current: Vec<Pose>,
    /// Occluders a snapshot or a baseline has placed.
    known: Vec<bool>,
    /// Counts pose changes, for the far sun cascade.
    pub(crate) generation: u64,
}

impl Poses {
    pub(crate) fn new(occluders: usize) -> Self {
        Self {
            current: vec![Pose::NONE; occluders],
            known: vec![false; occluders],
            generation: 0,
        }
    }

    /// Place `occluder` at `pose`, queueing its tiles when that changes its light.
    pub(crate) fn set(&mut self, occluder: usize, pose: Pose, doors: &Doors, queue: &mut Queue) {
        self.known[occluder] = true;
        if self.current[occluder].moved(&pose) {
            for &tile in &doors.by_occluder[occluder] {
                queue.push(tile);
            }
            self.generation = self.generation.wrapping_add(1);
        }
        self.current[occluder] = pose;
    }

    /// Place an occluder no snapshot has shown, at its spawn pose.
    pub(crate) fn set_unknown(
        &mut self,
        occluder: usize,
        pose: Pose,
        doors: &Doors,
        queue: &mut Queue,
    ) {
        if !self.known[occluder] {
            self.set(occluder, pose, doors, queue);
        }
    }

    pub(crate) fn any_unknown(&self) -> bool {
        self.known.iter().any(|known| !known)
    }
}

#[cfg(test)]
#[path = "mover_occlusion_tests.rs"]
mod tests;
