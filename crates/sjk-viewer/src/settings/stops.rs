//! Number settings picked from named stops.
//!
//! A slider suits a number the player feels their way along (a volume, a field of
//! view). It suits badly a number players set to one of a handful of known values:
//! the hub's settings usage (2,206 runs up to 21/11/2026) shows the FPS cap set to
//! Auto, 125, 250, 142 or "as far right as it goes" and almost nothing between, the
//! network rate to 90000 or 100000, the prediction smoothing to 0 or 500. Those rows
//! are a list here: each stop under a name that says what it is ("Unlimited", "3
//! minutes"), then "Custom...", which takes any number of the setting's range typed
//! in. Typing a digit on the row still opens the entry at once.
//!
//! One entry per setting, read by the rows' generic code ([`super::catalog::Setting::kind`]):
//! a new list is an entry, not code. The cvars keep their numbers.

use std::borrow::Cow;

/// One named value of a number setting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stop {
    pub(crate) value: f64,
    pub(crate) name: &'static str,
}

const fn stop(value: f64, name: &'static str) -> Stop {
    Stop { value, name }
}

/// The stops of one setting, in the list's order, and what follows a typed value on
/// its row (" FPS").
pub(super) struct Stops {
    pub(super) cvar: &'static str,
    pub(super) suffix: &'static str,
    pub(super) stops: &'static [Stop],
}

/// The last choice of every stops list: type any value of the range.
pub(super) const CUSTOM: &str = "Custom...";

const TABLE: &[Stops] = &[
    // -1 is AUTO (125, or the monitor's rate with Detect refresh rate) and 0 no cap.
    // 125, 142, 250 and 333 are the movement rates players know from Gameplay FPS;
    // 60 to 240 the monitors'.
    Stops {
        cvar: "com_maxfps",
        suffix: " FPS",
        stops: &[
            stop(-1.0, "Auto"),
            stop(60.0, "60 FPS"),
            stop(125.0, "125 FPS"),
            stop(142.0, "142 FPS"),
            stop(144.0, "144 FPS"),
            stop(165.0, "165 FPS"),
            stop(200.0, "200 FPS"),
            stop(240.0, "240 FPS"),
            stop(250.0, "250 FPS"),
            stop(300.0, "300 FPS"),
            stop(333.0, "333 FPS"),
            stop(500.0, "500 FPS"),
            stop(1000.0, "1000 FPS"),
            stop(0.0, "Unlimited"),
        ],
    },
    Stops {
        cvar: crate::runtime_settings::MENU_CAP_CVAR,
        suffix: " FPS",
        stops: &[
            stop(0.0, "Same as the game"),
            stop(30.0, "30 FPS"),
            stop(60.0, "60 FPS"),
            stop(90.0, "90 FPS"),
            stop(120.0, "120 FPS"),
            stop(144.0, "144 FPS"),
            stop(240.0, "240 FPS"),
        ],
    },
    Stops {
        cvar: crate::away_emote::IDLE_CVAR,
        suffix: " s",
        stops: &[
            stop(0.0, "Only when alt-tabbed"),
            stop(60.0, "1 minute"),
            stop(120.0, "2 minutes"),
            stop(180.0, "3 minutes"),
            stop(300.0, "5 minutes"),
            stop(600.0, "10 minutes"),
            stop(900.0, "15 minutes"),
            stop(1800.0, "30 minutes"),
        ],
    },
    Stops {
        cvar: "cg_errorDecay",
        suffix: " ms",
        stops: &[
            stop(0.0, "Off (snap)"),
            stop(50.0, "50 ms"),
            stop(100.0, "100 ms"),
            stop(200.0, "200 ms"),
            stop(350.0, "350 ms"),
            stop(500.0, "500 ms"),
        ],
    },
    // 25000 is the original game's, 90000 SJK's default.
    Stops {
        cvar: "rate",
        suffix: " bytes/s",
        stops: &[
            stop(25_000.0, "25 000 (original)"),
            stop(50_000.0, "50 000"),
            stop(90_000.0, "90 000"),
            stop(100_000.0, "100 000"),
        ],
    },
    Stops {
        cvar: "snaps",
        suffix: " a second",
        stops: &[
            stop(20.0, "20 a second"),
            stop(30.0, "30 a second"),
            stop(40.0, "40 a second"),
            stop(60.0, "60 a second"),
            stop(100.0, "100 a second"),
            stop(120.0, "120 a second"),
            stop(125.0, "125 a second"),
        ],
    },
];

/// The stops of setting `cvar`, when its row is a list of them.
pub(super) fn of(cvar: &str) -> Option<&'static Stops> {
    TABLE
        .iter()
        .find(|stops| stops.cvar.eq_ignore_ascii_case(cvar))
}

