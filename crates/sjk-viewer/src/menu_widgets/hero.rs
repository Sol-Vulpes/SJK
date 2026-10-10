//! Full-bleed "hero" widgets for the screens drawn over the live map that
//! have no classic or SJK UI version yet (Create game, the renderer settings):
//! a readability scrim, accent sweeps and keyboard key caps.

use super::contrast::{self, FadeSegment};
use super::{FormLayout, MenuCanvas};
use sjk_ui::{Color, DrawCommand, FontWeight, Gradient, Rect, TextAlign};

const INK: [f32; 3] = [0.005, 0.010, 0.018];

/// Width, as a fraction of the viewport, over which a readability-held scrim
/// eases back into its original fade past the text column.
const BACKING_FEATHER: f32 = 0.12;

/// Right edge of the widest hero text column (the form layout's).
fn text_column_right(viewport: [f32; 2]) -> f32 {
    let form = FormLayout::new(viewport);
    let hero = HeroColumn::new(viewport);
    (form.margin + form.column_width).max(hero.margin + hero.column_width)
}

fn ink(alpha: f32) -> Color {
    Color::new(INK[0], INK[1], INK[2], alpha)
}

fn horizontal(rect: Rect, start: Color, end: Color) -> DrawCommand {
    DrawCommand::GradientRect {
        rect,
        radius: 0.0,
        gradient: Gradient {
            start,
            end,
            vertical: false,
        },
    }
}

fn vertical(rect: Rect, start: Color, end: Color) -> DrawCommand {
    DrawCommand::GradientRect {
        rect,
        radius: 0.0,
        gradient: Gradient {
            start,
            end,
            vertical: true,
        },
    }
}

/// Geometry of a hero screen's left column for one viewport: everything
/// scales with viewport height so 1080p, ultrawide and 4K keep the same
/// proportions.
#[derive(Clone, Copy, Debug)]
struct HeroColumn {
    margin: f32,
    column_width: f32,
}

impl HeroColumn {
    fn new(viewport: [f32; 2]) -> Self {
        let scale = crate::ui_scale::height_scale(viewport[1]);
        Self {
            margin: (viewport[0] * 0.075).max(72.0 * scale),
            column_width: (viewport[0] * 0.42).clamp(360.0 * scale, 560.0 * scale),
        }
    }
}

/// How much of the live world a hero screen darkens.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Scrim {
    /// Light full-frame tint, a heavy fade on the left column and top/bottom
    /// bands: the main menu, where the world is scenery.
    Full,
    /// Only the left column fades, and it stops at the middle of the frame:
    /// screens that show something in the world (the player, the sabers)
    /// keep it at its real brightness.
    Column,
    /// The left fade reaches across the whole frame: data-dense screens
    /// (the server browser) whose columns run past the middle.
    Wide,
}

impl MenuCanvas {
    /// Begin a screen over the live world with `scrim` so the chrome stays
    /// readable over any map lighting. Everything up to [`Self::end_hero`]
    /// is drawn at `opacity`, which lets a screen fade in as the backdrop
    /// camera arrives on its shot.
    pub(crate) fn begin_hero(&mut self, viewport: [f32; 2], opacity: f32, scrim: Scrim) {
        self.begin_transparent(viewport);
        self.push_opacity(opacity);
        let [width, height] = viewport;
        let column = text_column_right(viewport);
        if scrim == Scrim::Column {
            self.held_fade(viewport, width * 0.54, 0.95, 0.0, 0.0, column);
            return;
        }
        let _ = self.draw.push(DrawCommand::SolidRect {
            rect: Rect::new(0.0, 0.0, width, height),
            color: ink(0.30),
        });
        if scrim == Scrim::Wide {
            self.held_fade(viewport, width, 0.94, 0.50, 0.30, width);
        } else {
            self.held_fade(viewport, width * 0.64, 0.94, 0.0, 0.30, column);
        }
        let _ = self.draw.push(vertical(
            Rect::new(0.0, 0.0, width, height * 0.18),
            ink(0.62),
            ink(0.0),
        ));
        let _ = self.draw.push(vertical(
            Rect::new(0.0, height * 0.70, width, height * 0.30),
            ink(0.0),
            ink(0.84),
        ));
    }

