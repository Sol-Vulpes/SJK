//! World shots of EJK mode's image (`ejk_mode.rs`) beside SJK's own: `ffa3` from its
//! start camera under Vanilla Plus, then EJK mode's settings at a few gammas and with
//! dynamic glow, to compare with EternalJK's screenshots. Ignored like the other world
//! shots; the sheet lands in `target/world-shots`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer ejk_look -- --ignored
//! --test-threads=1 --nocapture`.

use super::*;
use crate::graphics_quality::{Level, preset_cvars};

const SIZE: [u32; 2] = [1280, 720];

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn ejk_look_sheet() {
    let map = std::env::var("SJK_SHOT_MAP").unwrap_or_else(|_| "maps/mp/ffa3.bsp".to_owned());
    on_big_stack(move || {
        let ejk = crate::ejk_mode::values();
        let mut variants: Vec<(&str, Vec<(&str, &str)>)> =
            vec![("vanilla plus", preset_cvars(Level::VanillaPlus))];
        for (label, gamma, glow) in [
            ("ejk gamma 1.0", "1.0", "0"),
            ("ejk gamma 1.31", "1.311622", "0"),
            ("ejk gamma 1.31 glow", "1.311622", "1"),
        ] {
            let mut cvars = ejk.clone();
            cvars.retain(|(cvar, _)| *cvar != "r_DynamicGlow");
            cvars.extend([("r_gamma", gamma), ("r_DynamicGlow", glow)]);
            variants.push((label, cvars));
        }
        let mut shots = Vec::new();
        for (label, cvars) in &variants {
            let Some((mut gpu, _profile)) = open(&map, SIZE, None, cvars) else {
                return;
            };
            eprintln!("ejk look: {label}");
            shots.push(frame(&mut gpu, 60));
        }
        eprintln!("{}", sheet(&shots, 2, 640, "ejk-look").display());
    });
}
