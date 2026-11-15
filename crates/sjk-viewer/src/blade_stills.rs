//! The Collection's blade stills (Sol, 15/11/2026: "make blades look great on Collection
//! tiles"): every kyber drawn once by the game's own blade renderer into a small still
//! picture, its glow added, kept on disk, and shown by the wall's tiles, the list's rows,
//! the hall's and the Forge's strips; only the swatch the player looks at (the hovered or
//! chosen one, [`crate::blade_pictures::focus`]) stays live ([`crate::blade_pictures`]).
//!
//! A swatch asks for its skin's still ([`still`]) while the UI is built. The frame then
//! ([`crate::GpuState::step_blade_stills`]) finds each still asked for: in a cell of the
//! atlas already ([`CELLS`] cells of [`STILL`] pixels, one texture, [`ATLAS`]); on disk,
//! read and decoded by a thread of its own and uploaded, [`UPLOADS_PER_FRAME`] a frame at
//! most; or missing, then baked: [`BAKE_BATCH`] a frame at most (one with the menu
//! pictures on, `ui_menuPictures`), drawn by the blade renderer in the same pass as the live
//! pictures at a fixed moment of their animation ([`STILL_SECONDS`]) at twice the still's
//! size, then brought down to it with a soft glow round the light (`blade_stills.wgsl`),
//! into its atlas cell, and read back and written to disk by that thread. A signature
//! blade waits for its own pipelines (asked for at once, the Collection's urgency) so no
//! still keeps the shared look a build stands in with. While the Collection is open and
//! nothing visible is waiting, the other loaded kybers are baked too, one a frame, so a
//! scroll finds its stills on disk (not with the menu pictures on).
//!
//! A still is kept as `<id>-<key>.png` in `blades/` of the cache folder beside the settings
//! (`GameData/SJK/cache/blades`, [`set_folder`]; the off-screen renders keep none unless a
//! test names one). Its key ([`skin_key`]) hashes what the picture is drawn from: the
//! skin's file as loaded, its colour as worn (the player's own options, a chroma's turn),
//! its glow and core images, its signature's shader and file, the blade shader
//! (`saber.wgsl`) and this baker ([`VERSION`], `blade_stills.wgsl`): any change gives a
//! new key, and a new still replaces the old file. The cells go to the stills asked for,
//! the one asked least lately making room ([`take_cell`]); a still asked for this frame
//! is never moved. With nothing asked for [`IDLE`], the atlas is freed, as the baking
//! target is [`BAKER_IDLE`] after its last bake.
//!
//! Video memory: the atlas is 1536 x 768 RGBA8 (4.5 MiB) while the Collection or a strip
//! shows; the baking target 512 x 768 RGBA8 with its depth (3 MiB) only while baking. With
//! the menu pictures on (a card short of memory) the stills still show, the live pictures
//! do not, and baking goes one a frame, visible stills only.

use crate::camera_uniform::CameraUniform;
use glam::Vec3;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// A still's size in pixels.
pub(crate) const STILL: [u32; 2] = [256, 96];
/// The atlas's cells: six columns of eight.
const COLUMNS: u32 = 6;
const ROWS: u32 = 8;
pub(crate) const CELLS: usize = (COLUMNS * ROWS) as usize;
/// The atlas's size in pixels.
pub(crate) const ATLAS: [u32; 2] = [STILL[0] * COLUMNS, STILL[1] * ROWS];
/// What a still shows, in world units: a live picture's cell across
/// ([`crate::blade_pictures::BLADE_START`] before the blade, room after its tip) and its
/// middle this high.
pub(crate) const UNITS: [f32; 2] = [52.0, 19.5];
/// The baking target draws each still this many times its size.
const SUPERSAMPLE: u32 = 2;
const BAKE: [u32; 2] = [STILL[0] * SUPERSAMPLE, STILL[1] * SUPERSAMPLE];
/// Stills baked a frame at most; with the menu pictures on, [`BAKES_LOW`].
pub(crate) const BAKE_BATCH: usize = 4;
const BAKES_LOW: usize = 1;
/// Stills not on screen baked a frame while the Collection is open.
const BACKGROUND_BAKES: usize = 1;
/// Skins' keys worked out a frame at most (each hashes its file and images).
const KEYS_PER_FRAME: usize = 24;
/// Stills read from disk uploaded a frame at most.
const UPLOADS_PER_FRAME: usize = 8;
/// The moment of the blades' animation a still shows (seconds).
const STILL_SECONDS: f64 = 2.4;
/// How long the atlas is kept with no still asked for, and the baking target after its
/// last bake.
const IDLE: Duration = Duration::from_secs(10);
const BAKER_IDLE: Duration = Duration::from_secs(5);
/// The baker's version: raise it when a still's look changes in a way the key does not see.
pub(crate) const VERSION: u32 = 1;
/// The folder in the cache folder the stills go in.
pub(crate) const FOLDER: &str = "blades";
/// The texture ids: cell `n` is `TEXTURE_BASE + n`, its dimmed look (not held) `+ DIM`.
const TEXTURE_BASE: u32 = 0x6300_0000;
const DIM: u32 = 0x100;
/// Where the baked blades lie in the world, far from the live pictures' cells.
const BAKE_X: f32 = 10_000.0;

/// `TexturedQuad` texture naming cell `cell`, desaturated and dimmed when `dim`.
pub(crate) fn texture(cell: usize, dim: bool) -> sjk_ui::TextureId {
    let cell = cell.min(CELLS - 1) as u32;
    sjk_ui::TextureId(TEXTURE_BASE + cell + if dim { DIM } else { 0 })
}

