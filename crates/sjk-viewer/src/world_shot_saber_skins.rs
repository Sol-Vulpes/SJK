//! World shots of the Sun blade ([`crate::saber_skins`]) on duel6, without a session:
//! lit blades stand in front of the first spawn's view, the Sun between a stock orange
//! and a stock blue blade, at several animation times, then the Sun alone up close; and
//! the player screen's model holding the Sun blade (`cg_saberSkin saber_sun`, owned as
//! a hub profile listing it would make it). Ignored
//! like the other world shots.

use super::*;
use crate::saber::Blade;
use crate::saber_rgb::BladeColor;
use crate::saber_skins::BladeSkin;

/// Animation times the side-by-side views are drawn at, seconds.
const TIMES: [f64; 4] = [1.0, 2.6, 4.3, 7.9];

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sun_blade() {
    on_big_stack(|| {
        side_by_side();
        held_on_the_stage();
    });
}

fn side_by_side() {
    let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1280, 720], None, &[]) else {
        return;
    };
    let (spawn, yaw) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
    let turn = glam::Quat::from_rotation_z(yaw);
    let eye = Vec3::from_array(spawn);
    let ahead = |x: f32, y: f32, z: f32| eye + turn * Vec3::new(x, y, z);
    // Stock single-saber blades (40 units, radius 3), a little tilted as if held.
    let blade = |at: Vec3, lean: f32| Blade {
        base: at.to_array(),
        direction: (turn * Vec3::new(0.0, lean, 1.0)).normalize().to_array(),
        length: 40.0,
        radius: 3.0,
    };
    gpu.saber_skins.shot_blades = vec![
        (
            blade(ahead(48.0, 13.0, -22.0), 0.12),
            BladeColor::Retail(crate::saber::Color::Orange),
            1,
        ),
        (
            blade(ahead(48.0, 0.0, -22.0), 0.0),
            BladeColor::Skin(BladeSkin::Sun),
            2,
        ),
        (
            blade(ahead(48.0, -13.0, -22.0), -0.12),
            BladeColor::Retail(crate::saber::Color::Blue),
            3,
        ),
    ];
    // Looking down a little, so the blades stand over the shaded floor.
    let target = ahead(48.0, 0.0, -14.0);
    let (look_yaw, look_pitch) = look(eye.to_array(), target.to_array());
    aim(&mut gpu, eye.to_array(), look_yaw, look_pitch);
    let mut images = Vec::new();
    for (index, seconds) in TIMES.into_iter().enumerate() {
        gpu.saber_skins.shot_seconds = Some(seconds);
        images.push(frame(&mut gpu, if index == 0 { 40 } else { 4 }));
    }
    println!("{}", sheet(&images, 2, 960, "duel6-sun-blade").display());
    // Up close: the Sun alone, lying across the view over the floor, every 0.3 s.
    gpu.saber_skins.shot_blades = vec![(
        Blade {
            base: ahead(44.0, 20.0, -26.0).to_array(),
            direction: (turn * Vec3::new(0.0, -1.0, 0.12)).normalize().to_array(),
            length: 40.0,
            radius: 3.0,
        },
        BladeColor::Skin(BladeSkin::Sun),
        2,
    )];
    let camera = ahead(18.0, 0.0, -8.0);
    let (look_yaw, look_pitch) = look(camera.to_array(), ahead(44.0, 0.0, -24.0).to_array());
    aim(&mut gpu, camera.to_array(), look_yaw, look_pitch);
    let mut close = Vec::new();
    for step in 0..8 {
        let seconds = 3.0 + f64::from(step) * 0.3;
        gpu.saber_skins.shot_seconds = Some(seconds);
        close.push(frame(&mut gpu, 4));
    }
    println!(
        "{}",
        sheet(&close, 2, 960, "duel6-sun-blade-close").display()
    );
}

/// The player screen's model holding the Sun blade, as the Character page shows it.
fn held_on_the_stage() {
    let menu = menu::ClientMenu::new(true, String::new());
    let cvars = [
        ("ui_menuStyle", "sjk"),
        (crate::settings::quick::HIDE_CVAR, "1"),
        (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
    ];
    let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
    else {
        return;
    };
    // The shots have no hub: they own the unlock, as a profile listing it would.
    gpu.looks.shot_owns_unlocks = true;
    let _ = frame(&mut gpu, 4);
    if let (Some(menu), Some(console)) = (gpu.client_menu.as_mut(), gpu.console.as_ref()) {
        menu.open_player(console, crate::player_menu::ReturnTarget::MainMenu);
    }
    let _ = frame(&mut gpu, 2);
    gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
    println!("{}", shoot(&mut gpu, 8, "duel6-sun-blade-held").display());
}
