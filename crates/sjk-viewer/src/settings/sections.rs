//! How Advanced's long categories are cut into sections.
//!
//! Settings grew a row at a time: by 21/11/2026 HUD held 106 rows under 8 headings,
//! Gameplay 68 with 45 of them under "Effects" (sabers, Force powers, duels, map edits
//! and remaps together), Sound 59 and Chat's "Chat box" 30. This table says, for each of
//! those categories, which sections it shows and which settings each holds, in order: a
//! section is one subject, short enough to read at a glance, under a heading that wears
//! its own picture ([`super::section_icons`]).
//!
//! A setting named here belongs to that category, whatever classic group or tab it
//! comes from (the away settings are Gameplay's, not Sound's). One a category holds
//! that no section names still shows, after the sections, under [`MORE`], so a new
//! setting is never lost; a test keeps that heading empty.

use super::groups::Advanced;

/// One section of a category: its heading and its settings' cvars, in order.
pub(super) struct Section {
    pub(super) heading: &'static str,
    pub(super) rows: &'static [&'static str],
}

/// The heading of a category's settings that no section names.
pub(super) const MORE: &str = "More";

const fn section(heading: &'static str, rows: &'static [&'static str]) -> Section {
    Section { heading, rows }
}

const SOUND: &[Section] = &[
    section(
        "Volume",
        &[
            "s_volume",
            "s_musicVolume",
            "s_volumeVoice",
            "snd_mute_losefocus",
        ],
    ),
    section(
        "Game sounds",
        &[
            "cg_musicSync",
            "s_doppler",
            "s_saberSkinSounds",
            "cg_jetpackonsound",
            "cg_jetpackhoversound",
            "cg_tauntAntiSpam",
            "cg_hitsounds",
            "cg_footsteps",
            "cg_jumpFootsteps",
            "cg_footprints",
            "cg_footstepGuess",
            "s_weatherSounds",
            "s_muteSounds",
        ],
    ),
    section(
        "3D sound",
        &[
            "s_spatial",
            "s_spatialHeadphones",
            "s_spatialReverb",
            "cg_soundSonar",
        ],
    ),
    section(
        "Emotes",
        &[
            "s_emoteVolume",
            "s_muteEmotesInDuel",
            "cg_hideOthersEmotes",
            "cg_hiddenEmotes",
            "s_mutedEmotes",
            "cg_emoteAttackStop",
            "cg_emoteTypingKeep",
        ],
    ),
    section(
        "Toys",
        &[
            "s_toyVolume",
            "s_toyRagdollSound",
            "s_toyDroneSound",
            "s_toyBoomboxSound",
            "s_toyMonitorSound",
            "s_toyMysteryBoxSound",
            "s_toyTagSound",
            "s_toyMasterSound",
            "s_toyWheelOfDoomSound",
            "s_toyQuestsSound",
        ],
    ),
    section(
        "Notifications",
        &["cg_achievementSound", "cg_worldPingSound", "cg_wheelSounds"],
    ),
    section(
        "Chat sounds",
        &[
            "cg_chatSounds",
            "cg_chatBeep",
            "cl_chatSoundPublic",
            "chatsoundpreview public",
            "cg_teamChatBeep",
            "cl_chatSoundTeam",
            "chatsoundpreview team",
            "cg_privateChatBeep",
            "cl_chatSoundPrivate",
            "chatsoundpreview private",
            "cl_sjkChatSound",
            "cl_chatSoundSjk",
            "chatsoundpreview sjk",
            "cl_privateChatFlash",
        ],
    ),
];

const GAMEPLAY: &[Section] = &[
    section(
        "Players and models",
        &[
            "cg_forceModel",
            "cg_modelEligibility",
            "cg_defaultModelRandom",
            "cg_modelBlacklist",
            "cg_simpleItems",
            // What a saber kill does to the body; Sabers is full.
            "cg_finishers",
            "cg_tactical",
            "cg_tacticalDuel",
            "cg_tacticalModel",
            "r_flatWorld",
        ],
    ),
    section(
        "Sabers",
        &[
            "cg_saberTrail",
            "cg_saberClash",
            "cg_saberClashSize",
            "cg_saberCoreSize",
            "cg_fleshSparks",
            "cg_saberTeamColors",
            "cg_ctfTeamSabers",
            "cg_blacksabers",
            "cg_saberLabDesigns",
            "cg_stockBlades",
            "cg_saberRainSteam",
            "cg_drawholsteredsaber",
            "cg_holsteredsabersupport",
            "cg_holsteredstaffswap",
            "cg_holsteredStaffSound",
        ],
    ),
    section(
        "Force",
        &[
            "cg_auraShell",
            "cg_speedTrail",
            "cg_gripVictimFX",
            "cg_flameThrower",
            "cg_forceAnimFX",
            "cg_empowerEffect",
            "cg_spProtAbsColor",
            "cg_shieldSphere",
            "cg_shieldBrightness",
            "cg_reduceFlashes",
        ],
    ),
    section(
        "Duels",
        &[
            "cg_duelStockBlades",
            "cg_duelGhost",
            "cg_duelHideOthers",
            "cg_hideCorpsesInDuel",
            "cg_hidePickupsInDuel",
        ],
    ),
    section(
        "Maps and remaps",
        &[
            "cg_serverRemaps",
            "cg_remaps",
            "cg_serverRemapsMap",
            "cg_remapsShared",
            "cg_mapEdits",
            "mapedits_sync",
        ],
    ),
    section(
        "Toys",
        &[
            "cg_toyMonitorShow",
            "cg_toyTags",
            "cg_toyGifTags",
            "cg_toyIlluminateShow",
            "cg_toyRagdollShow",
            "cg_toyDroneShow",
            "cg_toyBoomboxShow",
            "cg_toyBoomboxRange",
        ],
    ),
    section(
        "Camera",
        &[
            "cg_cameraStyle",
            "cg_drawGun",
            "cg_fovViewmodel",
            "cg_cameraFPS",
            "cg_thirdPersonRange",
            "cg_thirdPersonAngle",
            "cg_thirdPersonPitchOffset",
            "cg_thirdPersonVertOffset",
            "cg_thirdPersonHorzOffset",
            "cg_thirdPersonCameraDamp",
            "cg_thirdPersonTargetDamp",
            "cg_wallGrabLock",
            "cg_heldViewLook",
        ],
    ),
    section("Prediction", &["cg_predictLegacy", "cg_errorDecay"]),
    // From Sound's Emotes: when you count as away is not a sound.
    section(
        "Away",
        &[
            "cg_awayEmote",
            "japlus.awayEmote",
            "cg_awayAltTab",
            "cg_awayIdle",
        ],
    ),
];

const INTERFACE: &[Section] = &[
    section(
        "Menus",
        &[
            "ui_ejkMode",
            "ui_menuStyle",
            "ui_classicLook",
            "ui_textScale",
            "ui_letterSpacing",
            "ui_gameFont",
            "ui_menuPictures",
            "ui_sliderPeek",
            "ui_rarityEffects",
            "ui_collectionHall",
            "cl_tradeWindowSize",
            "ui_launcherNotice",
        ],
    ),
    section("Theme", &["ui_theme", "ui_themeHue", "ui_themeSaturation"]),
    section(
        "Console",
        &[
            "con_style",
            "con_font",
            "con_scale",
            "con_drawNotify",
            "con_chatOnly",
            "con_copyColors",
            "con_shiftHeight",
            "con_overChat",
            "cg_devBar",
        ],
    ),
];

const HUD: &[Section] = &[
    section(
        "Layout",
        &[
            "cg_drawHud",
            "cg_hudHideElements",
            "cg_hudHideCrosshair",
            "cg_hudHideNames",
            "cg_hudHideChat",
            "cg_hudStyle",
            "cg_hudFiles",
            "cg_hudScale",
            "hud_edit",
            "cg_hudRadialSpread",
            "cg_hudRadialHeight",
            "cg_classicHudFont",
            "cg_drawStatus",
            "cg_drawWeapon",
            "cg_drawInventory",
        ],
    ),
    section(
        "Crosshair",
        &[
            "cg_crosshair",
            "cg_crosshairSize",
            "cg_hitMarker",
            "cg_hitMarkerShield",
        ],
    ),
    section(
        "Names and cards",
        &[
            "cg_drawCrosshairNames",
            "cg_drawPlayerNames",
            "cg_drawPlayerNamesScale",
            "cg_drawFriend",
            "cg_teamOutline",
            "cg_playerCard",
            "cg_playerCardDelay",
            "cg_playerCardOutline",
            "cg_worldPings",
        ],
    ),
    section(
        "Nameplates",
        &[
            "cg_nameplate",
            "cg_nameplateStyle",
            "cg_nameplateRange",
            "cg_nameplateNear",
            "cg_nameplateScale",
            "cg_nameplateOffset",
            "cg_nameplateAlpha",
            "cg_nameplateOutline",
            "cg_nameplateBars",
            "cg_nameplateIcons",
            "cg_nameplateDuel",
            "cg_nameplateSelf",
            "cg_nameplateHideDuelers",
            "cg_nameplateWalls",
            "cg_nameplateNpcs",
        ],
    ),
    section(
        "Playstyle",
        &[
            "cg_nameplatePlaystyle",
            "cg_playSaberDuels",
            "cg_playForceDuels",
            "cg_playFfa",
            "cg_playGoofing",
            "cg_playTeaching",
        ],
    ),
    section(
        "Readouts",
        &[
            "cg_drawFps",
            "r_drawTimings",
            "r_logTimings",
            "cg_lagometer",
            "cg_lagometerScale",
            "cg_drawClock",
            "cg_clock12h",
            "cg_drawVersion",
        ],
    ),
    section(
        "Match",
        &[
            "cg_drawTimer",
            "cg_drawTimerUpperRight",
            "cg_drawTeamScores",
            "cg_teamScoresBoxes",
            "cg_teamScoresScale",
            "cg_drawFlagStatus",
            "cg_killfeed",
            "cg_feats",
            "cg_featsMedals",
            "cg_featsVoice",
            "cg_announcer",
        ],
    ),
    section(
        "Movement",
        &[
            "cg_speedometer",
            "cg_movementKeys",
            "strafelines",
            "cg_strafeAura",
            "cg_strafeAuraGround",
            "cg_groundHud",
        ],
    ),
    section(
        "Status bar",
        &[
            "cg_statusBar",
            "cg_statusBarCredits",
            "cg_statusBarShards",
            "cg_statusBarCrystals",
            "cg_statusBarCoins",
            "cg_statusBarNew",
            "cg_statusBarSaber",
            "cg_statusBarEffects",
            "cg_statusBarWeapons",
            "cg_statusBarToys",
            "cg_statusBarTime",
            "cg_statusBarDate",
            "cg_statusBarTopForce",
            "cg_statusBarTopScore",
            "cg_statusBarTopTimeLeft",
            "cg_statusBarTopFeed",
            "cg_statusBarTopServer",
            "cg_statusBarTopMap",
            "cg_statusBarTopMode",
            "cg_statusBarTopPlayers",
            "cg_statusBarTopPing",
            "cg_statusBarTopFps",
        ],
    ),
    section(
        "Scoreboard",
        &[
            "cg_scoreboardStyle",
            "cg_compactScoreboard",
            "cg_smallScoreboard",
            "cg_scoreboardFps",
            "cg_scoreboardAfk",
            "cg_scoreboardDeathsRank",
            "cg_showClientIDs",
            "cg_scoreboardIDs",
            "cg_clanEmblems",
            "cg_drawScoreboardIcons",
        ],
    ),
];

const CHAT: &[Section] = &[
    section(
        "Chat box",
        &[
            "cg_drawChat",
            "cl_unfocusedChatbox",
            "cg_statusBalloons",
            "cg_chatBoxEmojis",
            "cg_chatRememberChannel",
            "cg_chatColor",
            "cg_pmOnlyChat",
            "cl_logChat",
            "cl_charset",
        ],
    ),
    section(
        "Chat look",
        &[
            "cg_chatBoxFontSize",
            "cg_chatFont",
            "cg_chatBoxLineSpacing",
            "cg_chatBoxLetterSpacing",
            "cg_chatBoxBackdrop",
            "cg_chatBoxOutline",
            "cg_darkTextOutline",
            "cg_chatCardScale",
        ],
    ),
    section(
        "Chat tabs",
        &[
            "cg_chatTabFilter",
            "cg_chatTabTeam",
            "cg_chatTabSjk",
            "cg_chatTabJof",
            "cg_chatTabS",
            "cg_chatTabAdmin",
            "cg_chatTabPm",
        ],
    ),
    section(
        "Do not disturb",
        &["cg_doNotDisturb", "cg_doNotDisturbDuel"],
    ),
    section(
        "Duel lines",
        &[
            "cg_duelStartLine",
            "cg_duelWinLine",
            "cg_duelLoseLine",
            "cg_duelLinesToAll",
        ],
    ),
    section(
        "SJK chat",
        &[
            "cl_sjkChat",
            "cl_sjkChatFeed",
            "cl_sjkChatConsole",
            "cl_sjkChatJof",
            "cl_sjkChatS",
            "cl_sjkChatClientIcons",
            "cl_sjkChatGifs",
            "cl_sjkChatVideo",
            "cl_giphyKey",
        ],
    ),
    section("Translation", &["cl_chatTranslate", "cl_chatTranslateSkip"]),
    section(
        "Privacy and blocks",
        &["cl_e2eWhispers", "cl_staffBlocks", "cl_ignoreMutesSounds"],
    ),
];

/// The sections of category `advanced`; none for one that keeps its plain headings
/// (Mods).
pub(super) fn of(advanced: Advanced) -> &'static [Section] {
    match advanced {
        Advanced::Sound => SOUND,
        Advanced::Gameplay => GAMEPLAY,
        Advanced::Interface => INTERFACE,
        Advanced::Hud => HUD,
        Advanced::Chat => CHAT,
        Advanced::Mods => &[],
    }
}

