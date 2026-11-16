//! The feats' medals under the top centre and the announcer's queue
//! ([`crate::feats`]): each feat of the viewed player pops in as a medal, its picture
//! and its name, holds [`HOLD_MS`] and fades over [`FADE_MS`]; the newest stands on
//! top, at most [`CAPACITY`] at once. Its line is queued for the announcer
//! ([`Feats::take_line`]), one every [`LINE_GAP_MS`], the oldest dropped past
//! [`QUEUE`].
//!
//! Like the kill feed, kills are read from the shared obituary tracker by count, never
//! from a second event scan, and the medals are drawn with the HUD's own list: they
//! hide with the HUD. Off unless `cg_feats` is on.

use crate::console::ViewerConsole;
use crate::feats::{Feat, Rules};
use crate::text::UiFont;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{
    Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow, TextureId,
    Theme,
};

/// Medals shown at most.
pub(crate) const CAPACITY: usize = 4;
/// Draw commands one medal takes: its plate, accent bar, picture and name.
pub(crate) const ENTRY_COMMANDS: usize = 4;
/// First text id of the medals, one per ring slot.
pub(crate) const TEXT_FIRST: u32 = 520;
/// Last text id of the medals.
pub(crate) const TEXT_LAST: u32 = TEXT_FIRST + CAPACITY as u32 - 1;
/// A medal stays fully visible this long (server milliseconds)...
pub(crate) const HOLD_MS: i32 = 2_500;
/// ...then fades out over this long.
pub(crate) const FADE_MS: i32 = 700;
/// It grows in over this long, from [`POP_SCALE`] of its size.
const POP_MS: i32 = 160;
const POP_SCALE: f32 = 1.35;
/// Least time between two announcer lines (server milliseconds).
pub(crate) const LINE_GAP_MS: i32 = 1_300;
/// Lines waiting at most; a new one pushes the oldest out.
const QUEUE: usize = 3;
/// A kill stamped this far ahead of the HUD's clock belongs to an earlier timeline.
const FUTURE_MS: i32 = 1_000;

/// The cvars.
pub(crate) const CVAR: &str = "cg_feats";
pub(crate) const VOICE_CVAR: &str = "cg_featsVoice";
pub(crate) const MEDALS_CVAR: &str = "cg_featsMedals";

// Geometry in 1080-line pixels, grown with the window and `cg_hudScale`.
/// Top of the newest medal: under the top centre's vote, timer and warmup line.
const TOP: f32 = 190.0;
const ICON: f32 = 56.0;
const TEXT: f32 = 30.0;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 6.0;
const SPACE: f32 = 12.0;
const ROW_GAP: f32 = 8.0;
const RADIUS: f32 = 6.0;
const ACCENT: f32 = 4.0;

/// Register the feats' options.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    for (name, value, help) in [
        (
            CVAR,
            false,
            "Feats: multi-kills and sprees called out with medals and an announcer",
        ),
        (VOICE_CVAR, true, "Feats: the announcer says each feat"),
        (MEDALS_CVAR, true, "Feats: each feat shows as a medal"),
    ] {
        cvars.register(CvarDefinition::new(name, value, CvarFlags::ARCHIVE, help))?;
    }
    cvars.register(CvarDefinition::new(
        crate::feats::announcer::CVAR,
        crate::feats::announcer::DEFAULT_PACK,
        CvarFlags::ARCHIVE,
        "Feats: the announcer's voice pack (a folder in sound/sjk/announcer)",
    ))?;
    Ok(())
}

/// The options, sampled once per HUD update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Options {
    pub(crate) enabled: bool,
    pub(crate) voice: bool,
    pub(crate) medals: bool,
}

impl Options {
    fn read(console: Option<&ViewerConsole>) -> Self {
        let on = |name: &str, fallback| {
            console
                .and_then(|console| console.bool_cvar(name))
                .unwrap_or(fallback)
        };
        Self {
            enabled: on(CVAR, false),
            voice: on(VOICE_CVAR, true),
            medals: on(MEDALS_CVAR, true),
        }
    }
}

