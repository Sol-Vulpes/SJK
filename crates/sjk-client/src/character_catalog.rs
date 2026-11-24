//! BaseJKA multiplayer character and multipart-species catalogue.
//!
//! Ordinary model enumeration mirrors `UI_BuildQ3Model_List` in OpenJK
//! `codemp/ui/ui_main.c:9427-9524`: the suffix after the first underscore is
//! the skin name, an icon is required, and duplicate model/skin pairs collapse.
//! Multipart species mirror `UI_BuildPlayerModel_List` at lines 9636-9792 and
//! `UI_ParseColorData` at lines 9561-9607. Model cvar strings use the exact
//! `model/head|torso|lower` format from `UI_UpdateCharacterCvars`, lines
//! 4981-5006.
//!
//! An SJK rig model (a folder holding `sjk_rig.cfg`, `docs/rig-models.md`) has no
//! `model.glm` or `.skin` files: it is listed as `<model>/default` when it has an
//! `icon_default` picture, and each `sjk_rig_<variant>.cfg` beside it as
//! `<model>/<variant>` when it has an `icon_<variant>` picture.

use crate::catalog_tokens::tokenize;
use sjk_vfs::VirtualFileSystem;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

/// One selectable ordinary multiplayer model skin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyCharacter {
    /// Directory component below `models/players`.
    pub model: String,
    /// Skin suffix used in the `model` userinfo cvar.
    pub skin: String,
    /// Exact userinfo value (`model/skin`).
    pub cvar_value: String,
    /// Existing icon selected using the retail JPG, PNG, TGA priority.
    pub icon: String,
    /// An SJK rig model (a glTF with its own skeleton, `sjk_rig.cfg`).
    pub rig: bool,
}

/// One colour choice parsed from a species `playerchoice.txt` block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacySpeciesColor {
    /// Shader used by the retail menu swatch.
    pub shader: String,
    /// RGB values assigned by the block's `setcvar` actions.
    pub rgb: [u8; 3],
}

/// Customisable multipart player species.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacySpecies {
    /// Directory component below `models/players`.
    pub model: String,
    /// Head skin names, including their `head_` prefix: those with an icon
    /// first, then those without.
    pub heads: Vec<String>,
    /// Torso skin names, including their `torso_` prefix, ordered as `heads`.
    pub torsos: Vec<String>,
    /// Lower-body skin names, including their `lower_` prefix, ordered as `heads`.
    pub legs: Vec<String>,
    /// How many of the heads, torsos and legs have an icon: the leading entries
    /// of each list, all that retail's menu (and SJK's classic one) offers.
    pub pictured: [usize; 3],
    /// Colour actions offered by `playerchoice.txt`.
    pub colors: Vec<LegacySpeciesColor>,
}

impl LegacySpecies {
    /// The parts of axis 0 (heads), 1 (torsos) or 2 (legs).
    pub fn parts(&self, axis: usize) -> &[String] {
        match axis {
            0 => &self.heads,
            1 => &self.torsos,
            _ => &self.legs,
        }
    }

    /// The parts of `axis` that have an icon, as retail lists them.
    pub fn pictured_parts(&self, axis: usize) -> &[String] {
        let parts = self.parts(axis);
        &parts[..self.pictured[axis.min(2)].min(parts.len())]
    }

    /// Builds the byte-exact multipart `model` cvar value used by OpenJK UI.
    pub fn cvar_value(&self, head: &str, torso: &str, legs: &str) -> String {
        format!("{}/{head}|{torso}|{legs}", self.model)
    }
}

/// Character-related half of the compatibility asset catalogue.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LegacyCharacterCatalog {
    /// Ordinary model/skin choices.
    pub characters: Vec<LegacyCharacter>,
    /// Multipart customisable species.
    pub species: Vec<LegacySpecies>,
}

