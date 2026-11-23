//! The WHEEL OF DOOM's outcomes and what they do to a player (Sol's design of
//! 07/11/2026; `docs/wheel-of-doom.md`). Until 20/11/2026 wheels stood on maps and were
//! spun for a DOOM coin; since then the coin rolls at a Force box
//! (`mystery_box.rs`, `docs/mystery-boxes.md`), for the same outcomes, and the wheels,
//! their spin, sounds and words left the client on 23/11/2026. What stays here: the
//! outcome table the hub's ids name ([`OUTCOMES`]), the [`effects`] on the roller's own
//! input and view with their HUD line, the coin count `toy_wheelofdoom` tells, the Quack
//! effect's quack, and the creator's command that removes a wheel the hub still keeps.

mod draw;
mod effects;
mod mirror;

pub(crate) use effects::{Effect, Effects, Saber};

use crate::GpuState;
use glam::{Quat, Vec3};
use std::time::{Duration, Instant};

/// The toy's command: it tells the player's DOOM coins and where to roll one (a Force
/// box, since 20/11/2026).
pub(crate) const COMMAND: &str = "toy_wheelofdoom";
/// Its line in `cmdlist`.
pub(crate) const HELP: &str = "Toy: your DOOM coins. Hold Use at a Force box to roll one for the WHEEL OF DOOM's outcomes (the teddy bear in a Force box brings a coin)";

/// The toy's icon (key bindings; Collection > Toys draws the quick wheel atlas's copy).
pub(crate) const ICON: &str = "gfx/sjk/toy_wheelofdoom.png";
/// The outcomes (20 since 11/11/2026: the holocron of every tier and the coin back
/// joined the 16), once the wheel's segments.
pub(crate) const SEGMENTS: usize = 20;
/// The Quack effect's quack (bundled, `scripts/wheel_of_doom_assets.py`).
const QUACK: &str = "sound/sjk/wheel_of_doom/quack.wav";
/// The player's own quack's sound source, past the other toys' (`0x5000_0000` to
/// `0x5500_0000`).
const QUACK_SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(0x5600_FFFF);

/// A bundled file's bytes, by its name in `assets/wheel_of_doom`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/wheel_of_doom/", $name))
    };
}

/// The bundled files at their game paths.
const FILES: [(&str, &[u8]); 2] = [
    (ICON, bundled!("toy_wheelofdoom.png")),
    (QUACK, bundled!("quack.wav")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK wheel of doom", FILES.iter().copied())?;
    Ok(())
}

/// The creator's console command, with its help: the hub still keeps the wheels placed
/// before 20/11/2026 for older clients.
pub(crate) const COMMANDS: &[(&str, &str)] = &[(
    "wheelofdoom_remove",
    "SJK creator: remove the WHEEL OF DOOM with the id given from the hub",
)];

/// Whether `name` is one of [`COMMANDS`].
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|(command, _)| *command == name)
}

/// What a segment does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A holocron of this tier (`mythical`, the gold jackpot, `legendary`, `rare`,
    /// `uncommon`).
    Holocron(&'static str),
    /// The DOOM coin back: spin again.
    CoinBack,
    Nothing,
    Effect(Effect),
    /// The game's `kill`, as a mystery box that blows up.
    Explode,
    /// Another bad effect, later.
    Curse,
}

/// One outcome: its hub id, what it does, its name and what it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) id: &'static str,
    pub(crate) kind: Kind,
    pub(crate) title: &'static str,
    pub(crate) line: &'static str,
}

const fn outcome(id: &'static str, kind: Kind, title: &'static str, line: &'static str) -> Outcome {
    Outcome {
        id,
        kind,
        title,
        line,
    }
}

const fn effect(
    id: &'static str,
    effect: Effect,
    title: &'static str,
    line: &'static str,
) -> Outcome {
    outcome(id, Kind::Effect(effect), title, line)
}

