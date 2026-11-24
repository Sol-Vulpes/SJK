//! Engine-native world and presentation state.
//!
//! This crate deliberately knows nothing about JKA entity numbers, snapshots,
//! BSP globals, or protocol limits. Compatibility layers translate external
//! state into these owned worlds.

use std::collections::BTreeMap;

mod pose;
pub use pose::{PoseAngleState, PoseState};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WorldId(u64);

impl WorldId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityId(u64);

impl EntityId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityKind {
    Actor,
    Corpse,
    Item,
    Projectile,
    Mover,
    Effect,
    Other,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Appearance {
    pub model: String,
    pub variant: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationTrackInput {
    pub clip: usize,
    pub revision: u64,
    /// Fixed-point playback multiplier where 1000 is normal speed.
    pub speed_milli: u16,
    /// Ghoul2 transition duration selected by the compatibility layer.
    pub blend_millis: u16,
    pub forced_frame: Option<usize>,
    /// Optional phase inherited from another synchronized bone track.
    pub resume_phase_millis: Option<i64>,
}

impl AnimationTrackInput {
    pub const fn normal(clip: usize, revision: u64) -> Self {
        Self {
            clip,
            revision,
            speed_milli: 1_000,
            blend_millis: 100,
            forced_frame: None,
            resume_phase_millis: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationTrackState {
    pub clip: usize,
    pub revision: u64,
    pub started_at_millis: i64,
    pub phase_millis: i64,
    pub speed_milli: u16,
    pub forced_frame: Option<usize>,
    pub transition: Option<AnimationTrackTransition>,
}

impl AnimationTrackState {
    pub fn elapsed_millis(self, time_millis: i64) -> i64 {
        self.phase_millis
            + (time_millis - self.started_at_millis).max(0) * i64::from(self.speed_milli) / 1_000
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationTrackTransition {
    pub clip: usize,
    pub revision: u64,
    pub started_at_millis: i64,
    pub phase_millis: i64,
    pub speed_milli: u16,
    pub forced_frame: Option<usize>,
    pub blend_started_at_millis: i64,
    pub blend_duration_millis: u16,
}

impl AnimationTrackTransition {
    /// Sample the outgoing track up to the blend boundary, then hold that pose.
    ///
    /// A renderer may present times before a future snapshot's transition has
    /// become current. Retaining the outgoing clock makes that delayed
    /// presentation continuous; clamping at `blend_started_at_millis` preserves
    /// the frozen-source-pose behavior used during the actual blend.
    pub fn elapsed_millis(self, time_millis: i64) -> i64 {
        let sample_time = time_millis.min(self.blend_started_at_millis);
        self.phase_millis
            + (sample_time - self.started_at_millis).max(0) * i64::from(self.speed_milli) / 1_000
    }

    /// Ghoul2 blend fraction at presentation time.
    ///
    /// `CG_SetLerpFrameAnimation` uses a zero-duration transition for death
    /// cuts. Those still retain the outgoing pose while presenting time before
    /// the future snapshot, then switch exactly at its timestamp.
    pub fn blend_fraction(self, time_millis: i64) -> f32 {
        if self.blend_duration_millis == 0 {
            return if time_millis < self.blend_started_at_millis {
                0.0
            } else {
                1.0
            };
        }
        ((time_millis - self.blend_started_at_millis).max(0) as f32
            / f32::from(self.blend_duration_millis))
        .clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationState {
    pub lower: AnimationTrackState,
    pub upper: AnimationTrackState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeldItemKind {
    EnergyBlade,
    Melee,
    Ranged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeldEquipment {
    /// The primary held item is not in the actor's hand.
    pub primary_in_flight: bool,
    pub kind: HeldItemKind,
    pub weapon: u8,
    pub active: bool,
    pub color: [u8; 3],
    /// Whether a second held item or secondary blade group is active.
    pub secondary_active: bool,
    /// Presentation color of a second held item.
    pub secondary_color: [u8; 3],
    /// Adapter-authored motion-trail duration; zero disables ordinary trails.
    pub trail_duration_millis: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ItemState {
    pub dropped: bool,
    pub weapon: bool,
    pub powerup: bool,
    pub vertical_offset: i8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionSample {
    pub time_millis: i64,
    pub transform: Transform,
}

/// Motion samples each entity keeps, oldest first.
///
/// Presentation trails the newest sample by about one sample interval plus the
/// arrival jitter: stock's clock drifts back 2 ms whenever a frame came within 5 ms of
/// the newest snapshot (`CL_AdjustTimeDelta`, `cl_cgame.cpp`), so when a snapshot
/// arrives the drawn time is usually still short of the one before it. cgame then
/// still interpolates the two snapshots before it (`CG_ProcessSnapshots` keeps
/// `cg.snap`/`cg.nextSnap` from its snapshot queue). With only the newest two kept, such
/// frames held the older one and then caught up, so remote players stepped at the
/// snapshot rate (hub #261). Four cover a drawn time three intervals behind.
pub const MOTION_HISTORY: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct SceneEntity {
    pub id: EntityId,
    pub kind: EntityKind,
    motion_revision: u64,
    motion_discontinuity_at_millis: Option<i64>,
    appearance: Option<Appearance>,
    animation: Option<AnimationState>,
    equipment: Option<HeldEquipment>,
    item: Option<ItemState>,
    /// Per-entity RGBA modulation applied by materials that opt into entity
    /// colour; `[255; 4]` is the neutral default.
    color: [u8; 4],
    /// Whether the entity drops a ground-contact shadow; adapters clear it
    /// for states the game hides (dead, cloaked, mounted).
    ground_shadow: bool,
    /// The newest [`MOTION_HISTORY`] samples, oldest first; the last is the current one.
    samples: [MotionSample; MOTION_HISTORY],
    /// The pose given with each sample ([`World::set_pose`]).
    poses: [Option<PoseState>; MOTION_HISTORY],
    /// Whether each sample must not be reached by interpolating from the one before.
    breaks: [bool; MOTION_HISTORY],
}

/// The samples a time falls between: `(earlier, later, fraction)`.
type Bracket = (usize, usize, f32);

const NEWEST: usize = MOTION_HISTORY - 1;

impl SceneEntity {
    fn new(id: EntityId, kind: EntityKind, sample: MotionSample) -> Self {
        Self {
            id,
            kind,
            motion_revision: 0,
            motion_discontinuity_at_millis: None,
            appearance: None,
            animation: None,
            equipment: None,
            item: None,
            color: [u8::MAX; 4],
            ground_shadow: true,
            samples: [sample; MOTION_HISTORY],
            poses: [None; MOTION_HISTORY],
            breaks: [false; MOTION_HISTORY],
        }
    }

    pub fn current(&self) -> MotionSample {
        self.samples[NEWEST]
    }

    /// The kept samples `time_millis` falls between, as cgame's `cg.snap` and
    /// `cg.nextSnap`: past the newest it holds the newest, before the oldest the
    /// oldest, and across a discontinuity the sample before it.
    fn bracket(&self, time_millis: i64) -> Bracket {
        if time_millis >= self.samples[NEWEST].time_millis {
            return (NEWEST, NEWEST, 0.0);
        }
        for later in (1..MOTION_HISTORY).rev() {
            let earlier = later - 1;
            let (start, end) = (
                self.samples[earlier].time_millis,
                self.samples[later].time_millis,
            );
            if time_millis >= start && end > start {
                if self.breaks[later] {
                    return (earlier, earlier, 0.0);
                }
                let fraction = (time_millis - start) as f32 / (end - start) as f32;
                return (earlier, later, fraction.clamp(0.0, 1.0));
            }
        }
        (0, 0, 0.0)
    }

    /// Monotonic revision changed when motion must not interpolate from its
    /// previous sample (for example, a portal or retained-world handoff).
    pub fn motion_revision(&self) -> u64 {
        self.motion_revision
    }

    /// Revision of the motion sample actually visible at `time_millis`.
    pub fn sampled_motion_revision(&self, time_millis: i64) -> u64 {
        if self
            .motion_discontinuity_at_millis
            .is_some_and(|boundary| time_millis < boundary)
        {
            self.motion_revision.wrapping_sub(1)
        } else {
            self.motion_revision
        }
    }

    pub fn appearance(&self) -> Option<&Appearance> {
        self.appearance.as_ref()
    }

    pub fn animation(&self) -> Option<AnimationState> {
        self.animation
    }

    pub fn equipment(&self) -> Option<HeldEquipment> {
        self.equipment
    }

    pub fn pose(&self) -> Option<PoseState> {
        self.poses[NEWEST]
    }

    /// Sample adapter-owned pose inputs on the same snapshot interval as the
    /// entity transform. Angles use Quake's shortest-arc `LerpAngle`; discrete
    /// state is the later sample's of the interval drawn, as cgame keeps one
    /// snapshot's `currentState` beside `cent->lerpAngles`.
    pub fn sample_pose(&self, time_millis: i64) -> Option<PoseState> {
        let newest = self.poses[NEWEST]?;
        let (earlier, later, fraction) = self.bracket(time_millis);
        let current = self.poses[later].unwrap_or(newest);
        if earlier == later {
            return Some(current);
        }
        let previous = self.poses[earlier].unwrap_or(current);
        Some(PoseState {
            view_angles_degrees: std::array::from_fn(|axis| {
                lerp_angle(
                    previous.view_angles_degrees[axis],
                    current.view_angles_degrees[axis],
                    fraction,
                )
            }),
            ..current
        })
    }

    pub fn item(&self) -> Option<ItemState> {
        self.item
    }

    /// Per-entity RGBA modulation consumed by entity-coloured material stages.
    pub fn color(&self) -> [u8; 4] {
        self.color
    }

    /// Whether the presentation should drop a ground-contact shadow.
    pub fn ground_shadow(&self) -> bool {
        self.ground_shadow
    }

    pub fn sample(&self, time_millis: i64) -> Transform {
        let (earlier, later, fraction) = self.bracket(time_millis);
        if earlier == later {
            return self.samples[earlier].transform;
        }
        interpolate_transform(
            self.samples[earlier].transform,
            self.samples[later].transform,
            fraction,
        )
    }

    /// Append a sample (or replace the newest at the same time). A `discontinuous`
    /// sample is never interpolated into. The newest pose carries over until
    /// [`World::set_pose`] gives the new sample its own.
    fn push(&mut self, sample: MotionSample, discontinuous: bool) -> bool {
        let newest = self.samples[NEWEST].time_millis;
        if sample.time_millis < newest {
            return false;
        }
        if sample.time_millis > newest {
            self.samples.rotate_left(1);
            self.poses.rotate_left(1);
            self.breaks.rotate_left(1);
            self.poses[NEWEST] = self.poses[NEWEST - 1];
            self.breaks[NEWEST] = false;
        }
        self.samples[NEWEST] = sample;
        self.breaks[NEWEST] |= discontinuous;
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct World {
    id: WorldId,
    entities: BTreeMap<EntityId, SceneEntity>,
}

impl World {
    pub fn new(id: WorldId) -> Self {
        Self {
            id,
            entities: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> WorldId {
        self.id
    }

    pub fn entities(&self) -> impl Iterator<Item = &SceneEntity> {
        self.entities.values()
    }

    pub fn entity(&self, id: EntityId) -> Option<&SceneEntity> {
        self.entities.get(&id)
    }

    pub fn upsert(&mut self, id: EntityId, kind: EntityKind, sample: MotionSample) -> bool {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.kind = kind;
            return entity.push(sample, false);
        }
        self.entities.insert(id, SceneEntity::new(id, kind, sample));
        true
    }

    /// Insert a motion sample that is intentionally discontinuous.
    ///
    /// Consumers retain the old endpoint before `sample`'s timestamp and see
    /// the new endpoint at it, rather than gliding across the discontinuity.
    pub fn upsert_discontinuous(
        &mut self,
        id: EntityId,
        kind: EntityKind,
        sample: MotionSample,
    ) -> bool {
        if let Some(entity) = self.entities.get_mut(&id) {
            if !entity.push(sample, true) {
                return false;
            }
            entity.kind = kind;
            entity.motion_revision = entity.motion_revision.wrapping_add(1);
            entity.motion_discontinuity_at_millis = Some(sample.time_millis);
            return true;
        }
        self.upsert(id, kind, sample)
    }

    pub fn remove(&mut self, id: EntityId) -> Option<SceneEntity> {
        self.entities.remove(&id)
    }

    pub fn set_appearance(&mut self, id: EntityId, appearance: Option<Appearance>) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };
        entity.appearance = appearance;
        true
    }

    pub fn set_animation(
        &mut self,
        id: EntityId,
        lower: AnimationTrackInput,
        upper: AnimationTrackInput,
        time_millis: i64,
    ) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };
        let update_track = |current: Option<AnimationTrackState>, input: AnimationTrackInput| {
            if let Some(mut current) = current
                && current.clip == input.clip
                && current.revision == input.revision
                && current.forced_frame == input.forced_frame
            {
                if current.speed_milli != input.speed_milli {
                    current.phase_millis = current.elapsed_millis(time_millis);
                    current.started_at_millis = time_millis;
                    current.speed_milli = input.speed_milli;
                    current.transition = None;
                }
                return current;
            }
            AnimationTrackState {
                clip: input.clip,
                revision: input.revision,
                started_at_millis: time_millis,
                phase_millis: input.resume_phase_millis.unwrap_or(0),
                speed_milli: input.speed_milli,
                forced_frame: input.forced_frame,
                transition: current.map(|previous| AnimationTrackTransition {
                    clip: previous.clip,
                    revision: previous.revision,
                    started_at_millis: previous.started_at_millis,
                    phase_millis: previous.phase_millis,
                    speed_milli: previous.speed_milli,
                    forced_frame: previous.forced_frame,
                    blend_started_at_millis: time_millis,
                    blend_duration_millis: input.blend_millis,
                }),
            }
        };
        let current = entity.animation;
        entity.animation = Some(AnimationState {
            lower: update_track(current.map(|state| state.lower), lower),
            upper: update_track(current.map(|state| state.upper), upper),
        });
        true
    }

    pub fn set_pose(&mut self, id: EntityId, pose: Option<PoseState>) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };
        if entity.poses[NEWEST].is_none() {
            // The first pose stands for the samples before it too.
            entity.poses = [pose; MOTION_HISTORY];
        }
        entity.poses[NEWEST] = pose;
        true
    }

    pub fn set_equipment(&mut self, id: EntityId, equipment: Option<HeldEquipment>) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };
        entity.equipment = equipment;
        true
    }

    /// Set the per-entity RGBA modulation; returns `false` for unknown ids.
    pub fn set_color(&mut self, id: EntityId, color: [u8; 4]) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };
        entity.color = color;
        true
    }

    pub fn set_item(&mut self, id: EntityId, item: Option<ItemState>) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };
        entity.item = item;
        true
    }

    pub fn set_ground_shadow(&mut self, id: EntityId, ground_shadow: bool) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };
        entity.ground_shadow = ground_shadow;
        true
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldSet {
    worlds: BTreeMap<WorldId, World>,
    foreground: Option<WorldId>,
}

impl WorldSet {
    pub fn insert(&mut self, world: World) -> Option<World> {
        self.worlds.insert(world.id(), world)
    }

    pub fn world(&self, id: WorldId) -> Option<&World> {
        self.worlds.get(&id)
    }

    pub fn world_mut(&mut self, id: WorldId) -> Option<&mut World> {
        self.worlds.get_mut(&id)
    }

    pub fn set_foreground(&mut self, id: WorldId) -> bool {
        if !self.worlds.contains_key(&id) {
            return false;
        }
        self.foreground = Some(id);
        true
    }

    pub fn foreground(&self) -> Option<&World> {
        self.foreground.and_then(|id| self.worlds.get(&id))
    }

    pub fn remove(&mut self, id: WorldId) -> Option<World> {
        if self.foreground == Some(id) {
            self.foreground = None;
        }
        self.worlds.remove(&id)
    }
}

fn interpolate_transform(start: Transform, end: Transform, fraction: f32) -> Transform {
    Transform {
        translation: lerp_array(start.translation, end.translation, fraction),
        rotation: normalized_lerp_quaternion(start.rotation, end.rotation, fraction),
        scale: lerp_array(start.scale, end.scale, fraction),
    }
}

fn lerp_array<const N: usize>(start: [f32; N], end: [f32; N], fraction: f32) -> [f32; N] {
    std::array::from_fn(|axis| start[axis] + (end[axis] - start[axis]) * fraction)
}

fn lerp_angle(start: f32, end: f32, fraction: f32) -> f32 {
    let delta = (end - start + 180.0).rem_euclid(360.0) - 180.0;
    start + fraction * delta
}

fn normalized_lerp_quaternion(start: [f32; 4], mut end: [f32; 4], fraction: f32) -> [f32; 4] {
    if start
        .iter()
        .zip(end)
        .map(|(left, right)| left * right)
        .sum::<f32>()
        < 0.0
    {
        end = end.map(|component| -component);
    }
    let interpolated = lerp_array(start, end, fraction);
    let length = interpolated
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if length <= f32::EPSILON {
        Transform::IDENTITY.rotation
    } else {
        interpolated.map(|component| component / length)
    }
}

#[cfg(test)]
mod motion_history_tests {
    use super::*;

    fn at(time_millis: i64, x: f32) -> MotionSample {
        MotionSample {
            time_millis,
            transform: Transform {
                translation: [x, 0.0, 0.0],
                ..Transform::IDENTITY
            },
        }
    }

    fn yawing(yaw: f32) -> Option<PoseState> {
        Some(PoseState {
            view_angles_degrees: [0.0, yaw, 0.0],
            ..PoseState::default()
        })
    }

    /// A player running 10 units and turning 10 degrees a snapshot, every 33 ms.
    fn running(world: &mut World, id: EntityId, snapshots: i64) {
        for snapshot in 0..snapshots {
            world.upsert(
                id,
                EntityKind::Actor,
                at(snapshot * 33, snapshot as f32 * 10.0),
            );
            world.set_pose(id, yawing(snapshot as f32 * 10.0));
        }
    }

    #[test]
    fn a_time_before_the_second_newest_sample_still_interpolates() {
        let mut world = World::new(WorldId::new(1));
        let id = EntityId::new(1);
        running(&mut world, id, 4);
        let entity = world.entity(id).expect("entity");
        // Drawn 10 ms short of the second newest sample (66 ms), as the stock clock
        // often is when the newest (99 ms) arrives: between the first two, not held.
        let x = entity.sample(56).translation[0];
        assert!((x - 560.0 / 33.0).abs() < 1e-3, "{x}");
        let yaw = entity.sample_pose(56).expect("pose").view_angles_degrees[1];
        assert!((yaw - 560.0 / 33.0).abs() < 1e-3, "{yaw}");
        // Past the newest, the newest is held; before the oldest, the oldest.
        assert_eq!(entity.sample(150).translation[0], 30.0);
        assert_eq!(entity.sample(-5).translation[0], 0.0);
    }

    #[test]
    fn a_teleport_is_never_glided_into_once_newer_samples_arrive() {
        let mut world = World::new(WorldId::new(1));
        let id = EntityId::new(1);
        world.upsert(id, EntityKind::Actor, at(0, 0.0));
        world.upsert_discontinuous(id, EntityKind::Actor, at(33, 1_000.0));
        world.upsert(id, EntityKind::Actor, at(66, 1_010.0));
        let entity = world.entity(id).expect("entity");
        assert_eq!(entity.sample(20).translation[0], 0.0);
        assert_eq!(entity.sample(33).translation[0], 1_000.0);
        assert!((entity.sample(50).translation[0] - (1_000.0 + 170.0 / 33.0)).abs() < 1e-3);
        assert_eq!(
            entity.sampled_motion_revision(20),
            entity.motion_revision().wrapping_sub(1)
        );
    }

    #[test]
    fn the_newest_pose_stands_until_its_sample_gets_its_own() {
        let mut world = World::new(WorldId::new(1));
        let id = EntityId::new(1);
        running(&mut world, id, 2);
        world.upsert(id, EntityKind::Actor, at(66, 20.0));
        let entity = world.entity(id).expect("entity");
        assert_eq!(entity.pose(), yawing(10.0));
        assert_eq!(entity.sample_pose(66), yawing(10.0));
    }
}
