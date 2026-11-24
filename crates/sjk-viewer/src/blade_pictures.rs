//! The Collection's blade pictures (Creyon's round 13: its list showed every blade as a
//! drawing that did not look like the blade): each blade skin a list row or card shows is
//! drawn by the game's own blade renderer, live, as it looks in the hand, and the swatch
//! shows that picture.
//!
//! A swatch asks for its skin's picture ([`picture`]) while the UI is built; the frame then
//! draws every skin asked for into a cell of one picture ([`SLOTS`] cells, kept for a skin
//! while it is still asked for), all in one pass with one camera: each blade lies across
//! its cell side on, in the world, at its own place, so their shared pipelines (and a
//! signature's own, once built) draw them as any blade. The blades go into an 8-bit
//! picture in the effect layer's display values, which the UI adds over the swatch's dark
//! glass ([`crate::ui_renderer::BLADE_PICTURES_TEXTURE`]). A swatch whose skin has no cell
//! yet (the first frame, or more swatches than cells) draws the old drawing.
//!
//! A frame draws only the cells that need it (25/10/2026, for weak PCs: fifteen signature
//! blades every frame cost a weak GPU dearly): a new cell, a cell a swatch the player
//! looks at asked for ([`focus`]: the hovered or chosen row or card, a large swatch),
//! every cell while no more than [`FULL_RATE_CELLS`] are in use, and the others
//! [`CALM_HZ`] times a second, each cell at its own moment so that the frames share them,
//! and at most [`CALM_PER_FRAME`] of them a frame, the longest waiting first: a GPU too
//! slow for that rate draws a bounded number a frame instead of every one (an integrated
//! Radeon took about 10 ms a picture, 25/10/2026). The UI over the pictures is still
//! drawn every frame; only the blades' animation in the cells drawn calmly steps (the
//! Shaders grid's fourteen at 60 FPS, about 26 times a second). A cell drawn alone is
//! cleared inside its rectangle, and the cells kept are shielded by depth, so a blade's
//! glow never adds into a picture kept from an earlier frame (`blade_pictures.wgsl`).
//!
//! With the menu pictures on (`ui_menuPictures`, a card short of video memory) no swatch
//! asks ([`set_live`]): every swatch shows the drawing, and the picture's target is
//! freed, or never made.
//!
//! The menus' light path for weak PCs ([`light_menus`], 24/11/2026, hub #328: "collection
//! menu drops me into single digit fps"): the same as with the menu pictures on, for the
//! rest of the session, once the Collection was measured slow ([`note_frame_rate`]: under
//! [`SLOW_FPS`] for [`SLOW_FOR`] while its kyber pictures show, the frame cap allowing
//! more). The kybers then show their first picture, still, baked one a frame and only those
//! on screen ([`crate::blade_stills`]), no blade is drawn live, and the acrylic glass draws
//! its plain tint (`GpuState::frame_overlays`). No setting changes: a faster PC never
//! falls under the mark, and a weak one finds its loops again at the next start if it
//! keeps up then.

use crate::camera_uniform::CameraUniform;
use glam::Vec3;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Times a second a picture no one looks at is drawn again.
const CALM_HZ: f64 = 30.0;
/// Cells in use up to which every one is drawn every frame (a card, a player's card, the
/// Saber tab with few shaders: few pictures cost little).
const FULL_RATE_CELLS: usize = 2;
/// Calm cells (neither new nor focused) drawn a frame at most.
const CALM_PER_FRAME: usize = 6;
/// How long the picture's target is kept with no picture asked for.
const TARGET_IDLE: std::time::Duration = std::time::Duration::from_secs(5);

/// Cells of the picture: two columns of eight.
const COLUMNS: usize = 2;
const ROWS: usize = 8;
pub(crate) const SLOTS: usize = COLUMNS * ROWS;
/// Blade instances one cell draws at most (glow and core, glints, an echo).
const PER_SLOT: usize = 12;
/// Blade instances the pictures and a frame's still bakes ([`crate::blade_stills`]) draw at
/// most (`saber_gpu.rs`).
pub(crate) const BLADES: usize = (SLOTS + crate::blade_stills::BAKE_BATCH) * PER_SLOT;
/// A cell in world units: the blade's 40 and room for its tip and effects, and above and
/// below it room for what a signature draws round it.
const CELL: [f32; 2] = [52.0, 26.0];
/// Where the blade starts in its cell and how long it is (a stock blade's 40 units).
pub(crate) const BLADE_START: f32 = 3.0;
pub(crate) const BLADE_LENGTH: f32 = 40.0;
const BLADE_RADIUS: f32 = 3.0;
/// Pixels a world unit.
const PIXELS: f32 = 8.0;
/// The picture's size in pixels.
pub(crate) const SIZE: [u32; 2] = [
    (CELL[0] * PIXELS) as u32 * COLUMNS as u32,
    (CELL[1] * PIXELS) as u32 * ROWS as u32,
];