/// The outcomes in the hub's order, as its contract of 11/11/2026 lists them (the gold
/// Mythical jackpot first; the holocrons spread among the bad ones).
pub(crate) const OUTCOMES: [Outcome; SEGMENTS] = [
    outcome(
        "mythical",
        Kind::Holocron("mythical"),
        "MYTHICAL HOLOCRON!",
        "The jackpot",
    ),
    effect(
        "reverse_keys",
        Effect::ReverseKeys,
        "REVERSE KEYS",
        "Forward is back for 30 s",
    ),
    outcome(
        "holo_uncommon",
        Kind::Holocron("uncommon"),
        "UNCOMMON HOLOCRON",
        "In your Collection",
    ),
    effect(
        "invert_mouse",
        Effect::InvertMouse,
        "INVERTED MOUSE",
        "The mouse turns the wrong way for 30 s",
    ),
    outcome("explode", Kind::Explode, "BOOM!", "It blows up"),
    outcome(
        "holo_rare",
        Kind::Holocron("rare"),
        "RARE HOLOCRON",
        "In your Collection",
    ),
    effect(
        "mirror",
        Effect::Mirror,
        "MIRROR",
        "The world is mirrored for 30 s",
    ),
    effect(
        "upside_down",
        Effect::UpsideDown,
        "UPSIDE DOWN",
        "The world turns over for 20 s",
    ),
    outcome(
        "doom_coin",
        Kind::CoinBack,
        "DOOM COIN BACK",
        "Spin it again",
    ),
    effect("blind", Effect::Blind, "BLIND", "Darkness for 15 s"),
    outcome(
        "holo_legendary",
        Kind::Holocron("legendary"),
        "LEGENDARY HOLOCRON!",
        "In your Collection",
    ),
    effect("drunk", Effect::Drunk, "DRUNK", "The room sways for 30 s"),
    effect(
        "tiny_fov",
        Effect::TinyFov,
        "TUNNEL VISION",
        "A narrow view for 20 s",
    ),
    outcome("nothing", Kind::Nothing, "NOTHING", "Nothing happens..."),
    effect(
        "huge_fov",
        Effect::HugeFov,
        "FISHEYE",
        "A huge view for 20 s",
    ),
    effect(
        "no_saber",
        Effect::NoSaber,
        "NO SABER",
        "Your saber stays off for 20 s",
    ),
    effect(
        "quack",
        Effect::Quack,
        "QUACK",
        "Every swing quacks for 30 s",
    ),
    effect("walk", Effect::Walk, "WALK", "No running for 30 s"),
    effect(
        "bunny",
        Effect::Bunny,
        "BUNNY",
        "You cannot stop jumping for 20 s",
    ),
    outcome(
        "curse",
        Kind::Curse,
        "CURSED...",
        "Something bad comes within a minute",
    ),
];

/// The outcome `id` names, with its place in the built-in order.
pub(crate) fn known(id: &str) -> Option<(usize, &'static Outcome)> {
    OUTCOMES
        .iter()
        .enumerate()
        .find(|(_, outcome)| outcome.id == id)
}

/// A number to vary a curse by.
fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos())
}

/// The toy's state: the roller's effects and what draws them.
#[derive(Default)]
pub(crate) struct WheelOfDoom {
    /// The roller's own effect and curse.
    pub(crate) effects: Effects,
    /// When the coins were last asked of the hub.
    read_at: Option<Instant>,
    /// The server and map the effects began on: another ends them.
    place: Option<(std::net::SocketAddr, String)>,
    sounds_registered: Option<u64>,
    pub(crate) draw: draw::Draw,
    mirror: mirror::Mirror,
}

impl WheelOfDoom {
    /// An effect or a curse runs: the Toys tab's switch shows on.
    pub(crate) fn running(&self) -> bool {
        self.effects.any()
    }

