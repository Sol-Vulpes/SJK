// The rules of `requires.rs` (included there): each `rule(setting, parents)` with the
// code that makes it so. Checked against the readers of the cvars on 21/11/2026.

/// Every dependency the rows show.
const RULES: &[Rule] = &[
    // Nameplates: `update()` clears the plates and returns while the switch is off
    // (hud/nameplate.rs), so none of their options is read.
    rule("cg_nameplateRange", &[on("cg_nameplate")]),
    rule("cg_nameplateNear", &[on("cg_nameplate")]),
    rule("cg_nameplateScale", &[on("cg_nameplate")]),
    rule("cg_nameplateOffset", &[on("cg_nameplate")]),
    rule(crate::hud::nameplate::OPACITY_CVAR, &[on("cg_nameplate")]),
    rule("cg_nameplateBars", &[on("cg_nameplate")]),
    rule(crate::hud::nameplate::STYLE_CVAR, &[on("cg_nameplate")]),
    rule("cg_nameplateOutline", &[on("cg_nameplate")]),
    rule("cg_nameplateSelf", &[on("cg_nameplate")]),
    rule("cg_nameplateIcons", &[on("cg_nameplate")]),
    rule("cg_nameplateNpcs", &[on("cg_nameplate")]),
    // The plain names: hidden while nameplates are on (hud_runtime.rs, `labels_hidden`),
    // and their size unread while they are off (hud/identification.rs).
    rule("cg_drawPlayerNames", &[off("cg_nameplate")]),
    rule("cg_drawPlayerNamesScale", &[off("cg_nameplate")]),
    rule("cg_drawPlayerNamesScale", &[not("cg_drawPlayerNames", "0")]),
    // The crosshair's size is 0 while it is hidden (hud/options.rs).
    rule("cg_crosshairSize", &[not("cg_crosshair", "0")]),
    // The lagometer's widget is not drawn at 0 (hud.rs); the card's delay is the look
    // path's, which only "look and key" (1) uses (hud/player_card.rs).
    rule("cg_lagometerScale", &[not(crate::hud::lagometer::CVAR, "0")]),
    rule("cg_playerCardDelay", &[is(crate::hud::player_card::CVAR, "1")]),
    // Only AUTO reaches the monitor's rate (runtime_settings.rs, `normal_cap`).
    rule(
        crate::runtime_settings::MONITOR_CAP_CVAR,
        &[is("com_maxfps", "-1")],
    ),
    // Chat sounds: `route()` mutes every kind while the master is off (chat_sounds.rs).
    rule("cl_chatSoundPublic", &[not(crate::chat_sounds::MASTER_CVAR, "0")]),
    rule("cl_chatSoundTeam", &[not(crate::chat_sounds::MASTER_CVAR, "0")]),
    rule("cl_chatSoundPrivate", &[not(crate::chat_sounds::MASTER_CVAR, "0")]),
    rule("cl_chatSoundSjk", &[not(crate::chat_sounds::MASTER_CVAR, "0")]),
    // Auto exposure: the mode is inactive with it off, and without HDR the darkening
    // limit is forced to 0 (post_exposure.rs).
    rule(
        crate::frame_target::aa::exposure::MIN_EV,
        &[on("r_autoExposure")],
    ),
    rule(crate::frame_target::aa::exposure::MIN_EV, &[on("r_sceneHdr")]),
    rule(
        crate::frame_target::aa::exposure::MAX_EV,
        &[on("r_autoExposure")],
    ),
    rule(
        crate::frame_target::aa::exposure::LEVEL,
        &[on("r_autoExposure")],
    ),
    // The glow target exists only with dynamic glow (post_aa.rs); the emission halo
    // also needs an emission map loaded (material_maps.rs).
    rule("r_dynamicGlowStyle", &[not("r_DynamicGlow", "0")]),
    rule("r_emissiveGlow", &[not("r_DynamicGlow", "0")]),
    rule("r_emissiveGlow", &[on("r_emissiveMaps")]),
    // The shafts' medium exists only with light shafts on (sun_shadows.rs); the dust
    // drifts in its beams (dust_motes.rs).
    rule("r_volumetricClarity", &[not("r_volumetrics", "0")]),
    rule(crate::dust_motes::CVAR, &[not("r_volumetrics", "0")]),
    // The parallax flag needs parallax and normal mapping; emission has nothing to
    // scale without its maps (material_maps.rs, material_map_images.rs).
    rule("r_parallaxStrength", &[on("r_parallaxMapping")]),
    rule("r_parallaxStrength", &[on("r_normalMapping")]),
    rule("r_emissionStrength", &[on("r_emissiveMaps")]),
    // Sun shadows: the runtime exists with world or character shadows, and the world
    // mode also comes with real-time lighting or light shafts
    // (sun_shadow_settings.rs). The distances are the world mode's alone.
    rule(
        "r_sunShadowResolution",
        &[
            on("r_worldSunShadows"),
            on("r_actorSunShadows"),
            on("r_dayNight"),
            not("r_volumetrics", "0"),
        ],
    ),
    rule(
        "r_sunShadowTaps",
        &[
            on("r_worldSunShadows"),
            on("r_actorSunShadows"),
            on("r_dayNight"),
            not("r_volumetrics", "0"),
        ],
    ),
    rule(
        "r_sunShadowDistance",
        &[
            on("r_worldSunShadows"),
            on("r_dayNight"),
            not("r_volumetrics", "0"),
        ],
    ),
    rule(
        "r_sunShadowNear",
        &[
            on("r_worldSunShadows"),
            on("r_dayNight"),
            not("r_volumetrics", "0"),
        ],
    ),
    // The hour drives the live sun, or the sky alone with the time-of-day sky
    // (day_live.rs); the rest is read by the real-time shaders only.
    rule(
        "r_dayHour",
        &[
            on("r_dayNight"),
            on(crate::world_materials::shadows::day::live::SKY_CVAR),
        ],
    ),
    rule(
        "r_dayMinutes",
        &[
            on("r_dayNight"),
            on(crate::world_materials::shadows::day::live::SKY_CVAR),
        ],
    ),
    rule("r_dayBrightness", &[on("r_dayNight")]),
    rule("r_indirectBoost", &[on("r_dayNight")]),
    rule(
        crate::world_materials::shadows::settings::BOUNCE_CVAR,
        &[on("r_dayNight")],
    ),
    rule(
        crate::world_materials::shadows::settings::LAMP_SHADOWS_CVAR,
        &[on("r_dayNight")],
    ),
    // Weather: `prepare` returns after the clouds while it is off (weather.rs). The
    // clouds, the quality (their steps), the map's fog and a forced weather still act.
    rule(crate::weather::DENSITY_CVAR, &[on(crate::weather::CVAR)]),
    rule(crate::weather::FOG_CVAR, &[on(crate::weather::CVAR)]),
    rule(
        crate::weather::REFLECTIONS_CVAR,
        &[on(crate::weather::CVAR)],
    ),
    rule(crate::weather::PUDDLES_CVAR, &[on(crate::weather::CVAR)]),
    rule(
        crate::weather::RAIN_OPACITY_CVAR,
        &[on(crate::weather::CVAR)],
    ),
    rule(crate::weather::HAZE_CVAR, &[on(crate::weather::CVAR)]),
    rule(
        crate::weather::FOG_DENSITY_CVAR,
        &[on(crate::weather::CVAR)],
    ),
];