/// The cell `texture` names, and whether it is the dimmed look.
pub(crate) fn cell_of(texture: sjk_ui::TextureId) -> Option<(usize, bool)> {
    let offset = texture.0.checked_sub(TEXTURE_BASE)?;
    let (cell, dim) = if offset >= DIM {
        (offset - DIM, true)
    } else {
        (offset, false)
    };
    ((cell as usize) < CELLS && offset < DIM * 2).then_some((cell as usize, dim))
}

/// Cell `cell`'s texture coordinates in the atlas (top left, bottom right).
pub(crate) fn uv_range(cell: usize) -> ([f32; 2], [f32; 2]) {
    let [x, y, width, height] = cell_pixels(cell.min(CELLS - 1));
    let uv = |x: u32, y: u32| [x as f32 / ATLAS[0] as f32, y as f32 / ATLAS[1] as f32];
    (uv(x, y), uv(x + width, y + height))
}

/// Cell `cell`'s rectangle in the atlas, in pixels.
fn cell_pixels(cell: usize) -> [u32; 4] {
    let cell = cell as u32;
    [
        (cell % COLUMNS) * STILL[0],
        (cell / COLUMNS) * STILL[1],
        STILL[0],
        STILL[1],
    ]
}

/// A cell of the atlas: whose still it holds and when it was last asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cell {
    id: &'static str,
    key: u64,
    /// Its picture is in the atlas: swatches show it.
    ready: bool,
    /// The frame it was last asked for in.
    used: u64,
}

/// What the UI asks for and the cells, shared by the UI and the frame.
struct Shared {
    wanted: Vec<&'static str>,
    cells: [Option<Cell>; CELLS],
    frame: u64,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    wanted: Vec::new(),
    cells: [None; CELLS],
    frame: 0,
});

fn lock() -> std::sync::MutexGuard<'static, Shared> {
    SHARED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The folder the stills are kept in: none, until the windowed client names one.
static FOLDER_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Keep stills in `folder` (the client's `cache/blades`), or none.
pub(crate) fn set_folder(folder: Option<PathBuf>) {
    *FOLDER_PATH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = folder;
}

fn folder() -> Option<PathBuf> {
    FOLDER_PATH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// The still of skin `id` for a swatch, once it is in the atlas: its texture, dimmed
/// (desaturated, for a kyber not held) when `dim`. Asking keeps it there, or reads or
/// bakes it.
pub(crate) fn still(id: &'static str, dim: bool) -> Option<sjk_ui::TextureId> {
    let mut shared = lock();
    if !shared.wanted.contains(&id) {
        shared.wanted.push(id);
    }
    let cell = shared
        .cells
        .iter()
        .position(|cell| cell.is_some_and(|cell| cell.ready && cell.id == id))?;
    Some(texture(cell, dim))
}

/// Forget every still in memory, as a client starting again (the world shots read them
/// back from disk).
#[cfg(test)]
pub(crate) fn forget_for_shot(stills: &mut Stills) {
    lock().cells = [None; CELLS];
    *stills = Stills::default();
}

/// Stills the last frame asked for that are not shown yet (being read, waiting for their
/// blade's pipelines or baked): for the world shots, which wait for none.
#[cfg(test)]
pub(crate) fn waiting(stills: &Stills) -> usize {
    let shared = lock();
    stills
        .wanted
        .iter()
        .filter(|id| stills.keys.contains_key(*id))
        .filter(|id| {
            !shared
                .cells
                .iter()
                .flatten()
                .any(|cell| cell.ready && cell.id == **id)
        })
        .count()
}

/// A cell for a new still at `frame`: a free one, else the one asked for least lately,
/// never one asked for this frame.
fn take_cell(cells: &[Option<Cell>; CELLS], frame: u64) -> Option<usize> {
    if let Some(free) = cells.iter().position(Option::is_none) {
        return Some(free);
    }
    cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| cell.map(|cell| (index, cell.used)))
        .filter(|&(_, used)| used < frame)
        .min_by_key(|&(_, used)| used)
        .map(|(index, _)| index)
}

/// From the stills waiting to be baked (`queue`, in the order asked), the ones baked this
/// frame: at most `max`, of those still `wanted`, whose blade is `ready` to draw as it
/// looks. The ones no longer wanted leave the queue; the ones not ready stay in it.
fn choose_bakes(
    queue: &mut Vec<&'static str>,
    wanted: &[&'static str],
    ready: impl Fn(&'static str) -> bool,
    max: usize,
) -> [Option<&'static str>; BAKE_BATCH] {
    queue.retain(|id| wanted.contains(id));
    let mut chosen = [None; BAKE_BATCH];
    let mut taken = 0;
    queue.retain(|&id| {
        if taken < max.min(BAKE_BATCH) && ready(id) {
            chosen[taken] = Some(id);
            taken += 1;
            false
        } else {
            true
        }
    });
    chosen
}

/// A 64-bit hash of `parts`, the same on every machine and in every build (FNV-1a over
/// eight bytes at a time, each part's length mixed in so parts cannot run together).
fn key_of(parts: &[&[u8]]) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |word: u64| {
        hash ^= word;
        hash = hash.wrapping_mul(PRIME).rotate_left(23);
    };
    for part in parts {
        mix(part.len() as u64);
        let mut chunks = part.chunks_exact(8);
        for chunk in &mut chunks {
            let mut word = [0; 8];
            word.copy_from_slice(chunk);
            mix(u64::from_le_bytes(word));
        }
        let mut last = [0; 8];
        last[..chunks.remainder().len()].copy_from_slice(chunks.remainder());
        mix(u64::from_le_bytes(last));
    }
    hash
}

/// The key of a skin's still: `skin` as loaded, worn as `color`. Its number among the
/// loaded skins is left out (another pack moves it, not its look).
pub(crate) fn skin_key(
    skin: &crate::saber_skins::LoadedSkin,
    color: crate::saber_skins::SkinColor,
) -> u64 {
    let def = format!("{:?}", skin.def);
    let color = format!("{:?}", crate::saber_skins::SkinColor { index: 0, ..color });
    let [wgsl, file] = skin
        .signature
        .map_or(["", ""], |signature| signature.sources());
    let design = skin.design.map_or("", |design| design.id);
    key_of(&[
        &VERSION.to_le_bytes(),
        include_str!("blade_stills.wgsl").as_bytes(),
        include_str!("saber.wgsl").as_bytes(),
        skin.id.as_bytes(),
        def.as_bytes(),
        color.as_bytes(),
        design.as_bytes(),
        wgsl.as_bytes(),
        file.as_bytes(),
        skin.glow.as_raw(),
        skin.core.as_raw(),
    ])
}

/// A still's file name.
fn file_name(id: &str, key: u64) -> String {
    format!("{id}-{key:016x}.png")
}

/// The skin id and key a still's file name holds.
fn parse_file_name(name: &str) -> Option<(&str, u64)> {
    let stem = name.strip_suffix(".png")?;
    let (id, key) = stem.rsplit_once('-')?;
    (key.len() == 16 && !id.is_empty())
        .then(|| u64::from_str_radix(key, 16).ok())
        .flatten()
        .map(|key| (id, key))
}

/// The stills kept on disk: each skin's key, as the folder's files name them.
#[derive(Debug, Default)]
struct Disk {
    folder: PathBuf,
    keys: HashMap<String, u64>,
}

impl Disk {
    /// The stills in `folder` (none when it cannot be read).
    fn read(folder: PathBuf) -> Self {
        let mut keys = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if let Some((id, key)) = name.to_str().and_then(parse_file_name) {
                    keys.insert(id.to_owned(), key);
                }
            }
        }
        Self { folder, keys }
    }

    /// Whether skin `id`'s still on disk has key `key`.
    fn has(&self, id: &str, key: u64) -> bool {
        self.keys.get(id) == Some(&key)
    }

    fn path(&self, id: &str, key: u64) -> PathBuf {
        self.folder.join(file_name(id, key))
    }

    /// Note skin `id`'s still kept with key `key`: the path to write it to, and the old
    /// file it replaces, if any.
    fn replace(&mut self, id: &str, key: u64) -> (PathBuf, Option<PathBuf>) {
        let old = self
            .keys
            .insert(id.to_owned(), key)
            .filter(|&old| old != key)
            .map(|old| self.path(id, old));
        (self.path(id, key), old)
    }
}

