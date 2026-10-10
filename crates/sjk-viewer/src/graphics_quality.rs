//! Graphics quality levels: one choice that sets the renderer's costly
//! settings together, from EJK (the most frames, the original game's look as
//! EternalJK players know it) to Ultra (the best look). High is the fresh profile's own values, so a new player reads High.
//! The level is not stored: it is whichever level the settings match, and
//! Custom once one of them is changed on its own. Only settings with a row in
//! Settings are touched, so every change a level makes can be seen and undone
//! there; the FPS cap, vertical sync, resolution, supersampling and anything
//! that changes gameplay or a map's content are left alone.

use crate::console::ViewerConsole;

/// Console command that names or sets the level.
pub(crate) const COMMAND: &str = "graphicsquality";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Show or set graphics quality: ejk, performance, balanced, high or ultra";
/// The Settings row's stand-in for a cvar name: the command that sets it.
pub(crate) const ROW_NAME: &str = COMMAND;
/// The EJK graphics switch's stand-in for a cvar name: it is on while the
/// settings are EJK's, not a value of its own. It was the Ultra low switch:
/// the old name stays, as the stand-in and in `graphicsquality ultralow`.
pub(crate) const EJK_ROW: &str = "ultralow";
/// What the EJK graphics switch turned off, `cvar=value` pairs joined by `;`,
/// so turning it off again restores the player's own settings.
pub(crate) const RESTORE_CVAR: &str = "r_ultraLowRestore";

/// A graphics quality level, cheapest first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Level {
    /// The original game's look at the least cost, named after EternalJK, the
    /// client closest to it that players know (Ultra low before): Performance with the
    /// material packs' normal and specular maps and ambient occlusion off too,
    /// so maps show their baked lightmaps and nothing of SJK's lighting.
    Ejk,
    /// The most frames short of that: the retail look with SJK's lighting,
    /// shadows, post-processing and weather extras off.
    Performance,
    /// Most of SJK's look at a lower cost: no floor mirrors or reflection
    /// probes, fewer light-shaft samples, smaller, softer-filtered shadows.
    Balanced,
    /// SJK's defaults.
    High,
    /// Everything at its best: larger, finer shadows and the richest weather.
    Ultra,
}

/// Each setting a level sets, with its value at EJK, Performance,
/// Balanced, High and Ultra. High is each setting's default (pinned by a test).
const VALUES: &[(&str, [&str; 5])] = &[
    // Image.
    ("r_sceneHdr", ["0", "0", "1", "1", "1"]),
    ("r_sceneBloom", ["0", "0", "1", "1", "1"]),
    ("r_DynamicGlow", ["0", "0", "1", "1", "1"]),
    ("r_fxaa", ["0", "0", "1", "1", "1"]),
    ("r_softParticles", ["0", "0", "1", "1", "1"]),
    ("r_ssao", ["0", "1", "1", "1", "1"]),
    (crate::dust_motes::CVAR, ["0", "0", "0.5", "1", "1"]),
    ("r_modelPixelLight", ["0", "0", "1", "1", "1"]),
    ("r_cubeMapping", ["0", "0", "0", "1", "1"]),
    ("r_floorReflections", ["0", "0", "0", "1", "1"]),
    ("r_normalMapping", ["0", "1", "1", "1", "1"]),
    ("r_specularMapping", ["0", "1", "1", "1", "1"]),
    ("r_parallaxMapping", ["0", "0", "1", "1", "1"]),
    ("r_emissiveMaps", ["0", "0", "1", "1", "1"]),
    ("r_emissiveGlow", ["0", "0", "1", "1", "1"]),
    // Lighting.
    ("r_dayNight", ["0", "0", "1", "1", "1"]),
    ("r_emissiveLights", ["0", "0", "1", "1", "1"]),
    ("r_volumetrics", ["0", "0", "1", "3", "3"]),
    // Shadows.
    ("r_worldSunShadows", ["0", "0", "1", "1", "1"]),
    ("r_actorSunShadows", ["0", "0", "1", "1", "1"]),
    (
        "r_sunShadowResolution",
        ["1024", "1024", "1024", "2048", "4096"],
    ),
    ("r_sunShadowTaps", ["8", "8", "8", "16", "24"]),
    // Weather.
    (crate::weather::DENSITY_CVAR, ["1", "1", "1.5", "2", "2"]),
    (crate::weather::QUALITY_CVAR, ["0", "0", "1", "2", "3"]),
    (crate::weather::CLOUDS_CVAR, ["0", "0", "1", "1", "1"]),
];

impl Level {
    pub(crate) const ALL: [Self; 5] = [
        Self::Ejk,
        Self::Performance,
        Self::Balanced,
        Self::High,
        Self::Ultra,
    ];

