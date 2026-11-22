//! The feats' announcer voices: packs of one sound per feat.
//!
//! A pack is a folder `sound/sjk/announcer/<pack>/` holding `<feat id>.wav` (or `.mp3`,
//! `.ogg`) for each line it has ([`super::Feat::id`]). SJK's own pack, [`DEFAULT_PACK`],
//! is bundled in memory below all game data, so a PK3 adds a pack with a folder of its
//! own, or replaces SJK's lines one by one with the same paths. `cg_announcer` names
//! the pack; a line the chosen pack lacks is taken from [`DEFAULT_PACK`], and one
//! neither has stays silent. The packs are listed once, from the startup file system
//! ([`discover`]), for the settings' list.

use super::Feat;
use sjk_vfs::VirtualFileSystem;
use std::sync::Mutex;

/// Where the packs' folders are.
pub(crate) const ROOT: &str = "sound/sjk/announcer";
/// SJK's own pack, and the fallback for lines a pack lacks.
pub(crate) const DEFAULT_PACK: &str = "sjk";
/// The cvar naming the pack.
pub(crate) const CVAR: &str = "cg_announcer";
/// `CHAN_ANNOUNCER`, as the siege announcer plays on.
const CHANNEL: sjk_audio::ChannelId = sjk_audio::ChannelId(7);
/// The announcer's own source, apart from every entity's.
const SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(0x5300_0001);

/// A bundled line at its game path, by its feat id.
macro_rules! line {
    ($id:literal) => {
        (
            concat!("sound/sjk/announcer/sjk/", $id, ".wav"),
            include_bytes!(concat!("../../assets/announcer/sjk/", $id, ".wav")).as_slice(),
        )
    };
}

/// The bundled pack's lines, one per [`Feat`] in its order
/// (`assets/announcer/README.md`).
const FILES: [(&str, &[u8]); 14] = [
    line!("hooked"),
    line!("first_blood"),
    line!("double_kill"),
    line!("triple_kill"),
    line!("quad_kill"),
    line!("penta_kill"),
    line!("massacre"),
    line!("killing_spree"),
    line!("rampage"),
    line!("dominating"),
    line!("unstoppable"),
    line!("godlike"),
    line!("spree_ended"),
    line!("revenge"),
];

/// Mount the bundled pack.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK announcer", FILES.iter().copied())?;
    Ok(())
}

static PACKS: Mutex<Option<&'static [&'static str]>> = Mutex::new(None);

/// List the installed packs for [`choices`]: [`DEFAULT_PACK`] first, then the others by
/// name.
pub(crate) fn discover(vfs: &VirtualFileSystem) {
    let found = packs(vfs.list_files(ROOT, "").iter().map(String::as_str));
    let found: Vec<&'static str> = found
        .into_iter()
        .map(|pack| &*Box::leak(pack.into_boxed_str()))
        .collect();
    *PACKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
        Some(Box::leak(found.into_boxed_slice()));
}

/// The pack names among `files` (`<pack>/<file>` under [`ROOT`]), [`DEFAULT_PACK`] first.
fn packs<'a>(files: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut packs: Vec<String> = files
        .filter_map(|file| file.split_once('/'))
        .map(|(pack, _)| pack.to_ascii_lowercase())
        .filter(|pack| !pack.is_empty() && pack != DEFAULT_PACK)
        .collect();
    packs.sort();
    packs.dedup();
    packs.insert(0, DEFAULT_PACK.to_owned());
    packs
}

/// The packs `cg_announcer` may name (the settings' list).
pub(crate) fn choices() -> &'static [&'static str] {
    PACKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .unwrap_or(&[DEFAULT_PACK])
}

/// The game path of `feat`'s line in `pack`, without its extension.
fn line(pack: &str, feat: Feat) -> String {
    format!("{ROOT}/{pack}/{}", feat.id())
}

/// Say `feat` in `pack`'s voice, or [`DEFAULT_PACK`]'s when `pack` has no such line.
pub(crate) fn say(
    audio: &mut crate::audio::GameAudio,
    vfs: &VirtualFileSystem,
    pack: &str,
    feat: Feat,
) {
    let pack = if pack.is_empty() { DEFAULT_PACK } else { pack };
    let mut path = line(pack, feat);
    if !audio.register_effect(vfs, &path) {
        if pack.eq_ignore_ascii_case(DEFAULT_PACK) {
            return;
        }
        path = line(DEFAULT_PACK, feat);
        if !audio.register_effect(vfs, &path) {
            return;
        }
    }
    audio.play_local(&path, 1.0, SOURCE, CHANNEL);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_are_the_folders_with_sjk_first() {
        let files = [
            "zz/double_kill.wav",
            "sjk/rampage.wav",
            "Arena/hooked.ogg",
            "arena/godlike.wav",
            "loose.wav",
        ];
        assert_eq!(packs(files.into_iter()), ["sjk", "arena", "zz"]);
        assert_eq!(packs(std::iter::empty()), ["sjk"]);
    }

    #[test]
    fn sjk_has_a_line_for_every_feat() {
        for (feat, (path, bytes)) in Feat::ALL.iter().zip(FILES) {
            assert_eq!(path, format!("{}.wav", line(DEFAULT_PACK, *feat)));
            assert!(bytes.starts_with(b"RIFF"), "{path}");
        }
    }

    #[test]
    fn a_line_is_named_by_its_pack_and_feat() {
        assert_eq!(
            line("arena", Feat::Hooked),
            "sound/sjk/announcer/arena/hooked"
        );
    }
}
