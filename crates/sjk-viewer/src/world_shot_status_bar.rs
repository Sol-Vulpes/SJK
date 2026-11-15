//! World shots of the status bar ([`crate::status_bar`]) over duel6: at the top under the
//! build label with every part, at the bottom, and at 4K with the largest menu text.
//! The bar's numbers, blade, effects and toys are made up ([`State::preview`]). Off-screen
//! renders (no game window), ignored like the other world shots: they need a GPU adapter
//! and the installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer status_bar_shots -- --ignored
//! --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.
//!
//! [`State::preview`]: crate::status_bar::State::preview

use super::*;
use crate::status_bar::{Effect, Inputs, TOY_COUNT, toys};

/// A player with a Legendary blade and a stock second, a few things on and four toys out.
fn made_up() -> Inputs {
    let legendary = crate::unlockables::blade_skins()
        .find(|skin| skin.tier == crate::unlockables::Rarity::Legendary)
        .map(|skin| skin.id);
    let mut inputs = Inputs {
        credits: Some(12_340),
        shards: Some(875),
        crystals: Some(3),
        coins: Some(2),
        saber: legendary,
        second: Some(None),
        ..Inputs::default()
    };
    for (words, colour) in [
        ("Flare trail", crate::menu::sjk::color::GOLD_BRIGHT),
        (
            "Disintegrate",
            crate::unlockables::Rarity::Mythical.colour(),
        ),
        ("Tactical medium", crate::menu::sjk::color::HOLO),
        ("Do not disturb", crate::menu::sjk::color::EMBER),
    ] {
        inputs.push_effect(Effect { words, colour });
    }
    let mut on = [false; TOY_COUNT];
    for toy in [toys::ILLUMINATE, toys::HAX, toys::BOX_HUNT, toys::PROPGUN] {
        on[toy] = true;
    }
    inputs.toys = on;
    inputs
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn status_bar_shots() {
    on_big_stack(|| {
        for (size, scale, place, name) in [
            ([1920, 1080], "1", "1", "status-bar-top"),
            ([1920, 1080], "1", "2", "status-bar-bottom"),
            ([3840, 2160], "1.4", "1", "status-bar-4k-large-text"),
        ] {
            let cvars = [
                (crate::status_bar::CVAR, place),
                (crate::text::style::SCALE_CVAR, scale),
            ];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &cvars) else {
                return;
            };
            gpu.status_bar.preview = Some(made_up());
            let _ = frame(&mut gpu, 20);
            assert!(gpu.status_bar.drawn(), "{name}: the bar is drawn");
            println!("{}", shoot(&mut gpu, 2, name).display());
        }
    });
}
