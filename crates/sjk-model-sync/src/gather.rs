//! Collects what a shared model needs from wherever the game has it, so a viewer sees
//! and hears it as its sender does (docs/model-sync.md, "What a share takes"):
//!
//! - the model's folder, with its voice tables (`sounds.cfg`, `sounds_<skin>.cfg`)
//!   and `animevents.cfg`;
//! - its shaders, from any `.shader` file (community packs keep many models' shaders
//!   in one), kept in `shaders/<name>.shader`;
//! - its voice: `sound/chars/<name>/`, or a voice folder of another name copied there
//!   with its tables pointed at the copy;
//! - textures, shaders and sounds it uses from outside its folder, copied under it
//!   (`models/players/<name>/x/...`, `sound/chars/<name>/x/...`) with every reference
//!   rewritten.
//!
//! What the retail game has is never taken or moved: every player has it. The result
//! goes to [`crate::pack::pack`], which checks it.

use crate::{
    is_model_table, is_rig_manifest, is_sounds_table, model_folder, shader, shader_file, sound_dir,
    sound_folder,
};
use std::collections::{BTreeMap, BTreeSet};

/// Where the game's files are read from.
pub trait Source {
    /// The full paths, lower case, of the files of `dir` (no trailing `/`) and of the
    /// folders one below it, ending in `extension` (`.tga`); the highest-priority
    /// source's first.
    fn list(&self, dir: &str, extension: &str) -> Vec<String>;
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn has(&self, path: &str) -> bool {
        self.read(path).is_some()
    }
    /// Whether the retail game has `path`, which every player has.
    fn stock(&self, path: &str) -> bool;
}

/// What [`gather`] found.
#[derive(Debug, Default)]
pub struct Gathered {
    /// Game paths with their bytes, for [`crate::pack::pack`].
    pub files: Vec<(String, Vec<u8>)>,
    /// How many sound files the model's voice and animation events brought.
    pub sounds: usize,
    /// The voice it uses when its own is not shared: the game's (`jan`) or one this
    /// game lacks too.
    pub other_voice: Option<String>,
    /// How many files came from outside the model's folder.
    pub pulled_in: usize,
    /// Textures or shaders its skins name that this game has nowhere.
    pub missing: Vec<String>,
}

const IMAGE_EXTENSIONS: [&str; 4] = ["tga", "jpg", "jpeg", "png"];

/// Collects model `name`'s files from `source`.
pub fn gather(name: &str, source: &impl Source) -> Gathered {
    let name = name.to_ascii_lowercase();
    let folder = model_folder(&name);
    let mut out = Gathered::default();
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for extension in [
        ".glm", ".skin", ".tga", ".jpg", ".jpeg", ".png", ".gltf", ".glb", ".bin", ".cfg",
    ] {
        for path in source.list(folder.trim_end_matches('/'), extension) {
            let Some(file) = path.strip_prefix(&folder) else {
                continue;
            };
            if extension == ".cfg" && !(is_rig_manifest(file) || is_model_table(file)) {
                continue;
            }
            if let Some(mut bytes) = source.read(&path) {
                // Tables are read without case: lower case keeps the references below
                // comparable.
                if matches!(extension, ".skin" | ".cfg") {
                    bytes.make_ascii_lowercase();
                }
                files.insert(path, bytes);
            }
        }
    }
    let mut mover = Mover {
        source,
        folder: folder.clone(),
        pulled_in: 0,
    };
    let shaders = gather_shaders(&name, &mut files, &mut mover, &mut out.missing);
    if !shaders.is_empty() {
        files.insert(shader_file(&name), shaders.into_bytes());
    }
    gather_sounds(&name, &mut files, source, &mut out);
    out.pulled_in = mover.pulled_in;
    out.files = files.into_iter().collect();
    out
}

/// Copies files from outside the model's folder under it.
struct Mover<'a, S: Source> {
    source: &'a S,
    folder: String,
    pulled_in: usize,
}

