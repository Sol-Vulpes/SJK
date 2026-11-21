//! World shots of the notification banners (`unlock_toast.rs`, 16/11/2026): over Hoth's
//! snow under its pale cloudy sky (siege_hoth, the brightest backdrop a banner meets) and
//! over the SJK UI's main menu on duel6, at 1080p and 4K (in 4K with SJK's names for
//! holocrons and medals, `jof.honorNames 0`; at 1080p with JoF's, the default). A
//! holocron of each tier and a dormant kyber, several holocrons folded into one, an
//! achievement, a medal and the update banner, held at rest; the slide down, its settling,
//! the way back up, and the fade `ui_rarityEffects 0` gives instead, on sheets of the top
//! of the screen. Off-screen renders (no game window), ignored like the other world shots:
//! they need a GPU adapter and the installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer --bin sjk
//! notify_banner -- --ignored --test-threads=1 --nocapture`; the pictures are in
//! `target/world-shots` (`banner-*`).

use super::*;
use crate::unlock_toast::banner::{ENTER, FADE_IN, HOLD, LEAVE};
use crate::unlock_toast::{Many, Unlock, UnlockToast};

/// The camera: at siege_hoth's first spawn, looking over the snow at the bunker with
/// the sky above.
const FROM: [f32; 3] = [7_544.0, -400.0, 336.0];
const YAW: f32 = 180.0;
const PITCH: f32 = 6.0;

/// Seconds into a banner where it rests.
const REST: f32 = ENTER + 1.5;

fn holocron(tier: &'static crate::holocrons::Tier, id: u64, gift: bool) -> Unlock {
    Unlock::Holocron {
        tier,
        id,
        gift,
        earned: None,
        rehearsal: false,
    }
}

/// Every kind the shots show, with a file name each.
fn kinds() -> Vec<(&'static str, Unlock)> {
    let tiers = &crate::holocrons::TIERS;
    let mut kinds: Vec<(&'static str, Unlock)> = vec![
        ("holocron-uncommon", holocron(&tiers[0], 1, false)),
        ("holocron-rare", holocron(&tiers[1], 2, false)),
        ("holocron-legendary", holocron(&tiers[2], 3, true)),
        ("holocron-mythical", holocron(&tiers[3], 4, false)),
        (
            "dormant-kyber",
            Unlock::Holocron {
                tier: &crate::kyber::TIERS[2],
                id: 5,
                gift: false,
                earned: Some(crate::kyber::Earned::of("kills_100")),
                rehearsal: false,
            },
        ),
        (
            "holocrons-five",
            Unlock::Many {
                many: Many::Holocrons {
                    best: &tiers[2],
                    alike: false,
                    top: 30,
                    rehearsal: false,
                },
                count: 5,
            },
        ),
        (
            "achievement",
            Unlock::Achievement {
                kind: crate::achievements::find("dark_side_25").expect("Unlimited Power"),
                crystal: true,
            },
        ),
        (
            "medal",
            Unlock::Medal {
                medal: crate::medals::Medal::BugHunter,
                count: 2,
                promoted: None,
            },
        ),
    ];
    kinds.push((
        "update",
        Unlock::Update {
            version: crate::update::Version::new("2026.1117.1").expect("a version"),
            manual: false,
        },
    ));
    kinds
}

/// The top of `image` where the banner shows, as wide as it is and a bit.
fn top(image: &image::RgbaImage) -> image::RgbaImage {
    let s = image.height() as f32 / 1080.0;
    let width = ((760.0 * s) as u32).min(image.width());
    let height = (190.0 * s) as u32;
    let x = (image.width() - width) / 2;
    image::imageops::crop_imm(image, x, 0, width, height).to_image()
}

/// The whole frame with `unlock` held `at` seconds in (faded when `reduced`).
fn shot(gpu: &mut GpuState, unlock: Unlock, at: f32, reduced: bool) -> image::RgbaImage {
    gpu.unlock_toast = UnlockToast::preview_with(&[unlock], at, reduced);
    frame(gpu, 3)
}

