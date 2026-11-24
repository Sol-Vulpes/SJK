//! The Collection's blade loops (Sol, 15/11/2026: "make blades look great on Collection
//! tiles", then "the sabers on the UI should be straight, maybe we should pre render a
//! gif or video"): every kyber drawn by the game's own blade renderer into a short
//! looping flipbook ([`FRAMES`] pictures of [`STILL`] pixels, [`FRAME_STEP`] seconds of
//! its animation apart), its glow added, kept on disk, and played on the wall's tiles,
//! the list's rows, the hall's and the Forge's strips and the cards' swatches, straight
//! across them; a swatch crossfades from one picture to the next, [`PLAY_FPS`] a second,
//! each kyber at its own place in its loop. Only the large swatch of the chosen blade
//! stays live ([`crate::blade_pictures::focus`]), once its own pipelines are built.
//!
//! A swatch asks for its skin's loop ([`still`]) while the UI is built. The frame then
//! ([`crate::GpuState::step_blade_stills`]) finds each loop asked for: in a slot of the
//! atlas already (a slot is [`FRAMES`] cells, one atlas row); on disk, read and decoded by
//! a thread of its own and uploaded, [`UPLOADS_PER_FRAME`] loops a frame at most; or
//! missing, then baked, [`BAKE_BATCH`] pictures a frame at most (half a loop; one picture
//! with the menu pictures on, `ui_menuPictures`): drawn by the blade renderer in the same
//! pass as the live pictures, each at its moment of the loop ([`STILL_SECONDS`] on), at
//! twice the picture's size, then brought down to it with a soft glow round the light
//! (`blade_stills.wgsl`) into its atlas cell; a loop whole is read back and written to
//! disk by that thread. A signature blade waits for its own pipelines (asked for at once,
//! the Collection's urgency) so no loop keeps the shared look a build stands in with.
//! While the Collection is open and nothing visible is waiting, the other loaded kybers
//! are baked too, one loop at a time, so a scroll finds its loop on disk (not with the
//! menu pictures on).
//!
//! A loop is kept as `<id>-<key>.png` in `blades/` of the cache folder beside the settings
//! (`GameData/SJK/cache/blades`, [`set_folder`]; the off-screen renders keep none unless a
//! test names one): its pictures one under the other, RGB (the alpha is the brightest
//! channel, worked out again when read). Its key ([`skin_key`]) hashes what the picture is
//! drawn from: the skin's file as loaded, its colour as worn (the player's own options, a
//! chroma's turn), its glow and core images, its signature's shader and file, the blade
//! shader (`saber.wgsl`) and this baker ([`VERSION`], `blade_stills.wgsl`): any change
//! gives a new key, and a new loop replaces the old file. The slots go to the loops asked
//! for, the one asked least lately making room ([`take_slot`]); a loop asked for this
//! frame is never moved. With nothing asked for [`IDLE`], the atlas is freed, as the
//! baking target is [`BAKER_IDLE`] after its last bake.
//!
//! Video memory, a fixed budget for 3 GB cards (15/11/2026): the atlas is 3072 x 2304
//! RGBA8, 27 MiB ([`LOOPS`] loops of [`FRAMES`] pictures), only while the Collection or a
//! strip shows; the baking target 768 x 768 (RGBA16F) with its depth, 6.75 MiB, only while
//! baking. With the menu pictures on (a card short of memory) a kyber shows its first
//! picture only, still: the atlas holds [`LOW_SLOTS`] of them in 3072 x 576, 6.75 MiB, and
//! no live blade is drawn; a picture baked then is not kept (a loop read from disk gives
//! its first picture).

