//! A world shot of the mystery boxes ([`crate::mystery_box`]) on duel6: four boxes,
//! each just stopped on one tier's holocron (Uncommon to Mythical), every part of its form
//! drawn; and four on the prizes of 29/10/2026 (two piles of shards, a free box and the
//! boxes' blade). Ignored like the other world shots.

use super::*;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_mystery_box_holocrons() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1600, 900], None, &[]) else {
            return;
        };
        let shots = crate::menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
        let (from, at) = (
            Vec3::from_array(shots[0].from),
            Vec3::from_array(shots[0].at),
        );
        // Four boxes in a row across the view, 150 units ahead.
        let ahead = (at - from).with_z(0.0).normalize_or(Vec3::X);
        let across = Vec3::new(-ahead.y, ahead.x, 0.0);
        let floor = from + ahead * 150.0 - Vec3::Z * 60.0;
        let yaw = ahead.y.atan2(ahead.x).to_degrees() + 180.0;
        let boxes: Vec<([f32; 3], f32)> = (0..4)
            .map(|index| {
                (
                    (floor + across * (index as f32 - 1.5) * 60.0).to_array(),
                    yaw,
                )
            })
            .collect();
        gpu.shot_mystery_boxes(
            &boxes,
            crate::mystery_box::SHOT_ROLL + std::time::Duration::from_millis(900),
        );
        let (yaw, pitch) = look(from.to_array(), (floor + Vec3::Z * 40.0).to_array());
        aim(&mut gpu, from.to_array(), yaw, pitch);
        gpu.is_menu_world = false;
        println!(
            "{}",
            shoot(&mut gpu, 90, "duel6-mystery-box-holocrons").display()
        );
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_mystery_box_loot() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1600, 900], None, &[]) else {
            return;
        };
        let shots = crate::menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
        let (from, at) = (
            Vec3::from_array(shots[0].from),
            Vec3::from_array(shots[0].at),
        );
        // Four boxes in a row across the view, 150 units ahead.
        let ahead = (at - from).with_z(0.0).normalize_or(Vec3::X);
        let across = Vec3::new(-ahead.y, ahead.x, 0.0);
        let floor = from + ahead * 150.0 - Vec3::Z * 60.0;
        let yaw = ahead.y.atan2(ahead.x).to_degrees() + 180.0;
        let boxes: Vec<([f32; 3], f32)> = (0..4)
            .map(|index| {
                (
                    (floor + across * (index as f32 - 1.5) * 60.0).to_array(),
                    yaw,
                )
            })
            .collect();
        gpu.shot_mystery_box_loot(
            &boxes,
            crate::mystery_box::SHOT_ROLL + std::time::Duration::from_millis(900),
        );
        let (yaw, pitch) = look(from.to_array(), (floor + Vec3::Z * 40.0).to_array());
        aim(&mut gpu, from.to_array(), yaw, pitch);
        gpu.is_menu_world = false;
        println!(
            "{}",
            shoot(&mut gpu, 90, "duel6-mystery-box-loot").display()
        );
    });
}

