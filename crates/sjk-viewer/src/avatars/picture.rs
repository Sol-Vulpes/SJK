//! The pictures themselves: a player's own image made into what the hub takes (cropped
//! to a square from its middle, scaled to [`SIZE`] and written as a PNG), a picture the
//! hub serves read back into pixels, and the round cut the UI draws them with. Pure
//! functions on bytes, run on worker threads, never on the frame thread.

use image::{ImageEncoder, RgbaImage};
use sjk_identity::avatar::SIZE;

/// Largest file read as a picture, in bytes.
pub(crate) const FILE_MAX: u64 = 16 * 1024 * 1024;
/// Largest width or height a picture may have.
const EDGE_MAX: u32 = 8_192;
/// Smallest width or height a picture may have.
pub(crate) const EDGE_MIN: u32 = 32;
/// Most memory a decoder may take for one picture.
const DECODE_LIMIT: u64 = 256 * 1024 * 1024;

/// Why a file cannot be the player's picture, in words the player reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PictureError {
    /// The file could not be read.
    Unreadable(String),
    /// Over [`FILE_MAX`] bytes or [`EDGE_MAX`] pixels a side.
    TooLarge,
    /// Under [`EDGE_MIN`] pixels a side.
    TooSmall,
    /// Not a picture SJK reads.
    NotAPicture,
}

impl std::fmt::Display for PictureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(why) => write!(f, "Cannot read that file: {why}"),
            Self::TooLarge => write!(
                f,
                "That picture is too big: at most 16 MB and 8192 pixels a side"
            ),
            Self::TooSmall => write!(
                f,
                "That picture is too small: at least {EDGE_MIN} pixels a side"
            ),
            Self::NotAPicture => write!(
                f,
                "That file is not a picture SJK can read: use a PNG, JPEG or TGA"
            ),
        }
    }
}

/// A player's picture made ready to send: the PNG the hub takes and its pixels for the
/// preview.
#[derive(Clone, Debug)]
pub(crate) struct Prepared {
    /// [`SIZE`] square PNG, RGB when every pixel is opaque, else RGBA.
    pub(crate) png: Vec<u8>,
    /// Its [`SIZE`] square RGBA pixels, not yet cut round.
    pub(crate) rgba: Vec<u8>,
}

/// Whether `path` names a picture file by its extension (PNG, JPEG or TGA): a file
/// dropped on the window anywhere is taken as the player's new picture then.
pub(crate) fn looks_like_picture(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["png", "jpg", "jpeg", "tga"]
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

/// Read the file at `path` and make it a picture ([`prepare`]).
pub(crate) fn prepare_file(path: &std::path::Path) -> Result<Prepared, PictureError> {
    let size = std::fs::metadata(path)
        .map_err(|error| PictureError::Unreadable(error.kind().to_string()))?
        .len();
    if size > FILE_MAX {
        return Err(PictureError::TooLarge);
    }
    let bytes =
        std::fs::read(path).map_err(|error| PictureError::Unreadable(error.kind().to_string()))?;
    // A TGA has no signature to know it by: its extension says it.
    let tga = image::ImageFormat::from_path(path).ok() == Some(image::ImageFormat::Tga);
    prepare_as(&bytes, tga.then_some(image::ImageFormat::Tga))
}

/// Make `bytes` (a PNG, JPEG or TGA) a picture the hub takes: cropped to a square from
/// its middle, scaled to [`SIZE`] by area averaging and written as a PNG holding the
/// pixels only. (The game reads files, [`prepare_file`]; this is for tests.)
#[cfg(test)]
pub(crate) fn prepare(bytes: &[u8]) -> Result<Prepared, PictureError> {
    prepare_as(bytes, None)
}

/// [`prepare`], taking `bytes` as `fallback` when their format cannot be told from them.
fn prepare_as(
    bytes: &[u8],
    fallback: Option<image::ImageFormat>,
) -> Result<Prepared, PictureError> {
    if bytes.len() as u64 > FILE_MAX {
        return Err(PictureError::TooLarge);
    }
    let image = decode(bytes, None, fallback)?;
    let (width, height) = image.dimensions();
    if width.min(height) < EDGE_MIN {
        return Err(PictureError::TooSmall);
    }
    let edge = width.min(height);
    let square =
        image::imageops::crop_imm(&image, (width - edge) / 2, (height - edge) / 2, edge, edge)
            .to_image();
    let scaled = if edge == SIZE {
        square
    } else {
        image::imageops::thumbnail(&square, SIZE, SIZE)
    };
    let png = encode(&scaled).map_err(|error| PictureError::Unreadable(error.to_string()))?;
    Ok(Prepared {
        png,
        rgba: scaled.into_raw(),
    })
}

/// A picture the hub served, as [`SIZE`] square RGBA pixels; `None` when it is not a
/// square PNG.
pub(crate) fn decode_served(png: &[u8]) -> Option<Vec<u8>> {
    let image = decode(png, Some(image::ImageFormat::Png), None).ok()?;
    let (width, height) = image.dimensions();
    if width != height || width == 0 {
        return None;
    }
    if width == SIZE {
        return Some(image.into_raw());
    }
    Some(image::imageops::thumbnail(&image, SIZE, SIZE).into_raw())
}

/// Decode `bytes` (of `format`, or guessed from the bytes, else `fallback`) under the
/// size limits.
fn decode(
    bytes: &[u8],
    format: Option<image::ImageFormat>,
    fallback: Option<image::ImageFormat>,
) -> Result<RgbaImage, PictureError> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
    match format {
        Some(format) => reader.set_format(format),
        None => {
            reader = reader
                .with_guessed_format()
                .map_err(|error| PictureError::Unreadable(error.to_string()))?;
            if reader.format().is_none()
                && let Some(fallback) = fallback
            {
                reader.set_format(fallback);
            }
        }
    }
    let format = reader.format().ok_or(PictureError::NotAPicture)?;
    if !matches!(
        format,
        image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::Tga
    ) {
        return Err(PictureError::NotAPicture);
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(EDGE_MAX);
    limits.max_image_height = Some(EDGE_MAX);
    limits.max_alloc = Some(DECODE_LIMIT);
    reader.limits(limits);
    reader
        .decode()
        .map(image::DynamicImage::into_rgba8)
        .map_err(|error| match error {
            image::ImageError::Limits(_) => PictureError::TooLarge,
            image::ImageError::Unsupported(_) => PictureError::NotAPicture,
            // A TGA has no signature: bytes that are none of these fail here.
            _ if format == image::ImageFormat::Tga => PictureError::NotAPicture,
            other => PictureError::Unreadable(other.to_string()),
        })
}

/// `image` as a PNG: RGB when every pixel is opaque, else RGBA.
fn encode(image: &RgbaImage) -> image::ImageResult<Vec<u8>> {
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        &mut out,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::Adaptive,
    );
    let (width, height) = image.dimensions();
    if image.pixels().all(|pixel| pixel.0[3] == 255) {
        let rgb: Vec<u8> = image
            .pixels()
            .flat_map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2]])
            .collect();
        encoder.write_image(&rgb, width, height, image::ExtendedColorType::Rgb8)?;
    } else {
        encoder.write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )?;
    }
    Ok(out)
}

