//! Offline credit rain and its Toys card. Synthetic state; no hub traffic.
use super::*;
#[test]
#[ignore = "renders with the GPU and installed data named by JKA_GAME_DATA"]
fn duel6_beso_toy() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &[
                ("cl_identity", "0"),
                ("cl_discordPresence", "0"),
                (crate::settings::quick::HIDE_CVAR, "1"),
                ("ui_menuStyle", "sjk"),
            ],
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 4);
        gpu.console.as_mut().unwrap().preview_collection_alone(
            crate::profile_hub::Tab::Toys,
            Some(vec![]),
            4.0,
        );
        gpu.console.as_mut().unwrap().collection_toy_for_shot(10);
        println!("{}", shoot(&mut gpu, 4, "beso-toys-card").display());
        gpu.console.as_mut().unwrap().set_open(false);
        gpu.client_menu = None;
        let _ = frame(&mut gpu, 2);
        gpu.preview_beso_rain();
        println!("{}", shoot(&mut gpu, 4, "beso-credit-rain").display());
        assert!(
            gpu.world_pings.list.commands().len() >= 80,
            "all ten credit marks must be drawn"
        );
    });
}
