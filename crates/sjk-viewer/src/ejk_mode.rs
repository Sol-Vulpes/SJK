//! EJK mode (Sol, 22/11/2026): one switch, `ui_ejkMode`, that makes SJK look and feel
//! like JoF EJK (EternalJK, OpenJK's lineage), as close as SJK's own settings get: the
//! classic menus, loading screen and in-game menu in retail's look, the game's own fonts
//! in the menus, chat, console and HUD, EternalJK's console, HUD and scoreboard, the
//! Vanilla graphics, JoF EJK's chat (`cg_chatStyle classic`, `chat/style.rs`), and none of
//! what SJK adds over a match (nameplates, status balloons, feats, kybers, finishers, toys,
//! weather looks).
//!
//! It is a preset, not a mode the drawing code knows of: [`TABLE`] names each setting and
//! the value JoF EJK's look takes, the Vanilla graphics preset
//! (`graphics_quality.rs`) gives the renderer's, and nothing else reads [`CVAR`]. So every
//! change the switch makes can be seen, and changed again, on its own row in Settings.
//!
//! Turning it on puts the player's value of each of those settings aside ([`RESTORE_CVAR`],
//! as `renderer vanilla` and Tactical do, `cvar_restore.rs`, which tells the three of one
//! another) and sets JoF EJK's; turning it off gives them back. The switch writes only as it flips ([`follow`]): a setting the
//! player changes by hand while the mode is on stays as they set it, after a restart too,
//! and turning the mode off leaves that one alone, since it is no longer the mode's value
//! but theirs. What was put aside tells whether the mode has been applied, so the console
//! (`ui_ejkMode`, or the `ejkmode` command, whose `list` prints the table with its reasons),
//! the Settings row, the what's-new card and a config all switch it.
//!
//! What it leaves alone: the SJK hub's features (the SJK chat, the identity, the
//! Collection, trading), gameplay and the network, the player's own numbers (field of view,
//! sensitivity, HUD scale, brightness) and anything without a setting of its own.

use crate::console::ViewerConsole;

/// Archived, off by default: SJK looks and feels like JoF EJK.
pub(crate) const CVAR: &str = "ui_ejkMode";
/// Archived: what turning the mode on put aside, `cvar=value` pairs joined by `;`
/// (`cvar_restore.rs`). Not empty while the mode has been applied.
pub(crate) const RESTORE_CVAR: &str = "ui_ejkModeRestore";
/// Console command that names the mode, sets it or lists what it sets.
pub(crate) const COMMAND: &str = "ejkmode";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Show or set EJK mode (SJK looks and feels like JoF EJK, your \
     settings kept for when it is off): on, off, or list for each setting it sets and why";

/// One setting the mode sets.
pub(crate) struct Entry {
    /// The setting's cvar; it has a row in Settings (a test).
    pub(crate) cvar: &'static str,
    /// The value JoF EJK's look takes.
    pub(crate) value: &'static str,
    /// Why, with JoF EJK's source where a default of its own is followed
    /// (`codemp/cgame/cg_xcvar.h`, `codemp/client/cl_console.cpp`).
    pub(crate) why: &'static str,
}

const fn entry(cvar: &'static str, value: &'static str, why: &'static str) -> Entry {
    Entry { cvar, value, why }
}

