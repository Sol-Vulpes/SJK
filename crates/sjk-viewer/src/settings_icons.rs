//! The settings menu's icons: grey metal discs in the quick wheels' style, every one
//! drawn by `scripts/settings_icons_draw.py` (the disc computed, the symbol a glyph of
//! the Phosphor Icons fill font, MIT) so the set has one look; uploaded into the UI
//! atlas at start (`ui_renderer::settings_icon`). The Settings rail and its
//! sub-headings wear them (`settings/section_icons.rs`), the classic Settings lists
//! show one beside each group ([`crate::menu::classic::layout::Entry::icon`]), and
//! other screens borrow some (the in-game menu, the news).

/// One icon: its name and its picture, `assets/settings/<name>.png`.
macro_rules! icon {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!("../assets/settings/", $name, ".png")).as_slice(),
        )
    };
}

/// One small icon: its name and its picture, `assets/settings/small/<name>.png`.
macro_rules! small {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!("../assets/settings/small/", $name, ".png")).as_slice(),
        )
    };
}

/// Every whole-cell icon, in the script's order: the categories', the key bindings'
/// groups' and the ones other screens draw large.
pub(crate) const ICONS: [(&str, &[u8]); 32] = [
    icon!("settings"),
    icon!("first_setup"),
    icon!("video"),
    icon!("graphics"),
    icon!("sound"),
    icon!("mouse_joystick"),
    icon!("key_bindings"),
    icon!("gameplay"),
    icon!("game_options"),
    icon!("interface"),
    icon!("hud"),
    icon!("text"),
    icon!("options"),
    icon!("network"),
    icon!("movement"),
    icon!("interaction"),
    icon!("weapons"),
    icon!("force_powers"),
    icon!("other_controls"),
    icon!("image"),
    icon!("lighting"),
    icon!("shadows"),
    icon!("weather"),
    icon!("scoreboard"),
    icon!("camera"),
    icon!("crosshair"),
    icon!("nameplates"),
    icon!("identity"),
    icon!("changelog"),
    icon!("credits"),
    icon!("update"),
    icon!("import"),
];

/// The icons only the Settings sub-headings wear, [`SMALL_SIZE`] square, four to an
/// atlas cell after [`ICONS`]' cells: the atlas has no room for a cell each, and they
/// are drawn at a quarter of a cell's area.
pub(crate) const SMALL: [(&str, &[u8]); 32] = [
    small!("frame_rate"),
    small!("view"),
    small!("quality"),
    small!("textures"),
    small!("effects"),
    small!("sunlight"),
    small!("time"),
    small!("game_sounds"),
    small!("headphones"),
    small!("emotes"),
    small!("toys"),
    small!("notifications"),
    small!("players"),
    small!("duels"),
    small!("maps"),
    small!("prediction"),
    small!("away"),
    small!("theme"),
    small!("console"),
    small!("cards"),
    small!("playstyle"),
    small!("readouts"),
    small!("match"),
    small!("status_bar"),
    small!("dnd"),
    small!("chat_look"),
    small!("chat_tabs"),
    small!("sjk_chat"),
    small!("packs"),
    small!("server_mods"),
    small!("jof"),
    small!("online"),
];

/// A small icon's side: half the atlas cell's.
pub(crate) const SMALL_SIZE: u32 = crate::ui_renderer::ICON_SIZE / 2;

/// A picture to draw: the atlas cell holding it and the part of the cell it fills
/// (`DrawCommand::TexturedQuadUv`'s corners: top left, top right, bottom right, bottom
/// left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Picture {
    pub(crate) texture: sjk_ui::TextureId,
    pub(crate) uv: [[f32; 2]; 4],
}

impl Picture {
    /// The command drawing the picture over `rect`.
    pub(crate) fn quad(self, rect: sjk_ui::Rect) -> sjk_ui::DrawCommand {
        sjk_ui::DrawCommand::TexturedQuadUv {
            rect,
            texture: self.texture,
            color: sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0),
            uv: self.uv,
        }
    }
}

