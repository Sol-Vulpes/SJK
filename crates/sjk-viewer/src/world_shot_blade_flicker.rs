//! Does a blade flicker in the menus (Sol, 16/11/2026: "the blade flickered kinda in the
//! menu, hard to tell why")? Runs of consecutive frames of the main menu's stage, the
//! Collection's hall (its Kybers strips and the stage beside it) and the Kybers wall (its
//! tiles' loops and the chosen kyber's live card), each read back, and in each region the
//! light of every 16-pixel cell followed from frame to frame: how much it jitters, its
//! worst sudden drop, and how many drops of over 15% a second. Once with every loop in,
//! once while the loops are being baked. The loops play on their own clock
//! ([`crate::blade_stills::SHOT_CLOCK`]) stepped a 60th of a second a frame; the live
//! blades on the real one, the frames 16 ms apart. Ignored like the other world shots;
//! prints a table and keeps a few frames.

use super::*;
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;

/// Frames followed a run (two seconds at 60 a second: two and a half loops).
const FRAMES: usize = 120;
/// A cell's side in pixels.
const CELL: u32 = 16;
/// A cell lit enough to follow (mean light, 0 to 255).
const LIT: f32 = 10.0;

/// A region of the frame: its name and rectangle `[x, y, width, height]` at 1080p.
type Region = (&'static str, [u32; 4]);

/// The light of each cell of `region` in `image`: the mean of 0.2126 R + 0.7152 G +
/// 0.0722 B.
fn cells(image: &image::RgbaImage, [x0, y0, width, height]: [u32; 4]) -> Vec<f32> {
    let mut out = Vec::new();
    for cy in (y0..y0 + height).step_by(CELL as usize) {
        for cx in (x0..x0 + width).step_by(CELL as usize) {
            let mut sum = 0.0;
            let mut count = 0.0;
            for y in cy..(cy + CELL).min(y0 + height).min(image.height()) {
                for x in cx..(cx + CELL).min(x0 + width).min(image.width()) {
                    let [r, g, b, _] = image.get_pixel(x, y).0;
                    sum += 0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b);
                    count += 1.0;
                }
            }
            out.push(if count > 0.0 { sum / count } else { 0.0 });
        }
    }
    out
}

/// Per region: the median and 95th centile of the lit cells' jitter (mean |step| over
/// their mean light), the worst drop of any cell in one frame (share of its light), and
/// the drops over 15% a cell a second.
fn report(label: &str, regions: &[Region], series: &[Vec<Vec<f32>>], seconds: f64) {
    for (index, (name, _)) in regions.iter().enumerate() {
        let frames = &series[index];
        let cells = frames.first().map_or(0, Vec::len);
        let mut jitters = Vec::new();
        let mut worst: f32 = 0.0;
        let mut drops = 0usize;
        let mut lit = 0usize;
        for cell in 0..cells {
            let values: Vec<f32> = frames.iter().map(|frame| frame[cell]).collect();
            let mean = values.iter().sum::<f32>() / values.len() as f32;
            if mean < LIT {
                continue;
            }
            lit += 1;
            let mut jitter = 0.0;
            for pair in values.windows(2) {
                let step = (pair[1] - pair[0]) / mean;
                jitter += step.abs();
                let drop = (pair[0] - pair[1]) / pair[0].max(1.0);
                worst = worst.max(drop);
                if drop > 0.15 {
                    drops += 1;
                }
            }
            jitters.push(jitter / (values.len() - 1) as f32);
        }
        jitters.sort_by(f32::total_cmp);
        let at = |share: f32| {
            jitters
                .get(((jitters.len().saturating_sub(1)) as f32 * share) as usize)
                .copied()
                .unwrap_or_default()
        };
        println!(
            "{label:<14} {name:<12} {lit:>4} lit cells | jitter median {:5.3} p95 {:5.3} | \
             worst drop {:4.0}% | drops >15%: {:6.2} a cell a second",
            at(0.5),
            at(0.95),
            worst * 100.0,
            drops as f64 / lit.max(1) as f64 / seconds,
        );
    }
}

