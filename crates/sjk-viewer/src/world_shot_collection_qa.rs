//! The Collection before a release (Sol, 16/11/2026: "make sure all is optimized, looks
//! good, all the new stuffs we will activate work"): every page of it on one rich made-up
//! profile (kybers, a finisher, tags, cards, holocrons, dormant kybers, the Forge's day,
//! History, New marks) at 4:3 (1024 x 768 and 1600 x 1200), 1080p, 21:9 (2560 x 1080)
//! and 4K, each once its blade pictures are in; then the low-memory mode
//! (`ui_menuPictures 1`) and a profile holding nothing, at 1080p. Files
//! `qa-<size>-<page>`; set `SJK_QA_SIZES` to a comma list of widths for fewer. A page that
//! ran out of room is listed at the end, not asserted at once, so one run shows them all.
//! The kybers' looks come from the packs `SJK_TEST_PACKS` names. Ignored like the other
//! world shots.

use super::*;
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;

/// The sizes rendered, and the name each gets in a file.
const SIZES: [([u32; 2], &str); 5] = [
    ([1024, 768], "1024x768"),
    ([1600, 1200], "1600x1200"),
    ([1920, 1080], "1080p"),
    ([2560, 1080], "21x9"),
    ([3840, 2160], "4k"),
];

/// Whether a size's pages are timed too (1080p and 4K).
fn timing(size: [u32; 2]) -> bool {
    size == [1920, 1080] || size == [3840, 2160]
}

/// Draw frames until no blade picture is waiting (baked or read back), at most `limit`.
fn settle(gpu: &mut GpuState, limit: std::time::Duration) {
    let started = std::time::Instant::now();
    let mut quiet = 0;
    while quiet < 20 && started.elapsed() < limit {
        let before = gpu.blade_stills.baked;
        let _ = gpu.render(&mut None);
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let busy =
            gpu.blade_stills.baked != before || crate::blade_stills::waiting(&gpu.blade_stills) > 0;
        quiet = if busy { 0 } else { quiet + 1 };
    }
}

/// `frames` frames' host time (the GPU left to run), median and worst, and the median of
/// the phase that builds the menus (`view+menu+audio`), milliseconds.
fn timed(gpu: &mut GpuState, frames: usize) -> (f64, f64, f64) {
    let names = crate::frame_pacing::budget::NAMES;
    let menu = names
        .iter()
        .position(|name| *name == "view+menu+audio")
        .expect("the menus' phase");
    let mut host = Vec::new();
    let mut menus = Vec::new();
    let mut worst: (f64, usize, [f64; crate::frame_pacing::budget::NAMES.len()]) =
        (0.0, 0, [0.0; crate::frame_pacing::budget::NAMES.len()]);
    let mut tries = 0;
    while host.len() < frames && tries < 100_000 {
        tries += 1;
        let start = std::time::Instant::now();
        if gpu.render(&mut None) != FrameStatus::Rendered {
            std::thread::sleep(std::time::Duration::from_millis(1));
            continue;
        }
        let ms = start.elapsed().as_secs_f64() * 1_000.0;
        if ms > worst.0 {
            worst = (ms, host.len(), gpu.frame_pacer.last.phases);
        }
        host.push(ms);
        menus.push(gpu.frame_pacer.last.phases[menu]);
        gpu.frame_pacer.split.wait_previous();
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
    }
    let median = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values.get(values.len() / 2).copied().unwrap_or_default()
    };
    if worst.0 > 10.0 {
        let mut phases: Vec<(f64, &str)> = worst
            .2
            .iter()
            .zip(names)
            .filter(|(ms, _)| **ms >= 0.5)
            .map(|(ms, name)| (*ms, name))
            .collect();
        phases.sort_by(|a, b| b.0.total_cmp(&a.0));
        println!("  worst frame #{} {:.1} ms: {phases:?}", worst.1, worst.0);
    }
    (median(host), worst.0, median(menus))
}

