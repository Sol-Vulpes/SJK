//! Offline casino page review with synthetic balances and hands, no hub traffic.
use super::*;
#[test]
#[ignore = "renders with the GPU and installed data named by JKA_GAME_DATA"]
fn duel6_casino_pages() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &[
                ("cl_identity", "0"),
                (crate::settings::quick::HIDE_CVAR, "1"),
                ("cl_discordPresence", "0"),
                ("ui_menuStyle", "sjk"),
            ],
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 4);
        for (game, name) in ["poker", "roulette", "slots", "blackjack"]
            .iter()
            .enumerate()
        {
            gpu.console.as_mut().unwrap().preview_casino(game);
            println!(
                "{}",
                shoot(&mut gpu, 4, &format!("casino-{name}")).display()
            );
        }
    });
}
