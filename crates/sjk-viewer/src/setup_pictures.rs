//! First setup's pictures: the SJK look and each graphics preset, shown on the
//! pop-up's picture buttons (`settings/sjk_popup.rs`, `docs/sjk-ui.md`, First setup).
//! The classic menus' picture is not one of them: it shows the game's own menu
//! artwork, which SJK never ships, so the pop-up draws it from the player's files.
//!
//! They are the client's own off-screen renders, bundled as small JPEGs
//! (`assets/setup_pictures/`), as the menu pictures are (`menu_pictures.rs`):
//! `world_shot_setup_pictures.rs` takes them again when a look changes. Each has a
//! texture of its own ([`Picture::texture`]), outside the icon atlas, which has no room
//! left: a worker decodes them all the first time the pop-up draws one
//! ([`request`]), and [`service`] uploads them into the menu's renderer.

use crate::graphics_quality::Level;
use crate::menu::levelshot::LevelshotImage;
use sjk_ui::TextureId;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

/// One picture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Picture {
    /// The SJK UI's main page over its map.
    LookSjk,
    /// The same view under each graphics preset.
    Vanilla,
    VanillaPlus,
    SolarLow,
    SolarMedium,
    SolarHigh,
    SolarUltra,
}

macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/setup_pictures/", $name, ".jpg")) as &[u8]
    };
}

/// The first picture's texture id; the others follow it. Clear of the icon atlas's
/// cells and of every other picture's ids (a test).
const TEXTURE_BASE: u32 = 0x6400_0000;
/// The size the look's picture is kept at, and the presets' (their buttons are
/// smaller): enough for a 4K screen's buttons without weighing on the download.
#[cfg(test)]
const LOOK_SIZE: [u32; 2] = [960, 540];
#[cfg(test)]
const PRESET_SIZE: [u32; 2] = [768, 432];

impl Picture {
    pub(crate) const ALL: [Self; 7] = [
        Self::LookSjk,
        Self::Vanilla,
        Self::VanillaPlus,
        Self::SolarLow,
        Self::SolarMedium,
        Self::SolarHigh,
        Self::SolarUltra,
    ];
    pub(crate) const COUNT: usize = Self::ALL.len();

    pub(crate) fn index(self) -> usize {
        self as usize
    }

    /// The picture of graphics preset `level`.
    pub(crate) fn of_level(level: Level) -> Self {
        match level {
            Level::Vanilla => Self::Vanilla,
            Level::VanillaPlus => Self::VanillaPlus,
            Level::SolarLow => Self::SolarLow,
            Level::SolarMedium => Self::SolarMedium,
            Level::SolarHigh => Self::SolarHigh,
            Level::SolarUltra => Self::SolarUltra,
        }
    }

    /// Its file's name, without the extension, for the generator.
    #[cfg(test)]
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::LookSjk => "look-sjk",
            Self::Vanilla => "preset-vanilla",
            Self::VanillaPlus => "preset-vanilla-plus",
            Self::SolarLow => "preset-solar-low",
            Self::SolarMedium => "preset-solar-medium",
            Self::SolarHigh => "preset-solar-high",
            Self::SolarUltra => "preset-solar-ultra",
        }
    }

    /// The size it is kept at (16:9), which the generator saves and a test checks.
    #[cfg(test)]
    pub(crate) fn size(self) -> [u32; 2] {
        match self {
            Self::LookSjk => LOOK_SIZE,
            _ => PRESET_SIZE,
        }
    }

    /// The bundled JPEG.
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::LookSjk => bundled!("look-sjk"),
            Self::Vanilla => bundled!("preset-vanilla"),
            Self::VanillaPlus => bundled!("preset-vanilla-plus"),
            Self::SolarLow => bundled!("preset-solar-low"),
            Self::SolarMedium => bundled!("preset-solar-medium"),
            Self::SolarHigh => bundled!("preset-solar-high"),
            Self::SolarUltra => bundled!("preset-solar-ultra"),
        }
    }

    /// `TexturedQuad` texture naming this picture.
    pub(crate) fn texture(self) -> TextureId {
        TextureId(TEXTURE_BASE + self.index() as u32)
    }

    /// The picture `texture` names.
    pub(crate) fn of_texture(texture: TextureId) -> Option<Self> {
        let index = texture.0.checked_sub(TEXTURE_BASE)?;
        Self::ALL.get(index as usize).copied()
    }

    /// The picture decoded, for the snapshots and the tests.
    #[cfg(test)]
    pub(crate) fn decoded(self) -> image::RgbaImage {
        image::load_from_memory(self.bytes())
            .expect("a bundled picture")
            .into_rgba8()
    }
}