/// A cell's skin and when its picture was drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Slot {
    skin: &'static str,
    /// Its picture has been drawn: swatches show it.
    drawn: bool,
    /// It was drawn with its own pipelines (a signature's built, or none needed), not the
    /// shared ones standing in for a build.
    own: bool,
    /// The calm moment ([`calm_tick`]) it was last drawn in.
    tick: u64,
}

/// Who asked for a picture this frame and which cell each has.
#[derive(Default)]
struct State {
    /// Skins asked for while this frame's UI was built (kept with its room, so asking
    /// allocates nothing once the screen has been drawn).
    wanted: Vec<&'static str>,
    /// Skins a swatch the player looks at asked for ([`focus`]).
    focused: Vec<&'static str>,
    slots: [Option<Slot>; SLOTS],
}

static STATE: Mutex<State> = Mutex::new(State {
    wanted: Vec::new(),
    focused: Vec::new(),
    slots: [None; SLOTS],
});

/// Whether swatches show live pictures: off while the menus stand on pictures.
static LIVE: AtomicBool = AtomicBool::new(true);
/// Set for the session once the Collection was measured slow ([`note_frame_rate`]).
static SLOW: AtomicBool = AtomicBool::new(false);

/// Frames a second under which the Collection counts as slow.
pub(crate) const SLOW_FPS: u32 = 20;
/// How long it must stay under [`SLOW_FPS`] before the menus take their light path.
pub(crate) const SLOW_FOR: std::time::Duration = std::time::Duration::from_secs(3);

/// Show live pictures (`true`) or the drawings (`false`, with the menu pictures on).
pub(crate) fn set_live(live: bool) {
    LIVE.store(live, Ordering::Relaxed);
}

/// Whether swatches may show live pictures: the menu pictures off and the menus not on
/// their light path ([`light_menus`]).
pub(crate) fn live() -> bool {
    LIVE.load(Ordering::Relaxed) && !SLOW.load(Ordering::Relaxed)
}

/// Whether the menus take their light path for a weak PC: the menu pictures on (chosen,
/// or set at start for a card of 4 GB or less, `video_memory.rs`) or the Collection
/// measured slow this session. The kybers show still pictures and the acrylic glass its
/// plain tint.
pub(crate) fn light_menus() -> bool {
    !live()
}

/// Whether frames at `fps` under a cap of `cap` (0 uncapped) count as slow: under
/// [`SLOW_FPS`], with a cap that allows at least twice that (an unfocused or minimized
/// window's low cap, or a player's own, is no sign of a weak PC).
pub(crate) fn is_slow(fps: u32, cap: u32) -> bool {
    fps < SLOW_FPS && (cap == 0 || cap >= SLOW_FPS * 2)
}

/// Since when the frames have been slow in a row.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SlowWatch {
    since: Option<std::time::Instant>,
}

impl SlowWatch {
    /// Note the frame rate `fps` measured `now` under a cap of `cap`: true once the
    /// frames have been slow ([`is_slow`]) for [`SLOW_FOR`] in a row.
    pub(crate) fn note(&mut self, fps: Option<u32>, cap: u32, now: std::time::Instant) -> bool {
        if !fps.is_some_and(|fps| is_slow(fps, cap)) {
            self.since = None;
            return false;
        }
        let started = *self.since.get_or_insert(now);
        now.duration_since(started) >= SLOW_FOR
    }
}

static WATCH: Mutex<SlowWatch> = Mutex::new(SlowWatch { since: None });

/// Note the frame rate measured `now` while the Collection shows kyber pictures: after
/// [`SLOW_FOR`] of slow frames the menus take their light path for the rest of the
/// session. True on the frame they do.
pub(crate) fn note_frame_rate(fps: Option<u32>, cap: u32, now: std::time::Instant) -> bool {
    if !live() {
        return false;
    }
    let slow = WATCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .note(fps, cap, now);
    if slow {
        SLOW.store(true, Ordering::Relaxed);
    }
    slow
}