/// Opacity of a medal `age` milliseconds after its kill.
pub(crate) fn alpha(age: i32) -> f32 {
    if !(-FUTURE_MS..HOLD_MS + FADE_MS).contains(&age) {
        0.0
    } else if age <= HOLD_MS {
        1.0
    } else {
        1.0 - (age - HOLD_MS) as f32 / FADE_MS as f32
    }
}

/// Size of a medal `age` milliseconds after its kill, against its resting size.
fn pop(age: i32) -> f32 {
    if !(0..POP_MS).contains(&age) {
        1.0
    } else {
        let t = age as f32 / POP_MS as f32;
        POP_SCALE + (1.0 - POP_SCALE) * (1.0 - (1.0 - t) * (1.0 - t))
    }
}

#[derive(Clone, Copy, Debug)]
struct Medal {
    feat: Feat,
    start: i32,
}

/// The rules, the medals showing and the lines waiting.
pub(crate) struct Feats {
    rules: Rules,
    medals: [Option<Medal>; CAPACITY],
    /// Ring slot of the newest medal.
    newest: usize,
    lines: [Option<Feat>; QUEUE],
    /// When the next line may be said.
    next_line: i32,
    /// Obituaries of the tracker already taken in.
    consumed: u64,
    now: i32,
    options: Options,
    /// The announcer pack `cg_announcer` names.
    pack: String,
    /// The pictures, by [`Feat::index`].
    pictures: [Option<TextureId>; Feat::ALL.len()],
    /// The HUD edit mode is open: a sample medal shows to drag.
    preview: bool,
}

impl Default for Feats {
    fn default() -> Self {
        Self {
            rules: Rules::default(),
            medals: [None; CAPACITY],
            newest: CAPACITY - 1,
            lines: [None; QUEUE],
            next_line: i32::MIN,
            consumed: 0,
            now: 0,
            options: Options::default(),
            pack: String::from(crate::feats::announcer::DEFAULT_PACK),
            pictures: [None; Feat::ALL.len()],
            preview: false,
        }
    }
}

impl Feats {
    /// Read the options for this frame.
    pub(crate) fn sample(&mut self, console: Option<&ViewerConsole>) {
        self.options = Options::read(console);
        let pack = console
            .and_then(|console| console.text_cvar(crate::feats::announcer::CVAR).ok())
            .unwrap_or(crate::feats::announcer::DEFAULT_PACK);
        if pack != self.pack {
            self.pack.clear();
            self.pack.push_str(pack);
        }
    }

    /// Keep the pictures [`load`] uploaded.
    pub(crate) fn set_pictures(&mut self, pictures: [Option<TextureId>; Feat::ALL.len()]) {
        self.pictures = pictures;
    }

    /// While the HUD edit mode is open, a sample medal shows so it can be dragged.
    pub(crate) fn set_preview(&mut self, preview: bool) {
        self.preview = preview;
    }

    /// The pack the announcer speaks with.
    pub(crate) fn pack(&self) -> &str {
        &self.pack
    }

    /// Take in the tracker's new obituaries at `now` (the HUD's server time) for the
    /// viewed player `local`.
    pub(crate) fn observe(
        &mut self,
        tracker: &sjk_client::ObituaryTracker,
        game: &sjk_protocol::GameState,
        now: i32,
        local: u16,
    ) {
        // A new tracker (a new connection), or the clock gone back (a restart, a demo
        // rewound): what is shown and counted belongs to another timeline.
        if tracker.decoded() < self.consumed || now.saturating_add(FUTURE_MS) < self.now {
            self.consumed = 0;
            self.rules = Rules::default();
            self.clear();
        }
        self.now = now;
        self.rules.level(game, now);
        let fresh = tracker
            .decoded()
            .saturating_sub(self.consumed)
            .min(tracker.feed().len() as u64) as usize;
        self.consumed = tracker.decoded();
        for offset in (0..fresh).rev() {
            let Some(event) = tracker.feed().newest(offset) else {
                continue;
            };
            let earned = self.rules.kill(&event);
            if !self.options.enabled
                || event.attacker != local
                || alpha(now.saturating_sub(event.server_time)) <= 0.0
            {
                continue;
            }
            for feat in earned.iter() {
                self.push(feat, event.server_time);
            }
        }
        if !self.options.enabled {
            self.clear();
        }
        if self.preview && self.shown().next().is_none() {
            self.push_medal(Feat::DoubleKill, now);
        }
    }

