//! World shots of the status bar ([`crate::status_bar`]) over duel6: at the top under the
//! build label with every part, at the bottom, and at 4K with the largest menu text.
//! The bar's numbers, blade, effects and toys are made up ([`State::preview`]). Off-screen
//! renders (no game window), ignored like the other world shots: they need a GPU adapter
//! and the installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer status_bar_shots -- --ignored
//! --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.
//!
//! [`State::preview`]: crate::status_bar::State::preview

use super::*;
use crate::status_bar::{Effect, Inputs, TOY_COUNT, toys};

/// A player with a Legendary blade and a stock second, a few things on and four toys out.
fn made_up() -> Inputs {
    let legendary = crate::unlockables::blade_skins()
        .find(|skin| skin.tier == crate::unlockables::Rarity::Legendary)
        .map(|skin| skin.id);
    let mut inputs = Inputs {
        credits: Some(12_340),
        shards: Some(875),
        crystals: Some(3),
        coins: Some(2),
        saber: legendary,
        second: Some(None),
        ..Inputs::default()
    };
    for (words, colour) in [
        ("Flare trail", crate::menu::sjk::color::GOLD_BRIGHT),
        (
            "Disintegrate",
            crate::unlockables::Rarity::Mythical.colour(),
        ),
        ("Tactical medium", crate::menu::sjk::color::HOLO),
        ("Do not disturb", crate::menu::sjk::color::EMBER),
    ] {
        inputs.push_effect(Effect { words, colour });
    }
    let mut on = [false; TOY_COUNT];
    for toy in [toys::ILLUMINATE, toys::HAX, toys::BOX_HUNT, toys::PROPGUN] {
        on[toy] = true;
    }
    inputs.toys = on;
    inputs
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn status_bar_shots() {
    on_big_stack(|| {
        for (size, scale, place, name) in [
            ([1920, 1080], "1", "1", "status-bar-top"),
            ([1920, 1080], "1", "2", "status-bar-bottom"),
            ([3840, 2160], "1.4", "1", "status-bar-4k-large-text"),
        ] {
            let cvars = [
                (crate::status_bar::CVAR, place),
                (crate::text::style::SCALE_CVAR, scale),
            ];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &cvars) else {
                return;
            };
            gpu.status_bar.preview = Some(made_up());
            let _ = frame(&mut gpu, 20);
            assert!(gpu.status_bar.drawn(), "{name}: the bar is drawn");
            println!("{}", shoot(&mut gpu, 2, name).display());
        }
    });
}

/// The bar over a recorded match's HUD (the demo `SJK_TEST_DEMO` names, frozen on its
/// first snapshot): at the bottom between the HUD's corners and at the top under the
/// match timer, at 4:3, 16:9, 21:9 and 4K, with the game HUD, SJK's classic layout (its
/// weapon box in the bottom's middle) and the radial one. The numbers are made up.
/// Skipped when `SJK_TEST_DEMO` is not set.
#[test]
#[ignore = "renders with the GPU, the installed game data and the demo SJK_TEST_DEMO names"]
fn status_bar_on_a_demo() {
    on_big_stack(|| {
        let Some(demo) = std::env::var_os("SJK_TEST_DEMO").map(PathBuf::from) else {
            println!("SJK_TEST_DEMO is not set: skipped");
            return;
        };
        for (size, scale, style, position, name) in [
            ([1920, 1080], "1", "game", "0", "status-bar-demo-bottom"),
            ([1920, 1080], "1", "game", "1", "status-bar-demo-top"),
            ([1440, 1080], "1", "game", "0", "status-bar-demo-4x3-bottom"),
            ([1440, 1080], "1", "game", "1", "status-bar-demo-4x3-top"),
            (
                [2560, 1080],
                "1",
                "game",
                "0",
                "status-bar-demo-21x9-bottom",
            ),
            (
                [3840, 2160],
                "1.4",
                "game",
                "0",
                "status-bar-demo-4k-bottom",
            ),
            ([3840, 2160], "1.4", "game", "1", "status-bar-demo-4k-top"),
            (
                [1920, 1080],
                "1",
                "classic",
                "0",
                "status-bar-demo-classic-bottom",
            ),
            (
                [1920, 1080],
                "1",
                "radial",
                "0",
                "status-bar-demo-radial-bottom",
            ),
        ] {
            let Some(mut gpu) = super::card_outline_shots::open_demo(
                &demo,
                demo_playback::Camera::FirstPerson,
                size,
            ) else {
                return;
            };
            if let Some(console) = gpu.console.as_mut() {
                console.set_cvar("cl_freezeDemo", "1");
                console.set_cvar(crate::status_bar::CVAR, "1");
                console.set_cvar(crate::status_bar::POSITION_CVAR, position);
                console.set_cvar(crate::text::style::SCALE_CVAR, scale);
                console.set_cvar("cg_hudStyle", style);
                console.set_cvar("cg_drawTimer", "1");
            }
            gpu.status_bar.preview = Some(made_up());
            let _ = frame(&mut gpu, 70);
            println!("{}", shoot(&mut gpu, 2, name).display());
            assert!(gpu.status_bar.drawn(), "{name}: the bar is drawn");
        }
    });
}

/// What the bar costs the CPU a frame (release build): handing on the shapes kept, as
/// most frames do, and laying it out again, as a frame does when its numbers, switches,
/// place or the screen change, each over 2 000 frames with every part showing, at 1080p
/// and 4K. The text buffers are emptied before each, as a frame starts with them empty.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn status_bar_cost() {
    on_big_stack(|| {
        for (size, name) in [([1920, 1080], "1080p"), ([3840, 2160], "4K")] {
            let cvars = [(crate::status_bar::CVAR, "1")];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &cvars) else {
                return;
            };
            gpu.status_bar.preview = Some(made_up());
            let _ = frame(&mut gpu, 20);
            let viewport = [size[0] as f32, size[1] as f32];
            for (way, again) in [("kept", false), ("laid out", true)] {
                const FRAMES: u32 = 2_000;
                let start = std::time::Instant::now();
                for _ in 0..FRAMES {
                    if again {
                        gpu.status_bar.forget_layout();
                    }
                    gpu.text_vertices.clear();
                    gpu.game_fonts.clear_text();
                    assert!(gpu.append_status_bar(viewport, true));
                }
                let each = start.elapsed() / FRAMES;
                println!(
                    "status bar {name}, {way}: {:.1} us a frame",
                    each.as_secs_f64() * 1e6
                );
            }
        }
    });
}

/// The News panel's settings to review as an updated player first sees them: the status
/// bar's switch (`= cg_statusBar reset` in the changelog) among the release's settings.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn status_bar_in_the_news() {
    on_big_stack(|| {
        let menu = menu::ClientMenu::new(true, String::new());
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
        ];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
        else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        gpu.open_news(Some(crate::news_panel::Key::Review), true);
        println!(
            "{}",
            shoot(&mut gpu, 6, "duel6-news-review-status-bar").display()
        );
        assert!(gpu.news.is_open(), "the panel shows");
    });
}