    /// Rounded backing at the `ui_menuContrast` floor behind text that sits
    /// outside the text column; nothing while the setting is off.
    #[allow(dead_code)] // Unused since Create game's SJK UI look; goes with the hero look.
    pub(crate) fn text_backing(&mut self, rect: Rect) {
        let coverage = self.readability_coverage();
        if coverage > 0.0 {
            let _ = self.draw.push(DrawCommand::RoundedRect {
                rect,
                radius: self.theme.radii.lg,
                color: ink(coverage),
            });
        }
    }

    /// A scrim's full-height left fade from `start` to `end` over `width`,
    /// stacked on a uniform `base` tint and held at the `ui_menuContrast`
    /// floor up to `hold`, the right edge of the text it backs.
    fn held_fade(
        &mut self,
        viewport: [f32; 2],
        width: f32,
        start: f32,
        end: f32,
        base: f32,
        hold: f32,
    ) {
        let coverage = self.readability_coverage();
        let floor = contrast::layer_alpha(coverage, base);
        let feather = viewport[0] * BACKING_FEATHER;
        let (segments, count) = contrast::held_fade(width, start, end, hold, floor, feather);
        for FadeSegment { x0, x1, a0, a1 } in &segments[..count] {
            let rect = Rect::new(*x0, 0.0, x1 - x0, viewport[1]);
            let _ = self.draw.push(horizontal(rect, ink(*a0), ink(*a1)));
        }
        self.mark_backing(coverage);
    }

    /// Close the opacity group opened by [`Self::begin_hero`].
    pub(crate) fn end_hero(&mut self) {
        self.pop_opacity();
    }

    /// Accent gradient fading out to the right, starting `inset` left of
    /// `rect`: the selection/hover mark of box-free rows.
    pub(crate) fn accent_sweep(&mut self, rect: Rect, strength: f32, inset: f32) {
        let accent = self.theme.accent;
        let _ = self.draw.push(horizontal(
            Rect::new(rect.x - inset, rect.y, rect.width + inset, rect.height),
            Color::new(accent.r, accent.g, accent.b, strength),
            Color::new(accent.r, accent.g, accent.b, 0.0),
        ));
    }

    /// Faint hairline between box-free rows.
    pub(crate) fn separator_line(&mut self, rect: Rect) {
        let _ = self.draw.push(DrawCommand::SolidRect {
            rect,
            color: Color::new(1.0, 1.0, 1.0, 0.07),
        });
    }

    /// One keyboard key cap followed by its action; returns the next free x.
    pub(crate) fn key_hint(
        &mut self,
        key: &str,
        action: &str,
        origin: [f32; 2],
        scale: f32,
    ) -> f32 {
        let height = 22.0 * scale;
        let cap_width = (16.0 + 8.4 * key.len() as f32) * scale;
        let cap = Rect::new(origin[0], origin[1], cap_width, height);
        let _ = self.draw.push(DrawCommand::RoundedRect {
            rect: cap,
            radius: 4.0 * scale,
            color: Color::new(1.0, 1.0, 1.0, 0.09),
        });
        let _ = self.draw.push(DrawCommand::Border {
            rect: cap,
            radius: 4.0 * scale,
            width: 1.0,
            color: Color::new(1.0, 1.0, 1.0, 0.16),
        });
        self.text_aligned(
            key,
            Rect::new(cap.x, cap.y + 3.5 * scale, cap.width, 14.0 * scale),
            12.0 * scale,
            Color::new(0.955, 0.973, 0.991, 0.964),
            FontWeight::Semibold,
            0.6 * scale,
            TextAlign::Center,
        );
        let label_x = cap.right() + 10.0 * scale;
        let label_width = (7.2 * action.len() as f32 + 8.0) * scale;
        self.text(
            action,
            Rect::new(label_x, cap.y + 3.0 * scale, label_width, 16.0 * scale),
            13.0 * scale,
            self.theme.muted,
            FontWeight::Regular,
            0.2 * scale,
        );
        label_x + label_width + 22.0 * scale
    }
}