/// What the stills' thread is asked to do.
enum Job {
    /// Read and decode a still.
    Load {
        id: &'static str,
        key: u64,
        path: PathBuf,
    },
    /// Write a still (RGBA, [`STILL`] pixels) to `path`, and remove the one it replaces.
    Save {
        path: PathBuf,
        pixels: Vec<u8>,
        old: Option<PathBuf>,
    },
}

/// A still read: its pixels, none when the file was damaged or gone.
struct Loaded {
    id: &'static str,
    key: u64,
    pixels: Option<Vec<u8>>,
}

/// The stills' thread.
struct Worker {
    jobs: mpsc::Sender<Job>,
    done: mpsc::Receiver<Loaded>,
}

impl Worker {
    fn start() -> Option<Self> {
        let (jobs, inbox) = mpsc::channel::<Job>();
        let (outbox, done) = mpsc::channel();
        std::thread::Builder::new()
            .name("SJK blade stills".into())
            .spawn(move || {
                for job in inbox {
                    match job {
                        Job::Load { id, key, path } => {
                            let pixels = read_still(&path);
                            if outbox.send(Loaded { id, key, pixels }).is_err() {
                                return;
                            }
                        }
                        Job::Save { path, pixels, old } => {
                            if let Err(why) = write_still(&path, pixels) {
                                crate::log::progress(format_args!(
                                    "blade stills: {} not kept: {why}",
                                    path.display()
                                ));
                            }
                            if let Some(old) = old {
                                let _ = std::fs::remove_file(old);
                            }
                        }
                    }
                }
            })
            .ok()?;
        Some(Self { jobs, done })
    }
}

/// A still's pixels from `path`, if it is a still.
fn read_still(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    (image.dimensions() == (STILL[0], STILL[1])).then(|| image.into_raw())
}

/// Write a still to `path`, through a temporary file renamed over it.
fn write_still(path: &Path, pixels: Vec<u8>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let image = image::RgbaImage::from_raw(STILL[0], STILL[1], pixels)
        .ok_or_else(|| std::io::Error::other("wrong size"))?;
    let temporary = path.with_extension("png.part");
    image
        .save_with_format(&temporary, image::ImageFormat::Png)
        .map_err(std::io::Error::other)?;
    std::fs::rename(&temporary, path)
}

/// A still baked this frame: into atlas cell `cell`, skin `id` worn as `color`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bake {
    cell: usize,
    id: &'static str,
    key: u64,
    color: crate::saber_skins::SkinColor,
}

/// This frame's bakes, in the baking target's rows.
pub(crate) type Bakes = [Option<Bake>; BAKE_BATCH];