/// The made-up profile: credits, shards, holocrons and dormant kybers waiting, the Forge's
/// day with a deal, three History lines.
fn rich() -> crate::console::profile_panel::Preview {
    let mut preview = super::preview();
    if let Some(me) = preview.snapshot.me.as_mut() {
        me.credits = 12_340;
        me.shards = 865;
        me.holocron_counts = sjk_identity::HolocronCounts {
            uncommon: 4,
            rare: 2,
            legendary: 1,
            mythical: 0,
        };
        me.holocrons = serde_json::from_str(
            r#"[{"id": 91, "tier": "kyber_rare"}, {"id": 90, "tier": "kyber_uncommon"},
                {"id": 89, "tier": "kyber_uncommon"}]"#,
        )
        .expect("made-up crystals");
        me.gif_tags = Some(Vec::new());
    }
    let mut shop = crate::console::collection_panel::forge::sample_day(3_600);
    shop.day.offers[2].deal = true;
    preview.snapshot.shop = Some(shop);
    preview.snapshot.history = Some(std::sync::Arc::new(sjk_identity::HistoryState {
        serial: 1,
        lines: {
            use crate::console::collection_panel::history::tests::{came, opened};
            vec![
                opened(1_793_440_000, "rare", 120, 0),
                came(1_793_400_000, "rare", "play"),
                opened(1_793_300_000, "uncommon", 60, 4),
            ]
        },
        more: false,
        before: 0,
        message: String::new(),
    }));
    preview
}

/// What the rich profile holds: every third kyber (the Sun worn, some spares), the
/// finisher, a dropped tag.
fn holdings(tag: &str) -> Vec<sjk_identity::Unlock> {
    let mut unlocks: Vec<sjk_identity::Unlock> = crate::unlockables::blade_skins()
        .filter(|skin| skin.id != "saber_sun")
        .step_by(3)
        .enumerate()
        .map(|(index, skin)| sjk_identity::Unlock {
            id: skin.id.to_owned(),
            count: 1 + u32::from(index % 5 == 0) * 2,
            ..super::sun()
        })
        .collect();
    unlocks.push(super::sun());
    for id in ["finisher_disintegrate", tag] {
        unlocks.push(sjk_identity::Unlock {
            id: id.to_owned(),
            ..super::sun()
        });
    }
    unlocks
}

fn cards() -> Vec<sjk_identity::CardStack> {
    serde_json::from_str(
        r#"[{"card": "sjk_alpha_000", "rarity": "mythical", "finish": "normal", "edition": "alpha", "count": 1},
            {"card": "sjk_alpha_001", "rarity": "legendary", "finish": "holo", "edition": "alpha", "count": 1},
            {"card": "sjk_alpha_003", "rarity": "rare", "finish": "normal", "edition": "alpha", "count": 3},
            {"rarity": "uncommon", "finish": "normal", "edition": "alpha", "count": 2}]"#,
    )
    .expect("made-up cards")
}

