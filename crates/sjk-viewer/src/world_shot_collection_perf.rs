//! What a Collection frame costs (Sol, 15/11/2026: "quite laggy atm even for me with a
//! big PC"): the hall, the Kybers wall and list, and the Inventory at 1080p and 4K, every
//! other kyber owned, once their blade pictures are in; each page timed over frames on
//! the host (the frame's phases) and on the GPU (its sections, `gpu_phases`), with one
//! tile looked at. The kybers' looks come from the packs `SJK_TEST_PACKS` names. Ignored
//! like the other world shots; prints a table.

use super::*;
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;

/// Frames timed a page.
const FRAMES: usize = 90;

#[derive(Default)]
struct Times {
    host: Vec<f64>,
    /// Host and the GPU waited for: the frame's whole time.
    whole: Vec<f64>,
    phases: Vec<[f64; crate::frame_pacing::budget::NAMES.len()]>,
    gpu: Vec<(f32, Vec<(&'static str, f32)>)>,
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or_default()
}

/// `FRAMES` frames of what `gpu` shows, timed.
fn time(gpu: &mut GpuState) -> Times {
    let mut times = Times::default();
    if let Some(phases) = &gpu.gpu_phases {
        phases.show(true);
    }
    let mut skipped = 0;
    while times.host.len() < FRAMES {
        let start = std::time::Instant::now();
        let status = gpu.render(&mut None);
        let host = start.elapsed().as_secs_f64() * 1_000.0;
        if status != FrameStatus::Rendered {
            skipped += 1;
            assert!(skipped < 100_000, "frames never rendered");
            std::thread::sleep(std::time::Duration::from_millis(1));
            continue;
        }
        times.host.push(host);
        times.phases.push(gpu.frame_pacer.last.phases);
        // Submitted (the submit thread may hold it), then done on the GPU.
        gpu.frame_pacer.split.wait_previous();
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        times.whole.push(start.elapsed().as_secs_f64() * 1_000.0);
        if let Some(phases) = &gpu.gpu_phases {
            phases.report();
            let mut sections = Vec::new();
            if let Some(total) = phases.take_sections(|name, ms| sections.push((name, ms))) {
                times.gpu.push((total, sections));
            }
        }
    }
    if let Some(phases) = &gpu.gpu_phases {
        phases.show(false);
    }
    println!("({skipped} attempts skipped by the pacing)");
    times
}

fn print(label: &str, times: &Times) {
    let host = median(times.host.clone());
    let whole = median(times.whole.clone());
    let mut line = format!("{label:<28} whole {whole:6.2} ms host {host:6.2} ms");
    // The host phases over 0.3 ms, by median.
    for (index, name) in crate::frame_pacing::budget::NAMES.iter().enumerate() {
        let ms = median(times.phases.iter().map(|phases| phases[index]).collect());
        if ms >= 0.3 {
            line.push_str(&format!(" | {name} {ms:.2}"));
        }
    }
    println!("{line}");
    if times.gpu.is_empty() {
        println!("{label:<28} gpu: no timestamps");
        return;
    }
    let total = median(
        times
            .gpu
            .iter()
            .map(|(total, _)| f64::from(*total))
            .collect(),
    );
    let mut line = format!("{label:<28} gpu  {total:6.2} ms");
    let names: Vec<&'static str> = times.gpu[0].1.iter().map(|(name, _)| *name).collect();
    let mut seen = Vec::new();
    for name in names {
        if seen.contains(&name) {
            continue;
        }
        seen.push(name);
        let ms = median(
            times
                .gpu
                .iter()
                .map(|(_, sections)| {
                    sections
                        .iter()
                        .filter(|(each, _)| *each == name)
                        .map(|(_, ms)| f64::from(*ms))
                        .sum()
                })
                .collect(),
        );
        if ms >= 0.05 {
            line.push_str(&format!(" | {name} {ms:.2}"));
        }
    }
    println!("{line}");
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_collection_perf() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parity-reports/collection-perf-cache");
        crate::blade_stills::set_folder(Some(cache));
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let sizes: Vec<[u32; 2]> = match std::env::var("SJK_PERF_SIZES").ok().as_deref() {
            Some("1080") => vec![[1920, 1080]],
            Some("4k") => vec![[3840, 2160]],
            _ => vec![[1920, 1080], [3840, 2160]],
        };
        for size in sizes {
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
            let tag = if size[0] > 2_000 { "4k" } else { "1080" };
            for (tab, view, hover, name) in [
                (Tab::Hall, "0", None, "hall"),
                (Tab::Shaders, "0", None, "wall"),
                (Tab::Shaders, "0", Some(2), "wall, one looked at"),
                (Tab::Shaders, "1", None, "list"),
                (Tab::Inventory, "0", None, "inventory"),
            ] {
                if let Some(console) = gpu.console.as_mut() {
                    let _ =
                        console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, view);
                    console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
                    console.collection_hover_for_shot(hover);
                }
                gpu.profile_hub_show_for_shot(tab);
                // Every picture in (baked or read back), then a settled frame.
                let mut quiet = 0;
                let started = std::time::Instant::now();
                while quiet < 30 && started.elapsed().as_secs() < 300 {
                    let before = gpu.blade_stills.baked;
                    let _ = gpu.render(&mut None);
                    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
                    let busy = gpu.blade_stills.baked != before
                        || crate::blade_stills::waiting(&gpu.blade_stills) > 0;
                    quiet = if busy { 0 } else { quiet + 1 };
                }
                let times = time(&mut gpu);
                print(&format!("{tag} {name}"), &times);
                let shot = format!(
                    "duel6-collection-perf-{tag}-{}",
                    name.replace([',', ' '], "-").replace("--", "-")
                );
                println!("{}", shoot(&mut gpu, 2, &shot).display());
            }
            if let Some(console) = gpu.console.as_mut() {
                console.collection_hover_for_shot(None);
            }
        }
        crate::blade_stills::set_folder(None);
    });
}

