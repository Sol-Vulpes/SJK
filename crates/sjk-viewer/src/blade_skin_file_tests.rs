use super::*;

/// A made-up blade skin for the tests (no real skin's values): a teal blade.
pub(crate) const SAMPLE: &str = r#"{
  "version": 1,
  "glow_profile": {"width": 0.4, "peak": 0.5, "tail_width": 0.9, "tail_peak": 0.05},
  "core_profile": {"width": 0.3, "fringe_width": 0.6},
  "core": {
    "white": [0.9, 1.0, 1.0],
    "white_flare": 0.5,
    "fringe_cool": [0.1, 0.5, 0.6],
    "fringe_hot": [0.3, 0.9, 0.8],
    "fringe_heat": {"base": 0.25, "grain": 0.5},
    "fringe_brightness": {"base": 0.8, "grain": 0.4, "flare": 0.9},
    "breathe": {"amount": 0.05, "rate": 8.0, "along": 0.5, "seed": 3.0}
  },
  "corona": {
    "rim_cool": [0.0, 0.2, 0.4],
    "rim_hot": [0.1, 0.5, 0.7],
    "inner": [0.4, 0.9, 0.9],
    "inner_width": 0.7,
    "rim_heat": {"grain": 1.0, "flare": 0.5},
    "inner_mix": {"base": 0.5, "grain": 0.4, "flare": 0.2},
    "brightness": {"base": 0.6, "grain": 0.5, "flare": 1.0},
    "reach": 1.5,
    "swell": {"grain": 0.1, "flare": 0.3}
  },
  "granulation": {
    "octaves": [
      {"scale": 0.3, "speed": 1.5, "evolve": 0.4, "offset": 50.0, "weight": 0.7},
      {"scale": 0.9, "speed": 3.0, "evolve": 0.8, "offset": 90.0, "weight": 0.3}
    ],
    "low": 0.2,
    "high": 0.8
  },
  "flares": {
    "count": 2, "rate": 0.5, "rate_step": 0.1, "rate_seed": 0.05,
    "phase_seed": 5.0, "phase_step": 0.4, "threshold": 0.5,
    "overshoot": 4.0, "size": 3.0, "size_jitter": 1.0
  },
  "shimmer": [{"amount": 0.04, "rate": 8.0, "along": 0.6, "seed": 2.0}],
  "tongues": {
    "along": 0.4, "offset": 11.0, "out": 2.0, "speed": 3.0,
    "edge_low": 0.3, "edge_high": 0.8, "low": 0.4, "range": 1.2
  },
  "trail": [0.2, 0.8, 0.9],
  "light": {
    "color": [0.2, 0.7, 0.9],
    "flicker": {"amount": 0.1, "waves": [{"rate": 6.0, "weight": 0.5, "phase": 1.0}]}
  },
  "sounds": {
    "on": "sound/test/blade/on.wav",
    "off": "sound/test/blade/off.wav",
    "hum": "sound/test/blade/hum.wav",
    "swings": ["sound/test/blade/s1.wav", "sound/test/blade/s2.wav", "sound/test/blade/s3.wav"]
  }
}"#;

/// The sample with `field`'s text (as written in [`SAMPLE`]) replaced by `with`.
pub(crate) fn sample_with(field: &str, with: &str) -> String {
    assert!(SAMPLE.contains(field), "{field}");
    SAMPLE.replacen(field, with, 1)
}

#[test]
fn the_sample_reads_with_every_value() {
    let def = parse("saber_test", SAMPLE).unwrap();
    assert_eq!(def.version, 1);
    assert_eq!(def.glow_profile.width, 0.4);
    assert_eq!(def.core.white, [0.9, 1.0, 1.0]);
    assert_eq!(def.corona.reach, 1.5);
    assert_eq!(def.granulation.octaves.len(), 2);
    assert_eq!(def.flares.count, 2);
    assert_eq!(def.shimmer.len(), 1);
    assert_eq!(def.light.flicker.waves[0].rate, 6.0);
    assert_eq!(def.sounds.swings[2], "sound/test/blade/s3.wav");
    assert_eq!(def.glow_image, None);
}

