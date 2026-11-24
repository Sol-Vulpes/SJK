//! World shots of the chat's two styles (`cg_chatStyle`, `chat/style.rs`) from one
//! camera over ffa3's sand: SJK's chat, then the classic one (JoF EJK's), each with the
//! same lines (All, a long wrapped one, Team, SJK chat from SJK and from JoF EternalJK, a
//! whisper) and its field open on a draft, then the classic field on Team and on a
//! whisper, at 1080p and 4K, the game's own fonts on (`ui_gameFont`) as EJK mode sets
//! them. Every name is invented. Off-screen renders (no game window), ignored like the
//! other world shots: they need a GPU adapter and the installed game data named by
//! `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer --bin sjk
//! classic_chat_shots -- --ignored --test-threads=1 --nocapture`; the pictures are in
//! `target/world-shots`.

use super::*;

/// The camera: in ffa3's courtyard, looking down over the sand towards the walls.
const FROM: [f32; 3] = [1_050.0, 650.0, 230.0];
const YAW: f32 = 225.0;
const PITCH: f32 = -15.0;

fn message(id: u64, name: &str, text: &str, client: &str) -> sjk_identity::ChatMessage {
    sjk_identity::ChatMessage {
        reactions: Default::default(),
        id,
        at: 0,
        key_id: String::new(),
        name: name.to_owned(),
        verified: id.is_multiple_of(2),
        tick: sjk_identity::Tick::None,
        staff: false,
        creator: false,
        trust: None,
        emoji: String::new(),
        client: client.to_owned(),
        text: text.to_owned(),
        holocron: None,
        official: None,
        channel: Default::default(),
    }
}

fn state(messages: Vec<sjk_identity::ChatMessage>) -> sjk_identity::ChatState {
    sjk_identity::ChatState {
        messages: messages.into_iter().collect(),
        loaded: Some(1),
        ..sjk_identity::ChatState::default()
    }
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn classic_chat_shots() {
    for (size, suffix) in [([1920, 1080], "1080p"), ([3840, 2160], "4k")] {
        on_big_stack(move || {
            let cvars = [
                ("cg_chatbox", "600000"),
                ("cl_identity", "0"),
                ("cl_discordPresence", "0"),
                (crate::game_font::CVAR, "1"),
                // JoF EJK's size, in both styles.
                ("cg_chatBoxFontSize", "1"),
            ];
            let Some((mut gpu, _profile)) = open("maps/mp/ffa3.bsp", size, None, &cvars) else {
                return;
            };
            let _ = frame(&mut gpu, 1);
            aim(&mut gpu, FROM, YAW, PITCH);
            if let Some(console) = gpu.console.as_mut() {
                console.set_open(false);
            }
            let mut game = sjk_protocol::GameState::empty_local(0);
            for (slot, name) in [(0, "Padawan"), (3, "^1Dark^7Rider"), (4, "^5Tusken")] {
                game.replace_config_string(1_131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                    .expect("a player");
            }
            let now = std::time::Instant::now();
            let chat = &mut gpu.chat;
            chat.update_roster(&game);
            chat.for_shot = true;
            chat.sync_sjk(&state(Vec::new()), |_| false, now);
            use sjk_client::ServerEventKind::{Chat, TeamChat};
            for (kind, sender, line) in [
                (Chat, 3, "^1Dark^7Rider^7: ^2anyone up for a duel?"),
                (
                    Chat,
                    4,
                    "^5Tusken^7: ^2the quick brown fox jumps over the lazy dog and keeps on running \
                     all the way round the courtyard and back to the tower again",
                ),
                (
                    TeamChat,
                    3,
                    "(^1Dark^7Rider^7): ^5flag carrier is at the bridge",
                ),
            ] {
                chat.receive(kind, line.to_owned(), Some(sender), now);
            }
            chat.sync_sjk(
                &state(vec![
                    message(1, "^3Womp^7Rat", "hello from another server", "ejk"),
                    message(
                        2,
                        "^2Bantha",
                        "the SJK chat reaches every server, and a long message of it wraps under \
                         its first row as any other does in the box",
                        "",
                    ),
                ]),
                |_| false,
                now,
            );
            chat.receive(
                Chat,
                "[^5Tusken^7]: ^6meet me at the tower".to_owned(),
                Some(4),
                now,
            );
            for style in crate::chat::style::Style::NAMES {
                if let Some(console) = gpu.console.as_mut() {
                    assert!(console.set_cvar(crate::chat::style::CVAR, style));
                }
                gpu.chat.close_composer();
                let name = format!("chat-style-{style}-box-{suffix}");
                println!("{}", shoot(&mut gpu, 12, &name).display());
                gpu.chat.draft_for_shot("gg, one more?");
                let name = format!("chat-style-{style}-field-{suffix}");
                println!("{}", shoot(&mut gpu, 12, &name).display());
            }
            // The classic field on Team, then on a whisper.
            gpu.chat.close_composer();
            gpu.chat.open(true);
            gpu.chat.type_for_shot("push the left side");
            let name = format!("chat-style-classic-field-team-{suffix}");
            println!("{}", shoot(&mut gpu, 12, &name).display());
            gpu.chat.close_composer();
            assert!(gpu.chat.whisper_to(Some(4)));
            gpu.chat.type_for_shot("on my way");
            let name = format!("chat-style-classic-field-whisper-{suffix}");
            println!("{}", shoot(&mut gpu, 12, &name).display());
        });
    }
}
