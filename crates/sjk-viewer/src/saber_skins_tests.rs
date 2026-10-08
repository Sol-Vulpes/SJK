use super::*;
use crate::saber::{self, Blade, Color, Instance};
use crate::saber_rgb::{MATERIAL_COUNT, RGB_MATERIAL, SKIN_MATERIAL};
use crate::saber_submission::lights;

fn blade() -> Blade {
    Blade {
        base: [0.0; 3],
        direction: [0.0, 0.0, 1.0],
        length: 40.0,
        radius: 3.0,
    }
}

#[test]
fn the_sun_has_its_own_material_after_the_neutral_pair() {
    assert_eq!(SKIN_MATERIAL, RGB_MATERIAL + 1);
    assert_eq!(MATERIAL_COUNT, 8);
    let sun = BladeColor::Skin(BladeSkin::Sun);
    assert_eq!(sun.material(), 7);
    assert_eq!(sun.tint(), [1.0; 3]);
    // Grouped into its own range, after every other material.
    let mut instances: Vec<Instance> = [
        sun,
        BladeColor::Retail(Color::Blue),
        BladeColor::from_rgb([10, 20, 30]),
        sun,
        BladeColor::Retail(Color::Red),
    ]
    .into_iter()
    .flat_map(|color| Instance::pair(blade(), color))
    .collect();
    let ranges = saber::material_ranges(&mut instances);
    assert_eq!(ranges[7], 6..10);
    assert_eq!(ranges[6], 4..6);
    assert_eq!(ranges[0], 0..2);
    assert_eq!(ranges[4], 2..4);
    assert!(instances[6..].iter().all(|i| i.material() == 7));
}

#[test]
fn retail_and_rgb_colours_are_unchanged() {
    for color in Color::ALL {
        let retail = BladeColor::Retail(color);
        assert_eq!(retail.material(), color.index() as u32);
        assert_eq!(retail.tint(), [1.0; 3]);
        assert_eq!(retail.trail_rgb(), color.trail_rgb());
        assert_eq!(BladeColor::from_rgb(color.blade_rgb()), retail);
        assert_eq!(
            BladeColor::from_material(retail.material(), [1.0; 3]),
            retail
        );
    }
    let rgb = BladeColor::from_rgb([10, 200, 30]);
    assert_eq!(rgb.material(), RGB_MATERIAL);
    assert_eq!(rgb.trail_rgb(), [10.0 / 255.0, 200.0 / 255.0, 30.0 / 255.0]);
    assert_eq!(BladeColor::from_material(RGB_MATERIAL, rgb.tint()), rgb);
    assert_eq!(
        lights::rgb(BladeColor::Retail(Color::Blue)),
        [0.2, 0.4, 1.0]
    );
    assert_eq!(lights::rgb(rgb), rgb.tint());
}

#[test]
fn the_sun_trails_amber_and_lights_warm_orange_with_a_gentle_flicker() {
    let sun = BladeColor::Skin(BladeSkin::Sun);
    let [r, g, b] = sun.trail_rgb();
    assert!(
        r == 1.0 && g > b && g < 0.8 && b < 0.3,
        "{:?}",
        sun.trail_rgb()
    );
    let [r, g, b] = lights::rgb(sun);
    assert!(r == 1.0 && g > b && g < 0.7, "{:?}", lights::rgb(sun));
    let levels: Vec<f32> = (0..400)
        .map(|step| BladeSkin::Sun.light_flicker(step * 25, 3))
        .collect();
    let (low, high) = levels
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert!(
        (0.84..0.9).contains(&low) && (0.95..=1.0).contains(&high),
        "{low} {high}"
    );
    // Slow: no frame-to-frame strobe.
    assert!(
        levels
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 0.05)
    );
    // Seeds do not pulse together.
    assert_ne!(
        BladeSkin::Sun.light_flicker(1_000, 3),
        BladeSkin::Sun.light_flicker(1_000, 4)
    );
    // The light itself: warm, and dimmer or brighter than steady stock as it flickers.
    let mut list = crate::dynamic_lights::PointLightList::default();
    let mut lit = [None; 8];
    lit[0] = Some(blade());
    lights::append(&mut list, &lit, sun, 1, false, 1_000, 3);
    let gain = 1.8 * BladeSkin::Sun.light_flicker(1_000, 3);
    let color = lights::rgb(sun).map(|c| c * gain);
    assert!(list.as_slice().iter().any(|light| light.color == color));
}