/// Whether a section of any category names setting `cvar`.
pub(super) fn names(cvar: &str) -> bool {
    Advanced::ALL.iter().any(|advanced| {
        of(*advanced).iter().any(|section| {
            section
                .rows
                .iter()
                .any(|row| row.eq_ignore_ascii_case(cvar))
        })
    })
}

/// The section of category `advanced` holding setting `cvar`.
pub(super) fn holding(advanced: Advanced, cvar: &str) -> Option<&'static Section> {
    of(advanced).iter().find(|section| {
        section
            .rows
            .iter()
            .any(|row| row.eq_ignore_ascii_case(cvar))
    })
}

#[cfg(test)]
mod tests {
    use super::super::{Group, Line};
    use super::*;

    #[test]
    fn every_setting_named_exists_and_is_named_once() {
        let mut seen = std::collections::HashSet::new();
        for advanced in Advanced::ALL {
            for section in of(advanced) {
                assert!(!section.rows.is_empty(), "{}", section.heading);
                for cvar in section.rows {
                    assert!(
                        super::super::has_row(cvar),
                        "{}: {cvar} is not a setting",
                        section.heading
                    );
                    assert!(
                        seen.insert(cvar.to_ascii_lowercase()),
                        "{cvar} is in two sections"
                    );
                }
            }
        }
    }

    /// A setting a category holds that no section names would show under "More":
    /// give it a section.
    #[test]
    fn no_category_is_left_with_settings_under_more() {
        for advanced in Advanced::ALL {
            let group = Group::Advanced(advanced);
            let rows = group.rows();
            let mut under_more = false;
            let mut left = Vec::new();
            for line in group.lines() {
                match line {
                    Line::Heading(heading) => under_more = heading == MORE,
                    Line::Row(row) if under_more => left.push(rows[row].cvar),
                    Line::Row(_) | Line::Note(_) => {}
                }
            }
            assert!(left.is_empty(), "{advanced:?}: no section for {left:?}");
        }
    }

    #[test]
    fn sections_stay_short_enough_to_read() {
        for advanced in Advanced::ALL {
            for section in of(advanced) {
                // The status bar's parts are one list of 22 switches.
                let most = if section.heading == "Status bar" {
                    22
                } else {
                    15
                };
                assert!(
                    section.rows.len() <= most,
                    "{}: {} rows",
                    section.heading,
                    section.rows.len()
                );
            }
        }
    }
}