/// The cell of small icon `index` and the quarter of it the icon fills, as corners
/// half a texel inside the quarter so its neighbours never bleed in.
fn small_place(index: usize) -> Picture {
    let cell = ICONS.len() + index / 4;
    let (column, row) = ((index % 4 % 2) as f32, (index % 4 / 2) as f32);
    let inset = 0.5 / crate::ui_renderer::ICON_SIZE as f32;
    let (left, top) = (column * 0.5 + inset, row * 0.5 + inset);
    let (right, bottom) = (column * 0.5 + 0.5 - inset, row * 0.5 + 0.5 - inset);
    Picture {
        texture: crate::ui_renderer::settings_icon(cell),
        uv: [[left, top], [right, top], [right, bottom], [left, bottom]],
    }
}

/// The atlas cells holding [`SMALL`]: each cell's texture and its pixels (RGBA, the
/// atlas cell's size), four icons to a cell, for the renderer to upload at start and
/// the menu snapshots to draw from.
pub(crate) fn small_cells() -> Vec<(sjk_ui::TextureId, image::RgbaImage)> {
    let size = crate::ui_renderer::ICON_SIZE;
    let mut cells: Vec<(sjk_ui::TextureId, image::RgbaImage)> = Vec::new();
    for (index, (_, bytes)) in SMALL.iter().enumerate() {
        if index % 4 == 0 {
            cells.push((
                small_place(index).texture,
                image::RgbaImage::new(size, size),
            ));
        }
        let Ok(icon) = image::load_from_memory(bytes) else {
            continue;
        };
        let (x, y) = ((index % 4 % 2) as u32, (index % 4 / 2) as u32);
        if let Some((_, cell)) = cells.last_mut() {
            image::imageops::replace(
                cell,
                &icon.into_rgba8(),
                i64::from(x * SMALL_SIZE),
                i64::from(y * SMALL_SIZE),
            );
        }
    }
    cells
}

/// The picture named `name`, whole-cell or small, or a quick wheel icon's.
pub(crate) fn picture(name: &str) -> Option<Picture> {
    if let Some(index) = SMALL.iter().position(|(icon, _)| *icon == name) {
        return Some(small_place(index));
    }
    texture(name).map(|texture| Picture {
        texture,
        uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    })
}

/// The atlas texture of icon `name`, if there is one by that name: a settings
/// icon, else a quick wheel icon (Settings' Quick wheel category wears the
/// wheel's own, [`crate::quick_wheel::catalog::WHEEL_ICON`]).
pub(crate) fn texture(name: &str) -> Option<sjk_ui::TextureId> {
    ICONS
        .iter()
        .position(|(icon, _)| *icon == name)
        .map(crate::ui_renderer::settings_icon)
        .or_else(|| {
            crate::quick_wheel::catalog::icon_index(name).map(crate::ui_renderer::wheel_icon)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_one_atlas_cell_and_has_a_cell() {
        assert!(ICONS.len() + SMALL.len().div_ceil(4) <= crate::ui_renderer::SETTINGS_ICON_CELLS);
        for (name, bytes) in SMALL {
            let icon = image::load_from_memory(bytes).expect(name);
            assert_eq!(
                (icon.width(), icon.height()),
                (SMALL_SIZE, SMALL_SIZE),
                "{name}"
            );
            // A quick wheel icon may share a name (the wheel's "console"): a
            // sub-heading takes the small one, the wheel its own.
            assert!(
                !ICONS.iter().any(|(icon, _)| *icon == name),
                "{name} is small and whole"
            );
        }
        // Each small icon has its own quarter of a cell past the whole icons'.
        let cells = small_cells();
        assert_eq!(cells.len(), SMALL.len().div_ceil(4));
        let mut seen = Vec::new();
        for (name, _) in SMALL {
            let place = picture(name).expect(name);
            assert!(cells.iter().any(|(texture, _)| *texture == place.texture));
            assert!(!seen.contains(&(place.texture, place.uv[0])), "{name}");
            seen.push((place.texture, place.uv[0]));
        }
        for (name, bytes) in ICONS {
            let icon = image::load_from_memory(bytes).expect(name);
            let size = crate::ui_renderer::ICON_SIZE;
            assert_eq!((icon.width(), icon.height()), (size, size), "{name}");
        }
    }
}