/// The pages shot at each size: tab, `ui_shaderList`, and the name in the file.
const PAGES: [(Tab, &str, &str); 13] = [
    (Tab::Hall, "0", "hall"),
    (Tab::Medals, "0", "honors"),
    (Tab::Achievements, "0", "achievements"),
    (Tab::Shaders, "0", "kybers-wall"),
    (Tab::Shaders, "1", "kybers-list"),
    (Tab::Toys, "0", "toys"),
    (Tab::Tags, "0", "tags"),
    (Tab::Holocrons, "0", "echoes"),
    (Tab::Forge, "0", "forge"),
    (Tab::Cards, "0", "cards"),
    (Tab::History, "0", "history"),
    (Tab::Inventory, "0", "inventory"),
    (Tab::Inventory, "0", "inventory-card"),
];

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_collection_qa() {
    on_big_stack(|| {
        super::super::saber_skins::mount_test_packs();
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parity-reports/collection-qa-cache");
        crate::blade_stills::set_folder(Some(cache));
        let tag = crate::tags::CATALOGUE
            .iter()
            .find(|tag| !tag.free)
            .expect("a dropped tag")
            .id;
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            (crate::finishers::CVAR, "finisher_disintegrate"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let widths: Option<Vec<u32>> = std::env::var("SJK_QA_SIZES").ok().map(|list| {
            list.split(',')
                .filter_map(|w| w.trim().parse().ok())
                .collect()
        });
        let mut overflows = Vec::new();
        let mut first = true;
        for (size, label) in SIZES {
            if widths
                .as_ref()
                .is_some_and(|widths| !widths.contains(&size[0]))
            {
                continue;
            }
            let Some((mut gpu, _profile)) = open(
                "maps/mp/duel6.bsp",
                size,
                Some(menu::ClientMenu::new(true, String::new())),
                &cvars,
            ) else {
                return;
            };
            gpu.looks.shot_owns_unlocks = true;
            let _ = frame(&mut gpu, 10);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_profile(rich());
                console.preview_collection(Some(holdings(tag)), 0.8);
                console.preview_cards(cards(), 12_340, 865);
                console.collection_marks_for_shot(Some(crate::shader_marks::Marks::shown(
                    &[],
                    &[],
                    &["saber_void", "saber_storm"],
                    Some("saber_sun"),
                )));
                use crate::collection_seen::Kind;
                console.collection_news_for_shot(Some(crate::collection_seen::News::shown(&[
                    (Kind::Card, "sjk_alpha_001", 1),
                    (Kind::Tag, tag, 0),
                ])));
                console.open_profile_hub_page(Tab::Hall, ReturnTarget::MainMenu);
            }
            gpu.illuminate.set_on(true);
            if first {
                // Every signature pipeline once, so the loops bake in their own looks.
                gpu.saber_gpu.build_every_signature();
                let started = std::time::Instant::now();
                loop {
                    let (built, loaded) = gpu.saber_gpu.signatures_built();
                    if loaded > 0 && built == loaded {
                        break;
                    }
                    assert!(started.elapsed().as_secs() < 3_000, "signatures building");
                    let _ = frame(&mut gpu, 10);
                }
                first = false;
            }
            for _ in 0..240 {
                let _ = frame(&mut gpu, 1);
                if crate::medals::art::decoded().is_some() {
                    break;
                }
            }
            for (tab, view, name) in PAGES {
                if let Some(console) = gpu.console.as_mut() {
                    let _ =
                        console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, view);
                    console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
                    if name == "inventory-card" {
                        console.inventory_choose_for_shot(3, 1);
                    }
                }
                gpu.profile_hub_show_for_shot(tab);
                settle(&mut gpu, std::time::Duration::from_secs(120));
                gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
                let file = format!("qa-{label}-{name}");
                println!("{}", shoot(&mut gpu, 30, &file).display());
                if overflowed(&gpu) {
                    overflows.push(file);
                }
                if timing(size) {
                    let (median, worst, menus) = timed(&mut gpu, 60);
                    println!(
                        "timing {label} {name:<16} host median {median:5.2} ms, worst \
                         {worst:5.2} ms, menus {menus:5.2} ms"
                    );
                }
            }
            // The hall's tile growing into its page (Enter on Kybers), then shrinking back.
            if timing(size) {
                if let Some(console) = gpu.console.as_mut() {
                    console.open_profile_hub_page(Tab::Hall, ReturnTarget::MainMenu);
                }
                settle(&mut gpu, std::time::Duration::from_secs(60));
                if let Some(console) = gpu.console.as_mut() {
                    console.collection_hall_for_shot(0, true);
                }
                let (median, worst, menus) = timed(&mut gpu, 20);
                println!(
                    "timing {label} {:<16} host median {median:5.2} ms, worst {worst:5.2} ms, \
                     menus {menus:5.2} ms",
                    "hall->kybers"
                );
            }
        }
        // Low memory (`ui_menuPictures 1`) and a profile holding nothing, at 1080p.
        if widths.as_ref().is_none_or(|widths| widths.contains(&1920)) {
            for (low, empty, label) in [(true, false, "low"), (false, true, "empty")] {
                let mut with = cvars.to_vec();
                if low {
                    with.push(("ui_menuPictures", "1"));
                }
                let Some((mut gpu, _profile)) = open(
                    "maps/mp/duel6.bsp",
                    [1920, 1080],
                    Some(menu::ClientMenu::new(true, String::new())),
                    &with,
                ) else {
                    return;
                };
                gpu.looks.shot_owns_unlocks = !empty;
                let _ = frame(&mut gpu, 10);
                if let Some(console) = gpu.console.as_mut() {
                    if empty {
                        let mut preview = super::preview();
                        if let Some(me) = preview.snapshot.me.as_mut() {
                            me.credits = 0;
                            me.shards = 0;
                            me.holocron_counts = Default::default();
                        }
                        console.preview_profile(preview);
                        console.preview_collection(Some(Vec::new()), 0.8);
                        console.preview_cards(Vec::new(), 0, 0);
                    } else {
                        console.preview_profile(rich());
                        console.preview_collection(Some(holdings(tag)), 0.8);
                        console.preview_cards(cards(), 12_340, 865);
                    }
                    console.open_profile_hub_page(Tab::Hall, ReturnTarget::MainMenu);
                }
                for (tab, view, name) in PAGES {
                    if let Some(console) = gpu.console.as_mut() {
                        let _ = console
                            .set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, view);
                        console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
                    }
                    gpu.profile_hub_show_for_shot(tab);
                    settle(&mut gpu, std::time::Duration::from_secs(60));
                    gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
                    let file = format!("qa-{label}-{name}");
                    println!("{}", shoot(&mut gpu, 30, &file).display());
                    if overflowed(&gpu) {
                        overflows.push(file);
                    }
                }
            }
        }
        crate::blade_stills::set_folder(None);
        assert!(overflows.is_empty(), "pages out of room: {overflows:?}");
    });
}
