//! World shots of SJK's emblem on the main menus ([`crate::menu::emblem`]): the SJK UI's
//! main page over duel6 with the menu clock moved on a twelfth of a second at a time, six
//! seconds of the sun's motion (its disc turning, its flames licking, its sparks flying
//! out). The classic main page, where the emblem stays inside its window, is
//! `menu_snapshot`'s `main-classic`. Off-screen renders (no game window), ignored like the
//! other world shots: they need a GPU adapter and the installed game data named by
//! `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer emblem_shots --
//! --ignored --test-threads=1 --nocapture`; the pictures are in `target/world-shots`
//! (`emblem-motion-000.png` on).

use super::*;

/// Frames of the clip, a twelfth of a second apart.
const FRAMES: u32 = 72;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_emblem_motion() {
    on_big_stack(|| {
        let menu = menu::ClientMenu::new(true, String::new());
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
        ];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
        else {
            return;
        };
        // Past the first shot's fade-in.
        let _ = frame(&mut gpu, 20);
        gpu.ui_epoch -= std::time::Duration::from_millis(2_000);
        let _ = frame(&mut gpu, 4);
        for index in 0..FRAMES {
            gpu.ui_epoch -= std::time::Duration::from_millis(1_000 / 12);
            shoot(&mut gpu, 1, &format!("emblem-motion-{index:03}"));
        }
    });
}
