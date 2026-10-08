//! Blade skins: unlockable looks that replace a saber blade's colour (not its hilt),
//! the first being the **Sun blade** (`saber_sun`, see `docs/unlockables.md`).
//!
//! A skin is drawn as its own saber material after the retail pairs and the neutral RGB
//! pair ([`BladeColor::Skin`]), with engine-generated textures ([`sun_glow`],
//! [`sun_core`]) animated by time in `saber.wgsl`. It has its own trail and light
//! colours and its own sounds, bundled ([`FILES`], made by
//! `scripts/saber_skin_sounds.py`) and mounted below all game data, so a PK3 with the
//! same paths replaces them.
//!
//! Who wears which skin is the per-client [`SaberSkins`] table on [`GpuState`]. Saber
//! submission (in the hand, thrown, first person) asks it for each entity's blades, and
//! the audio adapter gets its sound overrides from it every frame
//! ([`GpuState::sync_saber_skins`]). For now only the local player's entry is set, from
//! `cg_saberSkin` ([`GpuState::local_saber_skin`]); the hub's looks fill the others.

use crate::saber_rgb::BladeColor;
use crate::{GameAudio, GpuState};
use sjk_vfs::{VfsError, VirtualFileSystem};

/// Archived: the blade-skin unlock id the local player wears; empty or unknown for the
/// stock blade.
pub(crate) const CVAR: &str = "cg_saberSkin";
/// Players a game server can hold (`MAX_CLIENTS`); entity ids 1..=32 are their bodies.
pub(crate) const MAX_CLIENTS: usize = sjk_client::SABER_SOUND_CLIENTS;
/// How far a skin's glow capsule reaches past the stock one, for its flares and shimmer;
/// `SUN_REACH` in `saber.wgsl`.
pub(crate) const GLOW_REACH: f32 = 1.6;

/// A blade skin: a look a player's blades wear instead of their colour.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum BladeSkin {
    /// White-gold core, orange corona with drifting granulation, flares along the blade.
    Sun,
}

/// The game paths of a skin's replacement saber sounds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SoundPaths {
    pub(crate) on: &'static str,
    pub(crate) off: &'static str,
    pub(crate) hum: &'static str,
    pub(crate) swings: [&'static str; 3],
}

const SUN_SOUNDS: SoundPaths = SoundPaths {
    on: "sound/sjk/sabers/sun/on.wav",
    off: "sound/sjk/sabers/sun/off.wav",
    hum: "sound/sjk/sabers/sun/hum.wav",
    swings: [
        "sound/sjk/sabers/sun/swing1.wav",
        "sound/sjk/sabers/sun/swing2.wav",
        "sound/sjk/sabers/sun/swing3.wav",
    ],
};

/// The bundled sounds at their game paths.
const FILES: [(&str, &[u8]); 6] = [
    (SUN_SOUNDS.on, include_bytes!("../assets/sabers/sun/on.wav")),
    (
        SUN_SOUNDS.off,
        include_bytes!("../assets/sabers/sun/off.wav"),
    ),
    (
        SUN_SOUNDS.hum,
        include_bytes!("../assets/sabers/sun/hum.wav"),
    ),
    (
        SUN_SOUNDS.swings[0],
        include_bytes!("../assets/sabers/sun/swing1.wav"),
    ),
    (
        SUN_SOUNDS.swings[1],
        include_bytes!("../assets/sabers/sun/swing2.wav"),
    ),
    (
        SUN_SOUNDS.swings[2],
        include_bytes!("../assets/sabers/sun/swing3.wav"),
    ),
];

/// Mount the bundled sounds.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    vfs.mount_memory("SJK saber skins", FILES.iter().copied())?;
    Ok(())
}

impl BladeSkin {
    /// Every skin, in material and sound-set order.
    pub(crate) const ALL: [Self; 1] = [Self::Sun];

    /// Position in [`Self::ALL`]: material slot after the neutral pair, sound set.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Sun => 0,
        }
    }

    /// The unlock id the hub and `cg_saberSkin` name it by.
    pub(crate) const fn unlock_id(self) -> &'static str {
        match self {
            Self::Sun => "saber_sun",
        }
    }

    /// The skin an unlock id names; an id that is not a blade skin is `None`.
    pub(crate) fn from_unlock_id(id: &str) -> Option<Self> {
        let id = id.trim();
        Self::ALL
            .into_iter()
            .find(|skin| skin.unlock_id().eq_ignore_ascii_case(id))
    }

    /// Its replacement saber sounds.
    pub(crate) const fn sounds(self) -> &'static SoundPaths {
        match self {
            Self::Sun => &SUN_SOUNDS,
        }
    }

    /// Vertex RGB of its blur trail: amber gold.
    pub(crate) const fn trail_rgb(self) -> [f32; 3] {
        match self {
            Self::Sun => [1.0, 0.6, 0.12],
        }
    }

    /// Colour of its dynamic light (before `saber_lights`' gain): warm orange.
    pub(crate) const fn light_rgb(self) -> [f32; 3] {
        match self {
            Self::Sun => [1.0, 0.55, 0.15],
        }
    }

    /// Its light's brightness at `time_millis`: a slow, uneven flicker of about ±10 %
    /// so blades of one hilt (`seed`) do not pulse together with another's.
    pub(crate) fn light_flicker(self, time_millis: i64, seed: u64) -> f32 {
        match self {
            Self::Sun => {
                let t = (time_millis.rem_euclid(1_024_000)) as f32 * 0.001;
                let phase = (seed % 1_000) as f32 * 0.618;
                let wave = (t * 7.3 + phase).sin() * 0.6 + (t * 12.9 + phase * 2.3).sin() * 0.4;
                0.92 + 0.08 * wave
            }
        }
    }
}