/// Enumerates player models from the visible VFS union.
pub fn legacy_character_catalog(
    vfs: &VirtualFileSystem,
) -> Result<LegacyCharacterCatalog, Box<dyn Error + Send + Sync>> {
    legacy_character_catalog_with_policy(vfs, "", true)
}

/// Enumerate player choices using content metadata and the local player override.
pub fn legacy_character_catalog_with_policy(
    vfs: &VirtualFileSystem,
    blacklist: &str,
    respect_metadata: bool,
) -> Result<LegacyCharacterCatalog, Box<dyn Error + Send + Sync>> {
    let eligibility = crate::player_model_eligibility::PlayerModelEligibility::load(vfs);
    let paths = vfs.paths();
    let model_dirs = model_directories(&paths);
    let mut characters = BTreeMap::new();
    let mut species = Vec::new();

    for model in model_dirs {
        if !eligibility.allows(&model, blacklist, respect_metadata) {
            continue;
        }
        let choice = format!("models/players/{model}/playerchoice.txt");
        if let Some(asset) = vfs.read(&choice)? {
            if let Some(parsed) = parse_species(vfs, &model, &asset.bytes, &paths)? {
                species.push(parsed);
            }
        }
        collect_ordinary_skins(vfs, &model, &paths, &mut characters)?;
    }
    for model in rig_directories(&paths) {
        if !eligibility.allows(&model, blacklist, respect_metadata) {
            continue;
        }
        for skin in rig_variants(&paths, &model) {
            let cvar_value = format!("{model}/{skin}");
            if characters.contains_key(&cvar_value) {
                continue;
            }
            if let Some(icon) = icon_path(vfs, &model, &skin)? {
                characters.insert(
                    cvar_value.clone(),
                    LegacyCharacter {
                        model: model.clone(),
                        skin,
                        cvar_value,
                        icon,
                        rig: true,
                    },
                );
            }
        }
    }

    Ok(LegacyCharacterCatalog {
        characters: characters.into_values().collect(),
        species,
    })
}

fn model_directories(paths: &[sjk_vfs::VirtualPath]) -> BTreeSet<String> {
    paths
        .iter()
        .filter_map(|path| {
            path.as_str()
                .strip_prefix("models/players/")?
                .strip_suffix("/model.glm")
                .filter(|model| !model.is_empty() && !model.contains('/'))
                .map(str::to_owned)
        })
        .collect()
}

/// The rig model's manifest, `models/players/<model>/sjk_rig.cfg`.
const RIG_MANIFEST: &str = "sjk_rig.cfg";

fn rig_directories(paths: &[sjk_vfs::VirtualPath]) -> BTreeSet<String> {
    let suffix = format!("/{RIG_MANIFEST}");
    paths
        .iter()
        .filter_map(|path| {
            path.as_str()
                .strip_prefix("models/players/")?
                .strip_suffix(suffix.as_str())
                .filter(|model| !model.is_empty() && !model.contains('/'))
                .map(str::to_owned)
        })
        .collect()
}

/// A rig model's variants: `default` (its `sjk_rig.cfg`), then one per
/// `sjk_rig_<variant>.cfg` beside it (`/model <model>/<variant>` wears that manifest).
fn rig_variants(paths: &[sjk_vfs::VirtualPath], model: &str) -> Vec<String> {
    let prefix = format!("models/players/{model}/sjk_rig_");
    let mut variants = vec!["default".to_owned()];
    variants.extend(
        paths
            .iter()
            .filter_map(|path| {
                path.as_str()
                    .strip_prefix(prefix.as_str())?
                    .strip_suffix(".cfg")
                    .filter(|variant| {
                        !variant.is_empty()
                            && !variant.contains('/')
                            && !variant.eq_ignore_ascii_case("default")
                    })
                    .map(str::to_ascii_lowercase)
            })
            .collect::<BTreeSet<_>>(),
    );
    variants
}