/// [`FRAMES`] frames of what `gpu` shows, each read back after its submission, the loops'
/// clock stepped from `clock` a 60th of a second a frame; the cells of `regions` in each.
/// Keeps every 10th frame as `name-<n>` when `keep`.
fn follow(
    gpu: &mut GpuState,
    regions: &[Region],
    clock: f32,
    name: &str,
    keep: bool,
) -> (Vec<Vec<Vec<f32>>>, f64) {
    let mut series = vec![Vec::new(); regions.len()];
    let started = std::time::Instant::now();
    let mut frame = 0;
    let mut attempts = 0;
    while frame < FRAMES {
        *crate::blade_stills::SHOT_CLOCK.lock().unwrap() = Some(clock + frame as f32 / 60.0);
        let status = gpu.render(&mut None);
        attempts += 1;
        assert!(attempts < 100_000, "frames never rendered");
        if status != FrameStatus::Rendered {
            std::thread::sleep(std::time::Duration::from_millis(1));
            continue;
        }
        gpu.frame_pacer.split.wait_previous();
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let texture = gpu.headless_frame.as_ref().expect("a world shot target");
        let image = read_back(&gpu.device, &gpu.queue, texture);
        for (index, (_, rect)) in regions.iter().enumerate() {
            series[index].push(cells(&image, *rect));
        }
        if keep && frame % 10 == 0 {
            std::fs::create_dir_all(directory()).expect("the shot directory");
            image
                .save(directory().join(format!("{name}-{frame:03}.png")))
                .expect("write the frame");
        }
        frame += 1;
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    *crate::blade_stills::SHOT_CLOCK.lock().unwrap() = None;
    (series, started.elapsed().as_secs_f64())
}

/// Render until no loop waits and nothing is baked for 30 frames.
fn settle(gpu: &mut GpuState) {
    let mut quiet = 0;
    let started = std::time::Instant::now();
    while quiet < 30 && started.elapsed().as_secs() < 600 {
        let before = gpu.blade_stills.baked;
        let _ = gpu.render(&mut None);
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let busy =
            gpu.blade_stills.baked != before || crate::blade_stills::waiting(&gpu.blade_stills) > 0;
        quiet = if busy { 0 } else { quiet + 1 };
    }
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_blade_flicker() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parity-reports/blade-flicker-cache");
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
        // The main menu: its stage, the whole right of the screen.
        let menu_regions: [Region; 2] = [
            ("stage", [960, 120, 960, 900]),
            ("left", [0, 120, 960, 900]),
        ];
        let (series, seconds) = follow(&mut gpu, &menu_regions, 0.0, "blade-flicker-menu", true);
        report("main menu", &menu_regions, &series, seconds);

        let owned: Vec<sjk_identity::Unlock> = crate::unlockables::ALL
            .iter()
            .map(|skin| sjk_identity::Unlock {
                id: skin.id.to_owned(),
                ..super::collection::sun()
            })
            .collect();
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(super::collection::preview());
            console.preview_collection(Some(owned), 0.0);
        }
        let hall: [Region; 2] = [
            ("strips", [110, 230, 170, 160]),
            ("stage", [1420, 300, 500, 720]),
        ];
        let wall: [Region; 3] = [
            ("tiles", [80, 430, 870, 510]),
            ("card blade", [1090, 270, 830, 190]),
            ("hilts", [80, 430, 120, 510]),
        ];
        for (warm, label) in [(false, "baking"), (true, "warm")] {
            if !warm {
                crate::blade_stills::forget_for_shot(&mut gpu.blade_stills);
                let _ = std::fs::remove_dir_all(&cache);
            }
            for (tab, regions, name) in [
                (Tab::Hall, &hall[..], "hall"),
                (Tab::Shaders, &wall[..], "wall"),
            ] {
                if let Some(console) = gpu.console.as_mut() {
                    let _ =
                        console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
                    console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
                    if tab == Tab::Shaders {
                        console.collection_kyber_for_shot("saber_sun");
                    }
                }
                gpu.profile_hub_show_for_shot(tab);
                if warm {
                    settle(&mut gpu);
                } else {
                    let _ = frame(&mut gpu, 20);
                }
                let baked = gpu.blade_stills.baked;
                let (series, seconds) = follow(
                    &mut gpu,
                    regions,
                    1.0,
                    &format!("blade-flicker-{name}-{label}"),
                    true,
                );
                report(&format!("{name} {label}"), regions, &series, seconds);
                println!(
                    "{:<14} {} pictures baked over the run",
                    format!("{name} {label}"),
                    gpu.blade_stills.baked - baked
                );
            }
        }
        // Sernpidal dust, chosen: its live card and its loop.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_kyber_for_shot("saber_sernpidal_dust");
        }
        settle(&mut gpu);
        let (series, seconds) = follow(&mut gpu, &wall, 1.0, "blade-flicker-sernpidal", true);
        report("sernpidal", &wall, &series, seconds);
        let card: Vec<f32> = series[1]
            .iter()
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect();
        println!(
            "sernpidal card light by frame: {}",
            card.iter()
                .step_by(5)
                .map(|light| format!("{light:.1}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        crate::blade_stills::set_folder(None);
    });
}