use crate::camera_uniform::CameraUniform;
use glam::Vec3;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// One picture of a loop, in pixels.
pub(crate) const STILL: [u32; 2] = [384, 96];
/// Pictures in a loop, and the seconds of the blade's animation between two of them.
pub(crate) const FRAMES: usize = 8;
const FRAME_STEP: f64 = 0.1;
/// Pictures a second a swatch plays its loop at (crossfaded from one to the next).
const PLAY_FPS: f32 = 10.0;
/// The atlas's cells: eight columns (one slot a row with the loops).
const COLUMNS: u32 = 8;
/// Loops the atlas holds; with the menu pictures on, first pictures.
pub(crate) const LOOPS: usize = 24;
pub(crate) const LOW_SLOTS: usize = 48;
/// The most slots, and cells, either holds.
const SLOTS: usize = if LOOPS > LOW_SLOTS { LOOPS } else { LOW_SLOTS };
// Every slot either layout uses fits the shared table (checked as the crate builds).
const _: () = assert!(LOOPS <= SLOTS && LOW_SLOTS <= SLOTS);
pub(crate) const CELLS: usize = LOOPS * FRAMES;
/// The atlas's width in pixels (its height is its layout's, [`Layout::atlas`]).
const ATLAS_WIDTH: u32 = STILL[0] * COLUMNS;
/// What a picture shows, in world units: a live picture's cell across
/// ([`crate::blade_pictures::BLADE_START`] before the blade, room after its tip) and its
/// middle this high.
pub(crate) const UNITS: [f32; 2] = [52.0, 13.0];
/// The baking target draws each picture this many times its size.
const SUPERSAMPLE: u32 = 2;
const BAKE: [u32; 2] = [STILL[0] * SUPERSAMPLE, STILL[1] * SUPERSAMPLE];
/// Rows of the baking target, blades drawn a frame at most: two a picture, its moment of
/// the loop and the one a whole loop later ([`LOOP_SECONDS`]), blended so that the loop
/// runs on from its last picture into its first without a jump (`blade_stills.wgsl`).
pub(crate) const BAKE_BATCH: usize = 4;
/// Pictures baked a frame at most; with the menu pictures on, [`BAKES_LOW`].
const PICTURES_A_FRAME: usize = BAKE_BATCH / 2;
const BAKES_LOW: usize = 1;
/// A loop's length in the blade's animation (seconds).
const LOOP_SECONDS: f64 = FRAMES as f64 * FRAME_STEP;
/// Skins' keys worked out a frame at most (each hashes its file and images).
const KEYS_PER_FRAME: usize = 24;
/// Loops read from disk uploaded a frame at most.
const UPLOADS_PER_FRAME: usize = 3;
/// The moment of the blades' animation a loop starts at (seconds).
const STILL_SECONDS: f64 = 2.4;
/// How long the atlas is kept with no loop asked for, and the baking target after its
/// last bake.
const IDLE: Duration = Duration::from_secs(10);
const BAKER_IDLE: Duration = Duration::from_secs(5);
/// The baker's version: raise it when a loop's look changes in a way the key does not see.
pub(crate) const VERSION: u32 = 4;
/// The folder in the cache folder the loops go in.
pub(crate) const FOLDER: &str = "blades";
/// The texture ids: cell `n` is `TEXTURE_BASE + n`, its dimmed look (not held) `+ DIM`.
const TEXTURE_BASE: u32 = 0x6300_0000;
const DIM: u32 = 0x200;
/// Where the baked blades lie in the world, far from the live pictures' cells.
const BAKE_X: f32 = 10_000.0;

/// How the atlas is laid out: pictures a slot and slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    frames: usize,
    slots: usize,
}

impl Layout {
    /// Loops; with the menu pictures on (not `live`), first pictures only.
    fn of(live: bool) -> Self {
        if live {
            Self {
                frames: FRAMES,
                slots: LOOPS,
            }
        } else {
            Self {
                frames: 1,
                slots: LOW_SLOTS,
            }
        }
    }

    /// The atlas's size in pixels.
    fn atlas(self) -> [u32; 2] {
        let cells = (self.frames * self.slots) as u32;
        [ATLAS_WIDTH, cells.div_ceil(COLUMNS) * STILL[1]]
    }

    /// The cell of picture `frame` of slot `slot`.
    fn cell(self, slot: usize, frame: usize) -> usize {
        slot * self.frames + frame
    }
}

/// The atlas's height in pixels, for the UI's texture coordinates.
static ATLAS_HEIGHT: AtomicU32 = AtomicU32::new(STILL[1] * (CELLS as u32 / COLUMNS));

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
    let atlas_height = ATLAS_HEIGHT.load(Ordering::Relaxed).max(1) as f32;
    let uv = |x: u32, y: u32| [x as f32 / ATLAS_WIDTH as f32, y as f32 / atlas_height];
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

/// A slot of the atlas: whose loop it holds and when it was last asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Slot {
    id: &'static str,
    key: u64,
    /// Pictures of it in the atlas; swatches show it once all are.
    baked: usize,
    /// Every picture is in the atlas: swatches show it.
    ready: bool,
    /// The frame it was last asked for in.
    used: u64,
}

/// What the UI asks for and the slots, shared by the UI and the frame.
struct Shared {
    wanted: Vec<&'static str>,
    slots: [Option<Slot>; SLOTS],
    layout: Layout,
    frame: u64,
    /// The menu clock the swatches play their loops by (seconds).
    seconds: f32,
    /// While kybers are being baked: how many loaded ones are on disk, of how many
    /// ([`progress`]).
    progress: Option<(usize, usize)>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    wanted: Vec::new(),
    slots: [None; SLOTS],
    layout: Layout {
        frames: FRAMES,
        slots: LOOPS,
    },
    frame: 0,
    seconds: 0.0,
    progress: None,
});

/// How far the first baking of the kybers' loops has come, while it runs: the loaded kybers
/// kept on disk and how many there are. The Collection says so (Sol, 23/11/2026, after
/// players' reports of single-digit FPS in it on 2026.1123.1, the first release with the
/// loops: every kyber is baked once, then read from disk). None once none is baking.
pub(crate) fn progress() -> Option<(usize, usize)> {
    #[cfg(test)]
    if let Some(pretend) = *PRETEND_PROGRESS.lock().unwrap_or_else(|e| e.into_inner()) {
        return Some(pretend);
    }
    lock().progress
}

/// A progress the world shots show whatever the baking does.
#[cfg(test)]
pub(crate) static PRETEND_PROGRESS: Mutex<Option<(usize, usize)>> = Mutex::new(None);

/// How long after its last bake the baking still counts as running ([`progress`]), so the
/// notice does not blink between two loops.
const BAKING_SHOWS: Duration = Duration::from_millis(1500);
/// Frames between two counts of the loops on disk ([`progress`]).
const PROGRESS_EVERY: u64 = 30;