/// Put the menus on their light path, or off it, as a slow Collection would (the world
/// shots that time it, `SJK_LIGHT_MENUS`).
#[cfg(test)]
pub(crate) fn set_slow_for_shot(slow: bool) {
    SLOW.store(slow, Ordering::Relaxed);
}

/// Whether a swatch the player looks at asked for skin `id` while this UI was built
/// ([`focus`]): it shows the live picture, the others their still.
pub(crate) fn focused(id: &str) -> bool {
    live() && lock().focused.contains(&id)
}

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The picture of skin `id` for a swatch: the texture coordinates `[u0, v0, u1, v1]` of
/// its cell, from its left edge ([`BLADE_START`] units before the blade) to its end, the cell's middle `shown` units
/// high (`shown` at most the cell's height), once drawn, and whether it was drawn with its
/// own pipelines. Asking keeps (or gets) it a cell.
pub(crate) fn picture(id: &'static str, shown: f32) -> Option<([f32; 4], bool)> {
    if !LIVE.load(Ordering::Relaxed) {
        return None;
    }
    let mut state = lock();
    if !state.wanted.contains(&id) {
        state.wanted.push(id);
    }
    let slot = state
        .slots
        .iter()
        .position(|slot| slot.is_some_and(|slot| slot.drawn && slot.skin == id))?;
    let own = state.slots[slot].is_some_and(|slot| slot.own);
    let (column, row) = (slot % COLUMNS, slot / COLUMNS);
    let width = CELL[0] * COLUMNS as f32;
    let height = CELL[1] * ROWS as f32;
    let shown = shown.clamp(1.0, CELL[1]);
    let top = row as f32 * CELL[1] + (CELL[1] - shown) * 0.5;
    Some((
        [
            column as f32 * CELL[0] / width,
            top / height,
            (column as f32 + 1.0) * CELL[0] / width,
            (top + shown) / height,
        ],
        own,
    ))
}

/// Draw skin `id`'s picture every frame: a swatch the player looks at (a hovered or
/// chosen row or card, a large swatch) asks so as it asks for its [`picture`]; the others
/// are drawn [`CALM_HZ`] times a second.
pub(crate) fn focus(id: &'static str) {
    if !LIVE.load(Ordering::Relaxed) {
        return;
    }
    let mut state = lock();
    if !state.focused.contains(&id) {
        state.focused.push(id);
    }
}

/// The calm moment of cell `slot` at `seconds`: it changes [`CALM_HZ`] times a second,
/// each cell's a share of a moment after the one before, so the cells drawn calmly
/// spread over the frames instead of all falling on one.
fn calm_tick(seconds: f64, slot: usize) -> u64 {
    (seconds.max(0.0) * CALM_HZ + slot as f64 / SLOTS as f64) as u64
}

