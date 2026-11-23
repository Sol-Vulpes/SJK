//! First setup's pictures ([`crate::setup_pictures`]) taken again.
//!
//! `setup_pictures_generate` renders one view of mp/duel6 from the same camera under
//! each graphics preset, and `setup_look_pictures_generate` the two looks (SJK and
//! EJK): the same view of a match on mp/duel6 under each, with the player and the HUD. Each is written as a JPEG into `crates/sjk-viewer/assets/setup_pictures/`. Run
//! them again after the renderer's look, a preset, the HUD or what EJK mode sets
//! changes:
//!
//! ```text
//! JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer --release setup_pictures_generate -- --ignored --test-threads=1
//! ```
//!
//! They are the client's own renders of the game being played, as the menu pictures and
//! the site's screenshots are; no file of the game's is copied. The looks' pictures are
//! taken with the retail archives only, so nothing in them is a pack of the machine's
//! owner, and the only name in them is the default player's (Padawan).

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

/// The match the two looks are shown in: a local game of mp/duel6, the player put on
/// one spot facing one way, so both pictures are the same view and only the look
/// differs. No bot is in it: one fights the player the moment it finds them, and a
/// fight in the frame (a Force power's flash fills the screen) would differ between
/// the two pictures more than the looks do.
const LOOK_MAP: &str = "mp/duel6";
/// Where the player stands (`setviewpos` units and degrees) and how far up they look:
/// one of the map's own spawn points, on level ground (further in, the ground falls
/// away under the bridge and a player slides down it), facing the temple.
const LOOK_SPOT: [f32; 3] = [0.0, -1024.0, 180.0];
const LOOK_YAW: f32 = 90.0;
const LOOK_PITCH: f32 = -0.06;
/// How long after the player is put on the spot the picture is taken (being put there
/// shows the game's spawn effect, a column of light over them, and hides them for a
/// moment). By the clock, not by frames: the looks draw at different rates, and the
/// same time after gives both pictures the same stance on the same ground. How near
/// the spot the player must have landed for that (units), and how long the whole is
/// given (seconds).
const LOOK_SETTLE: std::time::Duration = std::time::Duration::from_secs(9);
const LOOK_LANDED: f32 = 160.0;
const LOOK_WAIT: u64 = 120;

/// The settings a look's picture is taken under: a new profile's (Vanilla Plus) for
/// SJK, everything EJK mode sets for EJK (`ejk_mode::values`). Neither shows the
/// build's version line, which would date the picture.
fn look_cvars(picture: Picture) -> Vec<(&'static str, &'static str)> {
    let mut cvars = match picture {
        Picture::LookEjk => crate::ejk_mode::values(),
        _ => preset_cvars(Level::DEFAULT),
    };
    cvars.retain(|(cvar, _)| *cvar != version_overlay::CVAR);
    cvars.extend([
        (version_overlay::CVAR, "0"),
        (crate::settings::quick::HIDE_CVAR, "1"),
        // No hub in a picture, and no word of the match to the taker's Discord.
        ("cl_identity", "0"),
        (crate::discord_presence::CVAR, "0"),
    ]);
    cvars
}

/// The two looks' pictures: the same view of a match under each, an in-match frame
/// with the HUD, taken on a local server (`JKA_DEDICATED` names `sjk-server`). Run it with a `JKA_GAME_DATA` that holds the retail archives only (`base` with
/// `assets0.pk3` to `assets3.pk3`), so the HUD, the models and the fonts are the stock
/// game's and no pack of the machine's owner is in the picture:
///
/// ```text
/// JKA_GAME_DATA=<retail GameData> JKA_DEDICATED=<sjk-server> cargo test -p sjk-viewer --release setup_look_pictures_generate -- --ignored --test-threads=1
/// ```
#[test]
#[ignore = "renders with the GPU, the retail game data named by JKA_GAME_DATA and a local sjk-server named by JKA_DEDICATED"]
fn setup_look_pictures_generate() {
    use super::graphics_reload::{step, until};
    on_big_stack(|| {
        for picture in [Picture::LookSjk, Picture::LookEjk] {
            let size = render_size(picture);
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = look_cvars(picture);
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, Some(menu), &cvars)
            else {
                return;
            };
            let mut parked = crate::menu_world::Parked::new();
            gpu.start_local_game(crate::connection::local_server::HostSettings {
                map: LOOK_MAP.into(),
                gametype: "ffa",
                hostname: "SJK".into(),
                bots: Vec::new(),
                bot_skill: 1,
                fraglimit: 0,
                timelimit: 0,
                capturelimit: 0,
                allow_lan: false,
                cheats: true,
                server_mod: crate::connection::local_server::BASE_SERVER_MOD,
            });
            until(
                &mut gpu,
                &mut parked,
                size,
                180,
                "joined the match",
                |gpu| gpu.live_map_installed && gpu.live_session.is_some(),
            );
            let queue = |gpu: &mut GpuState, command: &str| {
                gpu.console
                    .as_mut()
                    .expect("a console")
                    .queue_command(command)
                    .expect("the command queues");
            };
            let place = format!(
                "setviewpos {} {} {} {LOOK_YAW}",
                LOOK_SPOT[0], LOOK_SPOT[1], LOOK_SPOT[2]
            );
            let started = std::time::Instant::now();
            // When the player was last put on the spot.
            let mut placed: Option<std::time::Instant> = None;
            loop {
                gpu.camera_yaw = LOOK_YAW.to_radians();
                gpu.camera_pitch = LOOK_PITCH;
                let me = gpu
                    .live_session
                    .as_ref()
                    .map(|session| session.latest_snapshot().player.origin());
                let landed = me.is_some_and(|me| {
                    (me[0] - LOOK_SPOT[0]).hypot(me[1] - LOOK_SPOT[1]) < LOOK_LANDED
                });
                // Put there once; again only if the command did not take.
                let again = placed
                    .is_none_or(|at| !landed && at.elapsed() > std::time::Duration::from_secs(3));
                if again {
                    queue(&mut gpu, &place);
                    placed = Some(std::time::Instant::now());
                }
                step(&mut gpu, &mut parked, size);
                if landed && placed.is_some_and(|at| at.elapsed() >= LOOK_SETTLE) {
                    break;
                }
                assert!(
                    started.elapsed() < std::time::Duration::from_secs(LOOK_WAIT),
                    "{picture:?}: the player at {me:?} did not stand on the spot in {LOOK_WAIT} s"
                );
            }
            eprintln!(
                "{picture:?}: taken after {:.1} s, the player at {:?}",
                started.elapsed().as_secs_f32(),
                gpu.live_session
                    .as_ref()
                    .map(|session| session.latest_snapshot().player.origin())
            );
            // The frame as rendered too, to look at beside the small picture.
            let image = frame(&mut gpu, 1);
            let path = directory().join(format!("setup-{}.png", picture.name()));
            std::fs::create_dir_all(directory()).expect("the shot directory");
            image.save(&path).expect("write the shot");
            println!("{}", path.display());
            save(picture, &image);
        }
    });
}
