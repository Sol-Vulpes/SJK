//! World shots of the blossoms a swung blade sheds ([`crate::saber_petals`]), without a
//! session: petals are queued as a swing would queue them, in front of the camera on
//! duel6, and the frame spawns and draws them; one shot as they leave the blade, then about
//! 0.6 and 1.3 seconds on, as they sink and settle. Ignored like the other world shots. The
//! Blossom mist extra's (`blossom_mist`) from a blade really swung across the view through
//! the emitter, so the shots show the mist it breathes out as a player would see it.

use super::*;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn petals_drift_down_ahead_of_the_spawn() {
    shoot(None, false, "petals_duel6");
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn blossom_mist_drifts_down_with_the_petals() {
    shoot(
        Some(crate::saber_petals::PetalKind::BlossomMist),
        false,
        "blossom_mist_duel6",
    );
}

/// The same swing seen edge on (swept toward the camera), where a sheet of mist would look
/// flat (Kushina, 20/11/2026: it must be puffy from every side).
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn blossom_mist_is_thick_seen_edge_on() {
    shoot(
        Some(crate::saber_petals::PetalKind::BlossomMist),
        true,
        "blossom_mist_edge_duel6",
    );
}

/// The Varykino Blossom's petals (as `kind`, else its own) shed across the view (or, `edge`,
/// a swing toward the camera), shot as they leave and about 0.6 and 1.3 seconds on.
fn shoot(kind: Option<crate::saber_petals::PetalKind>, edge: bool, name: &'static str) {
    on_big_stack(move || {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1280, 720], None, &[]) else {
            return;
        };
        let (spawn, yaw) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let turn = glam::Quat::from_rotation_z(yaw);
        let eye = Vec3::from_array(spawn);
        let ahead = eye + turn * Vec3::new(70.0, 0.0, 0.0);
        let (look_yaw, look_pitch) = look(eye.to_array(), ahead.to_array());
        aim(&mut gpu, eye.to_array(), look_yaw, look_pitch);
        let _ = frame(&mut gpu, 10);
        let mut spec = crate::blade_signatures::of("saber_varykino_blossom")
            .and_then(|signature| signature.petals())
            .expect("the blossom's petals");
        if let Some(kind) = kind {
            spec.kind = kind;
        }
        let queue = gpu.saber_states.petals();
        if spec.kind == crate::saber_petals::PetalKind::BlossomMist {
            // A 36-unit blade, leaning back a little, swept 140 units across the view in 12
            // frames of 16 ms (about 730 units a second at its tip), its wearer standing.
            let direction = (turn * Vec3::new(-0.15, 0.0, 1.0)).normalize();
            let mut emitter = crate::saber_petals::PetalEmitter::default();
            for step in 0..=12 {
                let across = (step as f32 / 12.0 - 0.5) * 140.0;
                let way = if edge {
                    Vec3::new(across * 0.5 + 40.0, 0.0, -24.0)
                } else {
                    Vec3::new(0.0, across, -24.0)
                };
                let base = ahead + turn * way;
                let tip = base + direction * 36.0;
                emitter.shed(
                    base,
                    direction,
                    36.0,
                    tip,
                    5_000 + step * 16,
                    7,
                    spec,
                    queue,
                );
            }
        } else {
            // A swing's worth along a blade sweeping across the view.
            for i in 0..40u32 {
                let along = i as f32 / 40.0;
                let across = turn * Vec3::new(0.0, (along - 0.5) * 60.0, (along * 7.0).sin() * 8.0);
                queue.push(crate::saber_petals::PetalSpawn {
                    origin: ahead + across,
                    velocity: turn * Vec3::new(0.0, 10.0, 0.0) - Vec3::Z * spec.fall * 0.5,
                    seed: 0x5eed ^ i.wrapping_mul(2_654_435_761),
                    spec,
                });
            }
        }
        let queued = queue.len();
        let before = gpu.particles.len();
        let start = std::time::Instant::now();
        let first = frame(&mut gpu, 1);
        let spawned = gpu.particles.len() - before;
        println!("{queued} queued, {spawned} spawned");
        assert!(
            spawned >= queued.min(10),
            "every queued petal is a particle"
        );
        // The particles age by the clock; a shot each about 0.6 and 1.3 seconds on.
        let at = |gpu: &mut GpuState, seconds: f32| {
            while start.elapsed().as_secs_f32() < seconds {
                let _ = frame(gpu, 1);
            }
            println!("shot at {:.2} s", start.elapsed().as_secs_f32());
            frame(gpu, 1)
        };
        let mid = at(&mut gpu, 0.6);
        let later = at(&mut gpu, 1.3);
        println!("{}", sheet(&[first, mid, later], 3, 1280, name).display());
    });
}
