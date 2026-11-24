//! World shots of Low FPS Night (`r_lowFpsNight`, `docs/rendering.md`, Low FPS Night):
//! `ffa3` from its start camera on Vanilla Plus (baked lighting), the map's own light
//! beside Low FPS Night at its defaults and darker, each with two saber lights near the
//! camera (codemp `CG_AddSaberBlade`'s light, as `world_shot_dynamic_light_cost.rs`
//! places them). Ignored like the other world shots; the sheet and each picture land in
//! `target/world-shots`.
//!
//! `low_fps_night_saber_glow` shows the saber glow (`r_lowFpsNightSaber`) at its least,
//! middle and most.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer low_fps_night_ --
//! --ignored --test-threads=1 --nocapture`.

use super::*;
use crate::dynamic_lights::PointLight;
use crate::graphics_quality::{Level, preset_cvars};
use glam::Vec3;

const MAP: &str = "maps/mp/ffa3.bsp";
const SIZE: [u32; 2] = [1280, 720];

/// Two blades' lights a little ahead of the camera, blue left and red right.
fn sabers(gpu: &mut GpuState) {
    let eye = gpu.camera_position;
    let forward = Vec3::new(gpu.camera_yaw.cos(), gpu.camera_yaw.sin(), 0.0);
    let side = Vec3::new(-forward.y, forward.x, 0.0);
    let ground = eye - Vec3::Z * 30.0;
    let lights = &mut gpu.effect_aux.held_lights;
    lights.clear();
    for (across, color) in [(1.0, [0.2, 0.4, 1.0]), (-1.0, [1.0, 0.2, 0.2])] {
        lights.push(PointLight {
            origin: (ground + forward * 160.0 + side * across * 60.0).to_array(),
            radius: 200.0,
            color,
        });
    }
}

/// The map's own light, Low FPS Night at its defaults and at a darker level, with and
/// without the saber lights.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn low_fps_night_sheet() {
    on_big_stack(|| {
        let night = crate::world_materials::shadows::day::live::LOW_FPS_NIGHT_CVAR;
        let mut shots = Vec::new();
        for (name, on, level) in [
            ("baked", "0", "0.2"),
            ("night", "1", "0.2"),
            ("night-darker", "1", "0.1"),
        ] {
            for lit in [false, true] {
                let mut cvars = preset_cvars(Level::VanillaPlus);
                cvars.extend([(night, on), ("r_lowFpsNightLevel", level)]);
                let Some((mut gpu, _profile)) = open(MAP, SIZE, None, &cvars) else {
                    return;
                };
                if lit {
                    sabers(&mut gpu);
                }
                let shot = frame(&mut gpu, 30);
                let file = format!(
                    "low-fps-night-{name}{}.png",
                    if lit { "-sabers" } else { "" }
                );
                std::fs::create_dir_all(directory()).expect("the shot directory");
                shot.save(directory().join(&file)).expect("write the shot");
                eprintln!("{file}");
                shots.push(shot);
            }
        }
        eprintln!("{}", sheet(&shots, 2, 640, "low-fps-night").display());
    });
}

/// One lilac blade's light low over the ground ahead of the camera, as a saber held at
/// the hip lights the floor around its holder.
fn lilac_saber(gpu: &mut GpuState) {
    let eye = gpu.camera_position;
    let forward = Vec3::new(gpu.camera_yaw.cos(), gpu.camera_yaw.sin(), 0.0);
    let lights = &mut gpu.effect_aux.held_lights;
    lights.clear();
    lights.push(PointLight {
        origin: (eye - Vec3::Z * 20.0 + forward * 140.0).to_array(),
        radius: 200.0,
        color: [0.8, 0.5, 1.0],
    });
}

/// The saber glow (`r_lowFpsNightSaber`) at its least, its middle and its most, at the
/// night's default level.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn low_fps_night_saber_glow() {
    on_big_stack(|| {
        let night = crate::world_materials::shadows::day::live::LOW_FPS_NIGHT_CVAR;
        let mut shots = Vec::new();
        for (name, glow) in [("least", "0.01"), ("middle", "0.5"), ("most", "1")] {
            let mut cvars = preset_cvars(Level::VanillaPlus);
            cvars.extend([(night, "1"), ("r_lowFpsNightSaber", glow)]);
            let Some((mut gpu, _profile)) = open(MAP, SIZE, None, &cvars) else {
                return;
            };
            lilac_saber(&mut gpu);
            let shot = frame(&mut gpu, 30);
            let file = format!("low-fps-night-saber-glow-{name}.png");
            std::fs::create_dir_all(directory()).expect("the shot directory");
            shot.save(directory().join(&file)).expect("write the shot");
            eprintln!("{file}");
            shots.push(shot);
        }
        eprintln!(
            "{}",
            sheet(&shots, 3, 640, "low-fps-night-saber-glow").display()
        );
    });
}