fn collect_ordinary_skins(
    vfs: &VirtualFileSystem,
    model: &str,
    paths: &[sjk_vfs::VirtualPath],
    output: &mut BTreeMap<String, LegacyCharacter>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let prefix = format!("models/players/{model}/");
    for path in paths {
        let Some(file) = path.as_str().strip_prefix(&prefix) else {
            continue;
        };
        let Some(stem) = file.strip_suffix(".skin") else {
            continue;
        };
        let Some((_, skin)) = stem.split_once('_') else {
            continue;
        };
        if skin.is_empty() {
            continue;
        }
        let Some(icon) = icon_path(vfs, model, skin)? else {
            continue;
        };
        let cvar_value = format!("{model}/{skin}");
        output.entry(cvar_value.clone()).or_insert(LegacyCharacter {
            model: model.to_owned(),
            skin: skin.to_owned(),
            cvar_value,
            icon,
            rig: false,
        });
    }
    Ok(())
}

fn parse_species(
    vfs: &VirtualFileSystem,
    model: &str,
    bytes: &[u8],
    paths: &[sjk_vfs::VirtualPath],
) -> Result<Option<LegacySpecies>, Box<dyn Error + Send + Sync>> {
    let prefix = format!("models/players/{model}/");
    // Each list's parts with whether they have an icon: the pictured ones make
    // the folder a species as in retail, and are listed first.
    let mut lists: [Vec<(bool, String)>; 3] = Default::default();
    for path in paths {
        let Some(stem) = path
            .as_str()
            .strip_prefix(&prefix)
            .and_then(|file| file.strip_suffix(".skin"))
        else {
            continue;
        };
        let kind = if stem.starts_with("head_") {
            0
        } else if stem.starts_with("torso_") {
            1
        } else if stem.starts_with("lower_") {
            2
        } else {
            continue;
        };
        // A part without an icon is listed too (retail left it out), after the
        // pictured ones: the Hoth gear, `torso_g1` and `lower_e1` of every stock
        // species, has none (24/11/2026). SJK's creator offers it; the classic
        // menu keeps retail's list (`pictured`).
        let icon = icon_path(vfs, model, stem)?.is_some();
        lists[kind].push((icon, stem.to_owned()));
    }
    let mut pictured = [0; 3];
    for (count, list) in pictured.iter_mut().zip(&mut lists) {
        list.sort_by_key(|(icon, _)| !*icon);
        *count = list.iter().filter(|(icon, _)| *icon).count();
    }
    if pictured.contains(&0) {
        return Ok(None);
    }
    let [heads, torsos, legs] = lists.map(|list| list.into_iter().map(|(_, stem)| stem).collect());
    Ok(Some(LegacySpecies {
        model: model.to_owned(),
        heads,
        torsos,
        legs,
        pictured,
        colors: parse_colors(&String::from_utf8_lossy(bytes)),
    }))
}

fn parse_colors(source: &str) -> Vec<LegacySpeciesColor> {
    let tokens = tokenize(source);
    let mut colors = Vec::new();
    let mut cursor = 0;
    while cursor + 1 < tokens.len() {
        let shader = tokens[cursor].clone();
        cursor += 1;
        if tokens.get(cursor).map(String::as_str) != Some("{") {
            cursor += 1;
            continue;
        }
        cursor += 1;
        let mut rgb = [255_u8; 3];
        while cursor < tokens.len() && tokens[cursor] != "}" {
            if tokens[cursor].eq_ignore_ascii_case("setcvar") && cursor + 2 < tokens.len() {
                let value = tokens[cursor + 2].parse::<u8>().unwrap_or(255);
                match tokens[cursor + 1].to_ascii_lowercase().as_str() {
                    "ui_char_color_red" => rgb[0] = value,
                    "ui_char_color_green" => rgb[1] = value,
                    "ui_char_color_blue" => rgb[2] = value,
                    _ => {}
                }
                cursor += 3;
            } else {
                cursor += 1;
            }
        }
        cursor += usize::from(cursor < tokens.len());
        colors.push(LegacySpeciesColor { shader, rgb });
    }
    colors
}

