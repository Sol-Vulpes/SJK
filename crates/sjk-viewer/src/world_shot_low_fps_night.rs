//! World shots of Low FPS Night (`r_lowFpsNight`, `docs/rendering.md`, Low FPS Night):
//! `ffa3` from its start camera on Vanilla Plus (baked lighting), the map's own light
//! beside Low FPS Night at its defaults and darker, each with two saber lights near the
//! camera (codemp `CG_AddSaberBlade`'s light, as `world_shot_dynamic_light_cost.rs`
//! places them). Ignored like the other world shots; the sheet and each picture land in
//! `target/world-shots`.
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