fn lock() -> std::sync::MutexGuard<'static, Shared> {
    SHARED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The folder the loops are kept in: none, until the windowed client names one.
static FOLDER_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Keep loops in `folder` (the client's `cache/blades`), or none.
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

/// What a swatch draws of a loop now: picture `from` fading into picture `to` by `mix`
/// (0 shows `from` alone).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) from: sjk_ui::TextureId,
    pub(crate) to: sjk_ui::TextureId,
    pub(crate) mix: f32,
}

/// Where skin `id` starts its loop, in pictures: each kyber at its own place, so a wall
/// does not breathe as one.
fn offset_of(id: &str) -> f32 {
    let hash = id.bytes().fold(0_u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u32::from(byte))
    });
    (hash % 997) as f32 * 0.618_034
}

/// The pictures of a loop of `frames` shown at `seconds`, starting `offset` pictures in.
fn playing(frames: usize, seconds: f32, offset: f32) -> (usize, usize, f32) {
    if frames <= 1 {
        return (0, 0, 0.0);
    }
    let at = (seconds.max(0.0) * PLAY_FPS + offset).rem_euclid(frames as f32);
    let from = (at.floor() as usize).min(frames - 1);
    (from, (from + 1) % frames, at - at.floor())
}

/// The loop of skin `id` for a swatch, once it is in the atlas: what to draw of it now,
/// dimmed (desaturated, for a kyber not held) when `dim`. Asking keeps it there, or reads
/// or bakes it.
pub(crate) fn still(id: &'static str, dim: bool) -> Option<Shown> {
    let mut shared = lock();
    if !shared.wanted.contains(&id) {
        shared.wanted.push(id);
    }
    let slot = shared
        .slots
        .iter()
        .position(|slot| slot.is_some_and(|slot| slot.ready && slot.id == id))?;
    let layout = shared.layout;
    let (from, to, mix) = playing(layout.frames, shared.seconds, offset_of(id));
    Some(Shown {
        from: texture(layout.cell(slot, from), dim),
        to: texture(layout.cell(slot, to), dim),
        mix,
    })
}

/// The loops' clock for the world shots: `Some` plays every loop at those seconds instead of
/// the real time, so a run of frames steps it exactly.
#[cfg(test)]
pub(crate) static SHOT_CLOCK: Mutex<Option<f32>> = Mutex::new(None);

/// Forget every loop in memory, as a client starting again (the world shots read them
/// back from disk).
#[cfg(test)]
pub(crate) fn forget_for_shot(stills: &mut Stills) {
    lock().slots = [None; SLOTS];
    *stills = Stills::default();
}

/// Loops the last frame asked for that are not shown yet (being read, waiting for their
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
                .slots
                .iter()
                .flatten()
                .any(|slot| slot.ready && slot.id == **id)
        })
        .count()
}

/// A slot for a new loop at `frame`, among the first `count`: a free one, else the one
/// asked for least lately, never one asked for this frame.
fn take_slot(slots: &[Option<Slot>], count: usize, frame: u64) -> Option<usize> {
    let slots = &slots[..count.min(slots.len())];
    if let Some(free) = slots.iter().position(Option::is_none) {
        return Some(free);
    }
    slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| slot.map(|slot| (index, slot.used)))
        .filter(|&(_, used)| used < frame)
        .min_by_key(|&(_, used)| used)
        .map(|(index, _)| index)
}

/// From the loops waiting to be baked (`queue`, in the order asked), the next to bake: the
/// first still `wanted` whose blade is `ready` to draw as it looks. The ones no longer
/// wanted leave the queue; the ones not ready stay in it.
fn next_bake(
    queue: &mut Vec<&'static str>,
    wanted: &[&'static str],
    ready: impl Fn(&'static str) -> bool,
) -> Option<&'static str> {
    queue.retain(|id| wanted.contains(id));
    let index = queue.iter().position(|&id| ready(id))?;
    Some(queue.remove(index))
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

/// A loop's file name.
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

/// The loops kept on disk: each skin's key, as the folder's files name them.
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
    /// Read and decode a loop.
    Load {
        id: &'static str,
        key: u64,
        path: PathBuf,
    },
    /// Write a loop (RGBA, its [`FRAMES`] pictures one under the other) to `path`, and
    /// remove the file it replaces.
    Save {
        path: PathBuf,
        pixels: Vec<u8>,
        old: Option<PathBuf>,
    },
}

/// A loop read: its pixels (RGBA, its pictures one under the other), none when the file
/// was damaged or gone.
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

/// A loop file's size in pixels: its pictures one under the other.
const LOOP_PIXELS: [u32; 2] = [STILL[0], STILL[1] * FRAMES as u32];

/// A loop's pixels from `path` (RGBA, the alpha its brightest channel as baked), if it is
/// a loop.
fn read_still(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .ok()?
        .into_rgb8();
    if image.dimensions() != (LOOP_PIXELS[0], LOOP_PIXELS[1]) {
        return None;
    }
    let mut pixels = Vec::with_capacity(image.as_raw().len() / 3 * 4);
    for rgb in image.as_raw().chunks_exact(3) {
        pixels.extend_from_slice(rgb);
        pixels.push(rgb[0].max(rgb[1]).max(rgb[2]));
    }
    Some(pixels)
}

/// Write a loop to `path` (RGB: its alpha is its brightest channel), through a temporary
/// file renamed over it.
fn write_still(path: &Path, pixels: Vec<u8>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let rgb: Vec<u8> = pixels
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    let image = image::RgbImage::from_raw(LOOP_PIXELS[0], LOOP_PIXELS[1], rgb)
        .ok_or_else(|| std::io::Error::other("wrong size"))?;
    let temporary = path.with_extension("png.part");
    image
        .save_with_format(&temporary, image::ImageFormat::Png)
        .map_err(std::io::Error::other)?;
    std::fs::rename(&temporary, path)
}

/// A picture baked this frame: picture `frame` of skin `id`'s loop worn as `color`, into
/// slot `slot`'s cell `cell`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bake {
    cell: usize,
    slot: usize,
    frame: usize,
    id: &'static str,
    key: u64,
    color: crate::saber_skins::SkinColor,
}