impl<S: Source> Mover<'_, S> {
    /// Where `path` (outside the folder) goes under it; `None` when that is too long.
    fn place(&self, path: &str) -> Option<String> {
        let rest = path.strip_prefix("models/players/").unwrap_or(path);
        let moved = format!("{}x/{rest}", self.folder);
        (moved.len() <= 150).then_some(moved)
    }

    /// The image `reference` names (as written, or with another image extension),
    /// when the game has it.
    fn image(&self, reference: &str) -> Option<String> {
        let stem = strip_image_extension(reference);
        std::iter::once(reference.to_owned())
            .filter(|_| stem.len() != reference.len())
            .chain(IMAGE_EXTENSIONS.iter().map(|ext| format!("{stem}.{ext}")))
            .find(|path| self.source.has(path))
    }

    /// The image `reference` names, taken into `files`: one in the folder as it is, a
    /// custom one outside it copied under it. The reference to write in its place
    /// when it moved.
    fn take_image(
        &mut self,
        reference: &str,
        files: &mut BTreeMap<String, Vec<u8>>,
    ) -> Option<String> {
        let found = self.image(reference)?;
        if found.starts_with(&self.folder) {
            if !files.contains_key(&found)
                && let Some(bytes) = self.source.read(&found)
            {
                files.insert(found, bytes);
            }
            return None;
        }
        if self.source.stock(&found) {
            return None;
        }
        let moved = self.place(&found)?;
        if !files.contains_key(&moved) {
            let bytes = self.source.read(&found)?;
            files.insert(moved.clone(), bytes);
            self.pulled_in += 1;
        }
        Some(moved)
    }
}

/// The model's shaders as one file's text, from wherever they are defined: its own
/// (named in its folder), and custom ones its skins or mesh name outside it, renamed
/// under it. The images they read come along (see [`Mover`]); the skins are rewritten
/// for what moved.
fn gather_shaders<S: Source>(
    name: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
    mover: &mut Mover<'_, S>,
    missing: &mut Vec<String>,
) -> String {
    let folder = model_folder(name);
    // Every shader the game has, by name without an image extension: the
    // highest-priority definition, with the file it is in.
    let mut index: BTreeMap<String, (String, String)> = BTreeMap::new();
    for file in mover.source.list("shaders", ".shader") {
        let Some(bytes) = mover.source.read(&file) else {
            continue;
        };
        let Ok(blocks) = shader::blocks(&String::from_utf8_lossy(&bytes)) else {
            continue;
        };
        for (shader, text) in blocks {
            index
                .entry(strip_image_extension(&shader).to_owned())
                .or_insert((file.clone(), text));
        }
    }
    let mut kept: BTreeMap<String, String> = index
        .iter()
        .filter(|(shader, _)| shader.starts_with(&folder))
        .map(|(shader, (_, text))| (shader.clone(), text.clone()))
        .collect();
    // What the skins and the mesh name.
    let mut named: BTreeSet<String> = BTreeSet::new();
    for (path, bytes) in files.iter() {
        if path.ends_with(".skin") {
            named.extend(skin_references(&String::from_utf8_lossy(bytes)));
        } else if path.ends_with(".glm")
            && let Ok(glm) = sjk_model::Glm::parse(bytes)
        {
            named.extend(
                glm.hierarchy
                    .iter()
                    .map(|surface| surface.shader.trim().to_ascii_lowercase())
                    .filter(|shader| shader.contains('/')),
            );
        }
    }
    let mut swaps: BTreeMap<String, String> = BTreeMap::new();
    for reference in named {
        let stem = strip_image_extension(&reference).to_owned();
        if stem.starts_with(&folder) {
            if !kept.contains_key(&stem) && mover.image(&reference).is_none() {
                missing.push(reference.clone());
            }
            mover.take_image(&reference, files);
            continue;
        }
        if let Some((file, text)) = index.get(&stem) {
            if mover.source.stock(file) {
                continue;
            }
            if let Some(moved) = mover.place(&stem) {
                // The block starts with its name: give it the new one.
                let rest = &text[text.find(char::is_whitespace).unwrap_or(text.len())..];
                kept.insert(moved.clone(), format!("{moved}{rest}"));
                swaps.insert(stem, moved);
            }
        } else if let Some(moved) = mover.take_image(&reference, files) {
            swaps.insert(stem, moved);
        } else if mover.image(&reference).is_none() {
            missing.push(reference);
        }
    }
    // The images the kept shaders read: those outside the folder come under it.
    for text in kept.values_mut() {
        let mut moved_images = BTreeMap::new();
        for image in shader::images(text) {
            if let Some(moved) = mover.take_image(&image, files) {
                moved_images.insert(image, moved);
            }
        }
        if !moved_images.is_empty() {
            *text = shader::rewrite(text, |token| {
                moved_images.get(&token.to_ascii_lowercase()).cloned()
            });
        }
    }
    if !swaps.is_empty() {
        for (path, bytes) in files.iter_mut() {
            if path.ends_with(".skin") {
                let text = String::from_utf8_lossy(bytes);
                *bytes = rewrite_skin(&text, &swaps).into_bytes();
            }
        }
    }
    kept.into_values().collect::<Vec<_>>().join("\n")
}

