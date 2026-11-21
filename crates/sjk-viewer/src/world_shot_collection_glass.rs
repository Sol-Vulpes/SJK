//! The Collection's acrylic glass (18/11/2026, Sol: "make the acrylic work on the
//! collection now (if not too heavy)"): the hall, the Kybers wall (the worn Sun chosen,
//! another tile looked at), the Kybers list and the Inventory over duel6, in the warm and
//! the cold theme, at 1080p and 4K (`SJK_GLASS_SIZES` `1080` or `4k` for one), every other
//! kyber owned. Each page is shot, then timed on the GPU (medians of sampled frames: the
//! whole frame, the blur's own `ui-backdrop` section and `overlays+post+hud`, which holds
//! the 2D layer), with the glass blurred and again drawn as its plain tint
//! (`BACKDROP_OFF`), so the blur's share shows apart from the drift between runs; then
//! Toys, Tags, Holocrons, Forge and History at 1080p, shot only. The kybers' looks come
//! from the packs `SJK_TEST_PACKS` names; `SJK_GLASS_LABEL` names the shots (`glass`).
//! Ignored like the other world shots; prints a table.

use super::*;
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;

/// Sampled frames a page is timed over, each way.
const FRAMES: usize = 120;

/// The medians `section_medians` gave for `name` (0 when the frame had none).
fn section(medians: &[(&'static str, f32)], name: &str) -> f32 {
    medians
        .iter()
        .filter(|(each, _)| *each == name)
        .map(|(_, ms)| *ms)
        .sum()
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_collection_glass() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parity-reports/collection-glass-cache");
        crate::blade_stills::set_folder(Some(cache));
        let sizes: Vec<[u32; 2]> = match std::env::var("SJK_GLASS_SIZES").ok().as_deref() {
            Some("1080") => vec![[1920, 1080]],
            Some("4k") => vec![[3840, 2160]],
            _ => vec![[1920, 1080], [3840, 2160]],
        };
        let label = std::env::var("SJK_GLASS_LABEL").unwrap_or_else(|_| "glass".to_owned());
        let mut table = Vec::new();
        for size in sizes {
            for theme in ["warm", "cold"] {
                let cvars = [
                    ("ui_menuStyle", "sjk"),
                    (crate::settings::quick::HIDE_CVAR, "1"),
                    (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
                    ("name", "^1Sol^7Vulpes"),
                    (crate::discord_presence::CVAR, "0"),
                    (crate::menu::sjk::theme::CVAR, theme),
                ];
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
                let owned: Vec<sjk_identity::Unlock> = crate::unlockables::ALL
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| index % 2 == 0)
                    .map(|(_, skin)| sjk_identity::Unlock {
                        id: skin.id.to_owned(),
                        ..super::collection::sun()
                    })
                    .collect();
                if let Some(console) = gpu.console.as_mut() {
                    console.preview_profile(super::collection::preview());
                    console.preview_collection(Some(owned), 0.0);
                }
                let tag = if size[0] > 2_000 { "4k" } else { "1080" };
                for (tab, view, hover, name) in [
                    (Tab::Hall, "0", None, "hall"),
                    (Tab::Shaders, "0", Some(5), "wall"),
                    (Tab::Shaders, "1", None, "list"),
                    (Tab::Inventory, "0", None, "inventory"),
                ] {
                    if let Some(console) = gpu.console.as_mut() {
                        let _ = console
                            .set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, view);
                        console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
                        console.collection_hover_for_shot(hover);
                    }
                    gpu.profile_hub_show_for_shot(tab);
                    // Every picture in (baked or read back), the page's motion over.
                    let mut quiet = 0;
                    let started = std::time::Instant::now();
                    while quiet < 30 && started.elapsed().as_secs() < 600 {
                        let before = gpu.blade_stills.baked;
                        let _ = gpu.render(&mut None);
                        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
                        let busy = gpu.blade_stills.baked != before
                            || crate::blade_stills::waiting(&gpu.blade_stills) > 0;
                        quiet = if busy { 0 } else { quiet + 1 };
                    }
                    let _ = frame(&mut gpu, 60);
                    let shot = format!("duel6-collection-{label}-{theme}-{tag}-{name}");
                    println!("{}", shoot(&mut gpu, 2, &shot).display());
                    let blurred = super::dynamic_light_cost::section_medians(&mut gpu, FRAMES);
                    let used = gpu.ui_shapes.backdrop_used();
                    crate::world_shot::BACKDROP_OFF.with(|off| off.set(true));
                    let _ = frame(&mut gpu, 10);
                    let plain = super::dynamic_light_cost::section_medians(&mut gpu, FRAMES);
                    crate::world_shot::BACKDROP_OFF.with(|off| off.set(false));
                    let _ = frame(&mut gpu, 10);
                    let line = format!(
                        "{label:<6} {theme:<4} {tag:<4} {name:<10} frame {:6.3} ms | \
                         ui-backdrop {:6.3} ms | post+hud {:6.3} ms | plain tint: frame \
                         {:6.3} ms, post+hud {:6.3} ms | blurred {used}",
                        section(&blurred, "total"),
                        section(&blurred, "ui-backdrop"),
                        section(&blurred, "overlays+post+hud"),
                        section(&plain, "total"),
                        section(&plain, "overlays+post+hud"),
                    );
                    println!("{line}");
                    if name == "wall" {
                        let sections: Vec<String> = blurred
                            .iter()
                            .filter(|(_, ms)| *ms >= 0.02)
                            .map(|(name, ms)| format!("{name} {ms:.3}"))
                            .collect();
                        println!("  sections: {}", sections.join(", "));
                    }
                    table.push(line);
                }
                if let Some(console) = gpu.console.as_mut() {
                    console.collection_hover_for_shot(None);
                }
                // The other kinds that took glass, shot once each to look at, not timed.
                for tab in [
                    Tab::Toys,
                    Tab::Tags,
                    Tab::Holocrons,
                    Tab::Forge,
                    Tab::History,
                ] {
                    if tag != "1080" {
                        break;
                    }
                    if let Some(console) = gpu.console.as_mut() {
                        console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
                    }
                    gpu.profile_hub_show_for_shot(tab);
                    let shot = format!(
                        "duel6-collection-{label}-{theme}-{tag}-{}",
                        format!("{tab:?}").to_lowercase()
                    );
                    println!("{}", shoot(&mut gpu, 90, &shot).display());
                }
            }
        }
        println!("--- {label}");
        for line in table {
            println!("{line}");
        }
        crate::blade_stills::set_folder(None);
    });
}
