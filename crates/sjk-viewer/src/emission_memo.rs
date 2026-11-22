//! What the emission scans read off a large image, kept per decoded image (release review,
//! 16/11/2026). A player model's materials are compiled twice: by its loader while it reads
//! the model (`player_loads.rs`, to warm the textures) and on the render thread when the
//! model goes on its player. Each time the glow stages' emission was worked out again
//! from every pixel: for the old Creyon wolf (`sjk_creyonwolf`, 4096x4096 eye glow) the
//! render thread's share cost 140 to 200 ms here and a 719 ms hitch in Sol's log of
//! 16/11/2026. Now the loader's scan is kept and the render thread finds it.
//!
//! An entry is the image's address with a [`Weak`] to it: while the weak lives the address
//! is not reused, and an entry whose image is gone is a miss. Only images of
//! [`LARGE`] pixels or more are kept (a smaller one scans in a few milliseconds), at most
//! [`MOST`] of them: about 64 KiB each.

use glam::Vec4;
use image::RgbaImage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

/// The pixels from which an image's scans are kept.
pub(super) const LARGE: u64 = 1 << 20;
/// The images kept at most.
const MOST: usize = 64;

/// An image's area reduction to a grid (each cell's colour) with its mean colour.
pub(super) type Grid = Arc<(Vec<Vec4>, Vec4)>;

/// One image's kept scans.
#[derive(Default)]
struct Entry {
    image: Weak<RgbaImage>,
    /// Its linear additive mean (`super::additive_mean`).
    additive: Option<[f32; 3]>,
    /// Its area reduction to a grid, by the grid's size, with its mean
    /// (`super::texture::Layer`): each colour unweighted.
    grids: Vec<([usize; 2], Grid)>,
}

static ENTRIES: Mutex<Option<HashMap<usize, Entry>>> = Mutex::new(None);

fn large(image: &RgbaImage) -> bool {
    u64::from(image.width()) * u64::from(image.height()) >= LARGE
}

fn key(image: &Arc<RgbaImage>) -> usize {
    Arc::as_ptr(image) as usize
}

/// `image`'s entry, if kept for it.
fn with_entry<T>(image: &Arc<RgbaImage>, read: impl FnOnce(&Entry) -> Option<T>) -> Option<T> {
    let entries = ENTRIES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let entry = entries.as_ref()?.get(&key(image))?;
    let alive = entry
        .image
        .upgrade()
        .is_some_and(|kept| Arc::ptr_eq(&kept, image));
    if alive { read(entry) } else { None }
}

/// Change `image`'s entry (made when missing or stale).
fn keep(image: &Arc<RgbaImage>, change: impl FnOnce(&mut Entry)) {
    let mut entries = ENTRIES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let entries = entries.get_or_insert_with(HashMap::new);
    if !entries.contains_key(&key(image)) && entries.len() >= MOST {
        entries.retain(|_, entry| entry.image.strong_count() > 0);
        if entries.len() >= MOST {
            entries.clear();
        }
    }
    let entry = entries.entry(key(image)).or_default();
    if !entry
        .image
        .upgrade()
        .is_some_and(|kept| Arc::ptr_eq(&kept, image))
    {
        *entry = Entry {
            image: Arc::downgrade(image),
            ..Entry::default()
        };
    }
    change(entry);
}

/// `image`'s additive mean, worked out by `scan` unless kept.
pub(super) fn additive(image: &Arc<RgbaImage>, scan: impl FnOnce() -> [f32; 3]) -> [f32; 3] {
    if !large(image) {
        return scan();
    }
    if let Some(mean) = with_entry(image, |entry| entry.additive) {
        return mean;
    }
    let mean = scan();
    keep(image, |entry| entry.additive = Some(mean));
    mean
}

/// `image`'s reduction to a grid of `size`, with its mean, worked out by `scan` unless
/// kept.
pub(super) fn grid(
    image: &Arc<RgbaImage>,
    size: [usize; 2],
    scan: impl FnOnce() -> (Vec<Vec4>, Vec4),
) -> Grid {
    if !large(image) {
        return Arc::new(scan());
    }
    let found = with_entry(image, |entry| {
        entry
            .grids
            .iter()
            .find(|(kept, _)| *kept == size)
            .map(|(_, grid)| Arc::clone(grid))
    });
    if let Some(grid) = found {
        return grid;
    }
    let grid = Arc::new(scan());
    keep(image, |entry| entry.grids.push((size, Arc::clone(&grid))));
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(value: u8) -> Arc<RgbaImage> {
        Arc::new(RgbaImage::from_pixel(1024, 1024, image::Rgba([value; 4])))
    }

    /// A large image is scanned once; a small one every time; another image (or one at
    /// a freed address) is scanned for itself.
    #[test]
    fn a_large_image_is_scanned_once() {
        let image = big(200);
        let mut scans = 0;
        for _ in 0..3 {
            additive(&image, || {
                scans += 1;
                [0.5; 3]
            });
        }
        assert_eq!(scans, 1);
        let small = Arc::new(RgbaImage::from_pixel(8, 8, image::Rgba([1; 4])));
        for _ in 0..2 {
            additive(&small, || {
                scans += 1;
                [0.1; 3]
            });
        }
        assert_eq!(scans, 3);
        let other = big(10);
        assert_eq!(additive(&other, || [0.25; 3]), [0.25; 3]);
        assert_eq!(additive(&image, || [9.0; 3]), [0.5; 3], "kept apart");
        let first = grid(&image, [4, 4], || (vec![Vec4::ONE; 16], Vec4::ONE));
        let again = grid(&image, [4, 4], || unreachable!("kept"));
        assert!(Arc::ptr_eq(&first, &again));
        let other_size = grid(&image, [2, 2], || (vec![Vec4::ZERO; 4], Vec4::ZERO));
        assert_eq!(other_size.0.len(), 4);
    }
}