#[test]
fn absent_profiles_are_the_neutral_pairs() {
    let text = sample_with(
        r#""glow_profile": {"width": 0.4, "peak": 0.5, "tail_width": 0.9, "tail_peak": 0.05},
  "core_profile": {"width": 0.3, "fringe_width": 0.6},"#,
        r#""glow_image": "gfx/test/glow.png", "core_image": "gfx/test/core.png","#,
    );
    let def = parse("saber_test", &text).unwrap();
    assert_eq!(def.glow_profile, GlowProfile::NEUTRAL);
    assert_eq!(def.core_profile, CoreProfile::NEUTRAL);
    assert_eq!(def.glow_image.as_deref(), Some("gfx/test/glow.png"));
}

#[test]
fn unknown_or_missing_fields_and_bad_values_are_refused_by_name() {
    let refused = |text: &str, says: &str| {
        let why = parse("saber_test", text).unwrap_err();
        assert!(why.contains(says), "{why:?} should say {says:?}");
    };
    refused(
        &sample_with(r#""version": 1,"#, r#""version": 1, "colour": 3,"#),
        "unknown field `colour`",
    );
    refused(
        &sample_with(
            r#""white_flare": 0.5,"#,
            r#""white_flare": 0.5, "glint": 1,"#,
        ),
        "unknown field `glint`",
    );
    refused(
        &sample_with(r#""trail": [0.2, 0.8, 0.9],"#, ""),
        "missing field `trail`",
    );
    refused(
        &sample_with(r#""version": 1"#, r#""version": 2"#),
        "version must be 1",
    );
    refused(
        &sample_with(r#""reach": 1.5"#, r#""reach": 3.0"#),
        "corona.reach must be 1 to 2",
    );
    refused(
        &sample_with(r#""count": 2"#, r#""count": 9"#),
        "flares.count must be 0 to 8",
    );
    refused(
        &sample_with(r#""trail": [0.2, 0.8, 0.9]"#, r#""trail": [0.2, 1.8, 0.9]"#),
        "trail green must be 0 to 1",
    );
    refused(
        &sample_with(r#""low": 0.2"#, r#""low": 0.9"#),
        "granulation.low must be below",
    );
    refused(
        &sample_with(r#""edge_low": 0.3"#, r#""edge_low": 0.9"#),
        "tongues.edge_low must be below",
    );
    refused(
        &sample_with(r#""on": "sound/test/blade/on.wav""#, r#""on": "../on.wav""#),
        "sounds.on must be a game path",
    );
    refused(
        &sample_with(
            r#""on": "sound/test/blade/on.wav""#,
            r#""on": "/sound/on.wav""#,
        ),
        "sounds.on must be a game path",
    );
    refused(
        &sample_with(
            r#""shimmer": [{"amount": 0.04, "rate": 8.0, "along": 0.6, "seed": 2.0}]"#,
            r#""shimmer": [{"amount": 0.04, "rate": 8.0, "along": 0.6, "seed": 2.0},
              {"amount": 0.04, "rate": 8.0, "along": 0.6, "seed": 2.0},
              {"amount": 0.04, "rate": 8.0, "along": 0.6, "seed": 2.0}]"#,
        ),
        "shimmer must hold at most 2",
    );
    refused(
        &sample_with(r#", "sound/test/blade/s3.wav"]"#, "]"),
        "invalid length 2",
    );
    refused("3", "invalid type");
    refused("not json", "expected");
    assert!(
        parse("Saber Test", SAMPLE)
            .unwrap_err()
            .contains("not an unlock id")
    );
}

#[test]
fn the_folder_loads_by_id_and_names_the_bad_file() {
    let mut vfs = sjk_vfs::VirtualFileSystem::new();
    vfs.mount_memory(
        "pack",
        [
            (
                "skins/blades/saber_test.bladeskin",
                SAMPLE.as_bytes().to_vec(),
            ),
            ("skins/blades/saber_bad.bladeskin", b"{}".to_vec()),
            ("skins/blades/readme.txt", b"no".to_vec()),
        ],
    )
    .unwrap();
    let skins = load(&vfs);
    assert_eq!(skins.len(), 1);
    assert_eq!(skins[0].0, "saber_test");
}