/// This frame's bakes, each in two of the baking target's rows.
pub(crate) type Bakes = [Option<Bake>; PICTURES_A_FRAME];

/// Where row `row`'s blade starts, in world units (y = 0 the plane, x across, z up).
fn bake_base(row: usize) -> Vec3 {
    Vec3::new(
        BAKE_X + crate::blade_pictures::BLADE_START,
        0.0,
        -(row as f32 + 0.5) * UNITS[1],
    )
}

/// What the still shader blends for picture `frame` of a loop of `frames` from rows pair
/// `pair` (`blade_stills.wgsl` reads it back from the instance index): `frame / frames` of
/// the first row, its moment, and the rest of the second, a loop later.
fn still_pick(pair: usize, frame: usize, frames: usize) -> u32 {
    ((pair * 32 + frame.min(31)) * 32 + frames.clamp(1, 31)) as u32
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
    fn new(device: &wgpu::Device, layout: Layout) -> Self {
        let [width, height] = layout.atlas();
        ATLAS_HEIGHT.store(height, Ordering::Relaxed);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SJK blade stills"),
            size: wgpu::Extent3d {
                width,
                height,
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

/// Bytes of one picture read back, and of a loop.
const STILL_BYTES: u64 = STILL[0] as u64 * 4 * STILL[1] as u64;
const LOOP_BYTES: u64 = STILL_BYTES * FRAMES as u64;

/// A frame's whole loops being read back to be kept on disk.
struct Readback {
    buffer: wgpu::Buffer,
    /// Each loop in the buffer, in order: its slot, skin and key.
    stills: [Option<(usize, &'static str, u64)>; BAKE_BATCH],
    /// The frame that copies them ([`crate::frame_split::Splitter::recording`]): mapped
    /// once it has been submitted.
    frame: u64,
    /// Set once mapping was asked for.
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
    /// Loops to bake, in the order asked; loops being read; the pictures of the loops
    /// begun, still to bake.
    queue: Vec<&'static str>,
    pending: Vec<Bake>,
    loading: Vec<&'static str>,
    /// Skins whose signature pipelines have been asked for.
    asked: HashSet<&'static str>,
    worker: Option<Worker>,
    atlas: Option<Atlas>,
    baker: Option<Baker>,
    readbacks: Vec<Readback>,
    /// The atlas's layout (loops, or first pictures with the menu pictures on).
    layout: Option<Layout>,
    /// When the stills began, for the swatches' clock.
    epoch: Option<Instant>,
    last_wanted: Option<Instant>,
    last_bake: Option<Instant>,
    /// The next loaded skin the background baking looks at, and whether a round left one
    /// waiting for its pipelines.
    cursor: usize,
    skipped: bool,
    /// The last count of the loaded kybers kept on disk, and how many there are.
    kept: (usize, usize),
    /// Stills baked so far (the world shots time them).
    #[cfg(test)]
    pub(crate) baked: u64,
}

impl crate::GpuState {
    /// This frame's pictures: take the swatches' wishes, keep the slots they hold, read the
    /// loops on disk and plan the bakes (drawn by [`Self::encode_blade_pictures`]).
    pub(crate) fn step_blade_stills(&mut self) -> Bakes {
        let now = Instant::now();
        self.poll_still_readbacks();
        let layout = Layout::of(crate::blade_pictures::live());
        if self.blade_stills.layout != Some(layout) {
            // The menu pictures switched: the atlas is laid out again.
            self.blade_stills.layout = Some(layout);
            self.blade_stills.atlas = None;
            self.ui_shapes.clear_blade_stills();
            self.blade_stills.pending.clear();
            self.blade_stills.queue.clear();
            let mut shared = lock();
            shared.slots = [None; SLOTS];
            shared.layout = layout;
        }
        let epoch = *self.blade_stills.epoch.get_or_insert(now);
        let frame = {
            let mut shared = lock();
            shared.frame += 1;
            shared.seconds = (now - epoch).as_secs_f32();
            #[cfg(test)]
            if let Some(seconds) = *SHOT_CLOCK.lock().unwrap_or_else(|e| e.into_inner()) {
                shared.seconds = seconds;
            }
            std::mem::swap(&mut shared.wanted, &mut self.blade_stills.wanted);
            shared.wanted.clear();
            shared.frame
        };
        let mut bakes: Bakes = [None; PICTURES_A_FRAME];
        if self.blade_stills.wanted.is_empty() {
            lock().progress = None;
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
            // when the player looks at it (the Collection's urgency while its loop waits).
            let missing = !self
                .blade_stills
                .disk
                .as_ref()
                .is_some_and(|disk| disk.has(id, key));
            self.ask_still_build(id, missing);
            {
                let mut shared = lock();
                let held = shared
                    .slots
                    .iter()
                    .position(|slot| slot.is_some_and(|slot| slot.id == id));
                if let Some(index) = held {
                    if let Some(slot) = &mut shared.slots[index]
                        && slot.key == key
                    {
                        slot.used = frame;
                        continue;
                    }
                    // Its look changed: the slot is free again.
                    shared.slots[index] = None;
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
        self.upload_loaded_stills(frame, layout);
        let live = crate::blade_pictures::live();
        let max = if live { PICTURES_A_FRAME } else { BAKES_LOW };
        // The pictures of a loop begun whose slot still holds it, then new loops.
        {
            let shared = lock();
            self.blade_stills.pending.retain(|bake| {
                shared.slots[bake.slot]
                    .is_some_and(|slot| slot.id == bake.id && slot.key == bake.key)
            });
        }
        let skins = std::sync::Arc::clone(&self.blade_skins);
        while self.blade_stills.pending.len() < max {
            let Some(id) = next_bake(&mut self.blade_stills.queue, &wanted, |id| {
                skins
                    .index_of(id)
                    .is_some_and(|index| self.saber_gpu.skin_ready(index))
            }) else {
                break;
            };
            if !self.plan_bake(id, frame, layout) {
                break;
            }
        }
        // Nothing on screen waiting: the next kyber not on disk, while there is a disk.
        if self.blade_stills.pending.is_empty() && live && self.blade_stills.queue.is_empty() {
            self.plan_background_bake(frame, layout, &mut keys_left);
        }
        let take = self.blade_stills.pending.len().min(max);
        for (row, bake) in self.blade_stills.pending.drain(..take).enumerate() {
            bakes[row] = Some(bake);
        }
        self.blade_stills.wanted = wanted;
        if bakes.iter().any(Option::is_some) {
            self.blade_stills.last_bake = Some(now);
        }
        self.note_progress(frame, now);
        bakes
    }

    /// Count the loaded kybers kept on disk now and then, for [`progress`], while baking.
    fn note_progress(&mut self, frame: u64, now: Instant) {
        let baking = self
            .blade_stills
            .last_bake
            .is_some_and(|at| now - at < BAKING_SHOWS);
        if !baking {
            lock().progress = None;
            return;
        }
        if frame % PROGRESS_EVERY == 0 || self.blade_stills.kept.1 == 0 {
            let disk = self.blade_stills.disk.as_ref();
            let keys = &self.blade_stills.keys;
            let mut kept = 0;
            let mut total = 0;
            for (_, skin) in self.blade_skins.iter() {
                total += 1;
                let on_disk = keys
                    .get(skin.id)
                    .is_some_and(|&(_, key)| disk.is_some_and(|disk| disk.has(skin.id, key)));
                kept += usize::from(on_disk);
            }
            self.blade_stills.kept = (kept, total);
        }
        let (kept, total) = self.blade_stills.kept;
        lock().progress = (total > 0 && kept < total).then_some((kept, total));
    }

    /// Skin `id`'s loop key, worked out once per loaded generation, `budget` a frame; none
    /// while its pack is not loaded or the budget is spent.
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

    /// Ask, once, for skin `id`'s signature pipelines: soon when its loop is to be baked,
    /// else with the lowest urgency.
    fn ask_still_build(&mut self, id: &'static str, soon: bool) {
        if !self.blade_stills.asked.insert(id) {
            return;
        }
        if let Some(index) = self.blade_skins.index_of(id) {
            self.saber_gpu.ask_build(index, soon);
        }
    }

    /// The loops' thread, started the first time it is needed.
    fn still_worker(&mut self) -> Option<&Worker> {
        if self.blade_stills.worker.is_none() {
            self.blade_stills.worker = Worker::start();
        }
        self.blade_stills.worker.as_ref()
    }

    /// The atlas, made the first time a loop needs it and bound for the UI.
    fn still_atlas(&mut self, layout: Layout) -> &Atlas {
        if self.blade_stills.atlas.is_none() {
            let atlas = Atlas::new(&self.device, layout);
            self.ui_shapes.set_blade_stills(&self.device, &atlas.view);
            self.blade_stills.atlas = Some(atlas);
        }
        self.blade_stills
            .atlas
            .as_ref()
            .unwrap_or_else(|| unreachable!("made above"))
    }

    /// Upload the loops the thread has read, a few a frame (their first picture only in
    /// `layout` of first pictures); a damaged one is baked.
    fn upload_loaded_stills(&mut self, frame: u64, layout: Layout) {
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
            let Some(slot) = take_slot(&lock().slots, layout.slots, frame) else {
                continue;
            };
            let _ = self.still_atlas(layout);
            let Some(atlas) = &self.blade_stills.atlas else {
                return;
            };
            for (picture, bytes) in pixels
                .chunks_exact(STILL_BYTES as usize)
                .take(layout.frames)
                .enumerate()
            {
                let [x, y, width, height] = cell_pixels(layout.cell(slot, picture));
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &atlas.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x, y, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytes,
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
            }
            lock().slots[slot] = Some(Slot {
                id: loaded.id,
                key: loaded.key,
                baked: layout.frames,
                ready: true,
                used: frame,
            });
        }
    }

    /// Begin skin `id`'s loop in a slot taken for it: its pictures go to the pending bakes.
    /// False when its pack is not loaded or no slot is free.
    fn plan_bake(&mut self, id: &'static str, frame: u64, layout: Layout) -> bool {
        let Some(key) = self.blade_stills.keys.get(id).map(|kept| kept.1) else {
            return false;
        };
        let Some(color) = self
            .blade_skins
            .color_with(id, self.looks.own_options_off(id))
        else {
            return false;
        };
        let mut shared = lock();
        let Some(slot) = take_slot(&shared.slots, layout.slots, frame) else {
            return false;
        };
        shared.slots[slot] = Some(Slot {
            id,
            key,
            baked: 0,
            ready: false,
            used: frame,
        });
        drop(shared);
        self.blade_stills.pending.retain(|bake| bake.slot != slot);
        self.blade_stills
            .pending
            .extend((0..layout.frames).map(|picture| Bake {
                cell: layout.cell(slot, picture),
                slot,
                frame: picture,
                id,
                key,
                color,
            }));
        true
    }

    /// While the Collection is open and nothing on screen waits: begin the loop of the next
    /// loaded kyber that has none on disk, so a scroll finds it kept. Only where loops are
    /// kept. A signature whose pipelines are not built is asked for with the lowest urgency
    /// and looked at again on the next round.
    fn plan_background_bake(&mut self, frame: u64, layout: Layout, budget: &mut usize) {
        if self.blade_stills.disk.is_none() {
            return;
        }
        let skins = std::sync::Arc::clone(&self.blade_skins);
        let count = skins.iter().count();
        while *budget > 0 {
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
            let held = lock().slots.iter().flatten().any(|slot| slot.id == id);
            if kept || held {
                continue;
            }
            if !self.saber_gpu.skin_ready(index) {
                self.ask_still_build(id, false);
                self.blade_stills.skipped = true;
                continue;
            }
            let _ = self.plan_bake(id, frame, layout);
            return;
        }
    }

    /// The blade instances of this frame's `bakes`, appended to `blades`: each picture's
    /// blade twice, in its two baking rows, at its moment of the loop and a whole loop
    /// later; which pictures got both.
    pub(crate) fn push_still_blades(
        bakes: &Bakes,
        blades: &mut Vec<crate::saber::Instance>,
        per_blade: usize,
    ) -> [bool; PICTURES_A_FRAME] {
        let mut drawn = [false; PICTURES_A_FRAME];
        for (pair, bake) in bakes.iter().enumerate() {
            let Some(bake) = bake else {
                continue;
            };
            let color = crate::saber_rgb::BladeColor::Skin(bake.color);
            let moment = STILL_SECONDS + bake.frame as f64 * FRAME_STEP;
            drawn[pair] =
                [moment, moment + LOOP_SECONDS]
                    .into_iter()
                    .enumerate()
                    .all(|(later, seconds)| {
                        let start = blades.len();
                        blades.extend(crate::blade_pictures::blade_instances(
                            bake_base(pair * 2 + later),
                            color,
                            seconds,
                            0,
                            per_blade,
                        ));
                        blades.len() > start
                    });
        }
        drawn
    }

    /// Draw this frame's `bakes` (their blades already uploaded with the live pictures')
    /// into the baking target, make their pictures in the atlas, and copy the loops they
    /// finish out to be kept.
    pub(crate) fn encode_blade_stills(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        bakes: &Bakes,
        drawn: [bool; PICTURES_A_FRAME],
    ) {
        if bakes.iter().all(Option::is_none) {
            return;
        }
        let layout = self
            .blade_stills
            .layout
            .unwrap_or(Layout::of(crate::blade_pictures::live()));
        let _ = self.still_atlas(layout);
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
            for (pair, bake) in bakes.iter().enumerate() {
                let Some(bake) = bake else {
                    continue;
                };
                let [x, y, width, height] = cell_pixels(bake.cell);
                pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);
                pass.set_scissor_rect(x, y, width, height);
                let pick = still_pick(pair, bake.frame, layout.frames);
                pass.draw(0..3, pick..pick + 1);
            }
        }
        #[cfg(test)]
        {
            self.blade_stills.baked += drawn.iter().filter(|drawn| **drawn).count() as u64;
        }
        // Whole loops are kept (not the first pictures alone).
        let keeping = self.blade_stills.disk.is_some() && layout.frames == FRAMES;
        let mut finished = [None; BAKE_BATCH];
        let mut count = 0;
        {
            let mut shared = lock();
            for (bake, drawn) in bakes.iter().zip(drawn) {
                let Some(bake) = bake else {
                    continue;
                };
                let Some(slot) = &mut shared.slots[bake.slot] else {
                    continue;
                };
                if slot.id != bake.id || slot.key != bake.key {
                    continue;
                }
                if !drawn {
                    // Nothing drawn (its pack went): asked again, baked again.
                    shared.slots[bake.slot] = None;
                    continue;
                }
                slot.baked += 1;
                if slot.baked >= layout.frames && !slot.ready {
                    slot.ready = true;
                    if keeping {
                        finished[count] = Some((bake.slot, bake.id, bake.key));
                        count += 1;
                    }
                }
            }
        }
        if count == 0 {
            return;
        }
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK blade stills readback"),
            size: LOOP_BYTES * count as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        for (order, (slot, ..)) in finished.iter().flatten().enumerate() {
            for picture in 0..FRAMES {
                let [x, y, width, height] = cell_pixels(layout.cell(*slot, picture));
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
                            offset: LOOP_BYTES * order as u64 + STILL_BYTES * picture as u64,
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
        }
        self.blade_stills.readbacks.push(Readback {
            buffer,
            stills: finished,
            frame: self.frame_pacer.split.recording(),
            receiver: None,
        });
    }

    /// Map the loops copied out by frames now submitted, and keep the ones mapped.
    fn poll_still_readbacks(&mut self) {
        if self.blade_stills.readbacks.is_empty() {
            return;
        }
        // A buffer mapped before the frame copying into it reaches `Queue::submit` (on the
        // submit thread) fails that submit: each waits for its frame, a frame or two, without
        // holding this one (waiting here, every other frame while baking, made the menus
        // judder with vsync).
        let split = &self.frame_pacer.split;
        for readback in &mut self.blade_stills.readbacks {
            if readback.receiver.is_none() && split.submitted(readback.frame) {
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

    /// Send the loops of a mapped readback to the thread to be written; a loop left black
    /// (frames that were never submitted) is baked again. One black picture is kept: a
    /// blade that flickers out may be dark at one of its moments, and baking it again
    /// would never end.
    fn keep_stills(&mut self, readback: &Readback) {
        let loops: Vec<Vec<u8>> = {
            let Ok(view) = readback.buffer.slice(..).get_mapped_range() else {
                return;
            };
            view.chunks_exact(LOOP_BYTES as usize)
                .map(<[u8]>::to_vec)
                .collect()
        };
        readback.buffer.unmap();
        for ((slot, id, key), pixels) in readback.stills.iter().flatten().zip(loops) {
            let black = pixels.iter().all(|byte| *byte == 0);
            if black {
                let mut shared = lock();
                if shared.slots[*slot].is_some_and(|kept| kept.id == *id) {
                    shared.slots[*slot] = None;
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
            lock().slots = [None; SLOTS];
            self.blade_stills.queue.clear();
            self.blade_stills.pending.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(id: &'static str, used: u64) -> Option<Slot> {
        Some(Slot {
            id,
            key: 1,
            baked: FRAMES,
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
        // Not another picture's range (the map editor's at 0x6200_0000, the live
        // pictures' crops at 0x6300_0400, GIFs' 0x7000_0000).
        assert_eq!(cell_of(sjk_ui::TextureId(0x6200_0000)), None);
        assert_eq!(cell_of(sjk_ui::TextureId(0x6300_0400)), None);
        assert_eq!(cell_of(sjk_ui::TextureId(0x7000_0000)), None);
        let (low, high) = uv_range(CELLS - 1);
        assert!(high[0] <= 1.0 && high[1] <= 1.0 && low[0] < high[0] && low[1] < high[1]);
        assert_eq!(
            cell_pixels(COLUMNS as usize),
            [0, STILL[1], STILL[0], STILL[1]]
        );
    }

    /// The budget for a 3 GB card: the loops' atlas 27 MiB, the first pictures' 6.75 MiB,
    /// within the widest texture any card takes (wgpu's 8192 default limit).
    #[test]
    fn the_atlases_keep_their_video_memory_budget() {
        let bytes = |layout: Layout| {
            let [width, height] = layout.atlas();
            assert!(width <= 8192 && height <= 8192, "{width}x{height}");
            u64::from(width) * u64::from(height) * 4
        };
        let loops = Layout::of(true);
        let firsts = Layout::of(false);
        assert_eq!(bytes(loops), 27 << 20);
        assert_eq!(bytes(firsts), 27 << 18);
        // A loop's pictures fill one atlas row: its cells lie side by side.
        assert_eq!(loops.cell(3, 0) as u32 % COLUMNS, 0);
        assert_eq!(loops.cell(3, FRAMES - 1), loops.cell(3, 0) + FRAMES - 1);
        assert!(loops.cell(LOOPS - 1, FRAMES - 1) < CELLS);
        assert!(firsts.cell(LOW_SLOTS - 1, 0) < CELLS);
    }

    /// A loop plays through its pictures, fading from one to the next, and comes round;
    /// a first picture alone stands still.
    #[test]
    fn a_loop_plays_its_pictures_round() {
        assert_eq!(playing(FRAMES, 0.0, 0.0), (0, 1, 0.0));
        let (from, to, mix) = playing(FRAMES, 0.25 / PLAY_FPS, 0.0);
        assert_eq!((from, to), (0, 1));
        assert!((mix - 0.25).abs() < 1e-4);
        let (from, to, _) = playing(FRAMES, (FRAMES as f32 - 0.5) / PLAY_FPS, 0.0);
        assert_eq!((from, to), (FRAMES - 1, 0), "the last fades into the first");
        let (from, ..) = playing(FRAMES, FRAMES as f32 / PLAY_FPS, 0.0);
        assert_eq!(from, 0, "round again");
        assert_eq!(playing(1, 12.3, 4.0), (0, 0, 0.0));
        assert_ne!(offset_of("saber_sun"), offset_of("saber_moon"));
    }

    #[test]
    fn a_new_loop_takes_a_free_slot_then_the_one_asked_least_lately() {
        let mut slots = [None; SLOTS];
        assert_eq!(take_slot(&slots, LOOPS, 5), Some(0));
        for (index, kept) in slots.iter_mut().enumerate() {
            *kept = slot("saber_x", 5 + index as u64 % 3);
        }
        slots[9] = slot("saber_old", 2);
        assert_eq!(
            take_slot(&slots, LOOPS, 7),
            Some(9),
            "the one asked least lately"
        );
        // Only the layout's slots are taken.
        slots[9] = slot("saber_x", 7);
        slots[LOOPS] = slot("saber_older", 1);
        assert_ne!(take_slot(&slots, LOOPS, 7), Some(LOOPS));
        // Every slot asked for this frame: none is moved.
        for kept in &mut slots {
            *kept = slot("saber_x", 7);
        }
        assert_eq!(take_slot(&slots, LOOPS, 7), None);
    }

    /// The next loop baked is the first asked whose blade is ready; the ones not ready
    /// wait in the queue, the ones no longer shown leave it.
    #[test]
    fn bakes_wait_for_their_pipelines_in_the_order_asked() {
        let ids: Vec<&'static str> = (0..10)
            .map(|n| &*Box::leak(format!("saber_{n}").into_boxed_str()))
            .collect();
        let mut queue = ids.clone();
        let wanted = &ids[..8];
        let ready = |id: &'static str| id != "saber_0";
        assert_eq!(next_bake(&mut queue, wanted, ready), Some("saber_1"));
        assert_eq!(
            queue,
            [
                "saber_0", "saber_2", "saber_3", "saber_4", "saber_5", "saber_6", "saber_7"
            ]
        );
        assert_eq!(next_bake(&mut queue, wanted, |_| true), Some("saber_0"));
        let mut empty = Vec::new();
        assert_eq!(next_bake(&mut empty, wanted, |_| true), None);
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
        // Alpha is the brightest channel, as baked: kept RGB, read back the same.
        let pixels: Vec<u8> = (0..LOOP_BYTES / 4)
            .flat_map(|n| {
                let rgb = [(n % 251) as u8, 40, (n % 7) as u8 * 30];
                [rgb[0], rgb[1], rgb[2], rgb[0].max(rgb[1]).max(rgb[2])]
            })
            .collect();
        let mut disk = Disk::read(folder.clone());
        let (path, old) = disk.replace("saber_sun", 1);
        assert_eq!(old, None);
        write_still(&path, pixels.clone()).expect("kept");
        // A single picture (a version 1 still) is no loop.
        assert!(write_still(&disk.path("saber_old", 9), vec![0; STILL_BYTES as usize]).is_err());
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

    /// Each picture's two rows and blend reach the still shader through the instance index
    /// (`still_pick`, read back as `blade_stills.wgsl` reads it).
    #[test]
    fn a_picture_names_its_rows_and_blend_to_the_shader() {
        for pair in 0..PICTURES_A_FRAME {
            for frame in 0..FRAMES {
                let pick = still_pick(pair, frame, FRAMES);
                assert_eq!(pick / 1024, pair as u32);
                assert_eq!((pick / 32) % 32, frame as u32);
                assert_eq!(pick % 32, FRAMES as u32);
            }
        }
        assert_eq!(still_pick(0, 0, 1) % 32, 1);
    }

    /// The blend `blade_stills.wgsl` makes (picture i of N: i/N of its moment, the rest of the
    /// moment a loop later), on a light that drifts and never repeats: the pictures alone jump
    /// back the whole loop going round, while the blend's picture N (one past the last) is its
    /// picture 0, so going round is one more step of the same blend.
    #[test]
    fn a_blended_loop_has_no_seam() {
        let light = |seconds: f64| (seconds * 4.1).sin() + 0.6 * (seconds * 2.3 + 1.0).sin();
        let moment = |frame: usize| STILL_SECONDS + frame as f64 * FRAME_STEP;
        let step = |picture: &dyn Fn(usize) -> f64, frame: usize| {
            (picture((frame + 1) % FRAMES) - picture(frame)).abs()
        };
        let alone = |frame: usize| light(moment(frame));
        let inside = (0..FRAMES - 1)
            .map(|frame| step(&alone, frame))
            .fold(0.0, f64::max);
        assert!(
            step(&alone, FRAMES - 1) > inside * 2.0,
            "the pictures alone jump"
        );
        let blended = |frame: usize| {
            let now = frame as f64 / FRAMES as f64;
            now * light(moment(frame)) + (1.0 - now) * light(moment(frame) + LOOP_SECONDS)
        };
        assert!((blended(FRAMES) - blended(0)).abs() < 1e-12);
        assert!(step(&blended, FRAMES - 1) < step(&alone, FRAMES - 1) * 0.6);
    }

    #[test]
    fn the_still_program_validates() {
        crate::wgsl_source::validate(include_str!("blade_stills.wgsl"));
    }
}
