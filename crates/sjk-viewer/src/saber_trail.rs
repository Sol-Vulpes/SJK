//! Fixed-capacity stock saber motion trails.

use crate::saber::Extension;
use crate::saber_rgb::BladeColor;
use bytemuck::{Pod, Zeroable};

const MAX_ENTITIES: usize = sjk_protocol::ENTITY_CAPACITY;
const SABERS_PER_ENTITY: usize = 2;
const BLADES_PER_SABER: usize = 8;
pub(crate) const MAX_SEGMENTS: usize = 4_096;
/// A vertex style's bit for a trail that bends what is behind it (`saber_trail.wgsl`
/// `fragment_wake`, a skin's Heat wake), beside the style in its low byte.
pub(crate) const WAKE: u32 = 0x100;
/// The vertex style's low byte: the style itself.
pub(crate) const STYLE: u32 = 0xff;
/// How many times longer a plasma, flare, moonlight (and its eclipse and starfall) or wake
/// trail lives than the blur: their cooling, dust and shimmer need time to be seen.
const LONG_LIFE: u16 = 3;
/// The ink trail's vertex style (`blade_extras::TrailKind::Ink`): the blur trail with a stroke
/// of ink over it, so it reaches as the blur trail does; and how many times the blur's life
/// its slices live, the ink drying over all of it (`saber_trail.wgsl`'s `INK_LIFE`, the same).
const INK: u32 = crate::blade_extras::TrailKind::Ink.style();
const INK_LIFE: u16 = 12;
/// The tip's speed (units a second) below which the ink brush lifts off the paper, ending its
/// stroke, and the speeds over which its pressure rises from none to full (a stroke starts
/// when the tip passes the first): the stroke thins as the swing slows, so it ends in a point.
const INK_LIFTED: f32 = 60.0;
const INK_PRESS: (f32, f32) = (120.0, 700.0);
/// How long (ms) the tip must stay under [`INK_LIFTED`] before the brush lifts, so a turn of
/// the swing does not break the stroke, and how fast (ms, its time constant) the brush's
/// pressure follows the tip's speed, so the line keeps an even width (Kushina, 20/11/2026).
const INK_LIFT_AFTER: f32 = 70.0;
const INK_SMOOTH: f32 = 40.0;
/// A curved slice ([`Quad::curved`]) is drawn as one piece for each this many degrees the
/// blade turned across it, at most [`MOST_PIECES`], so the stroke runs along an arc.
const PIECE_DEGREES: f32 = 3.0;
const MOST_PIECES: usize = 8;

/// The ink stroke at a slice's two edges ([`crate::blade_extras::TrailKind::Ink`]): how far the
/// stroke had come there (blade lengths since the brush was set down) and how hard the brush
/// pressed (0 to 1, from the tip's speed). The shader draws the stroke's start as a round drop
/// and its width from the pressure (`saber_trail.wgsl`'s `ink_line`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InkEdges {
    pub(crate) old: (f32, f32),
    pub(crate) new: (f32, f32),
}

/// A slice with no swing behind it (a still trail): the middle of a stroke, fully pressed.
const INK_STILL: InkEdges = InkEdges {
    old: (100.0, 1.0),
    new: (100.0, 1.0),
};
/// How far (units) the flare trail's flames reach past the tip.
pub(crate) const FLARE_REACH: f32 = 10.0;

#[path = "thrown_saber_tilt.rs"]
pub(crate) mod tilt;

#[path = "saber_trail_edge.rs"]
mod edge;
pub(crate) use edge::{Edge, Edges};

/// One stock trail vertex in the order passed to `FX_AddPrimitive`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
    pub(crate) style: u32,
    /// The flare's V at its slice's far edge, past the tip (below 0, see [`FLARE_REACH`]);
    /// 0 for the other styles.
    pub(crate) past: f32,
}