/// Where row `row`'s blade starts, in world units (y = 0 the plane, x across, z up).
fn bake_base(row: usize) -> Vec3 {
    Vec3::new(
        BAKE_X + crate::blade_pictures::BLADE_START,
        0.0,
        -(row as f32 + 0.5) * UNITS[1],
    )
}

/// The camera seeing the baking rows side on.
fn bake_view_projection() -> glam::Mat4 {
    let height = UNITS[1] * BAKE_BATCH as f32;
    glam::camera::rh::proj::directx::orthographic(
        BAKE_X,
        BAKE_X + UNITS[0],
        -height,
        0.0,
        1.0,
        400.0,
    ) * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -200.0, 0.0), Vec3::ZERO, Vec3::Z)
}

/// The atlas on the GPU.
struct Atlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Atlas {
    fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SJK blade stills"),
            size: wgpu::Extent3d {
                width: ATLAS[0],
                height: ATLAS[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::ui_target::TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self { texture, view }
    }
}

/// The baking target, its camera and the pass that makes stills of it.
struct Baker {
    blades: wgpu::TextureView,
    depth: wgpu::TextureView,
    camera: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    still: wgpu::RenderPipeline,
    still_bind: wgpu::BindGroup,
}

impl Baker {
    fn new(device: &wgpu::Device, camera_layout: &wgpu::BindGroupLayout) -> Self {
        let texture = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: BAKE[0],
                        height: BAKE[1] * BAKE_BATCH as u32,
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
            "SJK blade stills baking",
            crate::frame_target::aa::effects::FORMAT,
        );
        let depth = texture(
            "SJK blade stills baking depth",
            crate::depth_target::DepthTarget::FORMAT,
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK blade stills"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blade_stills.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK blade stills"),
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
        let still = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK blade stills"),
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
        let still_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK blade stills"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&blades),
            }],
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK blade stills camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK blade stills camera"),
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        Self {
            blades,
            depth,
            camera,
            camera_bind,
            still,
            still_bind,
        }
    }
}

/// Bytes of one still read back.
const STILL_BYTES: u64 = STILL[0] as u64 * 4 * STILL[1] as u64;