/// Every setting the mode sets besides the Vanilla graphics preset's ([`values`]).
pub(crate) const TABLE: &[Entry] = &[
    // Menus and type.
    entry(
        crate::menu::style::CVAR,
        "classic",
        "The retail menus' pages and flow; the loading screen and the in-game menu follow it",
    ),
    entry(
        crate::game_font::CVAR,
        "1",
        "The game's own fonts in the menus, chat, HUD text, scoreboard and console",
    ),
    entry(
        crate::text::system_font::CHAT_CVAR,
        "",
        "No system font over the chat: the game's own",
    ),
    entry(
        crate::text::system_font::CONSOLE_CVAR,
        "",
        "No system font over the console: the game's character set",
    ),
    // Console.
    entry(
        crate::console::console_options::STYLE_CVAR,
        "classic",
        "EternalJK's console (cl_console.cpp), whatever the menus are",
    ),
    entry(
        crate::console::console_options::DRAW_NOTIFY_CVAR,
        "1",
        "Console lines show at the top left while playing (con_notifytime 3); kills are read there",
    ),
    // HUD.
    entry(
        crate::menu_hud::STYLE_CVAR,
        "game",
        "The game-data HUD, not one of SJK's layouts; the HUD files and a HUD pack stay the player's",
    ),
    entry(
        "cg_classicHudFont",
        "1",
        "The retail numbers and letters on the status HUD",
    ),
    entry(
        crate::status_bar::CVAR,
        "0",
        "SJK's status bar is not JoF EJK's",
    ),
    entry(
        "cg_killfeed",
        "0",
        "No kill feed: JoF EJK prints kills as console lines",
    ),
    entry(crate::hud::feats::CVAR, "0", "No feat medals or announcer"),
    entry(
        crate::version_overlay::CVAR,
        "0",
        "No SJK version line over the game",
    ),
    entry("cg_drawTimer", "0", "JoF EJK's cg_drawTimer default"),
    entry(
        "cg_drawTimerUpperRight",
        "1",
        "A timer turned on stands in the upper right, where JoF EJK draws it",
    ),
    entry(
        "cg_teamScoresBoxes",
        "0",
        "The game's team scores, not SJK's boxes",
    ),
    entry(
        crate::hud::player_card::CVAR,
        "0",
        "No SJK player card on looking at a player",
    ),
    entry(
        crate::hud::team_outline::CVAR,
        "0",
        "No outlines round enemies and teammates",
    ),
    entry(crate::ground_hud::CVAR, "0", "No HUD round the feet"),
    // Scoreboard.
    entry(
        crate::scoreboard::style::CVAR,
        "classic",
        "The retail scoreboard, whatever the menus are",
    ),
    entry(
        crate::scoreboard::style::IDS_CVAR,
        "0",
        "JoF EJK's cg_showClientIDs default",
    ),
    entry(
        "cg_drawScoreboardIcons",
        "0",
        "JoF EJK's cg_drawScoreboardIcons default",
    ),
    entry(
        crate::scoreboard::AFK_CVAR,
        "0",
        "Away players stay in the server's order",
    ),
    // Chat.
    entry(
        crate::chat::style::CVAR,
        "classic",
        "JoF EJK's chat: the message field at the top left with its channels (cl_console.cpp),          the chat box as CG_ChatBox_DrawStrings lays it out (cg_draw.c), plain names, retail's          drop shadow, SJK chat lines as its cl_sjk.cpp shows them",
    ),
    entry(
        "cg_chatBoxFontSize",
        "1",
        "JoF EJK's cg_chatBoxFontSize default; SJK's 1 is its absolute size",
    ),
    entry(crate::chat::emoji::CVAR, "0", "No emoji pictures in chat"),
    entry(
        crate::chat::options::BACKDROP_CVAR,
        "0",
        "No band behind chat lines",
    ),
    entry(
        crate::chat::options::OUTLINE_CVAR,
        "0",
        "Chat text keeps the game's drop shadow",
    ),
    entry(
        crate::status_balloons::CVAR,
        "0",
        "The stock talk balloon over a typing player",
    ),
    // Over the players.
    entry(
        "cg_nameplate",
        "0",
        "No nameplates: names as cg_drawPlayerNames and the crosshair draw them",
    ),
    entry(
        crate::playstyle::SHOW_CVAR,
        "0",
        "No playstyle badges over players",
    ),
    entry(
        crate::world_ping::SHOW_CVAR,
        "0",
        "No world pings from other SJK players",
    ),
    entry(
        crate::dynamic_lights::TOON_MODELS_CVAR,
        "0",
        "Every model in the game's own shading",
    ),
    // Sabers.
    entry(
        crate::stock_blades::CVAR,
        "2",
        "Every blade stock, in its own colour: no kybers, trails or blade extras, yours included",
    ),
    entry(
        crate::audio::SKIN_SOUNDS_CVAR,
        "0",
        "The stock saber sounds, not the kybers'",
    ),
    entry(
        crate::finishers::SHOW_CVAR,
        "0",
        "A saber kill leaves the body as the game does",
    ),
    // The world.
    entry(
        crate::footprints::PRINTS_CVAR,
        "0",
        "No footprints or dust under steps",
    ),
    entry(
        crate::footprints::JUMP_CVAR,
        "0",
        "Jumps and landings sound as the game's",
    ),
    entry(
        crate::toy_sight::ILLUMINATE,
        "0",
        "No SJK toys in the world",
    ),
    entry(crate::toy_sight::RAGDOLL, "0", "No SJK toys in the world"),
    entry(crate::toy_sight::DRONE, "0", "No SJK toys in the world"),
    entry(crate::toy_sight::BOOMBOX, "0", "No SJK toys in the world"),
    entry(crate::monitor::SHOW_CVAR, "0", "No SJK toys in the world"),
    entry(crate::tags::SHOW_CVAR, "0", "No tags on the walls"),
    entry(crate::tags::GIF_SHOW_CVAR, "0", "No GIF tags on the walls"),
    // The image and the weather, past the Vanilla preset.
    entry(
        crate::frame_target::aa::exposure::ENABLED,
        "0",
        "A fixed exposure: eye adaptation brightens dark rooms even without HDR",
    ),
    entry("r_toneCurve", "0", "No filmic curve over the image"),
    entry(
        crate::frame_target::aa::color::SHARPEN_CVAR,
        "0",
        "No sharpening",
    ),
    entry(
        crate::weather::FORCE_CVAR,
        "0",
        "The map's own weather, none forced on it",
    ),
    entry(crate::weather::CLOUDS_CVAR, "0", "The map's own sky"),
    entry(
        crate::weather::LIGHTNING_CVAR,
        "0",
        "No lightning in storms",
    ),
    entry(crate::weather::SNOW_COVER_CVAR, "0", "Snow does not settle"),
    entry(
        crate::weather::sound::CVAR,
        "0",
        "No rain and wind sounds of SJK's",
    ),
];