/// The settings of the shots at `size`: SJK's names for holocrons and medals in 4K.
fn cvars(size: [u32; 2]) -> Vec<(&'static str, &'static str)> {
    vec![
        ("ui_menuStyle", "sjk"),
        (crate::settings::quick::HIDE_CVAR, "1"),
        ("cl_discordPresence", "0"),
        ("jof.honorNames", if size[0] > 1920 { "0" } else { "1" }),
    ]
}

fn save(image: &image::RgbaImage, name: &str) {
    std::fs::create_dir_all(directory()).expect("the shot directory");
    let path = directory().join(format!("{name}.png"));
    image.save(&path).expect("write the shot");
    println!("{}", path.display());
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn hoth_notify_banners() {
    on_big_stack(|| {
        for (size, prefix) in [([1920, 1080], "banner"), ([3840, 2160], "banner-4k")] {
            let Some((mut gpu, _profile)) =
                open("maps/mp/siege_hoth.bsp", size, None, &cvars(size))
            else {
                return;
            };
            let _ = frame(&mut gpu, 1);
            aim(&mut gpu, FROM, YAW, PITCH);
            if let Some(console) = gpu.console.as_mut() {
                console.close_for_connection();
            }
            let _ = frame(&mut gpu, 30);
            // Every kind at rest: the whole screen for a few, the tops on a sheet.
            let mut tops = Vec::new();
            for (name, unlock) in kinds() {
                let image = shot(&mut gpu, unlock, REST, false);
                if matches!(
                    name,
                    "holocron-legendary" | "holocrons-five" | "achievement" | "update"
                ) {
                    save(&image, &format!("{prefix}-hoth-{name}"));
                }
                tops.push(top(&image));
            }
            println!(
                "{}",
                sheet(&tops, 2, 760, &format!("{prefix}-hoth-kinds")).display()
            );
            if size[0] > 1920 {
                continue;
            }
            // The slide down, the settling, at rest, the way back up.
            let legendary = kinds()[2].1;
            let mut moments = Vec::new();
            for at in [
                0.05,
                ENTER * 0.2,
                ENTER * 0.35,
                ENTER * 0.5,
                ENTER * 0.62,
                ENTER * 0.8,
                REST,
                ENTER + HOLD + LEAVE * 0.3,
                ENTER + HOLD + LEAVE * 0.6,
                ENTER + HOLD + LEAVE * 0.9,
            ] {
                moments.push(top(&shot(&mut gpu, legendary, at, false)));
            }
            println!(
                "{}",
                sheet(&moments, 2, 760, &format!("{prefix}-hoth-slide")).display()
            );
            // `ui_rarityEffects 0`: a fade with a short drift.
            let mut faded = Vec::new();
            for at in [FADE_IN * 0.3, FADE_IN * 0.7, REST, FADE_IN + HOLD + 0.15] {
                faded.push(top(&shot(&mut gpu, legendary, at, true)));
            }
            println!(
                "{}",
                sheet(&faded, 2, 760, &format!("{prefix}-hoth-faded")).display()
            );
        }
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_notify_banners_over_the_main_menu() {
    on_big_stack(|| {
        for (size, prefix) in [([1920, 1080], "banner"), ([3840, 2160], "banner-4k")] {
            let menu = menu::ClientMenu::new(true, String::new());
            let Some((mut gpu, _profile)) =
                open("maps/mp/duel6.bsp", size, Some(menu), &cvars(size))
            else {
                return;
            };
            let _ = frame(&mut gpu, 10);
            for (name, unlock) in kinds() {
                if matches!(name, "holocron-mythical" | "holocrons-five" | "update") {
                    let image = shot(&mut gpu, unlock, REST, false);
                    save(&image, &format!("{prefix}-menu-{name}"));
                }
            }
        }
    });
}