    /// The level's name in Settings.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ejk => "EJK",
            Self::Performance => "Performance",
            Self::Balanced => "Balanced",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// The level `name` names, in any case, with or without its space; Ultra
    /// low is EJK's old name.
    fn parse(name: &str) -> Option<Self> {
        let name: String = name.split_whitespace().collect();
        if name.eq_ignore_ascii_case("ultralow") {
            return Some(Self::Ejk);
        }
        Self::ALL
            .into_iter()
            .find(|level| level.command_name().eq_ignore_ascii_case(&name))
    }

    /// The level's name in the command: its label without the space.
    fn command_name(self) -> String {
        self.label().replace(' ', "").to_ascii_lowercase()
    }

    /// The level a step of `direction` from this one reaches, stopping at the
    /// ends rather than wrapping from Ultra to Performance.
    pub(crate) fn step(self, direction: i32) -> Self {
        let last = Self::ALL.len() as i32 - 1;
        Self::ALL[(self.index() as i32 + direction).clamp(0, last) as usize]
    }

    /// The level whose values `console` holds, if every one matches.
    pub(crate) fn current(console: &ViewerConsole) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.matches(console) == VALUES.len())
    }

    /// The current level, or for custom settings the level they are nearest,
    /// sharing the most values (the higher on a tie). Steps and lists start here.
    pub(crate) fn nearest(console: &ViewerConsole) -> Self {
        Self::ALL
            .into_iter()
            .max_by_key(|level| level.matches(console))
            .unwrap_or(Self::High)
    }

    /// How many of the level's values `console` holds.
    fn matches(self, console: &ViewerConsole) -> usize {
        VALUES
            .iter()
            .filter(|(cvar, values)| holds(console, cvar, values[self.index()]))
            .count()
    }

    /// Set the level's values, saving the profile once.
    pub(crate) fn apply(self, console: &mut ViewerConsole) {
        console.set_cvars(
            VALUES
                .iter()
                .map(|(cvar, values)| (*cvar, values[self.index()])),
        );
    }
}

/// Whether the EJK graphics switch shows on: the settings are EJK's.
pub(crate) fn ejk(console: &ViewerConsole) -> bool {
    Level::current(console) == Some(Level::Ejk)
}

/// Flip the EJK graphics switch. On, it keeps the player's value of every
/// setting a level sets, then applies EJK; off, it puts the kept values
/// back (High when none were kept, as after picking EJK as a level).
pub(crate) fn toggle_ejk(console: &mut ViewerConsole) {
    if ejk(console) {
        let kept = console
            .cvar(RESTORE_CVAR)
            .map(sjk_shell::CvarValue::as_text)
            .unwrap_or_default();
        let restored: Vec<(&str, &str)> = kept
            .split(';')
            .filter_map(|pair| pair.split_once('='))
            .filter(|(cvar, _)| VALUES.iter().any(|(known, _)| known == cvar))
            .chain([(RESTORE_CVAR, "")])
            .collect();
        if restored.len() == 1 {
            Level::High.apply(console);
        }
        console.set_cvars(restored);
    } else {
        let kept = VALUES
            .iter()
            .filter_map(|(cvar, _)| Some(format!("{cvar}={}", console.cvar(cvar)?.as_text())))
            .collect::<Vec<_>>()
            .join(";");
        console.set_cvar(RESTORE_CVAR, &kept);
        Level::Ejk.apply(console);
    }
}

/// What the Settings row shows: the level, or Custom.
pub(crate) fn shown(console: &ViewerConsole) -> &'static str {
    Level::current(console).map_or("Custom", Level::label)
}

/// Whether `cvar` holds `value`: numbers compare as numbers ("1.0" holds "1").
fn holds(console: &ViewerConsole, cvar: &str, value: &str) -> bool {
    let Some(current) = console.cvar(cvar).map(sjk_shell::CvarValue::as_text) else {
        return false;
    };
    match (current.trim().parse::<f64>(), value.parse::<f64>()) {
        (Ok(current), Ok(value)) => (current - value).abs() < 1e-6,
        _ => current.trim().eq_ignore_ascii_case(value),
    }
}