impl Vertex {
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Float32x4,
            3 => Uint32,
            4 => Float32
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

/// Four source vertices for one motion slice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Quad {
    pub(crate) vertices: [Vertex; 4],
    pub(crate) lifetime_millis: u16,
    /// Drawn along the arc the blade swept rather than as one straight piece (the ink,
    /// [`curve`]).
    pub(crate) curved: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Trail {
    base: [f32; 3],
    tip: [f32; 3],
    last_time: i64,
    initialized: bool,
    /// The ink brush ([`InkEdges`]): whether it is on the paper, how far its stroke has come
    /// (blade lengths) and how hard it pressed at the last edge.
    inked: bool,
    stroke: f32,
    press: f32,
    /// How long (ms) the tip has been under [`INK_LIFTED`] with the brush down.
    slow_for: f32,
}

impl Trail {
    /// Add a slice and then retain the current endpoints exactly like
    /// `CG_AddSaberBlade` (`codemp/cgame/cg_players.c:6297-6319,6400-6470,
    /// 6504-6507`). `edge` is this frame's muzzle and wall-clipped tip
    /// ([`Edges::edge`]). `cg_saberTrail 2` deliberately follows mode 1
    /// because the disabled stencil experiment is outside this renderer.
    pub(crate) fn update(
        &mut self,
        edge: Edge,
        now: i64,
        authored_duration: u16,
        style: u8,
        color: BladeColor,
        enabled: bool,
    ) -> Option<Quad> {
        if style > 1 || now <= self.last_time + 2 {
            return None;
        }
        let Edge { base, mut tip } = edge;
        // A new blade's own trail stops short of its point, where its body narrows: it drew a thin line of
        // the blade's colour past the tip in a swing (Creyon, 4/11/2026). The ink is the blur
        // trail's with its stroke, so it reaches as far.
        if vertex_style(style, color) & STYLE >= 10 && vertex_style(style, color) & STYLE != INK {
            for axis in 0..3 {
                tip[axis] = base[axis] + (tip[axis] - base[axis]) * 0.84;
            }
        }
        let diff = now.saturating_sub(self.last_time);
        let ink = self.ink_edges(base, tip, diff);
        let quad = (enabled && self.initialized && now < self.last_time + 2_000)
            .then(|| {
                build_quad(
                    base,
                    tip,
                    self.base,
                    self.tip,
                    diff,
                    authored_duration,
                    style,
                    color,
                    ink,
                )
            })
            .flatten();
        self.base = base;
        self.tip = tip;
        self.last_time = now;
        self.initialized = true;
        quad
    }

    /// The ink brush from the last edge to this one ([`InkEdges`]): set down when the tip
    /// reaches [`INK_PRESS`]'s first speed (the stroke starting at 0), pressing harder the
    /// faster it goes (smoothed, [`INK_SMOOTH`]), and lifted once it has stayed under
    /// [`INK_LIFTED`] for [`INK_LIFT_AFTER`] (the stroke running on to this edge with no
    /// pressure, so it ends in a point); off the paper both edges are 0.
    fn ink_edges(&mut self, base: [f32; 3], tip: [f32; 3], diff: i64) -> InkEdges {
        let new_tip = glam::Vec3::from_array(tip);
        let length = new_tip.distance(glam::Vec3::from_array(base)).max(1.0);
        let travelled = if self.initialized {
            new_tip.distance(glam::Vec3::from_array(self.tip))
        } else {
            0.0
        };
        let millis = diff.max(1) as f32;
        let speed = travelled / (millis * 0.001);
        let rise = ((speed - INK_PRESS.0) / (INK_PRESS.1 - INK_PRESS.0)).clamp(0.0, 1.0);
        let aimed = rise * rise * (3.0 - 2.0 * rise);
        let press = self.press + (aimed - self.press) * (1.0 - (-millis / INK_SMOOTH).exp());
        self.slow_for = if speed < INK_LIFTED {
            self.slow_for + millis
        } else {
            0.0
        };
        if self.inked && self.slow_for >= INK_LIFT_AFTER {
            let old = (self.stroke, self.press);
            let new = (self.stroke + travelled / length, 0.0);
            (self.inked, self.stroke, self.press, self.slow_for) = (false, 0.0, 0.0, 0.0);
            return InkEdges { old, new };
        }
        if !self.inked {
            if speed < INK_PRESS.0 {
                return InkEdges {
                    old: (0.0, 0.0),
                    new: (0.0, 0.0),
                };
            }
            (self.inked, self.stroke, self.press) = (true, 0.0, 0.0);
        }
        let old = (self.stroke, self.press);
        self.stroke += travelled / length;
        self.press = press;
        InkEdges {
            old,
            new: (self.stroke, press),
        }
    }

