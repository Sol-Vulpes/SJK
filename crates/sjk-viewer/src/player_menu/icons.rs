//! Player-screen icons: decoded on a background thread, then uploaded into
//! the UI icon atlas a few per frame. The character grid's model icons share
//! cells `1..=MODEL_ICONS` as a cache (see `model_icons`); cell 0 stays free
//! for the HUD. The Force page's power icons and side emblems (see
//! `force_icons`) use [`FORCE_CELLS`] cells of their own after the HUD's.

use crate::ui_renderer::{ICON_SIZE, ShapeRenderer};
use sjk_ui::TextureId;
use sjk_vfs::VirtualFileSystem;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

/// Atlas cells the player screen owns.
pub(super) const MAX_ICONS: usize = crate::ui_renderer::ICON_CELLS as usize;
/// Atlas cells of the Force page, outside the player screen's range.
pub(super) const FORCE_CELLS: usize = crate::ui_renderer::FORCE_ICON_CELLS as usize;
/// Model icons the atlas holds at once: cells `1..=MODEL_ICONS`.
pub(super) const MODEL_ICONS: usize = MAX_ICONS - 1;
/// Every atlas cell a loader may fill and track.
const TRACKED_CELLS: usize = crate::ui_renderer::ATLAS_CELLS as usize;

/// One icon to decode: its atlas cell and the paths to try, in order.
pub(crate) type IconRequest = (TextureId, Vec<String>);

struct DecodedIcon {
    id: TextureId,
    rgba: Vec<u8>,
}

pub(crate) struct IconLoader {
    receiver: Option<Receiver<Vec<DecodedIcon>>>,
    decoded: Vec<DecodedIcon>,
    uploaded: usize,
    requested: bool,
    /// One bit per atlas cell that holds a finished upload.
    ready: [u64; TRACKED_CELLS.div_ceil(64)],
    /// Draw the pictures whole, their alpha set to opaque ([`Self::opaque`]).
    opaque: bool,
}

impl IconLoader {
    pub(crate) fn new() -> Self {
        Self {
            receiver: None,
            decoded: Vec::new(),
            uploaded: 0,
            requested: false,
            ready: [0; TRACKED_CELLS.div_ceil(64)],
            opaque: false,
        }
    }

    /// A loader whose pictures are drawn whole: character creation's part icons keep
    /// in their alpha the mask the colour swatches tint, not a cut-out, and drawn with
    /// it only the tinted bits of each outfit showed (24/11/2026).
    pub(crate) fn opaque() -> Self {
        Self {
            opaque: true,
            ..Self::new()
        }
    }

    /// Whether nothing has been requested yet.
    pub(crate) fn is_idle(&self) -> bool {
        !self.requested
    }

    /// Atlas cell `slot` (0..[`FORCE_CELLS`]) of the Force page.
    pub(super) fn force_texture(slot: usize) -> TextureId {
        debug_assert!(slot < FORCE_CELLS);
        TextureId(crate::ui_renderer::FORCE_ICON_FIRST + slot as u32)
    }

    /// Count atlas cell `texture` as uploaded (snapshots draw without a GPU).
    #[cfg(test)]
    pub(crate) fn mark_ready(&mut self, texture: TextureId) {
        let cell = texture.0 as usize;
        if cell < TRACKED_CELLS {
            self.ready[cell / 64] |= 1 << (cell % 64);
        }
    }

    /// Whether atlas cell `texture` holds a finished upload of this loader.
    pub(crate) fn is_texture_ready(&self, texture: TextureId) -> bool {
        let cell = texture.0 as usize;
        cell < TRACKED_CELLS && self.ready[cell / 64] & (1 << (cell % 64)) != 0
    }

    /// Decode `requests` off-thread; each takes the first path that decodes.
    pub(crate) fn request_paths(
        &mut self,
        vfs: Arc<VirtualFileSystem>,
        requests: Vec<IconRequest>,
    ) {
        if self.requested {
            return;
        }
        self.requested = true;
        self.spawn(vfs, requests);
    }

    fn spawn(&mut self, vfs: Arc<VirtualFileSystem>, requests: Vec<IconRequest>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        let opaque = self.opaque;
        std::thread::spawn(move || {
            let decoded = requests
                .into_iter()
                .filter_map(|(id, paths)| decode_first(&vfs, id, &paths, opaque))
                .collect();
            let _ = sender.send(decoded);
        });
        self.receiver = Some(receiver);
    }

    pub(crate) fn poll(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(decoded) => {
                self.decoded = decoded;
                self.receiver = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => self.receiver = None,
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    /// Upload up to `limit` decoded icons into the atlas.
    pub(crate) fn upload_batch(
        &mut self,
        renderer: &ShapeRenderer,
        queue: &crate::frame_queue::FrameQueue,
        limit: usize,
    ) {
        let end = (self.uploaded + limit).min(self.decoded.len());
        for icon in &self.decoded[self.uploaded..end] {
            renderer.upload_icon(queue, icon.id, &icon.rgba);
            let cell = icon.id.0 as usize;
            if cell < TRACKED_CELLS {
                self.ready[cell / 64] |= 1 << (cell % 64);
            }
        }
        self.uploaded = end;
        if self.uploaded == self.decoded.len() {
            self.decoded.clear();
            self.uploaded = 0;
        }
    }
}

/// The first of `paths` that decodes, at the atlas cell's size; with `opaque` its
/// alpha is set to full.
fn decode_first(
    vfs: &VirtualFileSystem,
    id: TextureId,
    paths: &[String],
    opaque: bool,
) -> Option<DecodedIcon> {
    for path in paths {
        let Ok(Some(image)) = crate::decoded_image_cache::cached_decoded_image(vfs, path) else {
            continue;
        };
        let rgba = image::imageops::resize(
            image.as_ref(),
            ICON_SIZE,
            ICON_SIZE,
            image::imageops::FilterType::Triangle,
        )
        .into_raw();
        let mut rgba = rgba;
        if opaque {
            opaque_alpha(&mut rgba);
        }
        return Some(DecodedIcon { id, rgba });
    }
    None
}

/// Set every pixel of `rgba` fully opaque.
fn opaque_alpha(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[3] = u8::MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opaque_picture_keeps_its_colours_and_drops_its_mask() {
        let mut rgba = vec![10, 20, 30, 0, 40, 50, 60, 128];
        opaque_alpha(&mut rgba);
        assert_eq!(rgba, [10, 20, 30, 255, 40, 50, 60, 255]);
    }

    #[test]
    fn force_cells_leave_the_model_grid_its_full_range() {
        // Every menu cell but the HUD's cell 0: 207 model icons, as before the Force art.
        assert_eq!(MODEL_ICONS, MAX_ICONS - 1);
        assert_eq!(MODEL_ICONS, 207);
        let force = IconLoader::force_texture(0).0 as usize;
        assert!(force > MODEL_ICONS);
        assert!(force + FORCE_CELLS <= TRACKED_CELLS);
    }
}
