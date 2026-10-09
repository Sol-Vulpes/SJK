//! Graphics quality levels: one choice that sets the renderer's costly
//! settings together, from Performance (the most frames) to Ultra (the best
//! look). High is the fresh profile's own values, so a new player reads High.
//! The level is not stored: it is whichever level the settings match, and
//! Custom once one of them is changed on its own. Only settings with a row in
//! Settings are touched, so every change a level makes can be seen and undone
//! there; the FPS cap, vertical sync, resolution, supersampling and anything
//! that changes gameplay or a map's content are left alone.

use crate::console::ViewerConsole;

/// Console command that names or sets the level.
pub(crate) const COMMAND: &str = "graphicsquality";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Show or set graphics quality: performance, balanced, high or ultra";
/// The Settings row's stand-in for a cvar name: the command that sets it.
pub(crate) const ROW_NAME: &str = COMMAND;

/// A graphics quality level, cheapest first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Level {
    /// The most frames: the retail look with SJK's lighting, shadows,
    /// post-processing and weather extras off.
    Performance,
    /// Most of SJK's look at a lower cost: no floor mirrors or reflection
    /// probes, fewer light-shaft samples, smaller, softer-filtered shadows.
    Balanced,
    /// SJK's defaults.
    High,
    /// Everything at its best: larger, finer shadows and the richest weather.
    Ultra,
}

/// Each setting a level sets, with its value at Performance, Balanced, High
/// and Ultra. High is each setting's default (pinned by a test).
const VALUES: &[(&str, [&str; 4])] = &[
    // Image.
    ("r_sceneHdr", ["0", "1", "1", "1"]),
    ("r_sceneBloom", ["0", "1", "1", "1"]),
    ("r_DynamicGlow", ["0", "1", "1", "1"]),
    ("r_fxaa", ["0", "1", "1", "1"]),
    ("r_softParticles", ["0", "1", "1", "1"]),
    (crate::dust_motes::CVAR, ["0", "0.5", "1", "1"]),
    ("r_modelPixelLight", ["0", "1", "1", "1"]),
    ("r_cubeMapping", ["0", "0", "1", "1"]),
    ("r_floorReflections", ["0", "0", "1", "1"]),
    ("r_parallaxMapping", ["0", "1", "1", "1"]),
    ("r_emissiveMaps", ["0", "1", "1", "1"]),
    ("r_emissiveGlow", ["0", "1", "1", "1"]),
    // Lighting.
    ("r_dayNight", ["0", "1", "1", "1"]),
    ("r_emissiveLights", ["0", "1", "1", "1"]),
    ("r_volumetrics", ["0", "1", "3", "3"]),
    // Shadows.
    ("r_worldSunShadows", ["0", "1", "1", "1"]),
    ("r_actorSunShadows", ["0", "1", "1", "1"]),
    ("r_sunShadowResolution", ["1024", "1024", "2048", "4096"]),
    ("r_sunShadowTaps", ["8", "8", "16", "24"]),
    // Weather.
    (crate::weather::DENSITY_CVAR, ["1", "1.5", "2", "2"]),
    (crate::weather::QUALITY_CVAR, ["0", "1", "2", "3"]),
    (crate::weather::CLOUDS_CVAR, ["0", "1", "1", "1"]),
];

impl Level {
    pub(crate) const ALL: [Self; 4] = [Self::Performance, Self::Balanced, Self::High, Self::Ultra];

    /// The level's name in Settings.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Performance => "Performance",
            Self::Balanced => "Balanced",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// The level `name` names, in any case.
    fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.label().eq_ignore_ascii_case(name.trim()))
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
    let names = || {
        Level::ALL
            .map(|level| level.label().to_ascii_lowercase())
            .join(", ")
    };
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
                 reflection probes and material maps apply after a restart, emission-map \
                 lights on the next map",
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
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Performance));
    }

    #[test]
    fn no_level_costs_less_than_the_one_below_it() {
        for (cvar, values) in VALUES {
            let numbers: Vec<f64> = values.iter().map(|v| v.parse().unwrap()).collect();
            assert!(
                numbers.windows(2).all(|pair| pair[0] <= pair[1]),
                "{cvar}: {values:?}"
            );
            assert!(numbers[0] < numbers[3], "{cvar} is the same at every level");
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
        assert_eq!(Level::Performance.step(-1), Level::Performance);
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
    }
}