/// What a frame draws.
#[derive(Debug, PartialEq)]
struct Plan {
    /// The skin of each cell drawn this frame.
    draw: [Option<&'static str>; SLOTS],
    /// Every cell in use is drawn: the whole picture is cleared and drawn again.
    whole: bool,
}

/// The cells for this frame and which are drawn (at `seconds` on the menu clock): the
/// skins asked for since the last frame keep their cells, the others free theirs, and
/// new ones take free cells (not drawn yet). A cell is drawn when it is not drawn yet,
/// focused, on a `fresh` target, with few cells in use, or when its calm moment changed.
/// `None` with no cell in use. Clears the wishes, keeping their room.
fn assign(state: &mut State, seconds: f64, fresh: bool) -> Option<Plan> {
    let State {
        wanted,
        focused,
        slots,
    } = state;
    for slot in slots.iter_mut() {
        if slot.is_some_and(|slot| !wanted.contains(&slot.skin)) {
            *slot = None;
        }
    }
    for &id in wanted.iter() {
        if slots.iter().flatten().any(|slot| slot.skin == id) {
            continue;
        }
        if let Some(free) = slots.iter_mut().find(|slot| slot.is_none()) {
            *free = Some(Slot {
                skin: id,
                drawn: false,
                own: false,
                tick: 0,
            });
        }
    }
    let in_use = slots.iter().flatten().count();
    let every = fresh || in_use <= FULL_RATE_CELLS;
    let mut plan = Plan {
        draw: [None; SLOTS],
        whole: true,
    };
    // The calm cells whose moment changed, the longest waiting first.
    let mut calm = [None; CALM_PER_FRAME];
    for (index, slot) in slots.iter_mut().enumerate() {
        let Some(slot) = slot else {
            continue;
        };
        let tick = calm_tick(seconds, index);
        if every || !slot.drawn || focused.contains(&slot.skin) {
            slot.tick = tick;
            plan.draw[index] = Some(slot.skin);
        } else if slot.tick != tick {
            offer(&mut calm, (tick.saturating_sub(slot.tick), index));
        }
    }
    for (_, index) in calm.into_iter().flatten() {
        if let Some(slot) = &mut slots[index] {
            slot.tick = calm_tick(seconds, index);
            plan.draw[index] = Some(slot.skin);
        }
    }
    plan.whole = slots
        .iter()
        .zip(&plan.draw)
        .all(|(slot, drawn)| slot.is_none() || drawn.is_some());
    wanted.clear();
    focused.clear();
    (in_use > 0).then_some(plan)
}

/// Keep `candidate` (moments waited, cell) among the calm cells `chosen` keeps, the
/// longest waiting first; on a tie the cell offered first stays ahead.
fn offer(chosen: &mut [Option<(u64, usize)>], mut candidate: (u64, usize)) {
    for place in chosen {
        match place {
            None => {
                *place = Some(candidate);
                return;
            }
            Some(kept) if candidate.0 > kept.0 => std::mem::swap(kept, &mut candidate),
            Some(_) => {}
        }
    }
}

/// The blade instances of a blade from `base` along x in `color` at `seconds` (`seed` its
/// animation's), at most `per_blade`: its glow and core, its extras, their effects' frame as
/// on a wearer seen from behind (`saber_submission.rs`), so what spreads to the blade's
/// sides spreads up and down the picture.
pub(crate) fn blade_instances(
    base: Vec3,
    color: crate::saber_rgb::BladeColor,
    seconds: f64,
    seed: u32,
    per_blade: usize,
) -> impl Iterator<Item = crate::saber::Instance> {
    let blade = crate::saber::Blade {
        base: base.to_array(),
        direction: [1.0, 0.0, 0.0],
        length: BLADE_LENGTH,
        radius: BLADE_RADIUS,
    };
    crate::saber::Instance::pair(blade, color)
        .map(|instance| instance.with_animation(seconds, seed))
        .into_iter()
        .chain(crate::saber::skin_extras(blade, color, seconds, seed))
        .map(|instance| instance.with_frame([0.0, 1.0, 0.0], BLADE_LENGTH))
        .take(per_blade)
}

/// Cell `slot`'s rectangle in the picture, in pixels: `[x, y, width, height]`.
fn cell_pixels(slot: usize) -> [u32; 4] {
    let (width, height) = (SIZE[0] / COLUMNS as u32, SIZE[1] / ROWS as u32);
    [
        (slot % COLUMNS) as u32 * width,
        (slot / COLUMNS) as u32 * height,
        width,
        height,
    ]
}

/// Where the blade of cell `slot` starts, in world units (the picture's plane is y = 0,
/// x across, z up).
fn blade_base(slot: usize) -> Vec3 {
    let (column, row) = (slot % COLUMNS, slot / COLUMNS);
    Vec3::new(
        column as f32 * CELL[0] + BLADE_START,
        0.0,
        -(row as f32 + 0.5) * CELL[1],
    )
}

/// The camera seeing every cell side on: x from 0 to the picture's width, z from 0 down.
fn view_projection() -> glam::Mat4 {
    let width = CELL[0] * COLUMNS as f32;
    let height = CELL[1] * ROWS as f32;
    glam::camera::rh::proj::directx::orthographic(0.0, width, -height, 0.0, 1.0, 400.0)
        * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -200.0, 0.0), Vec3::ZERO, Vec3::Z)
}

/// The picture's textures and camera on the GPU.
pub(crate) struct Target {
    blades: wgpu::TextureView,
    depth: wgpu::TextureView,
    /// The blades faded toward each cell's edges (`blade_pictures.wgsl`): what the UI shows.
    display: wgpu::TextureView,
    camera: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    soften: wgpu::RenderPipeline,
    soften_bind: wgpu::BindGroup,
    /// Clears a cell drawn alone, its depth to the far plane (`blade_pictures.wgsl`).
    clear: wgpu::RenderPipeline,
}