/// The voice the model's tables name and the sounds its `animevents.cfg` plays, under
/// `sound/chars/<name>/`; the tables and events rewritten for what moved.
fn gather_sounds<S: Source>(
    name: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
    source: &S,
    out: &mut Gathered,
) {
    let folder = model_folder(name);
    let own = sound_folder(name);
    let mut sounds = 0usize;
    let clips = |dir: &str| -> Vec<String> {
        [".wav", ".mp3"]
            .iter()
            .flat_map(|extension| source.list(dir.trim_end_matches('/'), extension))
            .collect()
    };
    let mut take = |files: &mut BTreeMap<String, Vec<u8>>, from: &str, to: String| {
        if files.contains_key(&to) {
            return;
        }
        if let Some(bytes) = source.read(from) {
            files.insert(to, bytes);
            sounds += 1;
        }
    };
    // Voice folders moved under the model's: old `sound/chars/<dir>/` to new.
    let mut moved: BTreeMap<String, String> = BTreeMap::new();
    let tables: Vec<String> = files
        .keys()
        .filter(|path| path.strip_prefix(&folder).is_some_and(is_sounds_table))
        .cloned()
        .collect();
    for table in tables {
        let text = String::from_utf8_lossy(&files[&table]).into_owned();
        let Some(dir) = sound_dir(&text).map(str::to_owned) else {
            continue;
        };
        let from = format!("sound/chars/{dir}/");
        let found = clips(&from);
        if from.starts_with(&own) {
            for clip in found.iter().filter(|clip| !source.stock(clip)) {
                take(files, clip, clip.clone());
            }
            continue;
        }
        // The game's own voice (or one partly the game's): every player has it.
        if found.is_empty() || found.iter().any(|clip| source.stock(clip)) {
            out.other_voice.get_or_insert(dir);
            continue;
        }
        let skin = table[folder.len()..]
            .strip_prefix("sounds_")
            .and_then(|rest| rest.strip_suffix(".cfg"));
        let target = moved
            .entry(from.clone())
            .or_insert_with(|| match skin {
                Some(skin) => format!("{own}{skin}/"),
                None => own.clone(),
            })
            .clone();
        for clip in &found {
            take(files, clip, format!("{target}{}", &clip[from.len()..]));
        }
        let dir_line = target["sound/chars/".len()..]
            .trim_end_matches('/')
            .to_owned();
        let mut replaced = false;
        let lines: Vec<&str> = text
            .lines()
            .map(|line| {
                if !replaced && sound_dir(line).is_some() {
                    replaced = true;
                    dir_line.as_str()
                } else {
                    line
                }
            })
            .collect();
        files.insert(table, lines.join("\n").into_bytes());
    }
    let events = format!("{folder}animevents.cfg");
    let Some(bytes) = files.get(&events) else {
        out.sounds += sounds;
        return;
    };
    let text = String::from_utf8_lossy(bytes).into_owned();
    let mut swaps: BTreeMap<String, String> = BTreeMap::new();
    for word in text.split_whitespace() {
        let word = word.trim_matches('"');
        if !word.starts_with("sound/") || swaps.contains_key(word) {
            continue;
        }
        if let Some((from, to)) = moved
            .iter()
            .find(|(from, _)| word.starts_with(from.as_str()))
        {
            swaps.insert(word.to_owned(), format!("{to}{}", &word[from.len()..]));
            continue;
        }
        // A sound, or the stem its numbered variants share (`step` for step1..step4).
        let Some((dir, file)) = word.rsplit_once('/') else {
            continue;
        };
        let stem = strip_sound_extension(file);
        let matches: Vec<String> = clips(dir)
            .into_iter()
            .filter(|clip| {
                clip.strip_prefix(dir)
                    .and_then(|rest| rest.strip_prefix('/'))
                    .is_some_and(|rest| !rest.contains('/') && rest.starts_with(stem))
            })
            .collect();
        if word.starts_with(&own) {
            for clip in matches.iter().filter(|clip| !source.stock(clip)) {
                take(files, clip, clip.clone());
            }
        } else if !matches.is_empty() && matches.iter().all(|clip| !source.stock(clip)) {
            let base = format!("{own}x/{}", &dir["sound/".len()..]);
            for clip in &matches {
                take(files, clip, format!("{base}{}", &clip[dir.len()..]));
            }
            swaps.insert(word.to_owned(), format!("{base}/{file}"));
        }
    }
    if !swaps.is_empty() {
        let rewritten = rewrite_words(&text, &swaps);
        files.insert(events, rewritten.into_bytes());
    }
    out.sounds += sounds;
}

