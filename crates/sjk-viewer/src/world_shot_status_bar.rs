//! World shots of the status bars ([`crate::status_bar`]): both bars docked to duel6's
//! screen edges with every part (1080p, 4K with the largest menu text, 21:9 and 4:3);
//! the same over a recorded match's HUD in the game, classic and radial looks, with a
//! vote and the devmap bar, each checking that every HUD element stands inside the bars;
//! what they cost the CPU a frame; and the News panel's settings to review. What the
//! bars show is made up ([`State::preview`], [`State::preview_top`]): a wallet, a
//! Legendary blade, weapons, Force powers, a match with a busy join and leave feed.
//! Off-screen renders (no game window), ignored like the other world shots: they need a
//! GPU adapter and the installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> SJK_TEST_DEMO=<demo> cargo test --release -p
//! sjk-viewer status_bar -- --ignored --test-threads=1 --nocapture`; the pictures are in
//! `target/world-shots`.
//!
//! [`State::preview`]: crate::status_bar::State::preview
//! [`State::preview_top`]: crate::status_bar::State::preview_top

use super::*;
use crate::status_bar::{Effect, Inputs, Line, Score, TOY_COUNT, TopInputs, toys};

/// A player with a Legendary blade and a stock second, two things on, four toys out, a
/// blaster in hand among five weapons, at 21:07 on 16/11/2026.
fn made_up(gpu: &GpuState) -> Inputs {
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
        clock: Some(crate::status_bar::Clock {
            hour: 21,
            minute: 7,
        }),
        date: Some(crate::status_bar::Date {
            day: 16,
            month: 11,
            year: 2026,
        }),
        ..Inputs::default()
    };
    for (words, colour) in [
        ("Flare trail", crate::menu::sjk::color::gold_bright()),
        ("Do not disturb", crate::menu::sjk::color::ember()),
    ] {
        inputs.push_effect(Effect { words, colour });
    }
    let mut on = [false; TOY_COUNT];
    for toy in [toys::ILLUMINATE, toys::HAX, toys::BOX_HUNT, toys::PROPGUN] {
        on[toy] = true;
    }
    inputs.toys = on;
    // Melee, saber, pistol, blaster (in hand), thermal detonators.
    let mut slot = 0;
    for weapon in [2_u8, 3, 4, 5, 12] {
        if let Some(texture) = gpu.hud.icons.weapon_select(weapon, false, 0) {
            inputs.weapons.held[slot] = Some((texture, weapon == 5));
            slot += 1;
        }
    }
    inputs.weapons.ammo = Some((Some(112), false));
    inputs
}

/// A duel server's match: Push selected, Speed in use, second place, 12:34 left, three
/// players coming and going (the oldest fading), 7 of 24 players, 48 ms, 144 fps.
fn made_up_top(gpu: &GpuState) -> TopInputs {
    let icons = gpu.hud.power_icons();
    let mut top = TopInputs {
        score: Some(Score::Place {
            place: 2,
            tied: false,
            score: 15,
        }),
        left: Some(crate::status_bar::Left(754)),
        server: Line::from("^4JoF ^7| Duels and FFA ^1EU"),
        map: Line::from("duel6"),
        mode: Some("Duel"),
        players: Some((7, 24)),
        ping: Some(48),
        fps: Some(144),
        ..TopInputs::default()
    };
    // Heal, Speed, Push, Pull, Mind Trick, Grip, Lightning, Protect, Absorb.
    let mut index = 0;
    for slot in [0_u8, 2, 3, 4, 5, 6, 7, 9, 10] {
        if let Some(texture) = crate::hud::force_wheel::icon(&icons, slot, false) {
            top.powers[index] = Some((texture, slot == 3, slot == 2));
            index += 1;
        }
    }
    top.feed = crate::status_bar::feed_for_shot(&[
        ("^1Kyle^7Katarn", true, 16),
        ("Padawan", false, 16),
        ("^5Luke", true, 9),
    ]);
    top
}

/// Show both bars with their made-up contents.
fn preview(gpu: &mut GpuState) {
    gpu.status_bar.preview = Some(made_up(gpu));
    gpu.status_bar.preview_top = Some(made_up_top(gpu));
}