impl Target {
    fn new(device: &wgpu::Device, camera_layout: &wgpu::BindGroupLayout) -> Self {
        let texture = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: SIZE[0],
                        height: SIZE[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let blades = texture(
            "SJK blade pictures",
            crate::frame_target::aa::effects::FORMAT,
        );
        let depth = texture(
            "SJK blade pictures depth",
            crate::depth_target::DepthTarget::FORMAT,
        );
        let display = texture(
            "SJK blade pictures display",
            crate::ui_target::TEXTURE_FORMAT,
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK blade pictures edges"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blade_pictures.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK blade pictures edges"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let soften = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK blade pictures edges"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: crate::ui_target::TEXTURE_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let soften_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK blade pictures edges"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&blades),
            }],
        });
        let clear_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let clear = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK blade pictures clear"),
            layout: Some(&clear_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_clear"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_clear"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: crate::frame_target::aa::effects::FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::depth_target::DepthTarget::FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK blade pictures camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK blade pictures camera"),
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        Self {
            blades,
            depth,
            display,
            camera,
            camera_bind,
            soften,
            soften_bind,
            clear,
        }
    }
}

impl crate::GpuState {
    /// Draw this frame's blade pictures: of the skins a swatch asked for while the last UI
    /// was built, each in its cell, live, the ones [`assign`] plans; and the blade stills
    /// baked this frame ([`crate::blade_stills`]), whose blades go up with theirs.
    pub(crate) fn encode_blade_pictures(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let seconds = crate::menu::art::motion::seconds();
        let bakes = self.step_blade_stills();
        let baking = bakes.iter().any(Option::is_some);
        let plan = assign(&mut lock(), seconds, self.blade_pictures.is_none());
        // No picture asked for a while (the Collection closed), or the menu pictures on:
        // the target's video memory is given back.
        let now = std::time::Instant::now();
        if plan.is_some() {
            self.blade_pictures_used = Some(now);
        }
        let idle = self
            .blade_pictures_used
            .is_none_or(|used| now - used > TARGET_IDLE);
        if plan.is_none()
            && (idle || !LIVE.load(Ordering::Relaxed))
            && self.blade_pictures.take().is_some()
        {
            self.ui_shapes.clear_blade_pictures();
        }
        let plan = plan.filter(|plan| plan.draw.iter().any(Option::is_some));
        if plan.is_none() && !baking {
            return;
        }
        if plan.is_some() && self.blade_pictures.is_none() {
            let target = Target::new(&self.device, &self.camera_layout);
            self.ui_shapes
                .set_blade_pictures(&self.device, &target.display);
            self.blade_pictures = Some(target);
        }
        // The pictures and loops are drawn at their own moments: the clock the world's and
        // the stage's blades stamped stays for what follows it (a surging trail's colour).
        let stamped = crate::saber::animation_seconds();
        let mut blades = std::mem::take(&mut self.blade_stills.instances);
        blades.clear();
        let mut drawn = [false; SLOTS];
        let mut own = [false; SLOTS];
        if let Some(plan) = &plan {
            for (slot, id) in plan.draw.iter().enumerate() {
                let Some(id) = id else {
                    continue;
                };
                let Some(skin) = self
                    .blade_skins
                    .color_with(id, self.looks.own_options_off(id))
                else {
                    continue;
                };
                let color = crate::saber_rgb::BladeColor::Skin(skin);
                let start = blades.len();
                blades.extend(blade_instances(
                    blade_base(slot),
                    color,
                    seconds,
                    slot as u32,
                    PER_SLOT,
                ));
                drawn[slot] = blades.len() > start;
                own[slot] = self.saber_gpu.skin_ready(skin.index);
            }
        }
        let baked = Self::push_still_blades(&bakes, &mut blades, PER_SLOT);
        crate::saber::restore_animation_seconds(stamped);
        self.saber_gpu.prepare_pictures(&self.queue, &mut blades);
        let any_blade = !blades.is_empty();
        self.blade_stills.instances = blades;
        if let Some(plan) = &plan {
            self.encode_live_pictures(encoder, plan, seconds, any_blade, drawn, own);
        }
        self.encode_blade_stills(encoder, &bakes, baked);
    }