/// The shaders or images a `.skin` names: `surface,reference` lines.
fn skin_references(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines().filter_map(|line| {
        let (_, reference) = line.split_once(',')?;
        let reference = reference.trim().trim_matches('"');
        (reference.contains('/') && !reference.starts_with('*')).then(|| reference.to_owned())
    })
}

/// A `.skin`'s text with the references in `swaps` (by name without an image
/// extension) replaced.
fn rewrite_skin(text: &str, swaps: &BTreeMap<String, String>) -> String {
    text.lines()
        .map(|line| {
            if let Some((surface, reference)) = line.split_once(',')
                && let Some(moved) =
                    swaps.get(strip_image_extension(reference.trim().trim_matches('"')))
            {
                format!("{surface},{moved}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` with each whitespace-separated word (quotes aside) in `swaps` replaced.
fn rewrite_words(text: &str, swaps: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        let bare = word.trim_matches('"');
        match swaps.get(bare) {
            Some(new) => out.push_str(&word.replace(bare, new)),
            None => out.push_str(word),
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_whitespace() {
            flush(&mut word, &mut out);
            out.push(c);
        } else {
            word.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

fn strip_image_extension(path: &str) -> &str {
    match path.rsplit_once('.') {
        Some((stem, extension)) if IMAGE_EXTENSIONS.contains(&extension) => stem,
        _ => path,
    }
}

fn strip_sound_extension(file: &str) -> &str {
    file.strip_suffix(".wav")
        .or_else(|| file.strip_suffix(".mp3"))
        .unwrap_or(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game of `(path, bytes, stock)`.
    struct Game(Vec<(&'static str, Vec<u8>, bool)>);

    impl Source for Game {
        fn list(&self, dir: &str, extension: &str) -> Vec<String> {
            let prefix = format!("{dir}/");
            self.0
                .iter()
                .map(|(path, _, _)| path.to_string())
                .filter(|path| {
                    path.strip_prefix(&prefix).is_some_and(|rest| {
                        rest.matches('/').count() <= 1 && rest.ends_with(extension)
                    })
                })
                .collect()
        }
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0
                .iter()
                .find(|(known, _, _)| *known == path)
                .map(|(_, bytes, _)| bytes.clone())
        }
        fn stock(&self, path: &str) -> bool {
            self.0
                .iter()
                .any(|(known, _, stock)| *known == path && *stock)
        }
    }

    fn file(path: &'static str, text: &str) -> (&'static str, Vec<u8>, bool) {
        (path, text.as_bytes().to_vec(), false)
    }

    fn get<'a>(gathered: &'a Gathered, path: &str) -> Option<&'a str> {
        gathered
            .files
            .iter()
            .find(|(known, _)| known == path)
            .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap())
    }

    #[test]
    fn a_voice_of_another_name_shaders_elsewhere_and_outside_textures_come_along() {
        let game = Game(vec![
            file("models/players/fox/model.glm", "glm"),
            file(
                "models/players/fox/model_default.skin",
                "torso,models/players/fox/torso\nhead,models/players/shared/Head.tga\nhips,models/players/kyle/hips\nhair,models/players/foxhair/hair",
            ),
            file("models/players/fox/torso.tga", "t"),
            file("models/players/fox/sounds.cfg", "FoxVoice\nf"),
            file(
                "models/players/fox/animevents.cfg",
                "AEV_SOUND 3 sound/custom/step 1 2 100\nAEV_SOUND 4 sound/weapons/saber/hup 1 2 100",
            ),
            file("models/players/fox/readme.txt", "hi"),
            file("models/players/shared/head.jpg", "h"),
            ("models/players/kyle/hips.jpg", b"k".to_vec(), true),
            file(
                "shaders/jofmodels.shader",
                "models/players/fox/torso\n{\n\t{ map textures/glow/fox }\n}\nmodels/players/foxhair/hair\n{\n\t{ map models/players/foxhair/hair.tga }\n}\nmodels/players/other/x { }\n",
            ),
            file("models/players/foxhair/hair.tga", "hh"),
            file("textures/glow/fox.png", "g"),
            file("sound/chars/foxvoice/misc/pain25.wav", "p"),
            file("sound/chars/foxvoice/misc/death1.mp3", "d"),
            file("sound/custom/step1.wav", "s1"),
            file("sound/custom/step2.wav", "s2"),
            ("sound/weapons/saber/hup1.wav", b"w".to_vec(), true),
        ]);
        let gathered = gather("fox", &game);
        let paths: Vec<&str> = gathered
            .files
            .iter()
            .map(|(path, _)| path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "models/players/fox/animevents.cfg",
                "models/players/fox/model.glm",
                "models/players/fox/model_default.skin",
                "models/players/fox/sounds.cfg",
                "models/players/fox/torso.tga",
                "models/players/fox/x/foxhair/hair.tga",
                "models/players/fox/x/shared/head.jpg",
                "models/players/fox/x/textures/glow/fox.png",
                "shaders/fox.shader",
                "sound/chars/fox/misc/death1.mp3",
                "sound/chars/fox/misc/pain25.wav",
                "sound/chars/fox/x/custom/step1.wav",
                "sound/chars/fox/x/custom/step2.wav",
            ]
        );
        assert_eq!(
            get(&gathered, "models/players/fox/sounds.cfg"),
            Some("fox\nf")
        );
        let skin = get(&gathered, "models/players/fox/model_default.skin").unwrap();
        assert!(skin.contains("head,models/players/fox/x/shared/head.jpg"));
        assert!(
            skin.contains("hips,models/players/kyle/hips"),
            "stock stays"
        );
        assert!(skin.contains("hair,models/players/fox/x/foxhair/hair"));
        let shaders = get(&gathered, "shaders/fox.shader").unwrap();
        assert!(shaders.contains("map models/players/fox/x/textures/glow/fox.png"));
        assert!(shaders.contains(
            "models/players/fox/x/foxhair/hair\n{\n\t{ map models/players/fox/x/foxhair/hair.tga }"
        ));
        assert!(!shaders.contains("other"));
        let events = get(&gathered, "models/players/fox/animevents.cfg").unwrap();
        assert!(events.contains("sound/chars/fox/x/custom/step 1 2"));
        assert!(
            events.contains("sound/weapons/saber/hup 1 2"),
            "stock stays"
        );
        assert_eq!(gathered.sounds, 4);
        assert_eq!(gathered.pulled_in, 3);
        assert!(gathered.missing.is_empty() && gathered.other_voice.is_none());
    }

    #[test]
    fn a_stock_voice_stays_and_missing_textures_are_named() {
        let game = Game(vec![
            file("models/players/vix/model.glm", "glm"),
            file(
                "models/players/vix/model_default.skin",
                "torso,models/players/vix/gone",
            ),
            file("models/players/vix/sounds.cfg", "jan\nf"),
            ("sound/chars/jan/misc/pain25.wav", b"p".to_vec(), true),
        ]);
        let gathered = gather("vix", &game);
        assert_eq!(gathered.other_voice.as_deref(), Some("jan"));
        assert_eq!(gathered.sounds, 0);
        assert_eq!(gathered.missing, ["models/players/vix/gone"]);
        assert_eq!(
            get(&gathered, "models/players/vix/sounds.cfg"),
            Some("jan\nf")
        );
    }
}