/// Frame times over a run: their median, 95th and 99th centile, the worst, and the frames
/// over 33 ms (a hitch at 30 FPS).
fn spread(label: &str, times: &[f64]) {
    let mut sorted = times.to_vec();
    sorted.sort_by(f64::total_cmp);
    let at = |share: f64| sorted[((sorted.len() - 1) as f64 * share) as usize];
    println!(
        "{label:<28} {} frames: median {:.2} ms, p95 {:.2}, p99 {:.2}, worst {:.2}, {} over 33 ms",
        sorted.len(),
        at(0.5),
        at(0.95),
        at(0.99),
        sorted.last().copied().unwrap_or_default(),
        sorted.iter().filter(|ms| **ms > 33.0).count(),
    );
}

/// The Collection opened cold, as a player's first time (Sol, 15/11/2026: laggy): no
/// blade picture on disk and no signature pipeline built, the Kybers wall opened and
/// walked down a row every few frames, each frame's whole time (host and GPU waited for)
/// recorded for `SECONDS`; then the same walk again with everything built and kept, as a
/// second visit.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_collection_cold() {
    const SECONDS: u64 = 25;
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parity-reports/collection-cold-cache");
        let _ = std::fs::remove_dir_all(&cache);
        crate::blade_stills::set_folder(Some(cache.clone()));
        let size = match std::env::var("SJK_PERF_SIZES").ok().as_deref() {
            Some("4k") => [3840, 2160],
            _ => [1920, 1080],
        };
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
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
        let _ = frame(&mut gpu, 30);
        // What a picture's key costs to work out (each hashes its skin's file and images).
        let skins = std::sync::Arc::clone(&gpu.blade_skins);
        let started = std::time::Instant::now();
        let mut keyed = 0;
        let mut worst = (0.0, "");
        for (_, skin) in skins.iter() {
            let one = std::time::Instant::now();
            if let Some(color) = skins.color_with(skin.id, Default::default()) {
                let _ = crate::blade_stills::skin_key(skin, color);
                keyed += 1;
            }
            let ms = one.elapsed().as_secs_f64() * 1_000.0;
            if ms > worst.0 {
                worst = (ms, skin.id);
            }
        }
        println!(
            "keys: {keyed} in {:.1} ms, worst {:.2} ms ({})",
            started.elapsed().as_secs_f64() * 1_000.0,
            worst.0,
            worst.1
        );
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
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
            console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
        }
        gpu.profile_hub_show_for_shot(Tab::Shaders);
        for visit in ["cold", "warm"] {
            let mut whole = Vec::new();
            let started = std::time::Instant::now();
            let mut row = 0;
            while started.elapsed().as_secs() < SECONDS {
                if whole.len() % 6 == 5 {
                    row = (row + 1) % 300;
                    if let Some(console) = gpu.console.as_mut() {
                        console.collection_shader_for_shot(row);
                    }
                }
                let start = std::time::Instant::now();
                if gpu.render(&mut None) != FrameStatus::Rendered {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }
                let host = start.elapsed().as_secs_f64() * 1_000.0;
                gpu.frame_pacer.split.wait_previous();
                let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
                let ms = start.elapsed().as_secs_f64() * 1_000.0;
                whole.push(ms);
                if ms > 33.0 {
                    let phases = gpu.frame_pacer.last.phases;
                    let mut worst: Vec<(f64, &str)> = crate::frame_pacing::budget::NAMES
                        .iter()
                        .enumerate()
                        .map(|(index, name)| (phases[index], *name))
                        .collect();
                    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
                    println!(
                        "{visit} hitch at frame {}: {ms:.1} ms, host {host:.1}: {:?}",
                        whole.len(),
                        &worst[..3]
                    );
                }
            }
            let (built, loaded) = gpu.saber_gpu.signatures_built();
            spread(&format!("{visit} wall walk"), &whole);
            println!(
                "{visit}: {built} of {loaded} signatures built, {} pictures baked",
                gpu.blade_stills.baked
            );
            if visit == "cold" {
                // Everything built and every loop kept, then the memory forgotten: the
                // second visit reads them back.
                gpu.saber_gpu.build_every_signature();
                let started = std::time::Instant::now();
                while started.elapsed().as_secs() < 600 {
                    let (built, loaded) = gpu.saber_gpu.signatures_built();
                    let quiet = crate::blade_stills::waiting(&gpu.blade_stills) == 0;
                    if built == loaded && quiet {
                        break;
                    }
                    let _ = frame(&mut gpu, 10);
                }
                for _ in 0..600 {
                    let _ = gpu.render(&mut None);
                }
                crate::blade_stills::forget_for_shot(&mut gpu.blade_stills);
                // The frames above queued on the GPU: done before the second visit is timed.
                gpu.frame_pacer.split.wait_previous();
                let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
            }
        }
        crate::blade_stills::set_folder(None);
    });
}