    fn clear(&mut self) {
        self.medals = [None; CAPACITY];
        self.lines = [None; QUEUE];
        self.next_line = i32::MIN;
    }

    fn push(&mut self, feat: Feat, start: i32) {
        if self.options.medals {
            self.push_medal(feat, start);
        }
        if self.options.voice {
            if self.lines.iter().all(Option::is_some) {
                self.lines.rotate_left(1);
                self.lines[QUEUE - 1] = None;
            }
            if let Some(slot) = self.lines.iter_mut().find(|line| line.is_none()) {
                *slot = Some(feat);
            }
        }
    }

    fn push_medal(&mut self, feat: Feat, start: i32) {
        self.newest = (self.newest + 1) % CAPACITY;
        self.medals[self.newest] = Some(Medal { feat, start });
    }

    /// The next line to say now, if one waits and the last has had its time.
    pub(crate) fn take_line(&mut self) -> Option<Feat> {
        if self.lines[0].is_none() || self.now < self.next_line {
            return None;
        }
        let feat = self.lines[0].take();
        self.lines.rotate_left(1);
        self.next_line = self.now.saturating_add(LINE_GAP_MS);
        feat
    }

    /// Ring slots of the medals showing, newest first, with their age.
    fn shown(&self) -> impl Iterator<Item = (usize, Medal, i32)> + '_ {
        (0..CAPACITY).filter_map(move |rank| {
            let slot = (self.newest + CAPACITY - rank) % CAPACITY;
            let medal = self.medals[slot]?;
            let age = self.now.saturating_sub(medal.start);
            (alpha(age) > 0.0).then_some((slot, medal, age))
        })
    }

    /// The text of id `id` (`TEXT_FIRST..=TEXT_LAST`).
    pub(crate) fn text(&self, id: u32) -> &str {
        self.medals
            .get(id.saturating_sub(TEXT_FIRST) as usize)
            .copied()
            .flatten()
            .map_or("", |medal| medal.feat.label())
    }

    /// Emit the medals showing, centred on the window, at `scale` window pixels per
    /// 1080-line pixel.
    pub(super) fn emit(
        &self,
        list: &mut DrawList,
        viewport: [f32; 2],
        scale: f32,
        font: &UiFont,
        theme: Theme,
    ) {
        let mut y = TOP * scale;
        let centre = viewport[0] * 0.5;
        for (slot, medal, age) in self.shown() {
            let alpha = alpha(age);
            let k = scale * pop(age);
            let size = TEXT * k;
            let icon = ICON * k;
            let picture = self.pictures[medal.feat.index()];
            let label = crate::text::visible_text_width_style(
                font,
                medal.feat.label(),
                size / font.height.max(1.0),
                crate::text::TextFace::Semibold,
                0.0,
            );
            let row = icon.max(size) + 2.0 * PAD_Y * k;
            let width =
                2.0 * PAD_X * k + ACCENT * k + picture.map_or(0.0, |_| icon + SPACE * k) + label;
            let plate = Rect::new(centre - width * 0.5, y, width, row);
            let [r, g, b] = medal.feat.accent();
            let _ = list.push(DrawCommand::RoundedRect {
                rect: plate,
                radius: RADIUS * k,
                color: Color::new(0.02, 0.02, 0.03, 0.62 * alpha),
            });
            let _ = list.push(DrawCommand::RoundedRect {
                rect: Rect::new(plate.x, plate.y, ACCENT * k, row),
                radius: (ACCENT * 0.5 * k).min(RADIUS * k),
                color: Color::new(r, g, b, alpha),
            });
            let mut x = plate.x + ACCENT * k + PAD_X * k;
            if let Some(texture) = picture {
                let _ = list.push(DrawCommand::TexturedQuad {
                    rect: Rect::new(x, y + (row - icon) * 0.5, icon, icon),
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, alpha),
                });
                x += icon + SPACE * k;
            }
            let _ = list.push(DrawCommand::Text {
                rect: Rect::new(x, y + (row - size) * 0.5, label + 1.0, size),
                text: TextId(TEXT_FIRST + slot as u32),
                size,
                color: Color::new(
                    0.5 + 0.5 * r,
                    0.5 + 0.5 * g,
                    0.5 + 0.5 * b,
                    theme.foreground.a * alpha,
                ),
                align: TextAlign::Start,
                overflow: TextOverflow::Clip,
                weight: FontWeight::Semibold,
                letter_spacing: 0.0,
            });
            // Rows keep their resting height, so a medal popping in does not shove
            // the older ones down and back.
            y += (icon.max(size) / pop(age) + 2.0 * PAD_Y * scale) + ROW_GAP * scale;
        }
    }
}