#[test]
fn a_skinned_blade_keeps_its_skin_through_contacts_and_reaches_further() {
    let sun = Instance::pair(blade(), BladeColor::Skin(BladeSkin::Sun))
        .map(|i| i.with_contact(3, 0, 0, true, false));
    let (_, _, color, _) = sun[0].contact().expect("a contact source");
    assert_eq!(color, BladeColor::Skin(BladeSkin::Sun));
    // Wide enough that the capsule, not the hilt sprite, sets the reach.
    let wide = Blade {
        radius: 8.0,
        ..blade()
    };
    let reach = |color| Instance::pair(wide, color).map(|instance| instance.extent().2);
    let (sun_reach, stock) = (
        reach(BladeColor::Skin(BladeSkin::Sun)),
        reach(BladeColor::from_rgb([10, 20, 30])),
    );
    assert!(sun_reach[0] > stock[0] * 1.3);
    assert_eq!(sun_reach[1], stock[1]);
}

#[test]
fn the_table_gives_each_players_body_its_skin() {
    let mut skins = SaberSkins::default();
    assert_eq!(skins.get(1), None);
    skins.set(4, Some(BladeSkin::Sun));
    // Entity ids are client slots plus one.
    assert_eq!(skins.get(5), Some(BladeSkin::Sun));
    assert_eq!(skins.get(4), None);
    assert_eq!(skins.get(0), None);
    assert_eq!(skins.get(33), None);
    assert_eq!(skins.get(1_000), None);
    skins.set(32, Some(BladeSkin::Sun));
    assert!(
        skins
            .sound_sets()
            .iter()
            .filter(|set| set.is_some())
            .count()
            == 1
    );
    let red = BladeColor::Retail(Color::Red);
    assert_eq!(skins.blade_color(5, red), BladeColor::Skin(BladeSkin::Sun));
    assert_eq!(skins.blade_color(6, red), red);
    skins.set(4, None);
    assert_eq!(skins.get(5), None);
    assert_eq!(skins.sound_sets(), [None; MAX_CLIENTS]);
}

#[test]
fn the_local_choice_follows_the_players_slot() {
    let mut skins = SaberSkins::default();
    // In the menus: no slot, the preview still wears it.
    skins.set_local(None, Some(BladeSkin::Sun));
    assert_eq!(skins.local(), Some(BladeSkin::Sun));
    assert_eq!(skins.sound_sets(), [None; MAX_CLIENTS]);
    skins.set_local(Some(2), Some(BladeSkin::Sun));
    assert_eq!(skins.get(3), Some(BladeSkin::Sun));
    assert_eq!(skins.sound_sets()[2], Some(0));
    // A new slot (another server) clears the old one.
    skins.set_local(Some(7), Some(BladeSkin::Sun));
    assert_eq!(skins.get(3), None);
    assert_eq!(skins.get(8), Some(BladeSkin::Sun));
    skins.set_local(Some(7), None);
    assert_eq!(skins.get(8), None);
    assert_eq!(skins.local(), None);
}

#[test]
fn catalogue_ids_name_the_skins_and_anything_else_is_stock() {
    assert_eq!(BladeSkin::from_id("saber_sun"), Some(BladeSkin::Sun));
    assert_eq!(BladeSkin::from_id(" SABER_Sun "), Some(BladeSkin::Sun));
    assert_eq!(BladeSkin::from_id(""), None);
    assert_eq!(BladeSkin::from_id("saber_moon"), None);
    for skin in BladeSkin::ALL {
        let id = crate::unlockables::of_blade_skin(skin).id;
        assert_eq!(BladeSkin::from_id(id), Some(skin));
        assert_eq!(BladeSkin::ALL[skin.index()], skin);
    }
}

#[test]
fn other_players_skins_follow_the_looks_only_when_they_change() {
    use std::cell::Cell;
    let mut skins = SaberSkins::default();
    skins.set_local(Some(2), Some(BladeSkin::Sun));
    let asked = Cell::new(0);
    let looks = |client: usize| {
        asked.set(asked.get() + 1);
        // The hub says slot 2 (the local player's own) wears nothing: its own wins.
        (client == 5).then_some(BladeSkin::Sun)
    };
    skins.follow_looks(7, looks);
    assert_eq!(asked.get(), MAX_CLIENTS);
    assert_eq!(skins.get(6), Some(BladeSkin::Sun));
    assert_eq!(skins.get(3), Some(BladeSkin::Sun), "the local player's own");
    let sets = skins.sound_sets();
    assert_eq!(sets[5], Some(0));
    assert_eq!(sets[2], Some(0));
    assert_eq!(sets.iter().filter(|set| set.is_some()).count(), 2);
    // The same revision is not read again.
    skins.follow_looks(7, looks);
    assert_eq!(asked.get(), MAX_CLIENTS);
    // A new one is; the slot that took its look off goes stock, sounds too.
    skins.follow_looks(8, |_| None);
    assert_eq!(skins.get(6), None);
    assert_eq!(skins.sound_sets()[5], None);
    assert_eq!(skins.get(3), Some(BladeSkin::Sun));
    // The local player leaves the game: its old slot shows the hub's look again.
    skins.set(2, Some(BladeSkin::Sun));
    skins.set_local(None, None);
    assert_eq!(skins.get(3), Some(BladeSkin::Sun));
    assert_eq!(skins.local(), None);
}

