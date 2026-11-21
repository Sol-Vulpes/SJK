//! The banner's look and motion (16/11/2026, Sol: "holocrons and other notifications should
//! come up like an iOS notification, from the top of the screen then go back, maybe with
//! transparent background"), in place of the card that grew in under the top centre with a
//! burst of light, sparks and a glint.
//!
//! - Motion: it comes down from above the screen's top edge on a damped spring ([`ENTER`]
//!   seconds, settling past its place by a few pixels and back), rests [`HOLD`] seconds
//!   just under the top edge ([`MARGIN`] below it) and goes back up out of the
//!   screen, gathering speed ([`LEAVE`]); it never fades out in place. With
//!   `ui_rarityEffects 0` it fades in and out over a short drift instead ([`FADE_IN`],
//!   [`FADE_OUT`]), with no spring and its icon's ring already full.
//! - Look: acrylic glass (17/11/2026), the game blurred behind it under the ground's ember
//!   tint (`color::acrylic::BANNER`), lit along its top, with rounded corners, a soft
//!   shadow and a hairline edge, so its words read over a bright sky as over a dark
//!   corridor; the icon on the left on a tile tinted in the notification's colour (an
//!   achievement's category, a holocron's tier, the SJK UI's gold or amber), with a soft glow of that colour behind it that brightens once as
//!   it lands, and a count on a badge for several of one kind; the header (the kind in
//!   small spaced capitals and "now"), the title in bold and two lines under it.
//! - Size: [`WIDTH`] by [`HEIGHT`] in 1080-line pixels, scaled with the window's height as
//!   the SJK UI is, smaller when the window is too narrow for it.
//!
//! Everything is draw-list shapes of a fixed number: nothing allocated a frame.

use super::Look;
use crate::achievements::medallion::{self, Medallion};
use crate::holocrons::gem;
use crate::menu::sjk::{DISPLAY_CENTRE, color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{Color, DrawCommand, FontWeight, Gradient, Rect, TextAlign, TextureId};

/// Seconds the banner takes to come down into place.
pub(crate) const ENTER: f32 = 0.6;
/// Seconds it rests there.
pub(crate) const HOLD: f32 = 5.0;
/// Seconds it takes to go back up out of the screen.
pub(crate) const LEAVE: f32 = 0.4;
/// With `ui_rarityEffects 0`: seconds it takes to fade in, and to fade out.
pub(crate) const FADE_IN: f32 = 0.25;
pub(crate) const FADE_OUT: f32 = 0.3;

/// The banner, in 1080-line pixels: its size and its corners' radius.
pub(crate) const WIDTH: f32 = 600.0;
pub(crate) const HEIGHT: f32 = 112.0;
const RADIUS: f32 = 24.0;
/// Its top's distance under the screen's top edge (under the top status bar from
/// 17/11 to 21/11/2026, which is gone).
pub(crate) const MARGIN: f32 = 14.0;
/// The shadow's layers: how far each reaches out and how far it falls, each as dark as
/// [`SHADOW_ALPHA`].
const SHADOW_LAYERS: [(f32, f32); 8] = [
    (1.0, 0.5),
    (3.0, 1.5),
    (5.5, 2.5),
    (8.5, 3.5),
    (12.0, 5.0),
    (16.0, 6.5),
    (20.5, 8.0),
    (25.0, 9.0),
];
const SHADOW_ALPHA: f32 = 0.026;
/// How far its shadow reaches below it: the banner is out of sight once its top is this
/// far above the screen and its own height.
const SHADOW: f32 = 34.0;
/// How far it drifts while it fades with `ui_rarityEffects 0`.
const DRIFT: f32 = 14.0;
/// The icon's tile: its left, its side and its corners' radius.
const ICON_X: f32 = 18.0;
const ICON: f32 = 62.0;
const ICON_RADIUS: f32 = 15.0;
/// Where the words start and the room they have.
const TEXT_X: f32 = ICON_X + ICON + 16.0;
pub(crate) const TEXT_WIDTH: f32 = WIDTH - TEXT_X - 20.0;
/// The type sizes: the header's capitals (and their spacing), the title, the line under it
/// and the note under that.
pub(crate) const HEADER_SIZE: f32 = 13.0;
pub(crate) const HEADER_SPACING: f32 = 1.5;
pub(crate) const TITLE_SIZE: f32 = 21.0;
pub(crate) const BODY_SIZE: f32 = 16.0;
pub(crate) const NOTE_SIZE: f32 = 14.0;
/// The room "now" takes at the header's end, its gap included.
pub(crate) const NOW_ROOM: f32 = 44.0;
/// Each row's top and height.
const HEADER_ROW: [f32; 2] = [15.0, 16.0];
const TITLE_ROW: [f32; 2] = [33.0, 28.0];
const BODY_ROW: [f32; 2] = [61.0, 21.0];
const NOTE_ROW: [f32; 2] = [82.0, 18.0];
/// What the header ends with: banners show as things happen.
const NOW: &str = "now";

/// How long a banner lasts, coming in, resting and going.
pub(crate) fn lifetime(reduced: bool) -> f32 {
    if reduced {
        FADE_IN + HOLD + FADE_OUT
    } else {
        ENTER + HOLD + LEAVE
    }
}

/// When a banner starts going ([`lifetime`] less its way out).
pub(crate) fn leaves_at(reduced: bool) -> f32 {
    if reduced {
        FADE_IN + HOLD
    } else {
        ENTER + HOLD
    }
}

/// How the banner stands `t` seconds after it began.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pose {
    /// How far down it has come: 0 out of sight above the screen (or, faded, its drift
    /// above its place), 1 at rest; a little past 1 as the spring settles.
    pub(crate) down: f32,
    /// Its opacity.
    pub(crate) alpha: f32,
    /// The glow behind its icon against its steady strength (brighter once as it lands).
    pub(crate) glow: f32,
    /// How much of an achievement medallion's ring is drawn.
    pub(crate) sweep: f32,
}

