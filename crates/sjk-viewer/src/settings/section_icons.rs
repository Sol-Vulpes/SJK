//! The picture each Settings sub-heading wears.
//!
//! Every heading of the SJK UI's Settings (a category's sections, First setup's and
//! Simple's groups, a search's groups) has a settings icon
//! ([`crate::settings_icons`]) drawn before its name, so a long category reads as a
//! list of pictured sections and the eye finds "Nameplates" or "Sabers" without
//! reading every heading. One entry per heading; a test checks that no heading a
//! category shows is missing here.

/// Each heading (any case) and its icon's name.
const ICONS: &[(&str, &str)] = &[
    // Display.
    ("Screen", "video"),
    ("Frame rate", "frame_rate"),
    ("View", "view"),
    // Graphics.
    ("Quality", "quality"),
    ("Preset", "quality"),
    ("Image", "image"),
    ("Textures", "textures"),
    ("Effects", "effects"),
    ("Materials", "textures"),
    ("Solar Renderer lighting", "sunlight"),
    ("Light shafts", "sunlight"),
    ("Time of day", "time"),
    ("Shadows", "shadows"),
    ("Weather", "weather"),
    ("Rain looks", "weather"),
    // Sound.
    ("Volume", "sound"),
    ("Game sounds", "game_sounds"),
    ("3D sound", "headphones"),
    ("Emotes", "emotes"),
    ("Toys", "toys"),
    ("Notifications", "notifications"),
    ("Chat sounds", "text"),
    // Gameplay.
    ("Players and models", "players"),
    ("Sabers", "gameplay"),
    ("Force", "force_powers"),
    ("Duels", "duels"),
    ("Maps and remaps", "maps"),
    ("Camera", "camera"),
    ("Prediction", "prediction"),
    ("Away", "away"),
    // Interface.
    ("Menus", "interface"),
    ("Theme", "theme"),
    ("Console", "console"),
    // HUD.
    ("Layout", "hud"),
    ("HUD", "hud"),
    ("Crosshair", "crosshair"),
    ("Names and cards", "cards"),
    ("Nameplates", "nameplates"),
    ("Playstyle", "playstyle"),
    ("Readouts", "readouts"),
    ("Match", "match"),
    ("Movement", "movement"),
    ("Status bar", "status_bar"),
    ("Scoreboard", "scoreboard"),
    ("Over the game", "hud"),
    // Chat.
    ("Chat box", "text"),
    ("Chat", "text"),
    ("Chat look", "chat_look"),
    ("Chat tabs", "chat_tabs"),
    ("Do not disturb", "dnd"),
    ("Duel lines", "duels"),
    ("SJK chat", "sjk_chat"),
    ("Translation", "sjk_chat"),
    ("Privacy and blocks", "identity"),
    // Mods.
    ("Packs", "packs"),
    ("Server mods", "server_mods"),
    ("JA+ plugin options", "server_mods"),
    ("jaPRO plugin options", "server_mods"),
    ("JoF content", "jof"),
    ("Sith cloud", "jof"),
    ("Map tools", "other_controls"),
    ("Experimental", "options"),
    // Network.
    ("SJK online", "online"),
    ("SJK", "online"),
    ("Connection", "network"),
    // First setup.
    ("From another client", "import"),
    ("Styles", "theme"),
    ("Aim", "mouse_joystick"),
    ("Force shells", "force_powers"),
    // A search's groups, where the category's own picture does not answer.
    (super::search::BEST, "credits"),
    (super::sections::MORE, "options"),
    ("Display", "video"),
    ("Graphics", "graphics"),
    ("Sound", "sound"),
    ("Controls", "mouse_joystick"),
    ("Gameplay", "gameplay"),
    ("Interface", "interface"),
    ("Quick wheel", crate::quick_wheel::catalog::WHEEL_ICON),
    ("Mods", "options"),
    ("Network", "network"),
    ("First setup", "first_setup"),
    ("Graphics: image", "image"),
    ("Graphics: lighting", "lighting"),
    ("Graphics: shadows", "shadows"),
    ("Graphics: weather", "weather"),
];

/// The icon of the sub-heading `heading` (any case).
pub(super) fn of(heading: &str) -> Option<&'static str> {
    ICONS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(heading))
        .map(|(_, icon)| *icon)
}

#[cfg(test)]
mod tests {
    use super::super::{Group, Line};
    use super::*;

    #[test]
    fn every_icon_named_exists() {
        for (heading, icon) in ICONS {
            assert!(
                crate::settings_icons::picture(icon).is_some(),
                "{heading}: no icon {icon}"
            );
        }
    }

    /// Every heading a category of the SJK UI's Settings shows, and every group a
    /// search lists results under.
    #[test]
    fn every_heading_shown_has_its_picture() {
        let groups = Group::sjk_only().chain([Group::Quick]);
        for group in groups {
            for line in group.lines() {
                if let Line::Heading(heading) = line {
                    assert!(of(heading).is_some(), "{group:?}: {heading} has no picture");
                }
            }
        }
        for query in ["a", "e", "hud", "chat", "sound", "o"] {
            for line in super::super::search::lines(query) {
                if let Line::Heading(heading) = line {
                    assert!(of(heading).is_some(), "search: {heading} has no picture");
                }
            }
        }
    }
}