/// Every setting the mode sets and its value: the Vanilla graphics preset's, then
/// [`TABLE`]'s (which names none of the preset's, a test).
pub(crate) fn values() -> Vec<(&'static str, &'static str)> {
    let mut values = crate::graphics_quality::preset_cvars(crate::graphics_quality::Level::Vanilla);
    values.extend(TABLE.iter().map(|entry| (entry.cvar, entry.value)));
    values
}

/// Whether [`CVAR`] asks for the mode.
pub(crate) fn asked(console: &ViewerConsole) -> bool {
    console.cvar(CVAR).is_some_and(crate::settings::switch_on)
}

/// The mode as the settings put aside know it (`cvar_restore.rs`): it leaves alone a
/// setting the player changed by hand while it was on.
pub(crate) const MODE: crate::cvar_restore::Mode = crate::cvar_restore::Mode {
    name: "EJK mode",
    record: RESTORE_CVAR,
    values: held,
    keeps_hand_changes: true,
};

fn held(_: &ViewerConsole) -> Vec<(&'static str, &'static str)> {
    values()
}

/// Whether the mode has been applied: something is put aside.
pub(crate) fn applied(console: &ViewerConsole) -> bool {
    crate::cvar_restore::applied(console, &MODE)
}

/// Follow [`CVAR`]: apply the mode when it is asked for and not applied yet, give the
/// player's settings back when it is applied and no longer asked for, and write nothing
/// otherwise. Called every frame; it reads two cvars when there is nothing to do.
pub(crate) fn follow(console: &mut ViewerConsole) {
    match (asked(console), applied(console)) {
        (true, false) => turn_on(console),
        (false, true) => turn_off(console),
        _ => {}
    }
}

/// Put the player's value of each setting aside and set JoF EJK's. Settings this client
/// does not have (a build without a toy) are skipped. A setting another mode already
/// holds (Tactical, `renderer vanilla`) keeps the player's value that mode put aside.
fn turn_on(console: &mut ViewerConsole) {
    crate::cvar_restore::put_aside(console, &MODE, &values());
}