/// Cut `rgba` (`edge` square) round: pixels outside the inscribed circle become clear,
/// those on its edge as clear as they are outside it, so the disc's rim is smooth.
pub(crate) fn round(rgba: &mut [u8], edge: u32) {
    let centre = edge as f32 * 0.5;
    let radius = centre;
    for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
        let x = (index as u32 % edge) as f32 + 0.5 - centre;
        let y = (index as u32 / edge) as f32 + 0.5 - centre;
        let distance = (x * x + y * y).sqrt();
        let cover = (radius - distance + 0.5).clamp(0.0, 1.0);
        pixel[3] = (f32::from(pixel[3]) * cover).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `width` x `height` pixels from `pixel`, encoded as `format`.
    fn picture(
        width: u32,
        height: u32,
        format: image::ImageFormat,
        pixel: impl Fn(u32, u32) -> [u8; 4],
    ) -> Vec<u8> {
        let image = RgbaImage::from_fn(width, height, |x, y| image::Rgba(pixel(x, y)));
        let mut out = std::io::Cursor::new(Vec::new());
        match format {
            image::ImageFormat::Jpeg => image::DynamicImage::ImageRgba8(image)
                .into_rgb8()
                .write_to(&mut out, format)
                .unwrap(),
            _ => image.write_to(&mut out, format).unwrap(),
        }
        out.into_inner()
    }

    fn pixels_of_png(png: &[u8]) -> RgbaImage {
        image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .unwrap()
            .into_rgba8()
    }

    #[test]
    fn a_wide_picture_is_cropped_from_its_middle_and_scaled() {
        // Red at the left and right quarters, green in the middle half.
        let wide = picture(800, 400, image::ImageFormat::Png, |x, _| {
            if (200..600).contains(&x) {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            }
        });
        let prepared = prepare(&wide).unwrap();
        let png = pixels_of_png(&prepared.png);
        assert_eq!(png.dimensions(), (SIZE, SIZE));
        assert!(png.pixels().all(|pixel| pixel.0 == [0, 255, 0, 255]));
        assert_eq!(prepared.rgba.len(), (SIZE * SIZE * 4) as usize);
        // Opaque pictures are sent as RGB: colour type 2 in the header.
        assert_eq!(prepared.png[25], 2);
        assert!(prepared.png.len() <= sjk_identity::avatar::UPLOAD_MAX);
    }

    #[test]
    fn a_tall_jpeg_and_a_small_tga_become_squares_of_the_hubs_size() {
        let tall = picture(300, 900, image::ImageFormat::Jpeg, |_, y| {
            if (300..600).contains(&y) {
                [20, 40, 220, 255]
            } else {
                [250, 250, 250, 255]
            }
        });
        let png = pixels_of_png(&prepare(&tall).unwrap().png);
        let centre = png.get_pixel(SIZE / 2, SIZE / 2).0;
        assert!(centre[2] > 180 && centre[0] < 80, "{centre:?}");
        // A TGA is known by its file's extension.
        let small = picture(40, 40, image::ImageFormat::Tga, |_, _| [9, 99, 199, 255]);
        assert_eq!(prepare(&small).err(), Some(PictureError::NotAPicture));
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Me.TGA");
        std::fs::write(&file, &small).unwrap();
        let png = pixels_of_png(&prepare_file(&file).unwrap().png);
        assert_eq!(png.dimensions(), (SIZE, SIZE));
        assert_eq!(png.get_pixel(5, 5).0, [9, 99, 199, 255]);
    }

    #[test]
    fn transparency_is_kept_and_sent_as_rgba() {
        let clear = picture(256, 256, image::ImageFormat::Png, |x, _| {
            if x < 128 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 0, 0]
            }
        });
        let prepared = prepare(&clear).unwrap();
        assert_eq!(prepared.png[25], 6, "RGBA");
        let png = pixels_of_png(&prepared.png);
        assert_eq!(png.get_pixel(SIZE - 1, 0).0[3], 0);
        assert_eq!(png.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn other_files_and_sizes_are_refused_with_a_reason() {
        assert_eq!(
            prepare(b"just some text, not a picture").err(),
            Some(PictureError::NotAPicture)
        );
        let gif = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";
        assert_eq!(prepare(gif).err(), Some(PictureError::NotAPicture));
        let tiny = picture(16, 64, image::ImageFormat::Png, |_, _| [1, 2, 3, 255]);
        assert_eq!(prepare(&tiny).err(), Some(PictureError::TooSmall));
        let mut cut = picture(64, 64, image::ImageFormat::Png, |x, y| {
            [x as u8, y as u8, 0, 255]
        });
        cut.truncate(cut.len() / 2);
        assert!(matches!(prepare(&cut), Err(PictureError::Unreadable(_))));
        // A header claiming a huge picture is refused before its pixels are allocated.
        let huge = picture(64, 64, image::ImageFormat::Png, |_, _| [0; 4]);
        let mut header = huge.clone();
        header[16..20].copy_from_slice(&20_000_u32.to_be_bytes());
        header[20..24].copy_from_slice(&20_000_u32.to_be_bytes());
        assert!(prepare(&header).is_err());
        let file = std::env::temp_dir().join(format!("sjk-avatar-big-{}.png", std::process::id()));
        std::fs::File::create(&file)
            .unwrap()
            .set_len(FILE_MAX + 1)
            .unwrap();
        assert_eq!(prepare_file(&file).err(), Some(PictureError::TooLarge));
        std::fs::remove_file(&file).ok();
        assert!(matches!(
            prepare_file(std::path::Path::new("/nonexistent/sjk/avatar.png")),
            Err(PictureError::Unreadable(_))
        ));
    }

    #[test]
    fn pictures_are_known_by_their_extension() {
        assert!(looks_like_picture(std::path::Path::new("me.PNG")));
        assert!(looks_like_picture(std::path::Path::new("dir/me.jpeg")));
        assert!(looks_like_picture(std::path::Path::new("me.tga")));
        assert!(!looks_like_picture(std::path::Path::new("jampconfig.cfg")));
        assert!(!looks_like_picture(std::path::Path::new("png")));
    }

    #[test]
    fn served_pictures_are_read_back_at_the_hubs_size() {
        let served = picture(SIZE, SIZE, image::ImageFormat::Png, |_, _| [5, 6, 7, 255]);
        let rgba = decode_served(&served).unwrap();
        assert_eq!(rgba.len(), (SIZE * SIZE * 4) as usize);
        assert_eq!(&rgba[..4], &[5, 6, 7, 255]);
        let larger = picture(256, 256, image::ImageFormat::Png, |_, _| [5, 6, 7, 255]);
        assert_eq!(
            decode_served(&larger).unwrap().len(),
            (SIZE * SIZE * 4) as usize
        );
        let wide = picture(256, 128, image::ImageFormat::Png, |_, _| [0; 4]);
        assert_eq!(decode_served(&wide), None);
        let jpeg = picture(SIZE, SIZE, image::ImageFormat::Jpeg, |_, _| [0; 4]);
        assert_eq!(decode_served(&jpeg), None, "the hub serves PNG only");
    }

    #[test]
    fn the_round_cut_clears_the_corners_and_keeps_the_middle() {
        let mut rgba = vec![255; (SIZE * SIZE * 4) as usize];
        round(&mut rgba, SIZE);
        let alpha = |x: u32, y: u32| rgba[((y * SIZE + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(SIZE - 1, SIZE - 1), 0);
        assert_eq!(alpha(SIZE / 2, SIZE / 2), 255);
        assert!(alpha(SIZE / 2, 0) >= 250, "the top of the disc");
        assert!(alpha(1, SIZE / 2) >= 250);
        // Somewhere on the rim a pixel is part covered.
        assert!((0..SIZE).any(|x| (1..255).contains(&alpha(x, 10))));
    }
}