/// Every HUD element's box this frame stands between the bars, within half a pixel.
fn assert_inside(gpu: &GpuState, size: [u32; 2], name: &str) {
    let room = gpu.status_bar.reserved();
    assert!(
        room.top > 0.0 && room.bottom > 0.0,
        "{name}: both bars keep room"
    );
    let height = size[1] as f32;
    let mut seen = 0;
    for (element, rect) in gpu.hud.element_bounds.iter() {
        assert!(
            rect.y >= room.top - 0.5 && rect.bottom() <= height - room.bottom + 0.5,
            "{name}: {element:?} at {rect:?} crosses a bar ({room:?})"
        );
        seen += 1;
    }
    println!("{name}: {seen} HUD elements inside the bars");
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn status_bar_shots() {
    on_big_stack(|| {
        for (size, scale, name) in [
            ([1920, 1080], "1", "status-bars-1080p"),
            ([3840, 2160], "1.4", "status-bars-4k-large-text"),
            ([2560, 1080], "1", "status-bars-21x9"),
            ([1440, 1080], "1", "status-bars-4x3"),
        ] {
            let cvars = [(crate::text::style::SCALE_CVAR, scale)];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &cvars) else {
                return;
            };
            preview(&mut gpu);
            let _ = frame(&mut gpu, 20);
            assert_eq!(gpu.status_bar.drawn_bars(), [true, true], "{name}");
            println!("{}", shoot(&mut gpu, 2, name).display());
        }
    });
}

/// The bars over a recorded match's HUD (the demo `SJK_TEST_DEMO` names, frozen on its
/// first snapshot), every HUD element checked to stand inside them: the game HUD, SJK's
/// classic layout and the radial one at 16:9, the game HUD at 4K, 21:9 and 4:3, then
/// with a vote on and the devmap bar at the bottom or the top. Skipped when
/// `SJK_TEST_DEMO` is not set.
#[test]
#[ignore = "renders with the GPU, the installed game data and the demo SJK_TEST_DEMO names"]
fn status_bar_on_a_demo() {
    on_big_stack(|| {
        let Some(demo) = std::env::var_os("SJK_TEST_DEMO").map(PathBuf::from) else {
            println!("SJK_TEST_DEMO is not set: skipped");
            return;
        };
        // Size, text size, HUD look, a vote, the devmap bar (cg_devBar), name.
        for (size, scale, style, vote, devbar, name) in [
            (
                [1920, 1080],
                "1",
                "game",
                false,
                None,
                "status-bars-demo-game",
            ),
            (
                [1920, 1080],
                "1",
                "classic",
                false,
                None,
                "status-bars-demo-classic",
            ),
            (
                [1920, 1080],
                "1",
                "radial",
                false,
                None,
                "status-bars-demo-radial",
            ),
            (
                [3840, 2160],
                "1.4",
                "game",
                false,
                None,
                "status-bars-demo-4k",
            ),
            (
                [2560, 1080],
                "1",
                "radial",
                false,
                None,
                "status-bars-demo-21x9",
            ),
            (
                [1440, 1080],
                "1",
                "game",
                false,
                None,
                "status-bars-demo-4x3",
            ),
            (
                [1920, 1080],
                "1",
                "classic",
                true,
                Some("1"),
                "status-bars-demo-vote-devbar",
            ),
            (
                [1920, 1080],
                "1",
                "game",
                true,
                Some("2"),
                "status-bars-demo-devbar-top",
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
                console.set_cvar(crate::text::style::SCALE_CVAR, scale);
                console.set_cvar("cg_hudStyle", style);
                console.set_cvar("cg_drawTimer", "1");
                console.set_cvar("cg_drawFps", "1");
                console.set_cvar("cg_lagometer", "1");
                if let Some(devbar) = devbar {
                    console.set_cvar(crate::devmap_bar::CVAR, devbar);
                }
            }
            gpu.devmap_bar.forced = devbar.is_some();
            if vote {
                gpu.hud.preview_vote_command(&[]).unwrap();
            }
            preview(&mut gpu);
            let _ = frame(&mut gpu, 70);
            println!("{}", shoot(&mut gpu, 2, name).display());
            assert_eq!(gpu.status_bar.drawn_bars(), [true, true], "{name}");
            assert_inside(&gpu, size, name);
            if devbar.is_some() {
                assert!(gpu.devmap_bar.shown(), "{name}: the devmap bar shows");
            }
        }
    });
}