    /// The player's own view from `eye` at `target` with `up`, turned at `now` by the
    /// effect running (Upside down's roll, Drunk's sway): the new target and up. The aim
    /// is not turned, only what is seen.
    pub(crate) fn turn_view(
        &self,
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        now: Instant,
    ) -> (Vec3, Vec3) {
        let Some([roll, pitch, yaw]) = self.effects.view_turn(now) else {
            return (target, up);
        };
        let forward = (target - eye).normalize_or_zero();
        let right = forward.cross(up).normalize_or_zero();
        if forward == Vec3::ZERO || right == Vec3::ZERO {
            return (target, up);
        }
        let sway = Quat::from_axis_angle(up.normalize_or_zero(), yaw.to_radians())
            * Quat::from_axis_angle(right, pitch.to_radians());
        let forward = sway * forward;
        let up = Quat::from_axis_angle(forward, roll.to_radians()) * (sway * up);
        (eye + forward * target.distance(eye), up)
    }
}

impl GpuState {
    fn wheel_of_doom_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    /// The player's DOOM coins, when the hub said.
    pub(crate) fn doom_coins(&self) -> Option<u64> {
        crate::player_identity::with_snapshot(|snapshot| {
            snapshot.wheel_of_doom.as_ref().map(|wheel| wheel.coins)
        })
        .flatten()
    }

    /// `toy_wheelofdoom`: the player's DOOM coins and where to spend them, in the centre
    /// of the screen.
    pub(crate) fn wheel_of_doom_command(&mut self) {
        let now = Instant::now();
        self.read_wheel_of_doom(now);
        let coins = match self.doom_coins() {
            Some(1) => "You hold 1 DOOM coin".to_owned(),
            Some(coins) => format!("You hold {coins} DOOM coins"),
            None => "Asking the hub for your DOOM coins".to_owned(),
        };
        self.wheel_of_doom_centre(format!(
            "{coins}. Hold Use at a Force box to roll one. The teddy bear in a Force box brings a coin"
        ));
    }