/// A frame's bakes being read back to be kept on disk.
struct Readback {
    buffer: wgpu::Buffer,
    /// Each still in the buffer, in order: its cell, skin and key.
    stills: [Option<(usize, &'static str, u64)>; BAKE_BATCH],
    /// Set once the frame that copied them has been submitted (the next one).
    receiver: Option<mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
}

/// The stills' side of the frame: keys, the disk, the queues and the GPU's atlas.
#[derive(Default)]
pub(crate) struct Stills {
    /// The Collection's blade instances, the live pictures' and the bakes', kept with
    /// their room from frame to frame ([`crate::blade_pictures`]).
    pub(crate) instances: Vec<crate::saber::Instance>,
    /// This frame's wishes, taken from the UI's (kept with their room).
    wanted: Vec<&'static str>,
    /// Each skin's key, and the options switched off it was worked out with (a change of
    /// them works it out again).
    keys: HashMap<&'static str, (crate::saber_skin_options::OptionsOff, u64)>,
    /// The loaded skins' generation the keys are of.
    generation: Option<u64>,
    disk: Option<Disk>,
    /// Stills to bake, in the order asked; stills being read.
    queue: Vec<&'static str>,
    loading: Vec<&'static str>,
    /// Skins whose signature pipelines have been asked for.
    asked: HashSet<&'static str>,
    worker: Option<Worker>,
    atlas: Option<Atlas>,
    baker: Option<Baker>,
    readbacks: Vec<Readback>,
    last_wanted: Option<Instant>,
    last_bake: Option<Instant>,
    /// The next loaded skin the background baking looks at, and whether a round left one
    /// waiting for its pipelines.
    cursor: usize,
    skipped: bool,
    /// Stills baked so far (the world shots time them).
    #[cfg(test)]
    pub(crate) baked: u64,
}

impl crate::GpuState {
    /// This frame's stills: take the swatches' wishes, keep the cells they hold, read the
    /// stills on disk and plan the bakes (drawn by [`Self::encode_blade_pictures`]).
    pub(crate) fn step_blade_stills(&mut self) -> Bakes {
        let now = Instant::now();
        self.poll_still_readbacks();
        let frame = {
            let mut shared = lock();
            shared.frame += 1;
            std::mem::swap(&mut shared.wanted, &mut self.blade_stills.wanted);
            shared.wanted.clear();
            shared.frame
        };
        let mut bakes: Bakes = [None; BAKE_BATCH];
        if self.blade_stills.wanted.is_empty() {
            self.release_idle_stills(now);
            return bakes;
        }
        self.blade_stills.last_wanted = Some(now);
        let generation = self.blade_skins.generation();
        if self.blade_stills.generation != Some(generation) {
            self.blade_stills.generation = Some(generation);
            self.blade_stills.keys.clear();
            self.blade_stills.asked.clear();
            self.blade_stills.cursor = 0;
        }
        if self.blade_stills.disk.is_none()
            && let Some(folder) = folder()
        {
            self.blade_stills.disk = Some(Disk::read(folder));
        }
        let mut keys_left = KEYS_PER_FRAME;
        let wanted = std::mem::take(&mut self.blade_stills.wanted);
        for &id in &wanted {
            let Some(key) = self.still_key(id, &mut keys_left) else {
                continue;
            };
            // A signature's pipelines asked for once it shows, so the live one is ready
            // when the player looks at it (the Collection's urgency while its still waits).
            let missing = !self
                .blade_stills
                .disk
                .as_ref()
                .is_some_and(|disk| disk.has(id, key));
            self.ask_still_build(id, missing);
            {
                let mut shared = lock();
                let held = shared
                    .cells
                    .iter()
                    .position(|cell| cell.is_some_and(|cell| cell.id == id));
                if let Some(index) = held {
                    if let Some(cell) = &mut shared.cells[index]
                        && cell.key == key
                    {
                        cell.used = frame;
                        continue;
                    }
                    // Its look changed: the cell is free again.
                    shared.cells[index] = None;
                }
            }
            if self.blade_stills.loading.contains(&id) || self.blade_stills.queue.contains(&id) {
                continue;
            }
            let on_disk = self
                .blade_stills
                .disk
                .as_ref()
                .filter(|disk| disk.has(id, key))
                .map(|disk| disk.path(id, key));
            match on_disk {
                Some(path) if self.still_worker().is_some() => {
                    if let Some(worker) = &self.blade_stills.worker
                        && worker.jobs.send(Job::Load { id, key, path }).is_ok()
                    {
                        self.blade_stills.loading.push(id);
                    }
                }
                _ => self.blade_stills.queue.push(id),
            }
        }
        self.upload_loaded_stills(frame);
        let live = crate::blade_pictures::live();
        let max = if live { BAKE_BATCH } else { BAKES_LOW };
        let skins = std::sync::Arc::clone(&self.blade_skins);
        let chosen = choose_bakes(
            &mut self.blade_stills.queue,
            &wanted,
            |id| {
                skins
                    .index_of(id)
                    .is_some_and(|index| self.saber_gpu.skin_ready(index))
            },
            max,
        );
        let mut row = 0;
        for id in chosen.into_iter().flatten() {
            if let Some(bake) = self.plan_bake(id, frame) {
                bakes[row] = Some(bake);
                row += 1;
            }
        }
        // Nothing on screen waiting: the next kyber not on disk, while there is a disk.
        if row == 0 && live && self.blade_stills.queue.is_empty() {
            self.plan_background_bake(frame, &mut bakes, &mut keys_left);
        }
        self.blade_stills.wanted = wanted;
        if bakes.iter().any(Option::is_some) {
            self.blade_stills.last_bake = Some(now);
        }
        bakes
    }

    /// Skin `id`'s still key, worked out once per loaded generation, `budget` a frame;
    /// none while its pack is not loaded or the budget is spent.
    fn still_key(&mut self, id: &'static str, budget: &mut usize) -> Option<u64> {
        let off = self.looks.own_options_off(id);
        if let Some(&(kept, key)) = self.blade_stills.keys.get(id)
            && kept == off
        {
            return Some(key);
        }
        if *budget == 0 {
            return None;
        }
        let skin = self.blade_skins.get(id)?;
        let color = self.blade_skins.color_with(id, off)?;
        *budget -= 1;
        let key = skin_key(skin, color);
        self.blade_stills.keys.insert(id, (off, key));
        Some(key)
    }

    /// Ask, once, for skin `id`'s signature pipelines: soon when its still is to be
    /// baked, else with the lowest urgency.
    fn ask_still_build(&mut self, id: &'static str, soon: bool) {
        if !self.blade_stills.asked.insert(id) {
            return;
        }
        if let Some(index) = self.blade_skins.index_of(id) {
            self.saber_gpu.ask_build(index, soon);
        }
    }

    /// The stills' thread, started the first time it is needed.
    fn still_worker(&mut self) -> Option<&Worker> {
        if self.blade_stills.worker.is_none() {
            self.blade_stills.worker = Worker::start();
        }
        self.blade_stills.worker.as_ref()
    }

    /// The atlas, made the first time a still needs it and bound for the UI.
    fn still_atlas(&mut self) -> &Atlas {
        if self.blade_stills.atlas.is_none() {
            let atlas = Atlas::new(&self.device);
            self.ui_shapes.set_blade_stills(&self.device, &atlas.view);
            self.blade_stills.atlas = Some(atlas);
        }
        self.blade_stills
            .atlas
            .as_ref()
            .unwrap_or_else(|| unreachable!("made above"))
    }

    /// Upload the stills the thread has read, a few a frame; a damaged one is baked.
    fn upload_loaded_stills(&mut self, frame: u64) {
        for _ in 0..UPLOADS_PER_FRAME {
            let Some(loaded) = self
                .blade_stills
                .worker
                .as_ref()
                .and_then(|worker| worker.done.try_recv().ok())
            else {
                return;
            };
            self.blade_stills.loading.retain(|id| *id != loaded.id);
            if self.blade_stills.keys.get(loaded.id).map(|kept| kept.1) != Some(loaded.key) {
                continue;
            }
            let Some(pixels) = loaded.pixels else {
                self.blade_stills.queue.push(loaded.id);
                continue;
            };
            let Some(cell) = take_cell(&lock().cells, frame) else {
                continue;
            };
            let [x, y, width, height] = cell_pixels(cell);
            let _ = self.still_atlas();
            let Some(atlas) = &self.blade_stills.atlas else {
                return;
            };
            let texture = &atlas.texture;
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            lock().cells[cell] = Some(Cell {
                id: loaded.id,
                key: loaded.key,
                ready: true,
                used: frame,
            });
        }
    }

    /// A bake of skin `id` into a cell taken for it, if its pack is loaded and a cell free.
    fn plan_bake(&mut self, id: &'static str, frame: u64) -> Option<Bake> {
        let key = self.blade_stills.keys.get(id)?.1;
        let color = self
            .blade_skins
            .color_with(id, self.looks.own_options_off(id))?;
        let mut shared = lock();
        let cell = take_cell(&shared.cells, frame)?;
        shared.cells[cell] = Some(Cell {
            id,
            key,
            ready: false,
            used: frame,
        });
        Some(Bake {
            cell,
            id,
            key,
            color,
        })
    }

    /// While the Collection is open and nothing on screen waits: bake the next loaded
    /// kyber that has no still on disk ([`BACKGROUND_BAKES`] a frame), so a scroll finds
    /// its still kept. Only where stills are kept. A signature whose pipelines are not
    /// built is asked for with the lowest urgency and looked at again on the next round.
    fn plan_background_bake(&mut self, frame: u64, bakes: &mut Bakes, budget: &mut usize) {
        if self.blade_stills.disk.is_none() {
            return;
        }
        let skins = std::sync::Arc::clone(&self.blade_skins);
        let count = skins.iter().count();
        let mut planned = 0;
        while planned < BACKGROUND_BAKES && *budget > 0 {
            if self.blade_stills.cursor >= count {
                if !std::mem::take(&mut self.blade_stills.skipped) {
                    return;
                }
                self.blade_stills.cursor = 0;
            }
            let Some((index, skin)) = skins.iter().nth(self.blade_stills.cursor) else {
                return;
            };
            let id = skin.id;
            let Some(key) = self.still_key(id, budget) else {
                return;
            };
            self.blade_stills.cursor += 1;
            let kept = self
                .blade_stills
                .disk
                .as_ref()
                .is_some_and(|disk| disk.has(id, key));
            let held = lock().cells.iter().flatten().any(|cell| cell.id == id);
            if kept || held {
                continue;
            }
            if !self.saber_gpu.skin_ready(index) {
                self.ask_still_build(id, false);
                self.blade_stills.skipped = true;
                continue;
            }
            if let Some(bake) = self.plan_bake(id, frame) {
                bakes[planned] = Some(bake);
                planned += 1;
            } else {
                return;
            }
        }
    }

    /// The blade instances of this frame's `bakes`, each in its baking row, appended to
    /// `blades`; which rows got any.
    pub(crate) fn push_still_blades(
        bakes: &Bakes,
        blades: &mut Vec<crate::saber::Instance>,
        per_blade: usize,
    ) -> [bool; BAKE_BATCH] {
        let mut drawn = [false; BAKE_BATCH];
        for (row, bake) in bakes.iter().enumerate() {
            let Some(bake) = bake else {
                continue;
            };
            let color = crate::saber_rgb::BladeColor::Skin(bake.color);
            let start = blades.len();
            blades.extend(crate::blade_pictures::blade_instances(
                bake_base(row),
                color,
                STILL_SECONDS,
                0,
                per_blade,
            ));
            drawn[row] = blades.len() > start;
        }
        drawn
    }

    /// Draw this frame's `bakes` (their blades already uploaded with the live pictures')
    /// into the baking target, make their stills in the atlas, and copy them out to be kept.
    pub(crate) fn encode_blade_stills(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        bakes: &Bakes,
        drawn: [bool; BAKE_BATCH],
    ) {
        if bakes.iter().all(Option::is_none) {
            return;
        }
        let _ = self.still_atlas();
        if self.blade_stills.baker.is_none() {
            self.blade_stills.baker = Some(Baker::new(&self.device, &self.camera_layout));
        }
        let height = UNITS[1] * BAKE_BATCH as f32;
        let eye = Vec3::new(BAKE_X + UNITS[0] * 0.5 + 4_000.0, -20_000.0, -height * 0.5);
        let uniform = CameraUniform {
            view_projection: bake_view_projection().to_cols_array_2d(),
            camera_position: eye.to_array(),
            shader_time: STILL_SECONDS as f32,
            view_forward: [0.0, 1.0, 0.0],
            _padding: 17.0,
        };
        let (Some(baker), Some(atlas)) = (&self.blade_stills.baker, &self.blade_stills.atlas)
        else {
            return;
        };
        self.queue
            .write_buffer(&baker.camera, 0, bytemuck::bytes_of(&uniform));
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK blade stills baking"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &baker.blades,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &baker.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if drawn.iter().any(|drawn| *drawn) {
                self.saber_gpu.draw_pictures(&mut pass, &baker.camera_bind);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK blade stills"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &atlas.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&baker.still);
            pass.set_bind_group(0, &baker.still_bind, &[]);
            for (row, bake) in bakes.iter().enumerate() {
                let Some(bake) = bake else {
                    continue;
                };
                let [x, y, width, height] = cell_pixels(bake.cell);
                pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);
                pass.set_scissor_rect(x, y, width, height);
                pass.draw(0..3, row as u32..row as u32 + 1);
            }
        }
        #[cfg(test)]
        {
            self.blade_stills.baked += drawn.iter().filter(|drawn| **drawn).count() as u64;
        }
        let keeping = self.blade_stills.disk.is_some();
        let mut stills = [None; BAKE_BATCH];
        let mut count = 0;
        {
            let mut shared = lock();
            for (bake, drawn) in bakes.iter().zip(drawn) {
                let Some(bake) = bake else {
                    continue;
                };
                if shared.cells[bake.cell].is_some_and(|cell| cell.id == bake.id) {
                    if drawn {
                        if let Some(cell) = &mut shared.cells[bake.cell] {
                            cell.ready = true;
                        }
                    } else {
                        // Nothing drawn (its pack went): asked again, baked again.
                        shared.cells[bake.cell] = None;
                    }
                }
                if drawn && keeping {
                    stills[count] = Some((bake.cell, bake.id, bake.key));
                    count += 1;
                }
            }
        }
        if count == 0 {
            return;
        }
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK blade stills readback"),
            size: STILL_BYTES * count as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        for (order, (cell, ..)) in stills.iter().flatten().enumerate() {
            let [x, y, width, height] = cell_pixels(*cell);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &atlas.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: STILL_BYTES * order as u64,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.blade_stills.readbacks.push(Readback {
            buffer,
            stills,
            receiver: None,
        });
    }

    /// Map the bakes copied out last frame, and keep the ones mapped.
    fn poll_still_readbacks(&mut self) {
        if self.blade_stills.readbacks.is_empty() {
            return;
        }
        // Last frame may still be on its way to `Queue::submit` on the submit thread: a
        // buffer mapped before its copy is submitted fails that submit. Rare (a frame
        // that baked), and nothing is waited for when no frame is in flight.
        if self
            .blade_stills
            .readbacks
            .iter()
            .any(|readback| readback.receiver.is_none())
        {
            self.frame_pacer.split.wait_previous();
        }
        for readback in &mut self.blade_stills.readbacks {
            if readback.receiver.is_none() {
                let (sender, receiver) = mpsc::sync_channel(1);
                readback
                    .buffer
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |result| {
                        let _ = sender.try_send(result);
                    });
                readback.receiver = Some(receiver);
            }
        }
        let _ = self.device.poll(wgpu::PollType::Poll);
        let readbacks = std::mem::take(&mut self.blade_stills.readbacks);
        for readback in readbacks {
            let state = readback
                .receiver
                .as_ref()
                .map(mpsc::Receiver::try_recv)
                .unwrap_or(Err(mpsc::TryRecvError::Empty));
            match state {
                Err(mpsc::TryRecvError::Empty) => self.blade_stills.readbacks.push(readback),
                Ok(Ok(())) => self.keep_stills(&readback),
                Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {}
            }
        }
    }