impl Pose {
    pub(crate) fn at(t: f32, reduced: bool) -> Self {
        if reduced {
            let alpha = smooth(span(t, 0.0, FADE_IN))
                * (1.0 - smooth(span(t, FADE_IN + HOLD, lifetime(true))));
            return Self {
                down: alpha,
                alpha,
                glow: 1.0,
                sweep: 1.0,
            };
        }
        let down = if t < ENTER {
            spring(t / ENTER)
        } else if t < ENTER + HOLD {
            1.0
        } else {
            let out = span(t, ENTER + HOLD, lifetime(false));
            1.0 - out * out
        };
        let landing = span(t, ENTER * 0.5, ENTER + 0.8);
        Self {
            down,
            // In from the first moments, while it is still above the screen.
            alpha: smooth(span(t, 0.0, 0.12)),
            glow: 1.0 + 0.7 * (std::f32::consts::PI * landing).sin().max(0.0),
            sweep: ease_in_out(span(t, ENTER * 0.4, ENTER + 0.5)),
        }
    }
}

/// A damped spring from 0 to 1 over `u` from 0 to 1: it passes 1 by about 4% a little
/// after the middle, comes back and settles, exactly 1 at the end.
pub(crate) fn spring(u: f32) -> f32 {
    /// The damping ratio and the natural frequency (radians over the whole entrance).
    const ZETA: f32 = 0.72;
    const OMEGA: f32 = 7.4;
    let raw = |u: f32| {
        let damped = OMEGA * (1.0 - ZETA * ZETA).sqrt();
        let decay = (-ZETA * OMEGA * u).exp();
        1.0 - decay * ((damped * u).cos() + ZETA * OMEGA / damped * (damped * u).sin())
    };
    let u = u.clamp(0.0, 1.0);
    // What is left of the settling at the end, taken out smoothly so it lands on 1.
    raw(u) - (raw(1.0) - 1.0) * u.powi(4)
}

/// How far `t` is from `from` to `to`, from 0 to 1.
pub(crate) fn span(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

fn smooth(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

fn ease_in_out(x: f32) -> f32 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) * 0.5
    }
}

/// Window pixels per 1080-line pixel: the height's scale, smaller when the window is too
/// narrow for the banner.
pub(crate) fn scale(viewport: [f32; 2]) -> f32 {
    crate::ui_scale::height_scale(viewport[1]).min((viewport[0] - 32.0).max(1.0) / WIDTH)
}