/// `graphicsquality [level]`: name the level, or set it.
pub(crate) fn command(console: &mut ViewerConsole, args: &[String]) -> Result<Vec<String>, String> {
    let names = || Level::ALL.map(Level::command_name).join(", ");
    match args {
        [] => Ok(vec![format!(
            "Graphics quality: {} ({})",
            shown(console),
            names()
        )]),
        [name] => {
            let level =
                Level::parse(name).ok_or_else(|| format!("usage: {COMMAND} [{}]", names()))?;
            level.apply(console);
            Ok(vec![format!(
                "Graphics quality: {}; shadows, light shafts, HDR, FXAA, sun and sky, \
                 reflection probes and material maps apply after a graphics reload \
                 (vid_restart), emission-map lights on the next map",
                level.label()
            )])
        }
        _ => Err(format!("usage: {COMMAND} [{}]", names())),
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

    #[test]
    fn a_fresh_profile_is_high() {
        let (_directory, console) = console();
        for (cvar, values) in VALUES {
            let default = console
                .cvar_default(cvar)
                .unwrap_or_else(|| panic!("{cvar} is not registered"))
                .as_text();
            assert_eq!(
                default.parse::<f64>().ok(),
                values[Level::High.index()].parse::<f64>().ok(),
                "{cvar}: High is not its default"
            );
        }
        assert_eq!(Level::current(&console), Some(Level::High));
        assert_eq!(shown(&console), "High");
    }

    #[test]
    fn each_level_sets_its_values_and_reads_back() {
        let (directory, mut console) = console();
        for level in Level::ALL.into_iter().rev() {
            level.apply(&mut console);
            assert_eq!(Level::current(&console), Some(level));
            assert_eq!(shown(&console), level.label());
        }
        // The profile was saved: a new start reads the last level.
        drop(console);
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Ejk));
    }

    #[test]
    fn no_level_costs_less_than_the_one_below_it() {
        for (cvar, values) in VALUES {
            let numbers: Vec<f64> = values.iter().map(|v| v.parse().unwrap()).collect();
            assert!(
                numbers.windows(2).all(|pair| pair[0] <= pair[1]),
                "{cvar}: {values:?}"
            );
            assert!(numbers[0] < numbers[4], "{cvar} is the same at every level");
        }
    }

    #[test]
    fn every_value_is_one_its_settings_row_can_show() {
        let (_directory, mut console) = console();
        for level in Level::ALL {
            for (cvar, values) in VALUES {
                let value = values[level.index()];
                assert!(
                    crate::settings::row_takes(cvar, value),
                    "{cvar} {value}: no row in Settings shows it"
                );
                assert!(console.set_cvar(cvar, value), "{cvar} {value}");
                assert!(holds(&console, cvar, value), "{cvar} refused {value}");
            }
        }
    }

    #[test]
    fn a_setting_changed_on_its_own_is_custom_near_its_level() {
        let (_directory, mut console) = console();
        Level::Ultra.apply(&mut console);
        console.set_cvar("r_sunShadowTaps", "12");
        assert_eq!(Level::current(&console), None);
        assert_eq!(shown(&console), "Custom");
        assert_eq!(Level::nearest(&console), Level::Ultra);
        Level::Performance.apply(&mut console);
        console.set_cvar("r_sceneBloom", "1");
        assert_eq!(Level::nearest(&console), Level::Performance);
    }

    #[test]
    fn steps_stop_at_the_ends() {
        assert_eq!(Level::Ejk.step(-1), Level::Ejk);
        assert_eq!(Level::Performance.step(-1), Level::Ejk);
        assert_eq!(Level::Performance.step(1), Level::Balanced);
        assert_eq!(Level::High.step(1), Level::Ultra);
        assert_eq!(Level::Ultra.step(1), Level::Ultra);
    }

    #[test]
    fn the_command_names_and_sets_the_level() {
        let (_directory, mut console) = console();
        let lines = command(&mut console, &[]).unwrap();
        assert!(lines[0].starts_with("Graphics quality: High"), "{lines:?}");
        command(&mut console, &["ULTRA".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Ultra));
        assert!(command(&mut console, &["max".to_owned()]).is_err());
        assert_eq!(Level::current(&console), Some(Level::Ultra));
        command(&mut console, &["ultralow".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Ejk));
        command(&mut console, &["Ultra Low".to_owned()]).unwrap();
        assert_eq!(shown(&console), "EJK");
        Level::High.apply(&mut console);
        command(&mut console, &["ejk".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Ejk));
    }

    #[test]
    fn ejk_turns_sjk_lighting_and_material_maps_off() {
        let (_directory, mut console) = console();
        Level::Ejk.apply(&mut console);
        for cvar in [
            "r_dayNight",
            "r_worldSunShadows",
            "r_volumetrics",
            "r_cubeMapping",
            "r_floorReflections",
            "r_normalMapping",
            "r_specularMapping",
            "r_parallaxMapping",
            "r_emissiveMaps",
            "r_ssao",
            "r_sceneHdr",
        ] {
            assert!(holds(&console, cvar, "0"), "{cvar}");
        }
    }

    #[test]
    fn the_ejk_switch_restores_the_players_settings() {
        let (directory, mut console) = console();
        Level::Balanced.apply(&mut console);
        console.set_cvar("r_sunShadowTaps", "12");
        assert!(!ejk(&console));
        toggle_ejk(&mut console);
        assert!(ejk(&console));
        // Kept across a restart.
        drop(console);
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(ejk(&console));
        toggle_ejk(&mut console);
        assert!(!ejk(&console));
        assert!(holds(&console, "r_sunShadowTaps", "12"));
        console.set_cvar("r_sunShadowTaps", "8");
        assert_eq!(Level::current(&console), Some(Level::Balanced));
        assert!(holds(&console, RESTORE_CVAR, ""));
        // EJK picked as a level has nothing kept: off is High.
        Level::Ejk.apply(&mut console);
        toggle_ejk(&mut console);
        assert_eq!(Level::current(&console), Some(Level::High));
    }
}
