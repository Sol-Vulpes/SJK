//! The classic menus' artwork, redrawn (`ui_classicLook sjk`, `docs/classic-plus.md`).
//!
//! Retail's menus stand on pictures: a ground crossed by angular frame lines, a
//! window either side of a ring on the main page, bands behind the page bar, columns
//! of small writing down the sides. They are 640x480 art for a 4:3 screen. Here the
//! same pieces are drawn as shapes, in the theme's colours, so a line stays one
//! weight at any resolution.
//!
//! The menu itself keeps retail's proportions: the windows, the ring and the bar
//! stand where they stood on the 640x480 page ([`Placement`]), centred. A wide window
//! is not filled by stretching them (a first version did, 22/11/2026, and Sol found
//! the menu no longer looked like itself): what a 16:9 screen shows beside the page is
//! more of the room the page stands in, imagined: the frame lines running on to the
//! screen's corners, bulkhead plates with their own outlines, conduits from the
//! windows to the edges, further columns of writing.

use super::layout::{CANVAS, Placement};
use crate::menu::art::{ArtPiece, ArtSet, motion};
use crate::menu::sjk::color;
use crate::menu_widgets::MenuCanvas;
use sjk_ui::{Color, DrawCommand, Gradient, Rect};

/// The ring's centre and radius on the canvas: round the emblem
/// ([`crate::menu::emblem::CLASSIC_RING`]).
const RING_CENTRE: [f32; 2] = [320.5, 273.0];
const RING_RADIUS: f32 = 112.0;
/// The side windows on the canvas, where retail's stand: the left one's outer and
/// inner ends (the right is its mirror), and their top and bottom.
const WINDOW_X: [f32; 2] = [29.0, 232.0];
const WINDOW_Y: [f32; 2] = [190.0, 356.0];
/// The page bar's band on the canvas (the row of PLAY, PROFILE, SETTINGS, SJK).
const BAR: [f32; 2] = [122.0, 151.0];
/// A column of writing on the canvas: its width, top and height.
const COLUMN: [f32; 3] = [20.0, 64.0, 352.0];

fn line(canvas: &mut MenuCanvas, from: [f32; 2], to: [f32; 2], width: f32, colour: Color) {
    let _ = canvas.draw_list_mut().push(DrawCommand::Line {
        from,
        to,
        width,
        color: colour,
    });
}

fn path(canvas: &mut MenuCanvas, points: &[[f32; 2]], width: f32, colour: Color) {
    for pair in points.windows(2) {
        line(canvas, pair[0], pair[1], width, colour);
    }
}

fn fill(canvas: &mut MenuCanvas, rect: Rect, radius: f32, colour: Color) {
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect,
        radius,
        color: colour,
    });
}

fn edge(canvas: &mut MenuCanvas, rect: Rect, radius: f32, width: f32, colour: Color) {
    let _ = canvas.draw_list_mut().push(DrawCommand::Border {
        rect,
        radius,
        width,
        color: colour,
    });
}

fn arc(
    canvas: &mut MenuCanvas,
    centre: [f32; 2],
    radius: f32,
    width: f32,
    turn: [f32; 2],
    colour: Color,
) {
    let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
        center: centre,
        radius,
        width,
        start: turn[0],
        sweep: turn[1],
        color: colour,
        knockout: None,
    });
}

fn holo(alpha: f32) -> Color {
    color::alpha(color::holo(), alpha)
}

/// The ground every page stands on: the theme's ground over the map, deeper at the
/// top and the foot, a breath of the lit glass behind the middle, darker towards a
/// wide window's sides so the page is where the light is.
fn ground(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement) {
    let [width, height] = viewport;
    let space = color::space();
    let draw = canvas.draw_list_mut();
    let _ = draw.push(DrawCommand::SolidRect {
        rect: Rect::new(0.0, 0.0, width, height),
        color: color::alpha(space, 0.86),
    });
    for (top, start, end) in [(0.16, 0.0, 0.6), (0.54, 0.6, 0.0)] {
        let _ = draw.push(DrawCommand::GradientRect {
            rect: Rect::new(0.0, height * top, width, height * 0.38),
            radius: 0.0,
            gradient: Gradient {
                start: color::alpha(color::glass_lit(), start),
                end: color::alpha(color::glass_lit(), end),
                vertical: true,
            },
        });
    }
    for (top, start, end) in [(0.0, 0.75, 0.0), (0.84, 0.0, 0.85)] {
        let _ = draw.push(DrawCommand::GradientRect {
            rect: Rect::new(0.0, height * top, width, height * 0.16),
            radius: 0.0,
            gradient: Gradient {
                start: color::alpha(space, start),
                end: color::alpha(space, end),
                vertical: true,
            },
        });
    }
    // Beside the 4:3 page, the room falls into shadow towards the screen's edges.
    let side = place.origin[0];
    if side > 1.0 {
        for (x, start, end) in [(0.0, 0.62, 0.0), (width - side, 0.0, 0.62)] {
            let _ = draw.push(DrawCommand::GradientRect {
                rect: Rect::new(x, 0.0, side, height),
                radius: 0.0,
                gradient: Gradient {
                    start: color::alpha(space, start),
                    end: color::alpha(space, end),
                    vertical: false,
                },
            });
        }
    }
}