/// The banner's place and size in the window `t` seconds after it began.
pub(crate) fn rect(viewport: [f32; 2], t: f32, reduced: bool) -> Rect {
    let s = scale(viewport);
    let rest = MARGIN * s;
    let travel = if reduced {
        DRIFT * s
    } else {
        rest + (HEIGHT + SHADOW) * s
    };
    let down = Pose::at(t, reduced).down;
    Rect::new(
        viewport[0] * 0.5 - WIDTH * s * 0.5,
        rest - (1.0 - down) * travel,
        WIDTH * s,
        HEIGHT * s,
    )
}

/// What the banner's icon tile holds.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Icon {
    /// An achievement's medallion, its ring drawn as the banner lands.
    Medallion(&'static crate::achievements::Kind),
    /// A picture, at this share of the tile.
    Picture(TextureId, f32),
    /// A holocron tier's gem in its colour (where its picture is missing).
    Gem(Color),
}

/// Draw `look` on `canvas` as it stands `t` seconds after it began.
pub(crate) fn draw(
    canvas: &mut MenuCanvas,
    look: &Look<'_>,
    t: f32,
    reduced: bool,
    viewport: [f32; 2],
) {
    let pose = Pose::at(t, reduced);
    let card = rect(viewport, t, reduced);
    let k = card.width / WIDTH;
    let hue = look.hue;
    canvas.begin_transparent(viewport);
    canvas.push_opacity(pose.alpha);
    let list = canvas.draw_list_mut();

    // A soft shadow under it, falling a little below: thin layers, so no band shows.
    for (grow, down) in SHADOW_LAYERS {
        let _ = list.push(DrawCommand::RoundedRect {
            rect: grown(card, grow * k, down * k),
            radius: (RADIUS + grow) * k,
            color: Color::new(0.0, 0.0, 0.0, SHADOW_ALPHA),
        });
    }
    // The glass: acrylic, lit along its top, and the notification's colour washed in from
    // the left.
    let _ = list.push(DrawCommand::Backdrop {
        rect: card,
        radius: RADIUS * k,
        tint: color::tint(color::acrylic::BANNER),
    });
    let _ = list.push(DrawCommand::GradientRect {
        rect: Rect::new(card.x, card.y, card.width * 0.55, card.height),
        radius: RADIUS * k,
        gradient: Gradient {
            start: color::alpha(hue, 0.13),
            end: color::alpha(hue, 0.0),
            vertical: false,
        },
    });
    // The hairline edge, and a brighter one along the top as glass catches the light.
    let _ = list.push(DrawCommand::Border {
        rect: card,
        radius: RADIUS * k,
        width: k.max(1.0),
        color: color::alpha(color::gold_bright(), 0.16),
    });
    let top = Rect::new(
        card.x + RADIUS * k,
        card.y,
        card.width - RADIUS * 2.0 * k,
        k.max(1.0),
    );
    for (half, start, end) in [
        (
            Rect::new(top.x, top.y, top.width * 0.5, top.height),
            color::alpha(color::gold_bright(), 0.0),
            color::alpha(color::gold_bright(), 0.22),
        ),
        (
            Rect::new(top.x + top.width * 0.5, top.y, top.width * 0.5, top.height),
            color::alpha(color::gold_bright(), 0.22),
            color::alpha(color::gold_bright(), 0.0),
        ),
    ] {
        let _ = list.push(DrawCommand::GradientRect {
            rect: half,
            radius: 0.0,
            gradient: Gradient {
                start,
                end,
                vertical: false,
            },
        });
    }

    // The icon's tile, with its glow behind.
    let tile = Rect::new(
        card.x + ICON_X * k,
        card.y + (HEIGHT - ICON) * 0.5 * k,
        ICON * k,
        ICON * k,
    );
    for (grow, alpha) in [(15.0, 0.014), (10.0, 0.022), (6.0, 0.03), (3.0, 0.04)] {
        let _ = list.push(DrawCommand::RoundedRect {
            rect: grown(tile, grow * k, 0.0),
            radius: (ICON_RADIUS + grow) * k,
            color: color::alpha(hue, alpha * pose.glow),
        });
    }
    let _ = list.push(DrawCommand::GradientRect {
        rect: tile,
        radius: ICON_RADIUS * k,
        gradient: Gradient {
            start: mix(color::alpha(color::glass(), 0.96), hue, 0.34),
            end: mix(color::alpha(color::space(), 0.96), hue, 0.16),
            vertical: true,
        },
    });
    let _ = list.push(DrawCommand::Border {
        rect: tile,
        radius: ICON_RADIUS * k,
        width: k.max(1.0),
        color: color::alpha(hue, 0.5),
    });
    let centre = [tile.x + tile.width * 0.5, tile.y + tile.height * 0.5];
    match look.icon {
        Icon::Medallion(kind) => medallion::draw(
            canvas,
            Medallion {
                kind,
                centre,
                radius: ICON * 0.4 * k,
                fraction: pose.sweep,
                done: true,
            },
        ),
        Icon::Picture(texture, share) => {
            let side = ICON * share * k;
            let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: Rect::new(centre[0] - side * 0.5, centre[1] - side * 0.5, side, side),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        Icon::Gem(colour) => gem::draw(
            canvas.draw_list_mut(),
            centre,
            ICON * 0.3 * k,
            colour,
            1.0,
            gem::ROWS,
        ),
    }
    // How many, on a badge at the tile's corner.
    if !look.badge.is_empty() {
        let height = 22.0 * k;
        let width = (12.0 + 8.5 * look.badge.len() as f32) * k;
        let width = width.max(height);
        let badge = Rect::new(
            tile.right() - width * 0.5 - 2.0 * k,
            tile.y - height * 0.5 + 2.0 * k,
            width,
            height,
        );
        let list = canvas.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect: grown(badge, 2.0 * k, 0.0),
            radius: height * 0.5 + 2.0 * k,
            color: color::alpha(color::space(), 0.95),
        });
        let _ = list.push(DrawCommand::RoundedRect {
            rect: badge,
            radius: height * 0.5,
            color: light(hue, 0.25),
        });
        text(
            canvas,
            TextFamily::Display,
            format_args!("{}", look.badge),
            badge,
            15.0 * k,
            color::space(),
            FontWeight::Semibold,
            TextAlign::Center,
        );
    }

    // The words: the header and "now", the title, the line under it and the note.
    let row = |[y, height]: [f32; 2], width: f32| {
        Rect::new(card.x + TEXT_X * k, card.y + y * k, width * k, height * k)
    };
    spaced(
        canvas,
        look.header,
        row(HEADER_ROW, TEXT_WIDTH - NOW_ROOM),
        HEADER_SIZE * k,
        light(hue, 0.45),
        HEADER_SPACING * k,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!("{NOW}"),
        row(HEADER_ROW, TEXT_WIDTH),
        HEADER_SIZE * k,
        color::muted(),
        FontWeight::Regular,
        TextAlign::End,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!("{}", look.title),
        row(TITLE_ROW, TEXT_WIDTH),
        TITLE_SIZE * k,
        Color::new(1.0, 1.0, 1.0, 1.0),
        FontWeight::Semibold,
        TextAlign::Start,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!("{}", look.body),
        row(BODY_ROW, TEXT_WIDTH),
        BODY_SIZE * k,
        color::text(),
        FontWeight::Regular,
        TextAlign::Start,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!("{}", look.note),
        row(NOTE_ROW, TEXT_WIDTH),
        NOTE_SIZE * k,
        look.note_colour,
        FontWeight::Regular,
        TextAlign::Start,
    );
    canvas.pop_opacity();
    canvas.finish(0);
}

