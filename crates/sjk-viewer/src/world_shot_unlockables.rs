//! World shots of the Unlockables page ([`crate::console::unlockables_panel`]) over the
//! live duel6 in the SJK UI: the Sun blade owned and worn (a made-up grant with the
//! team's note, `cg_saberSkin saber_sun`) with its swatch at a few times of its flare's
//! run, then locked, then the page with the identity off (the shots' own), and the
//! owned page in a 4:3 window. Ignored like the other world shots.

use super::*;

/// The made-up grant.
fn sun() -> sjk_identity::Unlock {
    sjk_identity::Unlock {
        id: "saber_sun".to_owned(),
        granted: 1_791_336_225,
        note: "Thanks for testing the blades with us".to_owned(),
    }
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_unlockables() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
        ];
        for size in [[1920, 1080], [1440, 1080]] {
            let Some((mut gpu, _profile)) = open(
                "maps/mp/duel6.bsp",
                size,
                Some(menu::ClientMenu::new(true, String::new())),
                &cvars,
            ) else {
                return;
            };
            let _ = frame(&mut gpu, 10);
            if size[0] != 1920 {
                if let Some(console) = gpu.console.as_mut() {
                    console.preview_unlockables(Some(vec![sun()]), 0.8);
                }
                println!("{}", shoot(&mut gpu, 6, "duel6-unlockables-4x3").display());
                continue;
            }
            let mut swatches = Vec::new();
            for seconds in [0.8, 1.3, 2.2, 3.1] {
                if let Some(console) = gpu.console.as_mut() {
                    console.preview_unlockables(Some(vec![sun()]), seconds);
                }
                if seconds == 0.8 {
                    println!("{}", shoot(&mut gpu, 6, "duel6-unlockables").display());
                }
                swatches.push(
                    image::imageops::crop_imm(&frame(&mut gpu, 2), 96, 290, 880, 300).to_image(),
                );
            }
            println!(
                "{}",
                sheet(&swatches, 2, 880, "duel6-unlockables-swatch").display()
            );
            if let Some(console) = gpu.console.as_mut() {
                console.preview_unlockables(Some(Vec::new()), 0.8);
            }
            println!(
                "{}",
                shoot(&mut gpu, 6, "duel6-unlockables-locked").display()
            );
            if let Some(console) = gpu.console.as_mut() {
                console.preview_unlockables(None, 0.8);
            }
            println!("{}", shoot(&mut gpu, 6, "duel6-unlockables-off").display());
        }
    });
}