/// The angular frame lines of the page, mirrored left and right, on retail's 4:3
/// page: from each top corner of the page a line runs in, drops on a slant and runs
/// on towards the title; the same from the foot.
fn frame_lines(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement) {
    let height = viewport[1];
    let s = place.scale;
    let middle = viewport[0] * 0.5;
    let half = CANVAS[0] * 0.5 * s;
    for side in [-1.0_f32, 1.0] {
        let at = |x: f32| middle + side * x;
        path(
            canvas,
            &[
                [at(half), 62.0 * s],
                [at(half - 70.0 * s), 62.0 * s],
                [at(half - 150.0 * s), 150.0 * s],
                [at(96.0 * s), 150.0 * s],
            ],
            2.5 * s,
            holo(0.17),
        );
        path(
            canvas,
            &[
                [at(half - 40.0 * s), 18.0 * s],
                [at(half - 150.0 * s), 18.0 * s],
                [at(half - 205.0 * s), 78.0 * s],
            ],
            2.0 * s,
            holo(0.10),
        );
        path(
            canvas,
            &[
                [at(half), height - 40.0 * s],
                [at(half - 96.0 * s), height - 40.0 * s],
                [at(half - 150.0 * s), height - 100.0 * s],
                [at(130.0 * s), height - 100.0 * s],
                [at(112.0 * s), height - 60.0 * s],
            ],
            2.5 * s,
            holo(0.17),
        );
        line(
            canvas,
            [at(half - 150.0 * s), 150.0 * s],
            [at(half - 150.0 * s), height - 100.0 * s],
            1.5 * s,
            holo(0.09),
        );
    }
}

/// What a wide window shows beside the 4:3 page: the room going on. The page's frame
/// lines run out to the screen's corners; a bulkhead plate stands each side, cut at
/// its corners, with a rib and a row of studs; a conduit leaves each window for the
/// edge. Nothing is drawn on a window no wider than the page.
fn beyond(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement) {
    let [width, height] = viewport;
    let s = place.scale;
    let side = place.origin[0];
    if side < 24.0 * s {
        return;
    }
    let middle = width * 0.5;
    let half = CANVAS[0] * 0.5 * s;
    let window_y = |y: f32| place.rect([0.0, y, 0.0, 0.0]).y;
    for sign in [-1.0_f32, 1.0] {
        let at = |x: f32| middle + sign * x;
        let reach = middle;
        // The page's top and foot lines, carried to the screen's edge.
        line(
            canvas,
            [at(half), 62.0 * s],
            [at(reach), 62.0 * s],
            2.5 * s,
            holo(0.17),
        );
        line(
            canvas,
            [at(half), height - 40.0 * s],
            [at(reach), height - 40.0 * s],
            2.5 * s,
            holo(0.17),
        );
        // The bulkhead plate: a tall panel with cut corners between the page and
        // the edge.
        let inner = half + 16.0 * s;
        let outer = reach - 14.0 * s;
        if outer - inner > 40.0 * s {
            let cut = 16.0 * s;
            let (top, bottom) = (96.0 * s, height - 76.0 * s);
            let plate = [
                [at(inner + cut), top],
                [at(outer - cut), top],
                [at(outer), top + cut],
                [at(outer), bottom - cut],
                [at(outer - cut), bottom],
                [at(inner + cut), bottom],
                [at(inner), bottom - cut],
                [at(inner), top + cut],
                [at(inner + cut), top],
            ];
            // A faint fill, as a plate catching a little light.
            let left = at(inner).min(at(outer));
            fill(
                canvas,
                Rect::new(left, top, outer - inner, bottom - top),
                cut,
                color::alpha(color::glass(), 0.30),
            );
            path(canvas, &plate, 2.0 * s, holo(0.20));
            // A rib inside it, and a second, shorter one.
            let rib = inner + (outer - inner) * 0.42;
            line(
                canvas,
                [at(rib), top + 26.0 * s],
                [at(rib), bottom - 26.0 * s],
                1.5 * s,
                holo(0.12),
            );
            line(
                canvas,
                [at(rib + 7.0 * s), top + 60.0 * s],
                [at(rib + 7.0 * s), bottom - 60.0 * s],
                1.0 * s,
                holo(0.08),
            );
            // Studs along the plate's outer run.
            let studs = 9;
            for stud in 0..studs {
                let y =
                    top + 34.0 * s + (bottom - top - 68.0 * s) * stud as f32 / (studs - 1) as f32;
                let x = at(outer - 9.0 * s);
                fill(
                    canvas,
                    Rect::new(x - 1.5 * s, y - 1.5 * s, 3.0 * s, 3.0 * s),
                    1.5 * s,
                    holo(0.30),
                );
            }
        }
        // A conduit from the window's outer end to the edge: two rules and a node.
        let from = half - WINDOW_X[0] * s;
        for y in [window_y(WINDOW_Y[0] + 30.0), window_y(WINDOW_Y[1] - 30.0)] {
            line(canvas, [at(from), y], [at(reach), y], 1.5 * s, holo(0.22));
            line(
                canvas,
                [at(from), y + 5.0 * s],
                [at(reach), y + 5.0 * s],
                1.0 * s,
                holo(0.10),
            );
            let node = at(from + (reach - from) * 0.5);
            let rect = Rect::new(node - 5.0 * s, y - 3.0 * s, 10.0 * s, 11.0 * s);
            fill(canvas, rect, 2.0 * s, color::alpha(color::space(), 0.9));
            edge(canvas, rect, 2.0 * s, 1.5 * s, holo(0.40));
        }
    }
}