/// Every setting with stops, for the tests and the catalogue's checks.
#[cfg(test)]
pub(super) fn all() -> &'static [Stops] {
    TABLE
}

/// The stop `value` is, if it is one.
pub(super) fn index(stops: &[Stop], value: f64) -> Option<usize> {
    stops
        .iter()
        .position(|stop| (stop.value - value).abs() < 1e-6)
}

/// A value as its cvar takes it: whole for an integer setting, else without float
/// noise ("0.5", "100").
pub(super) fn text(value: f64, integer: bool) -> String {
    if integer || value.fract().abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.3}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// What a row shows for the value written `raw`: its stop's name, or the typed
/// number and the setting's unit ("300 FPS"). Text that is not a number is shown as
/// it is.
pub(super) fn shown<'a>(stops: &[Stop], suffix: &str, raw: &'a str) -> Cow<'a, str> {
    let Ok(value) = raw.trim().parse::<f64>() else {
        return Cow::Borrowed(raw);
    };
    match index(stops, value) {
        Some(at) => Cow::Borrowed(stops[at].name),
        None => Cow::Owned(format!("{}{suffix}", text(value, false))),
    }
}

/// The value one step `direction` along the list from `value`: the next stop from a
/// stop, the ends staying where they are; from a typed value, the nearest stop that
/// way among the plain numbers (a list's first and last may be special values, as
/// the FPS cap's Auto and Unlimited are).
pub(super) fn step(stops: &[Stop], value: f64, direction: i32) -> f64 {
    let Some(last) = stops.len().checked_sub(1) else {
        return value;
    };
    if let Some(at) = index(stops, value) {
        let next = (at as i64 + i64::from(direction)).clamp(0, last as i64) as usize;
        return stops[next].value;
    }
    let plain = |stop: &&Stop| stop.value > 0.0;
    let next = if direction > 0 {
        stops
            .iter()
            .filter(plain)
            .find(|stop| stop.value > value)
            .or(stops.last())
    } else {
        stops
            .iter()
            .filter(plain)
            .rfind(|stop| stop.value < value)
            .or(stops.first())
    };
    next.map_or(value, |stop| stop.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_shows_its_stops_name_or_the_typed_number_with_its_unit() {
        let fps = of("com_maxfps").unwrap();
        assert_eq!(shown(fps.stops, fps.suffix, "-1"), "Auto");
        assert_eq!(shown(fps.stops, fps.suffix, "0"), "Unlimited");
        assert_eq!(shown(fps.stops, fps.suffix, "250"), "250 FPS");
        assert_eq!(shown(fps.stops, fps.suffix, "275"), "275 FPS");
        let decay = of("cg_errorDecay").unwrap();
        assert_eq!(shown(decay.stops, decay.suffix, "100.0"), "100 ms");
        assert_eq!(shown(decay.stops, decay.suffix, "72.5"), "72.5 ms");
        assert_eq!(shown(decay.stops, decay.suffix, "?"), "?");
    }

    #[test]
    fn steps_follow_the_list_and_a_typed_value_joins_it_at_its_place() {
        let fps = of("com_maxfps").unwrap().stops;
        // Auto is the first stop, Unlimited the last: neither wraps.
        assert_eq!(step(fps, -1.0, -1), -1.0);
        assert_eq!(step(fps, -1.0, 1), 60.0);
        assert_eq!(step(fps, 1000.0, 1), 0.0);
        assert_eq!(step(fps, 0.0, 1), 0.0);
        assert_eq!(step(fps, 0.0, -1), 1000.0);
        // A typed 275 sits between 250 and 300; 2000 is past every number.
        assert_eq!(step(fps, 275.0, 1), 300.0);
        assert_eq!(step(fps, 275.0, -1), 250.0);
        assert_eq!(step(fps, 2000.0, 1), 0.0);
        assert_eq!(step(fps, 2000.0, -1), 1000.0);
        assert_eq!(step(fps, 30.0, -1), -1.0);
    }

    #[test]
    fn every_list_names_each_value_once_and_its_cvar_text_reads_back() {
        for stops in all() {
            assert!(stops.stops.len() >= 2, "{}", stops.cvar);
            for (at, stop) in stops.stops.iter().enumerate() {
                assert_eq!(index(stops.stops, stop.value), Some(at), "{}", stops.cvar);
                assert!(!stop.name.is_empty() && stop.name != CUSTOM);
                assert_eq!(text(stop.value, true).parse::<f64>().ok(), Some(stop.value));
            }
        }
    }
}