    /// Ask the hub for the coins, at most every 5 seconds.
    fn read_wheel_of_doom(&mut self, now: Instant) {
        if self
            .wheel_of_doom
            .read_at
            .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(5))
        {
            return;
        }
        self.wheel_of_doom.read_at = Some(now);
        let _ = crate::player_identity::with_service(|service| service.read_wheel_of_doom());
    }

    /// The frame's WHEEL OF DOOM logic, after the looks: the effects, which end on a new
    /// map or server, a disconnect or a death, and the curse's strike.
    pub(crate) fn update_wheel_of_doom(&mut self) {
        let now = Instant::now();
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.wheel_of_doom.effects.clear();
            self.wheel_of_doom.place = None;
            return;
        };
        let server = session.server();
        let alive = session.latest_snapshot().player.health() > 0;
        let map_name = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let moved = self
            .wheel_of_doom
            .place
            .as_ref()
            .is_none_or(|(at_server, at_map)| *at_server != server || *at_map != map_name);
        if moved || !alive {
            self.wheel_of_doom.effects.clear();
        }
        if moved {
            self.wheel_of_doom.place = Some((server, map_name));
        }
        if let Some(effect) = self.wheel_of_doom.effects.tick(now) {
            self.wheel_of_doom_centre(format!("The curse strikes: {}!", effect.hud()));
        }
        self.release_wheel_of_doom_mirror();
    }

    /// The player's own Force box coin roll gave `kind`: its effect starts and
    /// the centre of the screen says it (`title`, `line`).
    fn wheel_of_doom_outcome(&mut self, kind: Option<Kind>, title: &str, line: &str, now: Instant) {
        let text = match kind {
            Some(Kind::Effect(effect)) => {
                self.wheel_of_doom.effects.start(effect, now);
                format!("{title} {line}")
            }
            Some(Kind::Curse) => {
                self.wheel_of_doom.effects.curse(now, seed());
                format!("{title} {line}")
            }
            Some(Kind::Holocron(_)) => format!("{title} In your Collection"),
            Some(Kind::CoinBack) => format!("{title}! {line}"),
            Some(Kind::Explode) => title.to_owned(),
            Some(Kind::Nothing) => line.to_owned(),
            None => format!("{title}: {line}"),
        };
        self.wheel_of_doom_centre(text);
    }

    /// The player's own Force box coin roll stopped on WHEEL OF DOOM outcome `id` (the
    /// box blows up by itself, as for its credits roll).
    pub(crate) fn wheel_of_doom_strike(&mut self, id: &str, now: Instant) {
        match known(id) {
            Some((_, known)) => {
                self.wheel_of_doom_outcome(Some(known.kind), known.title, known.line, now);
            }
            None => self.wheel_of_doom_outcome(None, id, "a newer hub's outcome", now),
        }
    }

    /// `wheelofdoom_remove <id>`: the hub takes a wheel it still keeps away.
    pub(crate) fn wheel_of_doom_creator_command(
        &mut self,
        name: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        if name != "wheelofdoom_remove" {
            return Err(format!("Unknown command {name}"));
        }
        if !crate::player_identity::is_creator() {
            return Err("Only SJK's creator removes wheels of doom".into());
        }
        let id: i64 = match args.first() {
            Some(id) => id.parse().map_err(|_| format!("Not a wheel id: {id}"))?,
            None => {
                return Err("Usage: wheelofdoom_remove <id> (the hub's panel lists them)".into());
            }
        };
        crate::player_identity::with_service(|service| service.remove_wheel_of_doom(id))
            .ok_or("Not connected to the SJK hub")?;
        Ok(vec![format!("Removing WHEEL OF DOOM {id}...")])
    }

    /// Play the Quack effect's quack; the sound is handed to the audio the first time.
    pub(crate) fn play_wheel_of_doom_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let quack = self.wheel_of_doom.effects.take_quack();
        let Some(audio) = audio.as_mut().filter(|_| quack) else {
            return;
        };
        if self.wheel_of_doom.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [QUACK]);
            self.wheel_of_doom.sounds_registered = Some(audio.generation());
        }
        audio.play_toy(
            crate::toy_sounds::Toy::WheelOfDoom,
            QUACK,
            1.0,
            None,
            QUACK_SOURCE,
            sjk_audio::ChannelId(1),
        );
    }

    /// The saber as the player state says, for No saber.
    pub(crate) fn wheel_of_doom_saber(player: &sjk_protocol::PlayerState) -> Saber {
        Saber {
            weapon: player.weapon(),
            holstered: player.saber_holstered(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outcomes_are_the_hubs_twenty_in_order() {
        let ids: Vec<_> = OUTCOMES.iter().map(|outcome| outcome.id).collect();
        assert_eq!(
            ids,
            [
                "mythical",
                "reverse_keys",
                "holo_uncommon",
                "invert_mouse",
                "explode",
                "holo_rare",
                "mirror",
                "upside_down",
                "doom_coin",
                "blind",
                "holo_legendary",
                "drunk",
                "tiny_fov",
                "nothing",
                "huge_fov",
                "no_saber",
                "quack",
                "walk",
                "bunny",
                "curse"
            ]
        );
        // Every effect has its segment, and a holocron of every tier stands on it.
        for effect in Effect::ALL {
            assert!(
                OUTCOMES
                    .iter()
                    .any(|outcome| outcome.kind == Kind::Effect(effect))
            );
        }
        for tier in ["uncommon", "rare", "legendary", "mythical"] {
            assert!(
                OUTCOMES
                    .iter()
                    .any(|outcome| outcome.kind == Kind::Holocron(tier)),
                "{tier}"
            );
            assert!(crate::holocrons::by_id(tier).is_some(), "{tier}");
        }
    }

    #[test]
    fn a_new_map_or_a_death_ends_the_effects() {
        let mut toy = WheelOfDoom::default();
        let now = Instant::now();
        assert!(!toy.running());
        toy.effects.start(Effect::Walk, now);
        assert!(toy.running());
        toy.effects.clear();
        assert!(!toy.running());
    }
}