/// The pictures' game paths: `gfx/sjk/feats/<id>.png`, bundled and replaceable by a PK3.
pub(crate) fn picture_path(feat: Feat) -> String {
    format!("gfx/sjk/feats/{}.png", feat.id())
}

/// Decode each feat's picture and hand it to `upload` with its grid cell; absent ones
/// stay `None`.
pub(crate) fn load(
    vfs: &sjk_vfs::VirtualFileSystem,
    shaders: &sjk_shader::ShaderCatalog,
    mut upload: impl FnMut(u32, &[u8]),
) -> [Option<TextureId>; Feat::ALL.len()] {
    let mut pictures = [None; Feat::ALL.len()];
    for feat in Feat::ALL {
        let cell = feat.index() as u32;
        if let Some(pixels) = super::icons::assets::decode(vfs, shaders, &picture_path(feat)) {
            upload(cell, pixels.as_raw());
            pictures[feat.index()] = Some(crate::ui_renderer::feat_pictures::cell_texture(cell));
        }
    }
    pictures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medals_hold_then_fade() {
        assert_eq!(alpha(0), 1.0);
        assert_eq!(alpha(HOLD_MS), 1.0);
        assert!((alpha(HOLD_MS + FADE_MS / 2) - 0.5).abs() < 1e-6);
        assert_eq!(alpha(HOLD_MS + FADE_MS), 0.0);
        assert_eq!(alpha(-FUTURE_MS - 1), 0.0);
        assert_eq!(pop(0), POP_SCALE);
        assert_eq!(pop(POP_MS), 1.0);
    }

    #[test]
    fn lines_wait_their_turn_and_the_oldest_gives_way() {
        let mut feats = Feats {
            options: Options {
                enabled: true,
                voice: true,
                medals: true,
            },
            ..Feats::default()
        };
        for feat in [
            Feat::FirstBlood,
            Feat::DoubleKill,
            Feat::KillingSpree,
            Feat::Revenge,
        ] {
            feats.push(feat, 0);
        }
        feats.now = 10_000;
        assert_eq!(feats.take_line(), Some(Feat::DoubleKill));
        assert_eq!(feats.take_line(), None);
        feats.now += LINE_GAP_MS;
        assert_eq!(feats.take_line(), Some(Feat::KillingSpree));
        feats.now += LINE_GAP_MS;
        assert_eq!(feats.take_line(), Some(Feat::Revenge));
        feats.now += LINE_GAP_MS;
        assert_eq!(feats.take_line(), None);
        // Four medals pushed, all four show; the text ids name them.
        assert_eq!(feats.medals.iter().flatten().count(), CAPACITY);
        assert_eq!(feats.text(TEXT_FIRST), "FIRST BLOOD");
    }
}