fn icon_path(
    vfs: &VirtualFileSystem,
    model: &str,
    skin: &str,
) -> Result<Option<String>, sjk_vfs::VfsError> {
    for extension in ["jpg", "png", "tga"] {
        let path = format!("models/players/{model}/icon_{skin}.{extension}");
        if vfs.contains(&path)? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_species_lists_its_parts_without_an_icon_too() {
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "pack",
            [
                ("models/players/jedi_zf/model.glm", Vec::new()),
                ("models/players/jedi_zf/playerchoice.txt", Vec::new()),
                ("models/players/jedi_zf/head_a1.skin", Vec::new()),
                ("models/players/jedi_zf/icon_head_a1.jpg", Vec::new()),
                ("models/players/jedi_zf/torso_a1.skin", Vec::new()),
                ("models/players/jedi_zf/icon_torso_a1.png", Vec::new()),
                ("models/players/jedi_zf/torso_g1.skin", Vec::new()),
                ("models/players/jedi_zf/lower_a1.skin", Vec::new()),
                ("models/players/jedi_zf/icon_lower_a1.png", Vec::new()),
                ("models/players/jedi_zf/lower_e1.skin", Vec::new()),
                ("models/players/jedi_zf/model_default.skin", Vec::new()),
                // Skins with no icon at all make no species.
                ("models/players/bare/model.glm", Vec::new()),
                ("models/players/bare/playerchoice.txt", Vec::new()),
                ("models/players/bare/head_a1.skin", Vec::new()),
                ("models/players/bare/torso_a1.skin", Vec::new()),
                ("models/players/bare/lower_a1.skin", Vec::new()),
            ],
        )
        .expect("mounts");
        let catalog = legacy_character_catalog(&vfs).expect("the catalogue");
        assert_eq!(catalog.species.len(), 1);
        let species = &catalog.species[0];
        assert_eq!(species.model, "jedi_zf");
        assert_eq!(species.heads, ["head_a1"]);
        assert_eq!(species.torsos, ["torso_a1", "torso_g1"]);
        assert_eq!(species.legs, ["lower_a1", "lower_e1"]);
        assert_eq!(species.pictured, [1, 1, 1]);
        assert_eq!(species.pictured_parts(1), ["torso_a1"]);
    }

    #[test]
    fn rig_models_with_an_icon_are_listed_as_their_default_skin() {
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "pack",
            [
                ("models/players/kyle/model.glm", Vec::new()),
                ("models/players/kyle/model_default.skin", Vec::new()),
                ("models/players/kyle/icon_default.jpg", Vec::new()),
                ("models/players/wolf/sjk_rig.cfg", Vec::new()),
                ("models/players/wolf/model.gltf", Vec::new()),
                ("models/players/wolf/icon_default.png", Vec::new()),
                // A variant: its own manifest and icon.
                ("models/players/wolf/sjk_rig_cloak.cfg", Vec::new()),
                ("models/players/wolf/model_cloak.gltf", Vec::new()),
                ("models/players/wolf/icon_cloak.png", Vec::new()),
                // A variant without a picture stays out, as a skin without an icon does.
                ("models/players/wolf/sjk_rig_bare.cfg", Vec::new()),
                // Without a picture a rig model stays out, as a skin without an icon does.
                ("models/players/bare/sjk_rig.cfg", Vec::new()),
            ],
        )
        .expect("mounts");
        let catalog = legacy_character_catalog(&vfs).expect("the catalogue");
        let listed: Vec<_> = catalog
            .characters
            .iter()
            .map(|character| {
                (
                    character.cvar_value.as_str(),
                    character.icon.as_str(),
                    character.rig,
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                (
                    "kyle/default",
                    "models/players/kyle/icon_default.jpg",
                    false
                ),
                ("wolf/cloak", "models/players/wolf/icon_cloak.png", true),
                ("wolf/default", "models/players/wolf/icon_default.png", true),
            ]
        );
    }
}