    /// Keep the edge of a blade that is out (`blade`, its length zero) without drawing,
    /// as `CG_AddSaberBlade` does with `dontDraw` for a staff's extra blade or a second
    /// saber switched off (`codemp/cgame/cg_players.c:14278-14293`, `8292-8556`): the
    /// edge stays where the hilt is, so the first slice after the blade lights again
    /// sweeps from there. Kept from where the blade went out instead, a swing within two
    /// seconds drew one slice from that old place to the new one (hub report 116). SJK
    /// does this for every blade, the primary too, which stock leaves stale.
    pub(crate) fn hold(&mut self, blade: crate::saber::Blade, now: i64) {
        if now <= self.last_time + 2 {
            return;
        }
        let Edge { base, tip } = Edge::from_end(blade, edge::blade_end(blade));
        self.base = base;
        self.tip = tip;
        self.last_time = now;
        self.initialized = true;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BladeState {
    pub(crate) extension: Extension,
    pub(crate) trail: Trail,
    /// Its afterimages' poses, for a skin leaving them (`saber_ghosts.rs`).
    pub(crate) ghosts: crate::saber_ghosts::GhostTrail,
    /// Its swing, ignition and clashes, for the Blade Lab designs (`saber_motion.rs`).
    pub(crate) motion: crate::saber_motion::BladeMotion,
    /// Where its tip was, for a skin shedding blossoms (`saber_petals.rs`).
    pub(crate) petals: crate::saber_petals::PetalEmitter,
    /// Whether it was last drawn thrown ([`Self::fly`]).
    flying: bool,
    /// When it last lit, for its emitter stars' surge (`flash.ignite`).
    pub(crate) ignition: crate::saber::Ignition,
    /// Its emitter's smoke, for a skin with some (`saber_smoke.rs`).
    pub(crate) smoke: crate::saber_smoke::Emitter,
}

impl BladeState {
    /// Note whether the blade is drawn thrown (`flying`) or in the hand. Its afterimages
    /// and blossoms are kept in its wearer's frame in the hand and in the world in flight
    /// (`thrown_saber.rs`), so going from one to the other forgets them: a pose kept in one
    /// frame drawn in the other would stand far off, and the tip's jump would shed a burst.
    pub(crate) fn fly(&mut self, flying: bool) {
        if self.flying != flying {
            self.ghosts = Default::default();
            self.petals = Default::default();
            self.flying = flying;
        }
    }

    /// Stow a staff blade immediately; its next ignition still grows from zero.
    pub(crate) fn stow(&mut self, time: i64) {
        *self = Self::default();
        self.extension.update(false, 0.0, time);
    }
}

/// O(1) per-entity/per-saber/per-blade state with one load-time allocation,
/// plus each owner's thrown-saber tilt.
pub(crate) struct StateSlab {
    states: Box<[BladeState]>,
    tilts: Box<[tilt::ThrowTilt]>,
    /// The blossoms the blades shed this frame, for `saber_petals::frame` to spawn.
    petals: Vec<crate::saber_petals::PetalSpawn>,
}

impl Default for StateSlab {
    fn default() -> Self {
        Self {
            states: vec![
                BladeState::default();
                MAX_ENTITIES * SABERS_PER_ENTITY * BLADES_PER_SABER
            ]
            .into_boxed_slice(),
            tilts: vec![tilt::ThrowTilt::default(); MAX_ENTITIES].into_boxed_slice(),
            petals: Vec::with_capacity(crate::saber_petals::MOST_QUEUED),
        }
    }
}

impl StateSlab {
    pub(crate) fn blade_mut(
        &mut self,
        entity: u64,
        saber: usize,
        blade: usize,
    ) -> Option<&mut BladeState> {
        let entity = usize::try_from(entity.checked_sub(1)?).ok()?;
        let index = entity
            .checked_mul(SABERS_PER_ENTITY * BLADES_PER_SABER)?
            .checked_add(saber.checked_mul(BLADES_PER_SABER)?)?
            .checked_add(blade)?;
        self.states.get_mut(index)
    }

    /// A blade's state and the queue of blossoms it may shed into.
    pub(crate) fn blade_and_petals(
        &mut self,
        entity: u64,
        saber: usize,
        blade: usize,
    ) -> Option<(&mut BladeState, &mut Vec<crate::saber_petals::PetalSpawn>)> {
        let entity = usize::try_from(entity.checked_sub(1)?).ok()?;
        let index = entity
            .checked_mul(SABERS_PER_ENTITY * BLADES_PER_SABER)?
            .checked_add(saber.checked_mul(BLADES_PER_SABER)?)?
            .checked_add(blade)?;
        Some((self.states.get_mut(index)?, &mut self.petals))
    }

    /// The blossoms queued since the last frame.
    pub(crate) fn petals(&mut self) -> &mut Vec<crate::saber_petals::PetalSpawn> {
        &mut self.petals
    }

    /// The thrown-saber tilt kept on the owner's entity (`centity_t::bolt3`).
    pub(crate) fn tilt_mut(&mut self, owner_entity: u64) -> Option<&mut tilt::ThrowTilt> {
        self.tilts
            .get_mut(usize::try_from(owner_entity.checked_sub(1)?).ok()?)
    }
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    quad: Quad,
    spawned_at: i64,
    active: bool,
}

impl Default for Segment {
    fn default() -> Self {
        Self {
            quad: Quad {
                vertices: [Vertex::zeroed(); 4],
                lifetime_millis: 0,
                curved: false,
            },
            spawned_at: 0,
            active: false,
        }
    }
}

/// Bounded persistent trail slices; exhausted pools drop new slices. Beside them, this
/// frame's still trails ([`still_trail`]), drawn once and dropped.
pub(crate) struct SegmentPool {
    segments: Box<[Segment]>,
    cursor: usize,
    dropped: u64,
    inserted: u64,
    still: Vec<Vertex>,
}

impl Default for SegmentPool {
    fn default() -> Self {
        Self {
            segments: vec![Segment::default(); MAX_SEGMENTS].into_boxed_slice(),
            cursor: 0,
            dropped: 0,
            inserted: 0,
            still: Vec::new(),
        }
    }
}

impl SegmentPool {
    pub(crate) fn insert(&mut self, quad: Quad, now: i64) -> bool {
        for offset in 0..self.segments.len() {
            let index = (self.cursor + offset) % self.segments.len();
            let segment = &mut self.segments[index];
            let expired = now >= segment.spawned_at + i64::from(segment.quad.lifetime_millis);
            if !segment.active || expired {
                *segment = Segment {
                    quad,
                    spawned_at: now,
                    active: true,
                };
                self.cursor = (index + 1) % self.segments.len();
                self.inserted = self.inserted.saturating_add(1);
                return true;
            }
        }
        self.dropped = self.dropped.saturating_add(1);
        false
    }

    /// Draw `vertices` (a [`still_trail`]) with the slices this frame only.
    pub(crate) fn add_still(&mut self, vertices: impl IntoIterator<Item = Vertex>) {
        self.still.extend(vertices);
    }

    pub(crate) fn append_vertices(&mut self, now: i64, output: &mut Vec<Vertex>) -> usize {
        let start = output.len();
        for segment in &mut self.segments {
            if !segment.active {
                continue;
            }
            let age = now.saturating_sub(segment.spawned_at);
            if age >= i64::from(segment.quad.lifetime_millis) {
                segment.active = false;
                continue;
            }
            // The stock trail never fades its colour: `CTrail::Update`
            // (`codemp/cgame/FxPrimitives.cpp:1771-1789`) scrolls U from `ST`
            // toward `destST = ST + 1` (clamped at 1) over the lifetime, so the
            // clamp-mapped glow texture's dark edge sweeps across the slice.
            // The shader derives that scroll from `1 - color.a`.
            // The GPU buffer holds `MAX_SEGMENTS * 6` vertices. Curved ink slices take
            // several quads each, so a crowd of them can fill it: past that, skip the
            // slice this frame (it stays alive) instead of writing past the buffer.
            if output.len() + 6 > MAX_SEGMENTS * 6 {
                continue;
            }
            let fade = 1.0 - age as f32 / f32::from(segment.quad.lifetime_millis);
            let vertices = segment.quad.vertices.map(|mut vertex| {
                vertex.color[3] = fade;
                vertex
            });
            if segment.quad.curved {
                let room = (MAX_SEGMENTS * 6).saturating_sub(output.len()) / 6;
                curve(vertices, room, output);
            } else {
                output.extend(triangles(vertices));
            }
        }
        // The still trails after, in what room the slices leave.
        let room = (MAX_SEGMENTS * 6).saturating_sub(output.len()) / 6 * 6;
        output.extend(self.still.drain(..).take(room));
        self.still.clear();
        (output.len() - start) / 6
    }
}

/// `CTrail::Draw` (`codemp/client/FxPrimitives.cpp`) splits the slice along
/// new tip to old muzzle: (new muzzle, new tip, old muzzle) and (old muzzle,
/// old tip, new tip). The texture coordinates are interpolated per triangle,
/// so the other diagonal smears the fade differently across a slice whose
/// tip moves much further than its muzzle, which is every swing.
/// A slice drawn along the arc the blade swept (`vertices` as [`triangles`] takes them): in
/// pieces of [`PIECE_DEGREES`] it turned (at most [`MOST_PIECES`], and `room`), the blade
/// between its two edges turned about its muzzle (its direction rotated, its muzzle and
/// length eased across) rather than its tip cutting straight across, so a fast swing's stroke
/// is a smooth arc, not a chain of straight chords. The rest (texture, colour, the ink's brush)
/// eases across the pieces as across the slice.
fn curve(vertices: [Vertex; 4], room: usize, output: &mut Vec<Vertex>) {
    use glam::Vec3;
    let [new_base, new_tip, old_tip, old_base] = vertices;
    let at = |vertex: Vertex| Vec3::from_array(vertex.position);
    let (new_way, old_way) = (at(new_tip) - at(new_base), at(old_tip) - at(old_base));
    let (new_length, old_length) = (new_way.length(), old_way.length());
    let (new_dir, old_dir) = (new_way.normalize_or_zero(), old_way.normalize_or_zero());
    let turned = old_dir.dot(new_dir).clamp(-1.0, 1.0).acos().to_degrees();
    let pieces = ((turned / PIECE_DEGREES).ceil() as usize)
        .clamp(1, MOST_PIECES)
        .min(room);
    if pieces <= 1 || new_length < 0.001 || old_length < 0.001 || turned > 170.0 {
        output.extend(triangles(vertices));
        return;
    }
    let turn = glam::Quat::from_rotation_arc(old_dir, new_dir);
    let ease = |a: Vertex, b: Vertex, t: f32| Vertex {
        position: a.position,
        uv: [
            a.uv[0] + (b.uv[0] - a.uv[0]) * t,
            a.uv[1] + (b.uv[1] - a.uv[1]) * t,
        ],
        color: std::array::from_fn(|i| a.color[i] + (b.color[i] - a.color[i]) * t),
        style: a.style,
        past: a.past + (b.past - a.past) * t,
    };
    // The blade's edge a share `t` of the way from the old edge to the new.
    let edge = |t: f32| {
        let base = at(old_base).lerp(at(new_base), t);
        let dir = glam::Quat::IDENTITY.slerp(turn, t) * old_dir;
        let tip = base + dir * (old_length + (new_length - old_length) * t);
        let mut muzzle = ease(old_base, new_base, t);
        muzzle.position = base.to_array();
        let mut point = ease(old_tip, new_tip, t);
        point.position = tip.to_array();
        (muzzle, point)
    };
    for piece in 0..pieces {
        let (older, newer) = (
            piece as f32 / pieces as f32,
            (piece + 1) as f32 / pieces as f32,
        );
        let ((old_muzzle, old_point), (new_muzzle, new_point)) = (edge(older), edge(newer));
        output.extend(triangles([new_muzzle, new_point, old_point, old_muzzle]));
    }
}

fn triangles(vertices: [Vertex; 4]) -> [Vertex; 6] {
    let [new_muzzle, new_tip, old_tip, old_muzzle] = vertices;
    [
        new_muzzle, new_tip, old_muzzle, old_muzzle, old_tip, new_tip,
    ]
}

/// The trail a blade swinging evenly leaves, held still: `edges` from where the blade is
/// back to the oldest, each slice between two drawn as [`Trail::update`] would have
/// drawn it that much earlier (its age an even share of the trail's life, so the glow's
/// dark edge sweeps across them as it does across a real trail), in `style` and `color`.
pub(crate) fn still_trail(
    edges: &[Edge],
    authored_duration: u16,
    style: u8,
    color: BladeColor,
) -> impl Iterator<Item = Vertex> + '_ {
    let slices = edges.len().saturating_sub(1).max(1);
    let lifetime = i64::from(trail_duration(authored_duration));
    let diff = (lifetime / slices as i64).max(1);
    edges.windows(2).enumerate().flat_map(move |(age, pair)| {
        let quad = build_quad(
            pair[0].base,
            pair[0].tip,
            pair[1].base,
            pair[1].tip,
            diff,
            authored_duration,
            style,
            color,
            INK_STILL,
        );
        let fade = 1.0 - age as f32 / slices as f32;
        quad.map(|quad| {
            triangles(quad.vertices.map(|mut vertex| {
                vertex.color[3] = fade;
                vertex
            }))
        })
        .into_iter()
        .flatten()
    })
}

pub(crate) fn trail_duration(authored_duration: u16) -> u16 {
    let duration = authored_duration / 5;
    if duration == 0 { 40 } else { duration }
}

/// The vertex style a slice is drawn with: the hilt's stock `trailStyle` (0 the blur
/// trail, 1 the sword trail), except a black blade's blur trail (2, alpha-blended) and a
/// blade skin's (`SkinColor::trail_kind`: 3, the blur trail whose white core takes the
/// skin's trail colour too, so the trail reads as the blade's colour rather than white; 4
/// comic speed lines; 5 plasma; 6 the flare; 7 moonlight; 8 the eclipse; 9 the starfall), with [`WAKE`] when the skin's trail bends
/// what is behind it.
pub(crate) fn vertex_style(style: u8, color: BladeColor) -> u32 {
    match (style, color) {
        (1, _) => 1,
        (_, BladeColor::Black) => 2,
        (0, BladeColor::Skin(skin)) => skin.trail_kind.style() | if skin.wake { WAKE } else { 0 },
        _ => u32::from(style),
    }
}

fn build_quad(
    new_base: [f32; 3],
    new_tip: [f32; 3],
    old_base: [f32; 3],
    old_tip: [f32; 3],
    diff: i64,
    authored_duration: u16,
    style: u8,
    color: BladeColor,
    ink: InkEdges,
) -> Option<Quad> {
    if diff > 10_000 {
        return None;
    }
    let vertex_style = vertex_style(style, color);
    let kind = vertex_style & STYLE;
    // The plasma and flare, and a wake, live longer (`LONG_LIFE`); the ink longer (`INK_LIFE`)
    // than the blur trail it is drawn over, whose blur the shader draws over the
    // blur's own life, so the blur is as it was.
    let lifetime_millis = if kind == INK {
        trail_duration(authored_duration).saturating_mul(INK_LIFE)
    } else if kind >= 5 || vertex_style & WAKE != 0 {
        trail_duration(authored_duration).saturating_mul(LONG_LIFE)
    } else {
        trail_duration(authored_duration)
    };
    let old_alpha = 1.0 - diff as f32 / f32::from(lifetime_millis);
    let rgb = if style == 1 {
        [32.0 / 255.0; 3]
    } else {
        match color {
            // Rakata Infinite's surge (`rakata_infinite.glsl`, `gk_surge`) turns its trail too.
            BladeColor::Skin(skin)
                if skin.trail_kind == crate::blade_extras::TrailKind::Infinite =>
            {
                skin.trail_color_surging(crate::saber::animation_seconds())
            }
            _ => color.trail_rgb(),
        }
    };
    // The flare's flames lick off past the tip: its slice reaches `FLARE_REACH` further,
    // the texture's V going below 0 there (the tip stays at 0).
    let (new_tip, old_tip, tip_v) = if kind == 6 {
        let past = |base: [f32; 3], tip: [f32; 3]| {
            let base = glam::Vec3::from_array(base);
            let tip = glam::Vec3::from_array(tip);
            (tip + (tip - base).normalize_or_zero() * FLARE_REACH).to_array()
        };
        let length = glam::Vec3::from_array(new_tip).distance(glam::Vec3::from_array(new_base));
        (
            past(new_base, new_tip),
            past(old_base, old_tip),
            -FLARE_REACH / length.max(1.0),
        )
    } else {
        (new_tip, old_tip, 0.0)
    };
    // The ink's brush rides on its vertices: how far its stroke had come in `past`, and how
    // hard it pressed in the colour's brightness (its hue, all the blur reads, unchanged).
    let vertex = |position, uv, (stroke, press): (f32, f32)| {
        let (rgb, past) = if kind == INK {
            let top = rgb[0].max(rgb[1]).max(rgb[2]).max(0.001);
            let scale = (0.1 + 0.9 * press) / top;
            ([rgb[0] * scale, rgb[1] * scale, rgb[2] * scale], stroke)
        } else {
            (rgb, tip_v)
        };
        Vertex {
            position,
            uv,
            color: [rgb[0], rgb[1], rgb[2], 1.0],
            style: vertex_style,
            past,
        }
    };
    Some(Quad {
        vertices: [
            vertex(new_base, [0.0, 1.0], ink.new),
            vertex(new_tip, [0.0, tip_v], ink.new),
            vertex(old_tip, [1.0 - old_alpha, tip_v], ink.old),
            vertex(old_base, [1.0 - old_alpha, 1.0], ink.old),
        ],
        lifetime_millis: if style == 1 {
            lifetime_millis.saturating_mul(2)
        } else {
            lifetime_millis
        },
        curved: kind == INK,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saber::Color;

    #[test]
    fn the_ink_brush_is_set_down_presses_and_lifts_with_the_tip() {
        let mut trail = Trail::default();
        let at = |x: f32| Edge {
            base: [x, 0.0, 0.0],
            tip: [x, 0.0, 40.0],
        };
        // Still, then a swing (10 units a 10 ms: 1000 a second), then a stop long enough
        // to lift the brush (eight 10 ms frames, past `INK_LIFT_AFTER`).
        let mut edges = Vec::new();
        let path = [
            0.0, 0.0, 0.0, 10.0, 20.0, 30.0, 30.2, 30.2, 30.2, 30.2, 30.2, 30.2, 30.2, 30.2,
        ];
        for (step, x) in path.into_iter().enumerate() {
            let edge = at(x);
            let ink = trail.ink_edges(edge.base, edge.tip, 10);
            trail.tip = edge.tip;
            trail.base = edge.base;
            trail.initialized = true;
            trail.last_time = step as i64 * 10;
            edges.push(ink);
        }
        // Off the paper while still.
        assert_eq!(edges[1].new, (0.0, 0.0));
        // Set down: the stroke starts at 0 and runs a quarter of the blade a slice, its
        // pressure rising smoothly.
        assert_eq!(edges[3].old, (0.0, 0.0));
        assert!((edges[3].new.0 - 0.25).abs() < 1e-4);
        assert!(edges[3].new.1 > 0.1 && edges[4].new.1 > edges[3].new.1);
        assert!(edges[5].new.1 > edges[4].new.1);
        assert!((edges[5].new.0 - 0.75).abs() < 1e-4);
        // Slowing, still on the paper: the line thins but does not break off at once.
        assert!(edges[6].new.1 > 0.0 && edges[6].new.1 < edges[5].new.1);
        let lifted = edges
            .iter()
            .position(|ink| ink.new == (ink.old.0, 0.0) && ink.old.1 > 0.0);
        let lifted = lifted.expect("the brush lifts once the tip stays slow");
        assert!(lifted >= 12, "lifted at {lifted}");
        // Lifted: the stroke ends with no pressure, and the next starts afresh.
        assert!(!trail.inked && trail.stroke == 0.0);
    }

    #[test]
    fn a_curved_slice_follows_the_arc_the_tip_swept() {
        let vertex = |position: [f32; 3], u: f32| Vertex {
            position,
            uv: [u, 0.0],
            color: [1.0; 4],
            style: INK,
            past: 0.0,
        };
        // A 40-unit blade turned 30 degrees about its muzzle.
        let (angle, length) = (30f32.to_radians(), 40.0);
        let vertices = [
            vertex([0.0; 3], 0.0),
            vertex([length * angle.sin(), 0.0, length * angle.cos()], 0.0),
            vertex([0.0, 0.0, length], 0.5),
            vertex([0.0; 3], 0.5),
        ];
        let mut output = Vec::new();
        curve(vertices, 100, &mut output);
        assert_eq!(output.len(), 6 * MOST_PIECES);
        for vertex in &output {
            let distance = glam::Vec3::from_array(vertex.position).length();
            assert!(
                distance < 0.01 || (distance - length).abs() < 0.01,
                "{distance}"
            );
        }
        let mut cramped = Vec::new();
        curve(vertices, 1, &mut cramped);
        assert_eq!(cramped.len(), 6);
    }

    #[test]
    fn the_ink_lives_as_long_in_the_shader_as_here() {
        let wgsl = format!("const INK_LIFE: f32 = {}.0;", INK_LIFE);
        assert!(include_str!("saber_trail.wgsl").contains(&wgsl));
    }

    fn edge(x: f32) -> Edge {
        Edge {
            base: [x, 0.0, 0.0],
            tip: [x, 0.0, 44.0],
        }
    }

    fn color() -> BladeColor {
        BladeColor::Retail(Color::Yellow)
    }

    #[test]
    fn black_blades_select_alpha_trails_without_changing_slice_timing() {
        let make = |color, style| {
            build_quad(
                [0.; 3],
                [0., 0., 40.],
                [1., 0., 0.],
                [1., 0., 40.],
                8,
                200,
                style,
                color,
                INK_STILL,
            )
            .unwrap()
        };
        let black = make(BladeColor::Black, 0);
        let ordinary = make(color(), 0);
        assert_eq!(black.lifetime_millis, ordinary.lifetime_millis);
        assert_eq!(
            black.vertices.map(|v| v.position),
            ordinary.vertices.map(|v| v.position)
        );
        assert!(black.vertices.iter().all(|v| v.style == 2));
        assert!(ordinary.vertices.iter().all(|v| v.style == 0));
        assert!(
            make(BladeColor::Black, 1)
                .vertices
                .iter()
                .all(|v| v.style == 1)
        );
    }

    #[test]
    fn slice_spans_the_frame_in_texture_space() {
        let mut trail = Trail::default();
        assert!(
            trail
                .update(edge(0.0), 1_000, 200, 0, color(), true)
                .is_none()
        );
        // Within 2 ms nothing moves, as in stock.
        assert!(
            trail
                .update(edge(1.0), 1_002, 200, 0, color(), true)
                .is_none()
        );
        let quad = trail
            .update(edge(2.0), 1_008, 200, 0, color(), true)
            .expect("a fresh slice");
        assert_eq!(quad.lifetime_millis, 40);
        let [new_muzzle, new_tip, old_tip, old_muzzle] = quad.vertices;
        assert_eq!(new_muzzle.position, [2.0, 0.0, 0.0]);
        assert_eq!(new_tip.position, [2.0, 0.0, 44.0]);
        assert_eq!(old_tip.position, [0.0, 0.0, 44.0]);
        assert_eq!(old_muzzle.position, [0.0, 0.0, 0.0]);
        // ST[0] of the old edge is diff / trailDur = 8 / 40.
        assert!((old_tip.uv[0] - 0.2).abs() < 1e-6);
        assert_eq!(new_muzzle.uv, [0.0, 1.0]);
        assert_eq!(new_tip.uv, [0.0, 0.0]);
    }

    #[test]
    fn idle_moves_keep_the_edge_without_drawing() {
        let mut trail = Trail::default();
        assert!(
            trail
                .update(edge(0.0), 1_000, 0, 0, color(), false)
                .is_none()
        );
        assert!(
            trail
                .update(edge(1.0), 1_010, 0, 0, color(), false)
                .is_none()
        );
        let quad = trail
            .update(edge(2.0), 1_020, 150, 0, color(), true)
            .expect("the first attack frame bridges from the remembered edge");
        assert_eq!(quad.vertices[3].position, [1.0, 0.0, 0.0]);
        assert_eq!(quad.lifetime_millis, 30);
    }

    #[test]
    fn a_blade_lit_again_sweeps_from_its_hilt_not_from_where_it_went_out() {
        let mut trail = Trail::default();
        trail.update(edge(0.0), 1_000, 200, 0, color(), true);
        // Out, while the wearer runs on: the edge follows the hilt.
        for (step, at) in [(1_100, 50.0), (1_200, 100.0)] {
            trail.hold(
                crate::saber::Blade {
                    base: [at, 0.0, 0.0],
                    direction: [0.0, 0.0, 1.0],
                    length: 0.0,
                    radius: 1.0,
                },
                step,
            );
        }
        let quad = trail
            .update(edge(101.0), 1_210, 200, 0, color(), true)
            .expect("a fresh slice");
        let [_, _, old_tip, old_muzzle] = quad.vertices.map(|v| v.position);
        assert_eq!(old_muzzle, [100.0, 0.0, 0.0]);
        assert_eq!(old_tip, [100.0, 0.0, 4.0]);
    }

    #[test]
    fn slices_split_along_new_tip_to_old_muzzle() {
        let mut trail = Trail::default();
        trail.update(edge(0.0), 1_000, 200, 0, color(), true);
        let quad = trail
            .update(edge(2.0), 1_008, 200, 0, color(), true)
            .expect("a fresh slice");
        let mut pool = SegmentPool::default();
        assert!(pool.insert(quad, 1_008));
        let mut output = Vec::new();
        assert_eq!(pool.append_vertices(1_008, &mut output), 1);
        let positions: Vec<_> = output.iter().map(|vertex| vertex.position).collect();
        let [new_muzzle, new_tip, old_tip, old_muzzle] = quad.vertices.map(|v| v.position);
        assert_eq!(
            positions,
            [
                new_muzzle, new_tip, old_muzzle, old_muzzle, old_tip, new_tip
            ]
        );
    }

    #[test]
    fn slices_expire_after_their_lifetime() {
        let mut trail = Trail::default();
        trail.update(edge(0.0), 1_000, 200, 0, color(), true);
        let quad = trail
            .update(edge(2.0), 1_008, 200, 0, color(), true)
            .expect("a fresh slice");
        let mut pool = SegmentPool::default();
        pool.insert(quad, 1_008);
        let mut output = Vec::new();
        assert_eq!(pool.append_vertices(1_047, &mut output), 1);
        output.clear();
        assert_eq!(pool.append_vertices(1_048, &mut output), 0);
    }

    #[test]
    fn a_full_pool_of_curved_slices_fits_the_gpu_buffer() {
        let mut trail = Trail::default();
        trail.update(edge(0.0), 1_000, 200, 0, color(), true);
        let mut quad = trail
            .update(edge(2.0), 1_008, 200, 0, color(), true)
            .expect("a fresh slice");
        // A quarter turn about the muzzle: the most pieces `curve` cuts a slice into.
        quad.curved = true;
        quad.vertices[0].position = [0.0, 0.0, 0.0];
        quad.vertices[1].position = [4.0, 0.0, 0.0];
        quad.vertices[2].position = [0.0, 0.0, 4.0];
        quad.vertices[3].position = [0.0, 0.0, 0.0];
        let mut pool = SegmentPool::default();
        for _ in 0..MAX_SEGMENTS {
            assert!(pool.insert(quad, 1_008));
        }
        pool.add_still(std::iter::repeat_n(quad.vertices[0], 60));
        let mut output = Vec::new();
        assert_eq!(pool.append_vertices(1_008, &mut output), MAX_SEGMENTS);
        assert_eq!(output.len(), MAX_SEGMENTS * 6);
    }
}