/// What the bars cost the CPU a frame (release build), both showing every part: handing
/// on the shapes kept, as most frames do, and laying both out again, as a frame does
/// when what they show, their switches or the screen change, each over 2 000 frames at
/// 1080p and 4K. The reserve step (reads due four times a second, the feed's step) is
/// in both. The text buffers are emptied before each, as a frame starts with them empty.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn status_bar_cost() {
    on_big_stack(|| {
        for (size, name) in [([1920, 1080], "1080p"), ([3840, 2160], "4K")] {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &[]) else {
                return;
            };
            preview(&mut gpu);
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
                    gpu.reserve_status_bars(viewport, true);
                    assert_eq!(gpu.append_status_bars(viewport), [true, true]);
                }
                let each = start.elapsed() / FRAMES;
                println!(
                    "status bars {name}, {way}: {:.1} us a frame",
                    each.as_secs_f64() * 1e6
                );
            }
        }
    });
}

/// The News panel's settings to review as an updated player first sees them: both bars'
/// switches (`= cg_statusBar reset` and `= cg_statusBarTop reset` in the changelog)
/// among the release's settings.
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
            shoot(&mut gpu, 6, "duel6-news-review-status-bars").display()
        );
        assert!(gpu.news.is_open(), "the panel shows");
    });
}

/// What the bars' acrylic costs the GPU (`ui_renderer/backdrop.rs`): the blur's own
/// section (`ui-backdrop`, its passes round the two bars) and the whole frame's median,
/// with both bars over duel6, at 1080p and 4K (`SJK_COST_SIZE` adds a size), against the
/// same frames with the bars' glass drawn as its plain tint (no blur). Prints the medians
/// over 240 sampled frames after a warm-up, the plain tint's before and after the blur's.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn status_bar_acrylic_cost() {
    on_big_stack(|| {
        let extra = std::env::var("SJK_COST_SIZE").ok().and_then(|size| {
            let (w, h) = size.split_once('x')?;
            Some([w.parse::<u32>().ok()?, h.parse::<u32>().ok()?])
        });
        for size in [[1920, 1080], [3840, 2160]].into_iter().chain(extra) {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &[]) else {
                return;
            };
            preview(&mut gpu);
            let _ = frame(&mut gpu, 300);
            // The plain tint before and after, so a drift of the scene shows.
            crate::world_shot::BACKDROP_OFF.with(|off| off.set(true));
            let _ = frame(&mut gpu, 30);
            let before = super::dynamic_light_cost::section_medians(&mut gpu, 240);
            crate::world_shot::BACKDROP_OFF.with(|off| off.set(false));
            let _ = frame(&mut gpu, 30);
            let blurred = super::dynamic_light_cost::section_medians(&mut gpu, 240);
            assert!(gpu.ui_shapes.backdrop_used(), "the bars blur the frame");
            let backdrop = blurred
                .iter()
                .filter(|(name, _)| *name == "ui-backdrop")
                .map(|(_, ms)| *ms)
                .sum::<f32>();
            let total = |medians: &[(&'static str, f32)]| {
                medians
                    .iter()
                    .find(|(name, _)| *name == "total")
                    .map_or(f32::NAN, |(_, ms)| *ms)
            };
            crate::world_shot::BACKDROP_OFF.with(|off| off.set(true));
            let _ = frame(&mut gpu, 30);
            let after = super::dynamic_light_cost::section_medians(&mut gpu, 240);
            crate::world_shot::BACKDROP_OFF.with(|off| off.set(false));
            println!(
                "acrylic bars {}x{}: blur {backdrop:.3} ms, frame {:.3} ms (plain {:.3}, {:.3} ms)",
                size[0],
                size[1],
                total(&blurred),
                total(&before),
                total(&after),
            );
        }
    });
}