/// Give back what [`turn_on`] put aside. A setting that no longer holds the mode's value
/// was changed by hand since: it is the player's choice and stays. One the table no
/// longer names (put aside by an earlier version) is given back as it was, and one
/// another mode still holds stays on that mode's value.
fn turn_off(console: &mut ViewerConsole) {
    crate::cvar_restore::give_back(console, &MODE);
}

/// `ejkmode [on|off|list]`: name the mode, set it, or list what it sets and why.
pub(crate) fn command(console: &mut ViewerConsole, args: &[String]) -> Result<Vec<String>, String> {
    let usage = || format!("usage: {COMMAND} [on|off|list]");
    let shown = |console: &ViewerConsole| {
        if asked(console) {
            "EJK mode: on (JoF EJK's look; your settings are kept for when it is off)".to_owned()
        } else {
            "EJK mode: off (your own settings)".to_owned()
        }
    };
    match args {
        [] => Ok(vec![shown(console), usage()]),
        [word] if word.eq_ignore_ascii_case("list") => {
            let mut lines = vec![format!(
                "EJK mode sets the Vanilla graphics preset ({} settings) and:",
                crate::graphics_quality::preset_cvars(crate::graphics_quality::Level::Vanilla)
                    .len()
            )];
            lines.extend(
                TABLE
                    .iter()
                    .map(|entry| format!("  {} \"{}\": {}", entry.cvar, entry.value, entry.why)),
            );
            Ok(lines)
        }
        [word] => {
            let on = ["on", "1"]
                .iter()
                .any(|name| word.eq_ignore_ascii_case(name));
            let off = ["off", "0"]
                .iter()
                .any(|name| word.eq_ignore_ascii_case(name));
            if on == off {
                return Err(usage());
            }
            let changed = on != asked(console);
            console.set_cvar(CVAR, if on { "1" } else { "0" });
            follow(console);
            let mut lines = vec![shown(console)];
            if changed {
                lines.push(
                    "Some graphics settings apply after a graphics reload (vid_restart), the \
                     menus' map at the next start"
                        .to_owned(),
                );
            }
            Ok(lines)
        }
        _ => Err(usage()),
    }
}