    /// The live pictures' passes for `plan`: the cells planned cleared and drawn, then
    /// softened into the picture the UI shows; which cells got a blade (`drawn`), and with
    /// its own pipelines (`own`).
    fn encode_live_pictures(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        plan: &Plan,
        seconds: f64,
        any_blade: bool,
        drawn: [bool; SLOTS],
        own: [bool; SLOTS],
    ) {
        let Some(target) = self.blade_pictures.as_ref() else {
            return;
        };
        // The blades' shaders aim their view rays from the camera's position: far off in
        // front of the picture's middle, so every cell is seen straight on (Creyon's round 14:
        // from one near corner, the lower cells were seen at a slant and Abyssal Eye's cut sat
        // off its glow). The picture itself is orthographic (`view_projection`).
        // Round 20 (Creyon: the pictures showed rings round the blades as bars across them):
        // a little from the tip's side, as the blades were drawn before their effects were in
        // three dimensions, so a ring round a blade reads as a ring.
        let eye = Vec3::new(
            CELL[0] * COLUMNS as f32 * 0.5 + 4_000.0,
            -20_000.0,
            -CELL[1] * ROWS as f32 * 0.5,
        );
        let uniform = CameraUniform {
            view_projection: view_projection().to_cols_array_2d(),
            camera_position: eye.to_array(),
            shader_time: seconds as f32,
            view_forward: [0.0, 1.0, 0.0],
            // A secondary view, as the model preview's.
            _padding: 17.0,
        };
        self.queue
            .write_buffer(&target.camera, 0, bytemuck::bytes_of(&uniform));
        // The whole picture again, or only the planned cells over the ones kept.
        let (load, depth) = if plan.whole {
            (wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), 1.0)
        } else {
            (wgpu::LoadOp::Load, 0.0)
        };
        let planned = || {
            plan.draw
                .iter()
                .enumerate()
                .filter(|(_, id)| id.is_some())
                .map(|(slot, _)| cell_pixels(slot))
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK blade pictures"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.blades,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth),
                        // Cleared every frame: nothing reads it after the pass.
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !plan.whole {
                // Each planned cell cleared, its depth to the far plane; the cells kept
                // stay at depth 0, where no blade draws.
                pass.set_pipeline(&target.clear);
                for [x, y, width, height] in planned() {
                    pass.set_scissor_rect(x, y, width, height);
                    pass.draw(0..3, 0..1);
                }
                pass.set_scissor_rect(0, 0, SIZE[0], SIZE[1]);
            }
            if any_blade {
                self.saber_gpu.draw_pictures(&mut pass, &target.camera_bind);
            }
        }
        {
            // Soft edges for the cells drawn, into the picture the UI shows.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK blade pictures edges"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.display,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&target.soften);
            pass.set_bind_group(0, &target.soften_bind, &[]);
            if plan.whole {
                pass.draw(0..3, 0..1);
            } else {
                for [x, y, width, height] in planned() {
                    pass.set_scissor_rect(x, y, width, height);
                    pass.draw(0..3, 0..1);
                }
            }
        }
        let mut state = lock();
        for (index, (slot, planned)) in state.slots.iter_mut().zip(plan.draw.iter()).enumerate() {
            if let (Some(slot), Some(id)) = (slot, planned)
                && slot.skin == *id
            {
                slot.drawn = drawn[index];
                slot.own = own[index];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `count` distinct skin ids.
    fn ids(count: usize) -> Vec<&'static str> {
        (0..count)
            .map(|n| &*Box::leak(format!("saber_{n}").into_boxed_str()))
            .collect()
    }

    /// Mark every cell in use drawn, as a frame that drew them does.
    fn draw_all(state: &mut State) {
        for slot in state.slots.iter_mut().flatten() {
            slot.drawn = true;
        }
    }

    #[test]
    fn slow_menus_are_told_after_a_run_of_slow_frames_only() {
        let start = std::time::Instant::now();
        let at = |seconds: f32| start + std::time::Duration::from_secs_f32(seconds);
        let mut watch = SlowWatch::default();
        // 9 FPS uncapped: slow, told once it lasted SLOW_FOR.
        assert!(!watch.note(Some(9), 0, at(0.0)));
        assert!(!watch.note(Some(9), 0, at(2.5)));
        assert!(watch.note(Some(9), 0, at(3.0)));
        // A fast frame between starts the run again.
        let mut watch = SlowWatch::default();
        assert!(!watch.note(Some(9), 0, at(0.0)));
        assert!(!watch.note(Some(60), 0, at(2.0)));
        assert!(!watch.note(Some(9), 0, at(4.0)));
        assert!(!watch.note(Some(9), 0, at(6.5)));
        assert!(watch.note(Some(9), 0, at(7.0)));
        // No measure yet, a low cap (an unfocused window's, a player's 30) or a rate at
        // the mark: never slow.
        let mut watch = SlowWatch::default();
        for seconds in [0.0, 5.0, 10.0] {
            assert!(!watch.note(None, 0, at(seconds)));
            assert!(!watch.note(Some(9), 30, at(seconds)));
            assert!(!watch.note(Some(SLOW_FPS), 0, at(seconds)));
        }
        assert!(is_slow(9, 60) && is_slow(9, 40) && !is_slow(9, 39));
    }

    #[test]
    fn a_skin_keeps_its_cell_while_asked_for_and_frees_it_after() {
        let mut state = State {
            wanted: vec!["saber_a", "saber_b"],
            ..State::default()
        };
        let plan = assign(&mut state, 0.0, false).expect("cells in use");
        assert_eq!(&plan.draw[..2], &[Some("saber_a"), Some("saber_b")]);
        state.slots[0].as_mut().expect("a's cell").drawn = true;
        state.wanted = vec!["saber_c", "saber_a"];
        let plan = assign(&mut state, 0.0, false).expect("cells in use");
        // a keeps cell 0 (drawn), b's cell goes to c, drawn at once.
        let cell = |slot: Option<Slot>| slot.map(|slot| (slot.skin, slot.drawn));
        assert_eq!(cell(state.slots[0]), Some(("saber_a", true)));
        assert_eq!(cell(state.slots[1]), Some(("saber_c", false)));
        assert_eq!(plan.draw[1], Some("saber_c"));
        assert!(assign(&mut state, 0.0, false).is_none(), "no one asked");
        assert!(state.slots.iter().all(Option::is_none));
    }

    #[test]
    fn more_skins_than_cells_leave_the_rest_without() {
        let ids = ids(SLOTS + 3);
        let mut state = State {
            wanted: ids.clone(),
            ..State::default()
        };
        let plan = assign(&mut state, 0.0, false).expect("cells in use");
        assert!(plan.draw.iter().all(Option::is_some));
        assert!(!plan.draw.contains(&Some(ids[SLOTS])));
    }

    /// With many pictures at 240 FPS, a cell no one looks at is drawn 30 times a second,
    /// a few cells a frame; the focused one every frame.
    #[test]
    fn calm_cells_are_drawn_thirty_times_a_second_spread_over_the_frames() {
        let ids = ids(SLOTS);
        let mut state = State::default();
        state.wanted.extend(&ids);
        let plan = assign(&mut state, 10.0, false).expect("cells in use");
        assert!(plan.whole, "new cells are all drawn");
        draw_all(&mut state);
        let mut times = [0_u32; SLOTS];
        let frames = 240;
        for frame in 1..=frames {
            state.wanted.extend(&ids);
            state.focused.push(ids[3]);
            let plan =
                assign(&mut state, 10.0 + f64::from(frame) / 240.0, false).expect("cells in use");
            assert_eq!(plan.draw[3], Some(ids[3]), "the focused cell, every frame");
            assert!(!plan.whole, "frame {frame}");
            let cells = plan.draw.iter().flatten().count();
            assert!(cells <= 4, "frame {frame}: {cells} cells");
            for (slot, id) in plan.draw.iter().enumerate() {
                times[slot] += u32::from(id.is_some());
            }
        }
        for (slot, times) in times.into_iter().enumerate() {
            if slot == 3 {
                assert_eq!(times, frames);
            } else {
                assert!((29..=31).contains(&times), "cell {slot}: {times} times");
            }
        }
    }

    /// On a GPU too slow for the calm rate (10 FPS here), a frame draws the focused cell
    /// and at most [`CALM_PER_FRAME`] calm ones, taking turns, so none is left behind.
    #[test]
    fn slow_frames_draw_a_few_calm_cells_in_turn() {
        let ids = ids(SLOTS);
        let mut state = State::default();
        state.wanted.extend(&ids);
        let _ = assign(&mut state, 10.0, false);
        draw_all(&mut state);
        let mut times = [0_u32; SLOTS];
        for frame in 1..=5 {
            state.wanted.extend(&ids);
            state.focused.push(ids[3]);
            let plan =
                assign(&mut state, 10.0 + f64::from(frame) / 10.0, false).expect("cells in use");
            assert_eq!(plan.draw[3], Some(ids[3]));
            assert_eq!(plan.draw.iter().flatten().count(), 1 + CALM_PER_FRAME);
            assert!(!plan.whole);
            for (slot, id) in plan.draw.iter().enumerate() {
                times[slot] += u32::from(id.is_some());
            }
        }
        for (slot, times) in times.into_iter().enumerate() {
            if slot != 3 {
                assert!((1..=3).contains(&times), "cell {slot}: {times} times");
            }
        }
    }

    /// Two pictures or fewer are drawn every frame; with more, nothing new to draw within
    /// a calm moment, a picture not drawn yet (its skin's pack not loaded) is tried every
    /// frame, and a fresh target draws every cell.
    #[test]
    fn few_cells_a_failed_one_and_a_fresh_target_draw_every_frame() {
        let mut state = State::default();
        for frame in 0..4 {
            state.wanted.extend(["saber_a", "saber_b"]);
            let plan = assign(&mut state, 1.0, false).expect("cells in use");
            assert!(plan.whole, "{frame}");
            assert_eq!(&plan.draw[..2], &[Some("saber_a"), Some("saber_b")]);
            draw_all(&mut state);
        }
        let ids = ids(5);
        state.wanted.extend(&ids);
        let _ = assign(&mut state, 1.0, false);
        state.wanted.extend(&ids);
        let plan = assign(&mut state, 1.0, false).expect("cells in use");
        assert_eq!(
            plan.draw.iter().flatten().count(),
            5,
            "not drawn yet: again"
        );
        draw_all(&mut state);
        state.wanted.extend(&ids);
        let plan = assign(&mut state, 1.0, false).expect("cells in use");
        assert!(plan.draw.iter().all(Option::is_none), "drawn this moment");
        state.wanted.extend(&ids);
        let plan = assign(&mut state, 1.0, true).expect("cells in use");
        assert!(plan.whole);
        assert_eq!(plan.draw.iter().flatten().count(), 5);
    }

    /// A frame's wishes and plan take no allocation once the screen has been drawn.
    #[cfg(debug_assertions)]
    #[test]
    fn a_frame_of_wishes_allocates_nothing() {
        let ids = ids(SLOTS);
        let mut state = State::default();
        state.wanted.extend(&ids);
        state.focused.push(ids[0]);
        let _ = assign(&mut state, 0.0, false);
        let before = assert_no_alloc::violation_count();
        assert_no_alloc::assert_no_alloc(|| {
            for frame in 0..60 {
                for id in &ids {
                    if !state.wanted.contains(id) {
                        state.wanted.push(id);
                    }
                }
                state.focused.push(ids[frame % SLOTS]);
                let _ = assign(&mut state, frame as f64 / 60.0, false);
                draw_all(&mut state);
            }
        });
        assert_eq!(assert_no_alloc::violation_count(), before);
    }

    #[test]
    fn every_cell_lies_inside_the_camera() {
        let matrix = view_projection();
        for slot in 0..SLOTS {
            let base = blade_base(slot);
            for point in [base, base + Vec3::X * BLADE_LENGTH] {
                let clip = matrix * point.extend(1.0);
                let ndc = clip.truncate() / clip.w;
                assert!(ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0, "{slot}: {ndc}");
            }
        }
        // The first cell's blade in the picture's top left.
        let ndc = (matrix * blade_base(0).extend(1.0)).truncate();
        assert!(ndc.x < -0.9 && ndc.y > 0.8, "{ndc}");
    }

    /// The rectangle a cell drawn alone is cleared and drawn in holds its blade.
    #[test]
    fn each_blade_lies_in_its_cell_rectangle() {
        let matrix = view_projection();
        for slot in 0..SLOTS {
            let [x, y, width, height] = cell_pixels(slot).map(|value| value as f32);
            let base = blade_base(slot);
            for point in [base, base + Vec3::X * BLADE_LENGTH] {
                let ndc = (matrix * point.extend(1.0)).truncate();
                let pixel = [
                    (ndc.x + 1.0) * 0.5 * SIZE[0] as f32,
                    (1.0 - ndc.y) * 0.5 * SIZE[1] as f32,
                ];
                assert!(
                    pixel[0] > x && pixel[0] < x + width && pixel[1] > y && pixel[1] < y + height,
                    "{slot}: {pixel:?}"
                );
            }
        }
        let [width, height] = [SIZE[0] / COLUMNS as u32, SIZE[1] / ROWS as u32];
        assert_eq!(
            cell_pixels(SLOTS - 1),
            [width, SIZE[1] - height, width, height]
        );
    }
}

#[cfg(test)]
mod edge_tests {
    #[test]
    fn the_edge_program_validates() {
        crate::wgsl_source::validate(include_str!("blade_pictures.wgsl"));
    }
}