/// The columns of writing down the sides, in Aurebesh
/// ([`super::own_writing`]), tinted with the line-work and climbing slowly as
/// retail's do: one at each side of the 4:3 page, and on a wide window one more at
/// each edge of the screen. Without the picture (no font), dashes stand for them.
fn writing(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement, art: ArtSet) {
    let s = place.scale;
    let [column_width, top, column_height] = COLUMN;
    let y = place.rect([0.0, top, 0.0, 0.0]).y;
    let (w, h) = (column_width * s, column_height * s);
    let page = [
        (place.rect([5.0, 0.0, 0.0, 0.0]).x, 0.34),
        (
            place
                .rect([CANVAS[0] - 5.0 - column_width, 0.0, 0.0, 0.0])
                .x,
            0.34,
        ),
    ];
    let wide = place.origin[0] > 80.0 * s;
    let edges = [(20.0 * s, 0.22), (viewport[0] - 20.0 * s - w, 0.22)];
    let columns = page.into_iter().chain(edges.into_iter().filter(|_| wide));
    for (index, (x, alpha)) in columns.enumerate() {
        let rect = Rect::new(x, y, w, h);
        if art.has(ArtPiece::OwnWriting) {
            // The picture is eight times as tall as it is wide: show a stretch of it
            // at its own proportions, each column starting elsewhere along it.
            let span = (h / w) / 8.0;
            let scroll = motion::scroll(motion::SIDE_SCROLL, motion::seconds());
            let start = scroll[0][1] + index as f32 * 0.27;
            let uv = [
                [0.0, start],
                [1.0, start],
                [1.0, start + span],
                [0.0, start + span],
            ];
            let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuadUv {
                rect,
                texture: ArtPiece::OwnWriting.texture(),
                color: holo(alpha),
                uv,
            });
        } else {
            let rows = (h / (9.0 * s)) as usize;
            for row in 0..rows {
                let seed = (row * 7 + index * 13) % 11;
                if seed == 3 || seed == 8 {
                    continue;
                }
                let length = (10.0 + seed as f32 * 2.2) * s;
                fill(
                    canvas,
                    Rect::new(x, y + row as f32 * 9.0 * s, length.min(w), 3.0 * s),
                    1.0 * s,
                    holo(alpha * 0.4),
                );
            }
        }
    }
}

/// One of the main page's side windows, where retail's stands: a glass panel framed
/// in the line-work, with the slot across its middle between its two entries.
fn side_window(canvas: &mut MenuCanvas, place: &Placement, left: bool) {
    let s = place.scale;
    let (x, width) = (WINDOW_X[0], WINDOW_X[1] - WINDOW_X[0]);
    let x = if left { x } else { CANVAS[0] - x - width };
    let rect = place.rect([x, WINDOW_Y[0], width, WINDOW_Y[1] - WINDOW_Y[0]]);
    let radius = 5.0 * s;
    fill(canvas, rect, radius, color::alpha(color::glass(), 0.80));
    // A soft glow inside the frame, then the frame.
    edge(
        canvas,
        Rect::new(
            rect.x + 3.0 * s,
            rect.y + 3.0 * s,
            rect.width - 6.0 * s,
            rect.height - 6.0 * s,
        ),
        radius,
        3.0 * s,
        holo(0.10),
    );
    edge(canvas, rect, radius, 2.5 * s, holo(0.66));
    // The slot: a framed bar from the outer side towards the ring.
    let slot_height = 26.0 * s;
    let slot_y = rect.y + (rect.height - slot_height) * 0.5;
    let inset = 36.0 * s;
    let slot = if left {
        Rect::new(rect.x + inset, slot_y, rect.width - inset, slot_height)
    } else {
        Rect::new(rect.x, slot_y, rect.width - inset, slot_height)
    };
    fill(canvas, slot, 4.0 * s, color::alpha(color::space(), 0.55));
    edge(canvas, slot, 4.0 * s, 2.0 * s, holo(0.44));
}