impl crate::GpuState {
    /// Follow `ui_ejkMode` ([`follow`]).
    pub(crate) fn update_ejk_mode(&mut self) {
        if let Some(console) = self.console.as_mut() {
            follow(console);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    fn text(console: &ViewerConsole, cvar: &str) -> String {
        console
            .cvar(cvar)
            .map(sjk_shell::CvarValue::as_text)
            .unwrap_or_default()
    }

    fn snapshot(console: &ViewerConsole) -> Vec<(&'static str, String)> {
        values()
            .into_iter()
            .map(|(cvar, _)| (cvar, text(console, cvar)))
            .collect()
    }

    #[test]
    fn every_setting_is_registered_has_a_row_and_takes_its_value() {
        let (_directory, mut console) = console();
        for (cvar, value) in values() {
            assert!(console.cvar(cvar).is_some(), "{cvar} is registered");
            assert!(crate::settings::has_row(cvar), "{cvar} has a Settings row");
            // The HUD row opens a picker of the styles: it does not list its values.
            let hud = cvar == crate::menu_hud::STYLE_CVAR
                && crate::menu_hud::HudStyle::NAMES.contains(&value);
            assert!(
                hud || crate::settings::row_takes(cvar, value),
                "{cvar}'s Settings row offers {value:?}"
            );
            assert!(
                console.try_set_cvar(cvar, value).is_ok(),
                "{cvar} takes {value:?}"
            );
            assert!(
                crate::graphics_quality::holds(&console, cvar, value),
                "{cvar} holds {value:?} once set"
            );
        }
    }

    #[test]
    fn each_setting_is_named_once_and_says_why() {
        let values = values();
        for (index, (cvar, _)) in values.iter().enumerate() {
            assert!(
                !values[..index]
                    .iter()
                    .any(|(seen, _)| seen.eq_ignore_ascii_case(cvar)),
                "{cvar} is named twice (the Vanilla preset's are not the table's)"
            );
        }
        for entry in TABLE {
            assert!(!entry.why.is_empty(), "{} says why", entry.cvar);
            // The text put aside is `cvar=value` pairs joined by `;`.
            assert!(!entry.value.contains([';', '=']), "{}", entry.cvar);
        }
        assert!(!values.iter().any(|(cvar, _)| *cvar == CVAR));
    }

    #[test]
    fn the_switch_is_off_by_default_and_an_on_off_settings_row() {
        let (_directory, console) = console();
        assert!(!asked(&console));
        assert!(!applied(&console));
        assert!(crate::settings::switch_label(CVAR).is_some());
    }

    #[test]
    fn on_sets_every_value_and_off_gives_the_players_settings_back_exactly() {
        let (_directory, mut console) = console();
        // A profile with choices of its own, some of them the mode's values already.
        console.set_cvar(crate::stock_blades::CVAR, "1");
        console.set_cvar(crate::menu_hud::STYLE_CVAR, "radial");
        console.set_cvar(crate::text::system_font::CHAT_CVAR, "Segoe UI");
        console.set_cvar("cg_chatBoxFontSize", "1.25");
        console.set_cvar("cg_nameplate", "1");
        console.set_cvar("cg_drawTimer", "0");
        console.set_cvar("r_sunShadowResolution", "4096");
        let before = snapshot(&console);

        console.set_cvar(CVAR, "1");
        follow(&mut console);
        assert!(applied(&console));
        for (cvar, value) in values() {
            assert!(
                crate::graphics_quality::holds(&console, cvar, value),
                "{cvar} holds {value:?}"
            );
        }
        assert_eq!(
            crate::graphics_quality::Level::current(&console),
            Some(crate::graphics_quality::Level::Vanilla)
        );

        // The chat is JoF EJK's: the style, and what it reads at JoF EJK's defaults.
        assert_eq!(
            crate::chat::style::Style::read(Some(&console)),
            crate::chat::style::Style::Classic
        );
        assert!(crate::graphics_quality::holds(
            &console,
            "cg_chatBoxFontSize",
            "1"
        ));

        console.set_cvar(CVAR, "0");
        follow(&mut console);
        assert!(!applied(&console));
        assert_eq!(text(&console, RESTORE_CVAR), "");
        assert_eq!(snapshot(&console), before);
        assert_eq!(
            crate::chat::style::Style::read(Some(&console)),
            crate::chat::style::Style::Sjk
        );
    }

    /// A player who picks SJK's chat again while the mode is on keeps it, and the mode
    /// going off leaves it alone; one who chose the classic chat before the mode came
    /// on has it still when the mode goes off.
    #[test]
    fn the_chat_style_is_the_players_to_change_and_to_keep() {
        use crate::chat::style::{CVAR as STYLE, Style};
        let (_directory, mut console) = console();
        console.set_cvar(CVAR, "1");
        follow(&mut console);
        assert_eq!(Style::read(Some(&console)), Style::Classic);
        console.set_cvar(STYLE, "sjk");
        follow(&mut console);
        assert_eq!(Style::read(Some(&console)), Style::Sjk);
        console.set_cvar(CVAR, "0");
        follow(&mut console);
        assert_eq!(Style::read(Some(&console)), Style::Sjk);

        console.set_cvar(STYLE, "classic");
        console.set_cvar(CVAR, "1");
        follow(&mut console);
        console.set_cvar(CVAR, "0");
        follow(&mut console);
        assert_eq!(Style::read(Some(&console)), Style::Classic);
    }

    #[test]
    fn a_setting_changed_by_hand_while_on_is_left_alone() {
        let (_directory, mut console) = console();
        console.set_cvar(crate::menu_hud::STYLE_CVAR, "classic");
        console.set_cvar(CVAR, "1");
        follow(&mut console);
        assert_eq!(text(&console, crate::menu_hud::STYLE_CVAR), "game");

        // The player picks another HUD and wants the kill feed back.
        console.set_cvar(crate::menu_hud::STYLE_CVAR, "radial");
        console.set_cvar("cg_killfeed", "1");
        for _ in 0..3 {
            follow(&mut console);
        }
        assert_eq!(text(&console, crate::menu_hud::STYLE_CVAR), "radial");
        assert!(crate::graphics_quality::holds(&console, "cg_killfeed", "1"));
        // The rest is still the mode's.
        assert_eq!(text(&console, crate::menu::style::CVAR), "classic");

        // Off: what the mode still held goes back, what the player set since stays.
        console.set_cvar(CVAR, "0");
        follow(&mut console);
        assert_eq!(text(&console, crate::menu_hud::STYLE_CVAR), "radial");
        assert!(crate::graphics_quality::holds(&console, "cg_killfeed", "1"));
        assert_eq!(text(&console, crate::menu::style::CVAR), "sjk");
    }

    #[test]
    fn a_restart_with_the_mode_on_writes_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.cfg");
        {
            let mut console = ViewerConsole::new(path.clone()).unwrap();
            console.set_cvar(crate::stock_blades::CVAR, "1");
            console.set_cvar(CVAR, "1");
            follow(&mut console);
            // Changed by hand while on, then the client is closed.
            console.set_cvar(crate::scoreboard::style::CVAR, "sjk");
        }
        let mut console = ViewerConsole::new(path).unwrap();
        assert!(asked(&console) && applied(&console));
        let saved = snapshot(&console);
        let kept = text(&console, RESTORE_CVAR);
        follow(&mut console);
        assert_eq!(snapshot(&console), saved);
        assert_eq!(text(&console, RESTORE_CVAR), kept);
        assert_eq!(text(&console, crate::scoreboard::style::CVAR), "sjk");
        assert_eq!(text(&console, crate::stock_blades::CVAR), "2");

        console.set_cvar(CVAR, "0");
        follow(&mut console);
        assert_eq!(text(&console, crate::stock_blades::CVAR), "1");
        assert_eq!(text(&console, crate::scoreboard::style::CVAR), "sjk");
    }

    #[test]
    fn the_command_names_sets_and_lists_the_mode() {
        let (_directory, mut console) = console();
        let run = |console: &mut ViewerConsole, words: &[&str]| {
            let args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
            command(console, &args)
        };
        assert!(run(&mut console, &[]).unwrap()[0].contains("off"));
        let lines = run(&mut console, &["on"]).unwrap();
        assert!(lines[0].contains("on") && lines.len() == 2);
        assert!(asked(&console) && applied(&console));
        assert_eq!(text(&console, crate::menu::style::CVAR), "classic");
        assert_eq!(text(&console, crate::chat::style::CVAR), "classic");
        // Already on: nothing new to reload.
        assert_eq!(run(&mut console, &["1"]).unwrap().len(), 1);
        let list = run(&mut console, &["list"]).unwrap();
        assert_eq!(list.len(), TABLE.len() + 1);
        assert!(
            list.iter()
                .any(|line| line.contains("con_style \"classic\""))
        );
        assert!(
            list.iter()
                .any(|line| line.contains("cg_chatStyle \"classic\""))
        );
        run(&mut console, &["OFF"]).unwrap();
        assert!(!asked(&console) && !applied(&console));
        assert_eq!(text(&console, crate::menu::style::CVAR), "sjk");
        assert_eq!(text(&console, crate::chat::style::CVAR), "sjk");
        assert!(run(&mut console, &["maybe"]).is_err());
        assert!(run(&mut console, &["on", "off"]).is_err());
    }

    #[test]
    fn a_setting_the_table_no_longer_names_is_given_back() {
        let (_directory, mut console) = console();
        console.set_cvar(CVAR, "1");
        follow(&mut console);
        // An earlier version's table also held the field of view.
        let kept = format!("{};cg_fov=100;no_such_cvar=1", text(&console, RESTORE_CVAR));
        console.set_cvar(RESTORE_CVAR, &kept);
        console.set_cvar(CVAR, "0");
        follow(&mut console);
        assert!(crate::graphics_quality::holds(&console, "cg_fov", "100"));
        assert!(!applied(&console));
    }
}
