//! World shots of the Collection's blade stills (`blade_stills.rs`, 15/11/2026): every
//! other kyber owned, over the live duel6 in the SJK UI, from a fresh cache folder under
//! `target/parity-reports`: the stills baked (and what a bake costs the frame), the Kybers
//! wall (owned and not owned), the list and the hall; then the stills forgotten, as a
//! client starting again, and read back from disk without a bake. The kybers' looks come
//! from the packs `SJK_TEST_PACKS` names. Ignored like the other world shots.

use super::*;
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;

/// Frames timed with no bake, for the reference.
const REFERENCE_FRAMES: usize = 120;

/// One frame's time on the GPU (wall clock, waited for), in milliseconds, and the stills
/// baked in it.
fn timed(gpu: &mut GpuState) -> (f64, u64) {
    let before = gpu.blade_stills.baked;
    let start = std::time::Instant::now();
    let _ = gpu.render(&mut None);
    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
    (
        start.elapsed().as_secs_f64() * 1_000.0,
        gpu.blade_stills.baked - before,
    )
}

fn median(mut times: Vec<f64>) -> Option<f64> {
    times.sort_by(f64::total_cmp);
    times.get(times.len() / 2).copied()
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_blade_stills() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parity-reports/blade-stills-cache");
        let _ = std::fs::remove_dir_all(&cache);
        crate::blade_stills::set_folder(Some(cache.clone()));
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        // Every other kyber held, so the wall shows both looks side by side.
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
            console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
        }
        gpu.profile_hub_show_for_shot(Tab::Shaders);
        // Every signature's pipelines first, so the bakes are not held back by builds.
        gpu.saber_gpu.build_every_signature();
        let started = std::time::Instant::now();
        loop {
            let (built, loaded) = gpu.saber_gpu.signatures_built();
            if loaded > 0 && built == loaded {
                println!("{built} signature shaders built");
                break;
            }
            assert!(
                started.elapsed().as_secs() < 3_000,
                "signature shaders still building: {built} of {loaded}"
            );
            let _ = frame(&mut gpu, 10);
        }
        // Baked again from nothing (the wall baked its stills while the signatures built),
        // each frame timed: the wall's stills, then the others in the background.
        let _ = std::fs::remove_dir_all(&cache);
        crate::blade_stills::forget_for_shot(&mut gpu.blade_stills);
        let mut by_bakes: [Vec<f64>; crate::blade_stills::BAKE_BATCH + 1] = Default::default();
        let started = std::time::Instant::now();
        let mut idle = 0;
        while idle < 30 {
            let (ms, baked) = timed(&mut gpu);
            by_bakes[(baked as usize).min(crate::blade_stills::BAKE_BATCH)].push(ms);
            idle = if baked == 0 { idle + 1 } else { 0 };
            assert!(started.elapsed().as_secs() < 600, "still baking");
        }
        let reference =
            median((0..REFERENCE_FRAMES).map(|_| timed(&mut gpu).0).collect()).unwrap_or_default();
        println!(
            "{} stills baked; frame with none {reference:.3} ms",
            gpu.blade_stills.baked
        );
        for (count, times) in by_bakes.iter().enumerate().skip(1) {
            let frames = times.len();
            if let Some(median) = median(times.clone()) {
                println!(
                    "{count} bakes: {frames} frames, median {median:.3} ms, {:+.3} ms a bake",
                    (median - reference) / count as f64
                );
            }
        }
        assert_eq!(
            crate::blade_stills::waiting(&gpu.blade_stills),
            0,
            "every still on the wall shown"
        );
        let path = shoot(&mut gpu, 30, "duel6-blade-stills-wall");
        println!("{}", path.display());
        assert!(!super::collection::overflowed(&gpu));
        // A not-held tile hovered: its still dimmed; a held one: live.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_hover_for_shot(Some(2));
        }
        println!(
            "{}",
            shoot(&mut gpu, 30, "duel6-blade-stills-wall-hover").display()
        );
        if let Some(console) = gpu.console.as_mut() {
            console.collection_hover_for_shot(None);
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "1");
        }
        gpu.profile_hub_show_for_shot(Tab::Shaders);
        println!(
            "{}",
            shoot(&mut gpu, 60, "duel6-blade-stills-list").display()
        );
        gpu.profile_hub_show_for_shot(Tab::Hall);
        gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
        println!(
            "{}",
            shoot(&mut gpu, 120, "duel6-blade-stills-hall").display()
        );
        // Kept on disk: forgotten in memory, the wall's stills come back without a bake.
        let _ = frame(&mut gpu, 30);
        let kept = std::fs::read_dir(&cache).map_or(0, |entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "png"))
                .count()
        });
        println!("{kept} stills kept in {}", cache.display());
        assert!(kept > 0, "stills written");
        crate::blade_stills::forget_for_shot(&mut gpu.blade_stills);
        if let Some(console) = gpu.console.as_mut() {
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
        }
        gpu.profile_hub_show_for_shot(Tab::Shaders);
        let mut frames = 0;
        let mut first_bakes = 0;
        loop {
            let (_, baked) = timed(&mut gpu);
            frames += 1;
            if frames <= 20 {
                first_bakes += baked;
            }
            if frames > 3 && crate::blade_stills::waiting(&gpu.blade_stills) == 0 {
                break;
            }
            assert!(frames < 600, "stills not read back");
        }
        println!("read back from disk in {frames} frames, {first_bakes} baked meanwhile");
        assert_eq!(first_bakes, 0, "nothing on the wall baked again");
        println!(
            "{}",
            shoot(&mut gpu, 10, "duel6-blade-stills-wall-from-disk").display()
        );
        crate::blade_stills::set_folder(None);
    });
}