    /// Send the stills of a mapped readback to the thread to be written; a picture left
    /// black (a frame that was never submitted) is baked again.
    fn keep_stills(&mut self, readback: &Readback) {
        let pixels: Vec<Vec<u8>> = {
            let Ok(view) = readback.buffer.slice(..).get_mapped_range() else {
                return;
            };
            view.chunks_exact(STILL_BYTES as usize)
                .map(<[u8]>::to_vec)
                .collect()
        };
        readback.buffer.unmap();
        for ((cell, id, key), pixels) in readback.stills.iter().flatten().zip(pixels) {
            if pixels.iter().all(|byte| *byte == 0) {
                let mut shared = lock();
                if shared.cells[*cell].is_some_and(|kept| kept.id == *id) {
                    shared.cells[*cell] = None;
                }
                continue;
            }
            let Some(disk) = &mut self.blade_stills.disk else {
                return;
            };
            let (path, old) = disk.replace(id, *key);
            if let Some(worker) = self.still_worker() {
                let _ = worker.jobs.send(Job::Save { path, pixels, old });
            }
        }
    }

    /// With nothing asked for a while: free the atlas and the baking target.
    fn release_idle_stills(&mut self, now: Instant) {
        let idle = |since: Option<Instant>, limit| since.is_none_or(|at| now - at > limit);
        if self.blade_stills.baker.is_some() && idle(self.blade_stills.last_bake, BAKER_IDLE) {
            self.blade_stills.baker = None;
        }
        if self.blade_stills.atlas.is_some() && idle(self.blade_stills.last_wanted, IDLE) {
            self.blade_stills.atlas = None;
            self.ui_shapes.clear_blade_stills();
            lock().cells = [None; CELLS];
            self.blade_stills.queue.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: &'static str, used: u64) -> Option<Cell> {
        Some(Cell {
            id,
            key: 1,
            ready: true,
            used,
        })
    }

    #[test]
    fn texture_ids_name_their_cell_and_look() {
        for index in [0, 7, CELLS - 1] {
            assert_eq!(cell_of(texture(index, false)), Some((index, false)));
            assert_eq!(cell_of(texture(index, true)), Some((index, true)));
        }
        assert_eq!(cell_of(sjk_ui::TextureId(TEXTURE_BASE - 1)), None);
        assert_eq!(
            cell_of(sjk_ui::TextureId(TEXTURE_BASE + CELLS as u32)),
            None
        );
        assert_eq!(cell_of(sjk_ui::TextureId(TEXTURE_BASE + DIM * 2)), None);
        // Not another picture's range (the map editor's at 0x6200_0000, GIFs' 0x7000_0000).
        assert_eq!(cell_of(sjk_ui::TextureId(0x6200_0000)), None);
        assert_eq!(cell_of(sjk_ui::TextureId(0x7000_0000)), None);
        let (low, high) = uv_range(CELLS - 1);
        assert!(high[0] <= 1.0 && high[1] <= 1.0 && low[0] < high[0] && low[1] < high[1]);
        assert_eq!(
            cell_pixels(COLUMNS as usize),
            [0, STILL[1], STILL[0], STILL[1]]
        );
    }

    #[test]
    fn a_new_still_takes_a_free_cell_then_the_one_asked_least_lately() {
        let mut cells = [None; CELLS];
        assert_eq!(take_cell(&cells, 5), Some(0));
        for (index, slot) in cells.iter_mut().enumerate() {
            *slot = cell("saber_x", 5 + index as u64 % 3);
        }
        cells[9] = cell("saber_old", 2);
        assert_eq!(take_cell(&cells, 7), Some(9), "the one asked least lately");
        // Every cell asked for this frame: none is moved.
        for slot in &mut cells {
            *slot = cell("saber_x", 7);
        }
        assert_eq!(take_cell(&cells, 7), None);
    }

    /// At most the batch a frame, the ready ones in the order asked; the ones not ready
    /// wait in the queue, the ones no longer shown leave it.
    #[test]
    fn bakes_are_bounded_a_frame_and_wait_for_their_pipelines() {
        let ids: Vec<&'static str> = (0..10)
            .map(|n| &*Box::leak(format!("saber_{n}").into_boxed_str()))
            .collect();
        let mut queue = ids.clone();
        let wanted = &ids[..8];
        let chosen = choose_bakes(&mut queue, wanted, |id| id != "saber_1", BAKE_BATCH);
        assert_eq!(
            chosen,
            [
                Some("saber_0"),
                Some("saber_2"),
                Some("saber_3"),
                Some("saber_4")
            ]
        );
        assert_eq!(queue, ["saber_1", "saber_5", "saber_6", "saber_7"]);
        let chosen = choose_bakes(&mut queue, wanted, |_| true, BAKES_LOW);
        assert_eq!(chosen.iter().flatten().count(), 1);
        assert_eq!(chosen[0], Some("saber_1"));
        // Asking for more than the batch still bakes the batch at most.
        let mut queue = ids.clone();
        let chosen = choose_bakes(&mut queue, &ids, |_| true, 100);
        assert_eq!(chosen.iter().flatten().count(), BAKE_BATCH);
        assert_eq!(queue.len(), ids.len() - BAKE_BATCH);
    }

    #[test]
    fn keys_change_with_any_part_and_hold_across_runs() {
        let key = key_of(&[b"saber_sun", b"def", b"color"]);
        assert_eq!(key, key_of(&[b"saber_sun", b"def", b"color"]));
        assert_ne!(key, key_of(&[b"saber_sun", b"deg", b"color"]));
        assert_ne!(
            key,
            key_of(&[b"saber_sun", b"defcolor"]),
            "parts kept apart"
        );
        assert_ne!(key_of(&[b"12345678"]), key_of(&[b"12345678\0"]));
        // Pinned, so a build does not lose every player's stills by accident.
        assert_eq!(key_of(&[]), 0xcbf2_9ce4_8422_2325);
        assert_eq!(key_of(&[b"saber_sun"]), key_of(&[b"saber_sun"]));
    }

    #[test]
    fn file_names_carry_the_skin_and_its_key() {
        let name = file_name("saber_abyssal_eye", 0x0123_4567_89ab_cdef);
        assert_eq!(name, "saber_abyssal_eye-0123456789abcdef.png");
        assert_eq!(
            parse_file_name(&name),
            Some(("saber_abyssal_eye", 0x0123_4567_89ab_cdef))
        );
        assert_eq!(parse_file_name("saber_sun-12.png"), None);
        assert_eq!(parse_file_name("saber_sun-0123456789abcdef.png.part"), None);
        assert_eq!(parse_file_name("-0123456789abcdef.png"), None);
    }

    /// A changed blade has a new key: the still on disk no longer matches, and keeping
    /// the new one removes the old file.
    #[test]
    fn a_changed_blade_replaces_its_still_on_disk() {
        let folder = std::env::temp_dir().join(format!("sjk-blade-stills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("folder");
        let pixels = vec![200; STILL_BYTES as usize];
        let mut disk = Disk::read(folder.clone());
        let (path, old) = disk.replace("saber_sun", 1);
        assert_eq!(old, None);
        write_still(&path, pixels.clone()).expect("kept");
        let disk = Disk::read(folder.clone());
        assert!(disk.has("saber_sun", 1));
        assert!(!disk.has("saber_sun", 2), "the blade changed");
        assert_eq!(read_still(&disk.path("saber_sun", 1)), Some(pixels.clone()));
        let mut disk = disk;
        let (path, old) = disk.replace("saber_sun", 2);
        assert_eq!(
            old.as_deref(),
            Some(folder.join(file_name("saber_sun", 1)).as_path())
        );
        write_still(&path, pixels).expect("kept");
        if let Some(old) = old {
            std::fs::remove_file(old).expect("removed");
        }
        let disk = Disk::read(folder.clone());
        assert!(disk.has("saber_sun", 2));
        assert_eq!(disk.keys.len(), 1);
        // A file that is no still is read as none (baked again).
        std::fs::write(disk.path("saber_moon", 3), b"not a picture").expect("written");
        assert_eq!(read_still(&disk.path("saber_moon", 3)), None);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn every_baking_row_lies_inside_its_camera_and_row() {
        let matrix = bake_view_projection();
        for row in 0..BAKE_BATCH {
            let base = bake_base(row);
            for point in [base, base + Vec3::X * crate::blade_pictures::BLADE_LENGTH] {
                let ndc = (matrix * point.extend(1.0)).truncate();
                assert!(ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0, "{row}: {ndc}");
                let pixel_y = (1.0 - ndc.y) * 0.5 * (BAKE[1] * BAKE_BATCH as u32) as f32;
                let top = (row as u32 * BAKE[1]) as f32;
                assert!(
                    pixel_y > top && pixel_y < top + BAKE[1] as f32,
                    "{row}: {pixel_y}"
                );
            }
            // Far from every live picture's cell.
            assert!(base.x > 1_000.0);
        }
    }

    #[test]
    fn the_still_program_validates() {
        crate::wgsl_source::validate(include_str!("blade_stills.wgsl"));
    }
}