#[test]
fn the_local_skin_is_gated_by_the_own_profile() {
    let mut looks = crate::looks::Looks::default();
    // `cg_saberSkin saber_sun` without the unlock: the stock blade.
    let unowned = crate::looks::Worn::own("saber_sun", |_| false, false);
    looks.set_own(None, unowned);
    assert_eq!(looks.own_saber_skin().and_then(BladeSkin::from_id), None);
    let owned = crate::looks::Worn::own("saber_sun", |id| id == "saber_sun", false);
    looks.set_own(Some(4), owned);
    assert_eq!(
        looks.own_saber_skin().and_then(BladeSkin::from_id),
        Some(BladeSkin::Sun)
    );
    // The local slot in the looks wears it too, so following them agrees.
    let mut skins = SaberSkins::default();
    skins.follow_looks(looks.revision(), |client| {
        looks.saber_skin_id(client).and_then(BladeSkin::from_id)
    });
    assert_eq!(skins.get(5), Some(BladeSkin::Sun));
}

#[test]
fn the_cvar_is_registered_archived_and_empty() {
    let directory = tempfile::tempdir().unwrap();
    let console = crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
    assert_eq!(
        console.cvar(crate::unlockables::SABER_SKIN_CVAR),
        Some(&sjk_shell::CvarValue::Text(String::new()))
    );
}

/// A 16-bit mono PCM WAV's rate and samples.
fn read_wav(bytes: &[u8]) -> (u32, Vec<i16>) {
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut at = 12;
    let (mut rate, mut samples) = (None, None);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let body = &bytes[at + 8..at + 8 + size];
        match id {
            b"fmt " => {
                assert_eq!(u16::from_le_bytes([body[0], body[1]]), 1, "PCM");
                assert_eq!(u16::from_le_bytes([body[2], body[3]]), 1, "mono");
                assert_eq!(u16::from_le_bytes([body[14], body[15]]), 16, "16-bit");
                rate = Some(u32::from_le_bytes(body[4..8].try_into().unwrap()));
            }
            b"data" => {
                samples = Some(
                    body.as_chunks::<2>()
                        .0
                        .iter()
                        .map(|pair| i16::from_le_bytes(*pair))
                        .collect(),
                );
            }
            _ => {}
        }
        at += 8 + size + size % 2;
    }
    (rate.expect("a fmt chunk"), samples.expect("a data chunk"))
}

#[test]
fn the_bundled_sounds_mount_parse_and_the_hum_loops_cleanly() {
    let mut vfs = VirtualFileSystem::new();
    mount(&mut vfs).unwrap();
    let sounds = BladeSkin::Sun.sounds();
    let expect = [
        (sounds.on, 0.9..1.1),
        (sounds.off, 0.7..0.9),
        (sounds.hum, 1.9..2.1),
        (sounds.swings[0], 0.4..0.62),
        (sounds.swings[1], 0.4..0.62),
        (sounds.swings[2], 0.4..0.62),
    ];
    let mut total = 0;
    for (path, seconds) in expect {
        let asset = vfs.read(path).unwrap().expect(path);
        total += asset.bytes.len();
        let (rate, samples) = read_wav(&asset.bytes);
        assert_eq!(rate, 22_050, "{path}");
        let length = samples.len() as f32 / rate as f32;
        assert!(seconds.contains(&length), "{path}: {length} s");
        let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
        // Loud enough to hear, never clipped: -3 dBFS for the one-shots, lower for the hum.
        assert!((4_000..=23_300).contains(&peak), "{path}: peak {peak}");
        if path == sounds.hum {
            let step = samples
                .windows(2)
                .map(|pair| (i32::from(pair[1]) - i32::from(pair[0])).abs())
                .max()
                .unwrap();
            let seam = (i32::from(samples[0]) - i32::from(*samples.last().unwrap())).abs();
            assert!(
                seam <= step,
                "the loop point jumps {seam} (largest step {step})"
            );
        }
    }
    assert!(total < 400_000, "{total} bytes");
    // Every sound the skin names is bundled.
    let sets = sound_sets();
    assert_eq!(sets[0].on, sounds.on);
    assert_eq!(sets[0].swings, sounds.swings);
}