/// Walking the Kybers wall across New and seen kybers (the release review, 16/11/2026: the
/// stage put a lit blade out for each New kyber and lit it again for each seen one): Sun
/// chosen until its first look, then Sun and a New kyber in turn, six frames each; the
/// stage's blade above the card followed frame by frame, and the frames it was out counted.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_stage_sweep() {
    const NEW: [&str; 6] = [
        "saber_hosnian_daybreak",
        "saber_zeltros_violet",
        "saber_bardotta_calm",
        "saber_tython_jedaii",
        "saber_dorin_pewter",
        "saber_sorgan_lake",
    ];
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
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
        let owned: Vec<sjk_identity::Unlock> = crate::unlockables::ALL
            .iter()
            .map(|skin| sjk_identity::Unlock {
                id: skin.id.to_owned(),
                ..super::collection::sun()
            })
            .collect();
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(super::collection::preview());
            console.preview_collection(Some(owned), 0.0);
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
            console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
            console.collection_kyber_for_shot("saber_sun");
        }
        gpu.profile_hub_show_for_shot(Tab::Shaders);
        // Sun's pipelines in, and its first look had (a second and a half chosen).
        let sun = gpu.blade_skins.index_of("saber_sun").expect("Sun loaded");
        gpu.saber_gpu.ask_build(sun, true);
        let started = std::time::Instant::now();
        while !gpu.saber_gpu.skin_ready(sun) && started.elapsed().as_secs() < 600 {
            let _ = frame(&mut gpu, 5);
        }
        let _ = frame(&mut gpu, 100);
        // The six New, Sun seen.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_marks_for_shot(Some(crate::shader_marks::Marks::shown(
                &[],
                &[],
                &NEW,
                Some("saber_sun"),
            )));
        }
        let _ = frame(&mut gpu, 10);
        let stage: [Region; 1] = [("stage blade", [1090, 270, 830, 190])];
        let mut lights = Vec::new();
        for step in 0..12 {
            let id = if step % 2 == 0 {
                "saber_sun"
            } else {
                NEW[step / 2]
            };
            if let Some(console) = gpu.console.as_mut() {
                console.collection_kyber_for_shot(id);
            }
            let (series, _) = follow_frames(&mut gpu, &stage, 6);
            lights.extend(
                series[0]
                    .iter()
                    .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32),
            );
        }
        let mut sorted = lights.clone();
        sorted.sort_by(f32::total_cmp);
        let lit = sorted[sorted.len() * 3 / 4];
        let out = lights.iter().filter(|light| **light < lit * 0.5).count();
        println!(
            "stage sweep: {} frames, the blade out in {out} (light under half of {lit:.1}); by \
             frame: {}",
            lights.len(),
            lights
                .iter()
                .map(|light| format!("{light:.0}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    });
}

/// `frames` frames read back as [`follow`] reads them, the loops on the real clock.
fn follow_frames(
    gpu: &mut GpuState,
    regions: &[Region],
    frames: usize,
) -> (Vec<Vec<Vec<f32>>>, f64) {
    let mut series = vec![Vec::new(); regions.len()];
    let started = std::time::Instant::now();
    let mut done = 0;
    while done < frames {
        if gpu.render(&mut None) != FrameStatus::Rendered {
            std::thread::sleep(std::time::Duration::from_millis(1));
            continue;
        }
        gpu.frame_pacer.split.wait_previous();
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let texture = gpu.headless_frame.as_ref().expect("a world shot target");
        let image = read_back(&gpu.device, &gpu.queue, texture);
        for (index, (_, rect)) in regions.iter().enumerate() {
            series[index].push(cells(&image, *rect));
        }
        done += 1;
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    (series, started.elapsed().as_secs_f64())
}
