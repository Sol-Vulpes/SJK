//! Shots of `mp/ffa5` for the HD world proof of concept: the stock map under
//! retail baked light, SJK's default lighting and fully live light, from the
//! same cameras, so what live light alone does to the map is visible before
//! any mesh is replaced.

use super::*;

/// Cameras of the map: each spawn point (eye height) looking where its player
/// would, then four overviews from above the rim towards the middle.
fn views(bsp: &Bsp) -> Vec<(String, [f32; 3], [f32; 3])> {
    let mut views = Vec::new();
    for entity in sjk_entity::parse_entity_lump(bsp.entities()).expect("the entity lump") {
        if entity.classname() != Some("info_player_deathmatch") {
            continue;
        }
        let Some(origin) = entity.vector("origin").ok().flatten() else {
            continue;
        };
        let yaw = entity
            .number("angle")
            .ok()
            .flatten()
            .unwrap_or(0.0)
            .to_radians();
        let eye = [origin[0], origin[1], origin[2] + 40.0];
        let at = [
            eye[0] + yaw.cos() * 512.0,
            eye[1] + yaw.sin() * 512.0,
            eye[2] - 30.0,
        ];
        views.push((format!("spawn {}", views.len()), eye, at));
    }
    // A few spawns suffice; the overviews follow.
    let step = (views.len() / 8).max(1);
    let mut picked: Vec<_> = views.into_iter().step_by(step).take(8).collect();
    for (index, (x, y)) in [(1.0, -1.0), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .into_iter()
        .enumerate()
    {
        picked.push((
            format!("overview {index}"),
            [x * 2400.0, y * 2400.0, 2600.0],
            [0.0, 0.0, 0.0],
        ));
    }
    picked
}

/// One contact sheet of those cameras under `cvars`.
fn sheet_of(name: &str, cvars: &[(&str, &str)]) {
    let Some((mut gpu, _profile)) = open("maps/mp/ffa5.bsp", [960, 540], None, cvars) else {
        return;
    };
    let views = views(&gpu.bsp);
    let views: Vec<(&str, [f32; 3], [f32; 3])> = views
        .iter()
        .map(|(label, origin, at)| (label.as_str(), *origin, *at))
        .collect();
    sweep(&mut gpu, &views, name);
}

#[cfg(test)]
#[path = "hd_world_export.rs"]
mod export;

#[cfg(test)]
mod tests {
    use super::*;

    /// Write ffa5's drawn surfaces as `target/hd-world/ffa5/ffa5.glb` (with its
    /// textures), the starting point for the Blender rebuild.
    #[test]
    #[ignore = "reads the installed game data named by JKA_GAME_DATA"]
    fn ffa5_export() {
        on_big_stack(|| {
            let game_data = PathBuf::from(
                std::env::var_os("JKA_GAME_DATA")
                    .expect("JKA_GAME_DATA names the GameData directory"),
            );
            let (bsp, vfs) = assets::load_bsp(&game_data, "maps/mp/ffa5.bsp").expect("the map");
            let scene = StaticWorld::build(&bsp, MeshBuildOptions::default().with_sky_surfaces())
                .expect("the map's meshes");
            let shaders = assets::load_shaders(&vfs);
            let directory =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/hd-world/ffa5");
            let summary = export::write_map(&bsp, &scene, &vfs, &shaders, &directory, "ffa5")
                .expect("the export");
            println!(
                "{}: {} models, {} objects, {} triangles, {} textures; winding agrees/disagrees {:?}; no image: {:?}",
                summary.glb.display(),
                summary.models,
                summary.objects,
                summary.triangles,
                summary.textures,
                summary.winding,
                summary.missing_textures,
            );
        });
    }

    /// ffa5 as shipped (`r_dayNight 0`: retail baked light), under SJK's default
    /// lighting (live sun, baked indirect) and with everything live
    /// (`r_liveLighting 2`), the mode an HD world has to look right in.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn ffa5_baseline() {
        on_big_stack(|| {
            sheet_of("ffa5-retail", &[("r_dayNight", "0")]);
            sheet_of("ffa5-default", &[]);
            sheet_of("ffa5-live", &[("r_liveLighting", "2")]);
        });
    }
}