/// The main page's ring round the emblem: a dark disc, three rings of the
/// line-work, a heavy arc at each shoulder, and the run of ticks under it.
fn ring(canvas: &mut MenuCanvas, place: &Placement) {
    use std::f32::consts::{PI, TAU};
    let s = place.scale;
    let centre_rect = place.rect([RING_CENTRE[0], RING_CENTRE[1], 0.0, 0.0]);
    let centre = [centre_rect.x, centre_rect.y];
    let radius = RING_RADIUS * s;
    let disc = Rect::new(
        centre[0] - radius,
        centre[1] - radius,
        radius * 2.0,
        radius * 2.0,
    );
    fill(canvas, disc, radius, color::alpha(color::space(), 0.95));
    arc(canvas, centre, radius, 3.0 * s, [0.0, TAU], holo(0.72));
    arc(
        canvas,
        centre,
        radius - 9.0 * s,
        1.5 * s,
        [0.0, TAU],
        holo(0.30),
    );
    arc(
        canvas,
        centre,
        radius + 8.0 * s,
        1.5 * s,
        [0.0, TAU],
        holo(0.16),
    );
    for start in [PI * 1.16, PI * 1.60] {
        arc(
            canvas,
            centre,
            radius + 8.0 * s,
            5.0 * s,
            [start, PI * 0.24],
            holo(0.55),
        );
    }
    let ticks = 26;
    for tick in 0..ticks {
        let start = PI * 0.20 + (PI * 0.60) * (tick as f32 / ticks as f32);
        arc(
            canvas,
            centre,
            radius + 17.0 * s,
            5.0 * s,
            [start, PI * 0.60 / ticks as f32 * 0.55],
            holo(0.45),
        );
    }
    let reach = radius - 9.0 * s;
    for turn in 0..8 {
        let angle = turn as f32 * PI / 8.0;
        let (sin, cos) = angle.sin_cos();
        line(
            canvas,
            [centre[0] - cos * reach, centre[1] - sin * reach],
            [centre[0] + cos * reach, centre[1] + sin * reach],
            1.0 * s,
            holo(0.10),
        );
    }
}

/// The main page's artwork: the ground, the room beyond the page on a wide window,
/// the page's frame lines and columns of writing, a window either side of the ring,
/// and the ring.
pub(crate) fn main_page(
    canvas: &mut MenuCanvas,
    viewport: [f32; 2],
    place: &Placement,
    art: ArtSet,
) {
    ground(canvas, viewport, place);
    beyond(canvas, viewport, place);
    frame_lines(canvas, viewport, place);
    writing(canvas, viewport, place, art);
    side_window(canvas, place, true);
    side_window(canvas, place, false);
    ring(canvas, place);
}

/// A sub-page's artwork: the ground, the room beyond the page, the frame lines and
/// the writing, and the page bar's band across the page between two rules, which
/// run on, fainter, to a wide window's edges.
pub(crate) fn sub_page(
    canvas: &mut MenuCanvas,
    viewport: [f32; 2],
    place: &Placement,
    art: ArtSet,
) {
    ground(canvas, viewport, place);
    beyond(canvas, viewport, place);
    frame_lines(canvas, viewport, place);
    writing(canvas, viewport, place, art);
    let s = place.scale;
    let band = place.rect([0.0, BAR[0], CANVAS[0], BAR[1] - BAR[0]]);
    let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
        rect: band,
        color: color::alpha(color::glass(), 0.74),
    });
    for y in [band.y, band.y + band.height] {
        let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
            rect: Rect::new(0.0, y - 1.0 * s, viewport[0], 2.0 * s),
            color: holo(0.20),
        });
        let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
            rect: Rect::new(band.x, y - 1.5 * s, band.width, 3.0 * s),
            color: holo(0.62),
        });
    }
}

// The ring sits between the windows, which stand under the bar, on the canvas.
const _: () = assert!(
    RING_CENTRE[1] - RING_RADIUS < WINDOW_Y[0] && RING_CENTRE[1] + RING_RADIUS > WINDOW_Y[1]
);
const _: () = assert!(BAR[1] < WINDOW_Y[0] && WINDOW_X[1] < RING_CENTRE[0]);
// The page's columns of writing stand clear of the windows.
const _: () = assert!(COLUMN[1] + COLUMN[2] < CANVAS[1] && 5.0 + COLUMN[0] < WINDOW_X[0]);
