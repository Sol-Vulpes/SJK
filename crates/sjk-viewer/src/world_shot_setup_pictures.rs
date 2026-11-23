//! First setup's pictures ([`crate::setup_pictures`]) taken again.
//!
//! `setup_pictures_generate` renders each one off screen and writes it as a JPEG into
//! `crates/sjk-viewer/assets/setup_pictures/`: the SJK look (the SJK UI's main page
//! over mp/duel6) and one view of mp/duel6 from the same camera under each graphics
//! preset. The EJK look has no file: the pop-up draws the classic menus small from the
//! player's own game data, since a picture of them would be the game's menu artwork. Run it again after a menu,
//! the renderer's look or a preset changes:
//!
//! ```text
//! JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer --release setup_pictures_generate -- --ignored --test-threads=1
//! ```
//!
//! They are the client's own renders, as the menu pictures and the site's screenshots
//! are; no file of the game's is copied.

use crate::graphics_quality::{Level, preset_cvars};
use crate::setup_pictures::{self, Picture};
use crate::world_shot::*;
use crate::{menu, version_overlay};

/// The JPEG quality the pictures are saved at.
const QUALITY: u8 = 74;
/// How many times the saved size each picture is rendered at, then reduced.
const OVERSAMPLE: u32 = 2;
/// The map the presets are shown on, the one behind the SJK UI's menus, and the
/// camera: the courtyard from a corner (the menu tour's second shot), where the sky,
/// the sun on paving and grass, the trees' shade and the temple's stone all show.
/// Yaw and pitch are radians, in the backdrop's convention.
const PRESET_MAP: &str = "maps/mp/duel6.bsp";
const PRESET_ORIGIN: [f32; 3] = [-1100.0, -1100.0, 700.0];
const PRESET_YAW: f32 = std::f32::consts::FRAC_PI_4;
const PRESET_PITCH: f32 = -0.251_677_66;

/// Save `image` reduced to `picture`'s size, and say how heavy it is.
fn save(picture: Picture, image: &image::RgbaImage) {
    let [width, height] = picture.size();
    let small =
        image::imageops::resize(image, width, height, image::imageops::FilterType::Lanczos3);
    let directory = setup_pictures::source_directory();
    std::fs::create_dir_all(&directory).expect("the picture directory");
    let path = directory.join(format!("{}.jpg", picture.name()));
    let rgb = image::DynamicImage::ImageRgba8(small).to_rgb8();
    let file = std::fs::File::create(&path).expect("the picture file");
    image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), QUALITY)
        .encode_image(&rgb)
        .expect("write the picture");
    println!(
        "{} ({} KiB)",
        path.display(),
        std::fs::metadata(&path).map_or(0, |meta| meta.len() / 1024)
    );
}

/// The size `picture` is rendered at.
fn render_size(picture: Picture) -> [u32; 2] {
    picture.size().map(|side| side * OVERSAMPLE)
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn setup_pictures_generate() {
    on_big_stack(|| {
        // The SJK look: its main page, past the fade-in.
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (version_overlay::CVAR, "0"),
        ];
        let menu = menu::ClientMenu::new(true, String::new());
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            render_size(Picture::LookSjk),
            Some(menu),
            &cvars,
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 20);
        gpu.ui_epoch -= std::time::Duration::from_millis(2_000);
        save(Picture::LookSjk, &frame(&mut gpu, 6));
        drop(gpu);

        // Each preset from the same camera, its lamps and probes settled.
        for level in Level::ALL {
            let picture = Picture::of_level(level);
            let mut cvars = preset_cvars(level);
            cvars.push((version_overlay::CVAR, "0"));
            let Some((mut gpu, _profile)) = open(PRESET_MAP, render_size(picture), None, &cvars)
            else {
                return;
            };
            let _ = frame(&mut gpu, 90);
            gpu.camera_position = PRESET_ORIGIN.into();
            gpu.camera_yaw = PRESET_YAW;
            gpu.camera_pitch = PRESET_PITCH;
            save(picture, &frame(&mut gpu, 40));
        }
    });
}