/// `hue` lightened towards white by `by` (0 to 1), for words in it on the dark glass.
pub(crate) fn light(hue: Color, by: f32) -> Color {
    Color::new(
        hue.r + (1.0 - hue.r) * by,
        hue.g + (1.0 - hue.g) * by,
        hue.b + (1.0 - hue.b) * by,
        1.0,
    )
}

/// `base` with `share` of `hue` mixed in, keeping `base`'s opacity.
fn mix(base: Color, hue: Color, share: f32) -> Color {
    Color::new(
        base.r + (hue.r - base.r) * share,
        base.g + (hue.g - base.g) * share,
        base.b + (hue.b - base.b) * share,
        base.a,
    )
}

/// `rect` grown by `by` on every side and moved `down`.
fn grown(rect: Rect, by: f32, down: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by + down,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// Capitals in Rajdhani, spaced out, centred on `rect`'s middle line as [`text`] centres
/// its runs.
fn spaced(canvas: &mut MenuCanvas, value: &str, rect: Rect, size: f32, color: Color, spacing: f32) {
    let top = rect.y + rect.height * 0.5 - DISPLAY_CENTRE * size;
    canvas.set_family(TextFamily::Display);
    canvas.text_fmt_aligned(
        format_args!("{value}"),
        Rect::new(
            rect.x,
            top,
            rect.width,
            (size * 1.3).max(rect.bottom() - top),
        ),
        size,
        color,
        FontWeight::Semibold,
        spacing,
        TextAlign::Start,
    );
    canvas.set_family(TextFamily::Body);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The spring starts still, passes its place by a few pixels, comes back and lands on
    /// it exactly; nothing jumps on the way.
    #[test]
    fn the_spring_settles_on_its_place() {
        assert!(spring(0.0).abs() < 1e-6);
        assert!((spring(1.0) - 1.0).abs() < 1e-6);
        let steps: Vec<f32> = (0..=200).map(|i| spring(i as f32 / 200.0)).collect();
        let peak = steps.iter().copied().fold(0.0_f32, f32::max);
        assert!(peak > 1.02 && peak < 1.06, "{peak}");
        // At most a 148-pixel trip: the overshoot is under 9 pixels.
        assert!((peak - 1.0) * (HEIGHT + SHADOW + MARGIN) < 9.0);
        for pair in steps.windows(2) {
            assert!((pair[1] - pair[0]).abs() < 0.04, "{pair:?}");
        }
        // Rising until the peak.
        let top = steps.iter().position(|&v| v == peak).unwrap();
        assert!(steps[..=top].windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(top > 100 && top < 160, "{top}");
    }

    /// It comes down from out of sight, rests at its place for the whole hold, then goes
    /// back up out of sight without fading; faded, it only drifts.
    #[test]
    fn it_slides_down_rests_and_slides_back_up() {
        let viewport = [1920.0, 1080.0];
        let at = |t: f32| rect(viewport, t, false);
        let rest = MARGIN;
        assert!(at(0.0).bottom() <= -SHADOW + 0.01, "{:?}", at(0.0));
        for t in [ENTER, ENTER + 1.0, ENTER + HOLD] {
            assert!((at(t).y - rest).abs() < 1e-3, "{t}");
            let pose = Pose::at(t, false);
            assert!((pose.alpha - 1.0).abs() < 1e-6);
        }
        // Past its place for a moment as it lands.
        assert!((0..60).any(|i| at(ENTER * i as f32 / 60.0).y > rest + 1.0));
        // On its way out it is still opaque, and rising.
        let mut last = at(ENTER + HOLD).y;
        for i in 1..=20 {
            let t = ENTER + HOLD + LEAVE * i as f32 / 20.0;
            assert!(
                (Pose::at(t, false).alpha - 1.0).abs() < 1e-6,
                "no fade: {t}"
            );
            let y = at(t).y;
            assert!(y < last, "{t}");
            last = y;
        }
        assert!(at(lifetime(false)).bottom() <= -SHADOW + 0.01);
        // Faded: in place but for the drift, and transparent at both ends.
        assert!(Pose::at(0.0, true).alpha.abs() < 1e-6);
        assert!(Pose::at(lifetime(true), true).alpha.abs() < 1e-6);
        let faded = rect(viewport, 0.0, true);
        assert!((rest - faded.y - DRIFT).abs() < 1e-3);
        assert!((rect(viewport, FADE_IN + 1.0, true).y - rest).abs() < 1e-3);
        assert!(Pose::at(0.1, true).sweep == 1.0 && Pose::at(0.1, true).glow == 1.0);
        assert!((leaves_at(false) - (ENTER + HOLD)).abs() < 1e-6);
    }

    /// It rests in the middle of the top, under the top inset, at the SJK UI's scale; in 4K
    /// twice the size; in a narrow window it shrinks to fit.
    #[test]
    fn it_rests_at_the_top_centre_at_the_uis_scale() {
        for (viewport, s) in [([1920.0, 1080.0], 1.0), ([3840.0, 2160.0], 2.0)] {
            let card = rect(viewport, ENTER + 1.0, false);
            assert!((card.width - WIDTH * s).abs() < 1e-3);
            assert!((card.x + card.width * 0.5 - viewport[0] * 0.5).abs() < 1e-3);
            assert!((card.y - MARGIN * s).abs() < 1e-3);
            // About a third of the screen across, never its whole width.
            assert!(card.width / viewport[0] < 0.35);
        }
        let narrow = rect([480.0, 1080.0], ENTER + 1.0, false);
        assert!(narrow.x >= 0.0 && narrow.right() <= 480.0);
    }
}