/// Boxes stopped on the loot of 11/11/2026, close up: the three piles of gold coins and
/// the Ray Gun, then the four kyber crystals.
fn rework(name: &'static str, kybers: bool) {
    on_big_stack(move || {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1600, 900], None, &[]) else {
            return;
        };
        let shots = crate::menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
        let (from, at) = (
            Vec3::from_array(shots[0].from),
            Vec3::from_array(shots[0].at),
        );
        // Four boxes in a row across the view, 110 units ahead.
        let ahead = (at - from).with_z(0.0).normalize_or(Vec3::X);
        let across = Vec3::new(-ahead.y, ahead.x, 0.0);
        let floor = from + ahead * 110.0 - Vec3::Z * 60.0;
        let yaw = ahead.y.atan2(ahead.x).to_degrees() + 180.0;
        let boxes: Vec<([f32; 3], f32)> = (0..4)
            .map(|index| {
                (
                    (floor + across * (index as f32 - 1.5) * 64.0).to_array(),
                    yaw,
                )
            })
            .collect();
        gpu.shot_mystery_box_rework(
            &boxes,
            crate::mystery_box::SHOT_ROLL + std::time::Duration::from_millis(100),
            kybers,
        );
        let (yaw, pitch) = look(from.to_array(), (floor + Vec3::Z * 44.0).to_array());
        aim(&mut gpu, from.to_array(), yaw, pitch);
        gpu.is_menu_world = false;
        println!("{}", shoot(&mut gpu, 90, name).display());
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_mystery_box_rework() {
    rework("duel6-mystery-box-rework", false);
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_mystery_box_kybers() {
    rework("duel6-mystery-box-kybers", true);
}

/// A closed box beside two stopped on the teddy bear of a box that blows up (30/10/2026),
/// close up: the tomb's steps, horns and glyphs, the lid's parted slabs, the bear.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_mystery_box_teddy() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1600, 900], None, &[]) else {
            return;
        };
        let shots = crate::menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
        let (from, at) = (
            Vec3::from_array(shots[0].from),
            Vec3::from_array(shots[0].at),
        );
        let ahead = (at - from).with_z(0.0).normalize_or(Vec3::X);
        let across = Vec3::new(-ahead.y, ahead.x, 0.0);
        let floor = from + ahead * 120.0 - Vec3::Z * 60.0;
        // Turned a little, so an end and the front both show.
        let yaw = ahead.y.atan2(ahead.x).to_degrees() + 180.0 - 25.0;
        let boxes: Vec<([f32; 3], f32)> = (0..3)
            .map(|index| {
                (
                    (floor + across * (index as f32 - 1.0) * 75.0).to_array(),
                    yaw,
                )
            })
            .collect();
        gpu.shot_mystery_box_teddy(
            &boxes,
            crate::mystery_box::SHOT_ROLL + std::time::Duration::from_millis(400),
        );
        let (yaw, pitch) = look(from.to_array(), (floor + Vec3::Z * 25.0).to_array());
        aim(&mut gpu, from.to_array(), yaw, pitch);
        gpu.is_menu_world = false;
        println!(
            "{}",
            shoot(&mut gpu, 90, "duel6-mystery-box-teddy").display()
        );
    });
}

/// The Sith tomb of 23/11/2026 in its three states, shut, its lid parting and open on a
/// holocron: from the front, then from above and behind an end.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_mystery_box_tomb() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1600, 900], None, &[]) else {
            return;
        };
        let shots = crate::menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
        let (from, at) = (
            Vec3::from_array(shots[0].from),
            Vec3::from_array(shots[0].at),
        );
        let ahead = (at - from).with_z(0.0).normalize_or(Vec3::X);
        let across = Vec3::new(-ahead.y, ahead.x, 0.0);
        let floor = from + ahead * 150.0 - Vec3::Z * 60.0;
        let yaw = ahead.y.atan2(ahead.x).to_degrees() + 180.0 - 20.0;
        let boxes: Vec<([f32; 3], f32)> = (0..3)
            .map(|index| {
                (
                    (floor + across * (index as f32 - 1.0) * 100.0).to_array(),
                    yaw,
                )
            })
            .collect();
        gpu.shot_mystery_box_tomb(
            &boxes,
            crate::mystery_box::SHOT_ROLL + std::time::Duration::from_millis(900),
        );
        gpu.is_menu_world = false;
        let views = [
            ("duel6-mystery-box-tomb", from, floor + Vec3::Z * 30.0),
            (
                "duel6-mystery-box-tomb-shut",
                floor - across * 100.0 - ahead * 75.0 + across * 35.0 + Vec3::Z * 62.0,
                floor - across * 100.0 + Vec3::Z * 22.0,
            ),
            (
                "duel6-mystery-box-tomb-open",
                floor + across * 100.0 - ahead * 80.0 - across * 30.0 + Vec3::Z * 70.0,
                floor + across * 100.0 + Vec3::Z * 30.0,
            ),
            (
                "duel6-mystery-box-tomb-parting",
                floor - ahead * 70.0 + Vec3::Z * 80.0,
                floor + Vec3::Z * 26.0,
            ),
        ];
        for (name, from, at) in views {
            let (yaw, pitch) = look(from.to_array(), at.to_array());
            aim(&mut gpu, from.to_array(), yaw, pitch);
            println!("{}", shoot(&mut gpu, 90, name).display());
        }
    });
}