/// Where the generator writes the pictures.
#[cfg(test)]
pub(crate) fn source_directory() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/setup_pictures")
}

/// The pictures decoded, each with its mip chain, in [`Picture::ALL`]'s order.
type Decoded = Arc<Vec<Option<LevelshotImage>>>;

#[derive(Default)]
struct State {
    /// The worker's answer, awaited.
    pending: Option<Receiver<Decoded>>,
    decoded: Option<Decoded>,
    /// The renderer the pictures were uploaded into.
    uploaded_into: Option<u64>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn lock() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Ask for the pictures: the first call starts the worker that decodes them. The
/// pop-up calls it each frame it draws one.
pub(crate) fn request() {
    let mut guard = lock();
    let state = guard.get_or_insert_with(State::default);
    if state.pending.is_some() || state.decoded.is_some() {
        return;
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("setup pictures".to_owned())
        .spawn(move || {
            let decoded = Picture::ALL
                .iter()
                .map(|picture| {
                    image::load_from_memory(picture.bytes())
                        .ok()
                        .map(|image| LevelshotImage::from_rgba(image.into_rgba8()))
                })
                .collect();
            let _ = sender.send(Arc::new(decoded));
        });
    if spawned.is_ok() {
        state.pending = Some(receiver);
    }
}

/// Once a frame while Settings is up: take the worker's answer and upload the pictures
/// into `renderer` (again into a new one, after a graphics reload).
pub(crate) fn service(
    renderer: &mut crate::ui_renderer::ShapeRenderer,
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
) {
    let mut guard = lock();
    let Some(state) = guard.as_mut() else {
        return;
    };
    if let Some(receiver) = &state.pending {
        match receiver.try_recv() {
            Ok(decoded) => {
                state.decoded = Some(decoded);
                state.pending = None;
            }
            Err(TryRecvError::Disconnected) => state.pending = None,
            Err(TryRecvError::Empty) => {}
        }
    }
    let id = renderer.id();
    if state.uploaded_into == Some(id) {
        return;
    }
    let Some(decoded) = &state.decoded else {
        return;
    };
    for (picture, image) in Picture::ALL.iter().zip(decoded.iter()) {
        if let Some(image) = image {
            renderer.upload_setup_picture(device, queue, *picture, image);
        }
    }
    state.uploaded_into = Some(id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_name_their_textures_and_stay_apart_from_other_textures() {
        for picture in Picture::ALL {
            let texture = picture.texture();
            assert_eq!(Picture::of_texture(texture), Some(picture));
            assert!(crate::ui_renderer::feat_pictures::cell_of(texture).is_none());
            assert!(crate::cards::faces::index_of(texture).is_none());
            assert!(crate::chat_gifs::slot_of(texture).is_none());
            assert!(crate::blade_stills::cell_of(texture).is_none());
            assert!(crate::medals::Medal::from_art(texture).is_none());
            assert!(crate::menu::art::ArtPiece::from_texture(texture).is_none());
            assert!(crate::menu::emblem::EmblemLayer::from_texture(texture).is_none());
        }
        assert_eq!(Picture::of_texture(TextureId(TEXTURE_BASE - 1)), None);
        assert_eq!(
            Picture::of_texture(TextureId(TEXTURE_BASE + Picture::COUNT as u32)),
            None
        );
        assert_eq!(Picture::of_texture(TextureId(0)), None);
    }

    #[test]
    fn every_picture_decodes_at_its_size_and_every_preset_has_one() {
        for picture in Picture::ALL {
            let image = picture.decoded();
            assert_eq!(
                [image.width(), image.height()],
                picture.size(),
                "{picture:?}"
            );
        }
        let mut seen = Vec::new();
        for level in Level::ALL {
            let picture = Picture::of_level(level);
            assert!(!seen.contains(&picture));
            seen.push(picture);
        }
    }

    /// The pictures stay small: they ride in every download.
    #[test]
    fn the_pictures_weigh_under_a_megabyte() {
        let bytes: usize = Picture::ALL
            .iter()
            .map(|picture| picture.bytes().len())
            .sum();
        assert!(bytes < 1_000_000, "{bytes} bytes of setup pictures");
    }
}