/// Who wears which skin: one entry per client slot, and the local player's own choice
/// for the menus' preview (no client slot there).
#[derive(Clone, Debug, Default)]
pub(crate) struct SaberSkins {
    clients: [Option<BladeSkin>; MAX_CLIENTS],
    /// The local player's skin, also shown with no session (the Character preview).
    local: Option<BladeSkin>,
    /// The client slot `local` was last written to, so a new slot clears the old.
    local_client: Option<usize>,
    /// Blades the world shots add every frame, without a session (blade, colour, seed).
    #[cfg(test)]
    pub(crate) shot_blades: Vec<(crate::saber::Blade, BladeColor, u32)>,
    /// The time the world shots' blades are drawn at, instead of the frame's.
    #[cfg(test)]
    pub(crate) shot_seconds: Option<f64>,
}

impl SaberSkins {
    /// Set client slot `client`'s skin (`None` for the stock blade); out-of-range slots
    /// are ignored.
    pub(crate) fn set(&mut self, client: usize, skin: Option<BladeSkin>) {
        if let Some(slot) = self.clients.get_mut(client) {
            *slot = skin;
        }
    }

    /// The skin worn by the player whose body is entity `entity_id` (client + 1).
    pub(crate) fn get(&self, entity_id: u64) -> Option<BladeSkin> {
        let client = usize::try_from(entity_id.checked_sub(1)?).ok()?;
        self.clients.get(client).copied().flatten()
    }

    /// The colour entity `entity_id`'s blades are drawn with: its skin over `color`.
    pub(crate) fn blade_color(&self, entity_id: u64, color: BladeColor) -> BladeColor {
        self.get(entity_id).map_or(color, BladeColor::Skin)
    }

    /// The local player's skin.
    pub(crate) fn local(&self) -> Option<BladeSkin> {
        self.local
    }

    /// The local player wears `skin` and, in a session, sits in slot `client`.
    pub(crate) fn set_local(&mut self, client: Option<usize>, skin: Option<BladeSkin>) {
        if self.local_client != client
            && let Some(previous) = self.local_client
        {
            self.set(previous, None);
        }
        self.local = skin;
        self.local_client = client;
        if let Some(client) = client {
            self.set(client, skin);
        }
    }

    /// Each client slot's sound set ([`BladeSkin::index`]) for the audio adapter.
    pub(crate) fn sound_sets(&self) -> [Option<u8>; MAX_CLIENTS] {
        self.clients.map(|skin| skin.map(|skin| skin.index() as u8))
    }
}

/// The skins' sound paths in sound-set order, for the audio adapter.
pub(crate) fn sound_sets() -> [sjk_client::SaberSoundSet<'static>; BladeSkin::ALL.len()] {
    BladeSkin::ALL.map(|skin| {
        let sounds = skin.sounds();
        sjk_client::SaberSoundSet {
            on: sounds.on,
            off: sounds.off,
            hum: sounds.hum,
            swings: sounds.swings,
        }
    })
}

/// `cg_saberSkin`'s value as a skin; empty or unknown is the stock blade.
pub(crate) fn parse_cvar(value: Option<&sjk_shell::CvarValue>) -> Option<BladeSkin> {
    match value? {
        sjk_shell::CvarValue::Text(id) => BladeSkin::from_unlock_id(id),
        _ => None,
    }
}

impl GpuState {
    /// The skin the local player wears. **For now straight from `cg_saberSkin`**; the
    /// hub work replaces this with the choice gated by the player's own hub profile
    /// (only an unlock the profile lists may be worn).
    pub(crate) fn local_saber_skin(&self) -> Option<BladeSkin> {
        parse_cvar(self.console.as_ref().and_then(|console| console.cvar(CVAR)))
    }

