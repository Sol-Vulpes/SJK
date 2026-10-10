//! World shots of GIFs in SJK chat ([`crate::chat_gifs`]) over duel6: the SJK chat page
//! with a GIF shown, one loading and one that could not be had; the main page's dock and
//! the in-game menu's dock with a GIF. The GIFs are small test animations made here and
//! put in the cache as if fetched: nothing is downloaded. Off-screen renders (no game
//! window), ignored like the other world shots: they need a GPU adapter and the installed
//! game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer sjk_chat_gifs -- --ignored
//! --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.

use super::*;
use crate::chat_gifs::{GifId, decode};

/// The test GIFs: a wide one (GIPHY's 200 pixels high) and a square one.
fn gifs() -> (GifId, GifId) {
    let wide = GifId::new("ShotGifWide01").unwrap();
    let square = GifId::new("ShotGifSquare02").unwrap();
    crate::chat_gifs::insert_for_shot(&wide, &decode::test_animation([356, 200], 8, 80), 250);
    crate::chat_gifs::insert_for_shot(&square, &decode::test_animation([200, 200], 6, 90), 250);
    crate::chat_gifs::fail_for_shot(&GifId::new("ShotGifGone03").unwrap());
    (wide, square)
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_chat_gifs_page() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64);
    let lines: [(&str, &str, &str, bool); 5] = [
        (
            "9a0c51e2b7d34f80",
            "^5Creyon",
            "anyone up for duels on ffa3?",
            true,
        ),
        (
            "44f3d0b36c9b2510",
            "^1Sol^7Vulpes",
            "GIFs in chat now https://giphy.com/gifs/hello-wave-ShotGifWide01",
            true,
        ),
        (
            "0b1c2d3e4f5a6b7c",
            "Padawan^1Fox",
            "https://media.giphy.com/media/ShotGifGone03/giphy.gif",
            false,
        ),
        (
            "1f2e3d4c5b6a7980",
            "^3Lumaya",
            "https://i.giphy.com/ShotGifSquare02.gif gg",
            true,
        ),
        (
            "7c6b5a4f3e2d1c0b",
            "Kyle",
            "this one is still coming https://giphy.com/gifs/ShotGifLoading04",
            false,
        ),
    ];
    let messages = lines
        .iter()
        .enumerate()
        .map(
            |(index, (key, name, text, verified))| sjk_identity::ChatMessage {
                id: index as u64 + 1,
                at: now - (lines.len() - index) as i64 * 90,
                key_id: (*key).to_owned(),
                name: (*name).to_owned(),
                verified: *verified,
                staff: false,
                text: (*text).to_owned(),
                holocron: None,
            },
        )
        .collect();
    let state = sjk_identity::ChatState {
        messages,
        revision: 1,
        online: 5,
        people: None,
        live: true,
        outcome: None,
        loaded: Some(1),
    };
    for (size, scale, suffix) in [
        ([1920, 1080], None, "1080p"),
        ([3840, 2160], Some("1.2"), "4k-styled"),
    ] {
        let state = state.clone();
        on_big_stack(move || {
            let _ = gifs();
            let menu = menu::ClientMenu::new(true, String::new());
            let mut cvars = vec![
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
            ];
            if let Some(scale) = scale {
                cvars.push((crate::text::style::SCALE_CVAR, scale));
            }
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, Some(menu), &cvars)
            else {
                return;
            };
            let _ = frame(&mut gpu, 10);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_sjk_chat(state, None);
            }
            let name = format!("duel6-sjk-chat-gifs-{suffix}");
            println!("{}", shoot(&mut gpu, 8, &name).display());
        });
    }
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_chat_gifs_dock() {
    use crate::ingame_menu::sjk_view::Card;
    use crate::ingame_menu::{Page, ShotView};
    const LINES: [(&str, &str, bool); 4] = [
        ("^1Fox", "in 5 min, finishing a CTF", false),
        ("^3Lumaya", "gg", true),
        ("Kyle", "anyone up for duels?", false),
        (
            "^2Sol",
            "hello https://giphy.com/gifs/hello-wave-ShotGifWide01",
            true,
        ),
    ];
    for (size, suffix) in [([1920, 1080], "1080p"), ([3840, 2160], "4k")] {
        on_big_stack(move || {
            let _ = gifs();
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
                ("name", "^1Sol^7Vulpes"),
            ];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, Some(menu), &cvars)
            else {
                return;
            };
            gpu.ui_epoch -= std::time::Duration::from_millis(2_000);
            let _ = frame(&mut gpu, 20);
            if let Some(menu) = gpu.client_menu.as_mut() {
                menu.sjk_home_chat_for_shot(&LINES, 12, None);
            }
            let name = format!("duel6-chat-dock-gif-{suffix}");
            println!("{}", shoot(&mut gpu, 8, &name).display());
            let menu = gpu.client_menu.as_ref().expect("the menu");
            assert!(!menu.sjk_home_overflowed(), "{name}");
        });
        on_big_stack(move || {
            let _ = gifs();
            let menu = menu::ClientMenu::new(false, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
                ("name", "^1Sol^7Vulpes"),
            ];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, Some(menu), &cvars)
            else {
                return;
            };
            let shots = menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
            let shot = &shots[0];
            let (yaw, pitch) = look(shot.from, shot.at);
            aim(&mut gpu, shot.from, yaw, pitch);
            gpu.game_menu = true;
            if let Some(console) = gpu.console.as_mut() {
                console.close_for_connection();
            }
            let _ = frame(&mut gpu, 60);
            let ffa = ShotView {
                team: 0,
                team_game: false,
                red_players: 0,
                blue_players: 0,
                vote_active: false,
                staff: false,
                siege: false,
            };
            gpu.game_menu_page = Page::Main;
            gpu.game_menu_row = 0;
            gpu.in_game_menu
                .sjk_for_shot(Card::for_shot(false, false), ffa);
            gpu.in_game_menu.chat_for_shot(&LINES, 12, None);
            let name = format!("duel6-ingame-chat-gif-{suffix}");
            println!("{}", shoot(&mut gpu, 16, &name).display());
            assert!(!gpu.in_game_menu.overflowed(), "{name}");
        });
    }
}