    /// Once a frame, before the session's sabers are submitted: the local player's entry
    /// from [`Self::local_saber_skin`], and every client's sound override to the audio
    /// adapter. No allocation: a cvar lookup and two 32-entry copies.
    pub(crate) fn sync_saber_skins(
        &mut self,
        game_audio: &mut Option<GameAudio>,
        presentation_time: i64,
    ) {
        let client = self
            .live_session
            .as_ref()
            .map(sjk_client::ClientSession::latest_snapshot)
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(crate::demo_playback::Session::latest_snapshot)
            })
            .map(|snapshot| usize::from(snapshot.player.client_num()));
        let skin = self.local_saber_skin();
        self.saber_skins.set_local(client, skin);
        if let Some(audio) = game_audio {
            audio.set_saber_sound_overrides(&self.saber_skins.sound_sets());
        }
        #[cfg(test)]
        self.submit_shot_blades(presentation_time);
        #[cfg(not(test))]
        let _ = presentation_time;
    }

    /// The world shots' blades, lit, at their chosen time.
    #[cfg(test)]
    fn submit_shot_blades(&mut self, presentation_time: i64) {
        let millis = self
            .saber_skins
            .shot_seconds
            .map_or(presentation_time, |seconds| (seconds * 1_000.0) as i64);
        for (blade, color, seed) in self.saber_skins.shot_blades.clone() {
            self.saber_instances.extend(
                crate::saber::Instance::pair(blade, color)
                    .map(|i| i.with_animation(millis as f64 * 0.001, seed)),
            );
            let mut lit = [None; 8];
            lit[0] = Some(blade);
            crate::saber_submission::lights::append(
                &mut self.dynamic_lights,
                &lit,
                color,
                1,
                false,
                millis,
                u64::from(seed),
            );
        }
    }
}

/// Side length of the generated glow image (as the neutral pair's).
const GLOW_SIZE: u32 = 128;
/// Size of the generated core image (as the neutral pair's).
const CORE_WIDTH: u32 = 64;
const CORE_HEIGHT: u32 = 256;

/// The Sun's glow: the neutral pair's Gaussian with a longer, fainter corona tail, so
/// the blade sits in a wider haze. Grey; `saber.wgsl` colours it.
pub(crate) fn sun_glow() -> Vec<u8> {
    let half = GLOW_SIZE as f32 / 2.0;
    let mut pixels = Vec::with_capacity((GLOW_SIZE * GLOW_SIZE * 4) as usize);
    for y in 0..GLOW_SIZE {
        for x in 0..GLOW_SIZE {
            let dx = (x as f32 + 0.5 - half) / half;
            let dy = (y as f32 + 0.5 - half) / half;
            let distance = (dx * dx + dy * dy).sqrt();
            let falloff =
                (-(distance / 0.46).powi(2)).exp() * 0.48 + (-(distance / 0.8).powi(2)).exp() * 0.1;
            let edge = (1.0 - distance).clamp(0.0, 0.1) * 10.0;
            let value = (falloff * edge * 255.0).round() as u8;
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }
    pixels
}

/// The Sun's core: red the white-hot centre (narrower than the neutral pair's), green
/// the broad gold fringe, both rounded off at the ends like the neutral core.
pub(crate) fn sun_core() -> Vec<u8> {
    const END_ROWS: f32 = 16.0;
    let half = CORE_WIDTH as f32 / 2.0;
    let mut pixels = Vec::with_capacity((CORE_WIDTH * CORE_HEIGHT * 4) as usize);
    for y in 0..CORE_HEIGHT {
        let from_end = (y as f32 + 0.5).min(CORE_HEIGHT as f32 - y as f32 - 0.5);
        let dy = (1.0 - from_end / END_ROWS).max(0.0);
        for x in 0..CORE_WIDTH {
            let dx = (x as f32 + 0.5 - half) / half;
            let distance = (dx * dx + dy * dy).sqrt();
            let core = (-(distance / 0.36).powi(2)).exp();
            let fringe = (-(distance / 0.75).powi(2)).exp();
            let channel = |value: f32| (value * 255.0).round() as u8;
            pixels.extend_from_slice(&[channel(core), channel(fringe), 0, 255]);
        }
    }
    pixels
}

/// Upload every skin's glow/core pair, in [`BladeSkin::ALL`] order.
pub(crate) fn create_materials(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    layout: &wgpu::BindGroupLayout,
    samplers: &crate::saber::Samplers,
) -> impl Iterator<Item = wgpu::BindGroup> {
    BladeSkin::ALL.into_iter().map(move |skin| match skin {
        BladeSkin::Sun => {
            let glow = image::RgbaImage::from_raw(GLOW_SIZE, GLOW_SIZE, sun_glow())
                .expect("the Sun glow has GLOW_SIZE² RGBA texels");
            let core = image::RgbaImage::from_raw(CORE_WIDTH, CORE_HEIGHT, sun_core())
                .expect("the Sun core has CORE_WIDTH×CORE_HEIGHT RGBA texels");
            crate::saber::material(device, queue, layout, samplers, &glow, &core)
        }
    })
}

#[cfg(test)]
#[path = "saber_skins_tests.rs"]
mod tests;
