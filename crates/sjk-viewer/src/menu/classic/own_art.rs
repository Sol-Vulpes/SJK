//! The classic menus' artwork, redrawn (`ui_classicLook sjk`, `docs/classic-plus.md`).
//!
//! Retail's menus stand on pictures: a ground crossed by angular frame lines, a
//! window either side of a ring on the main page, bands behind the page bar, columns
//! of small writing down the sides. They are 640x480 art for a 4:3 screen. Here the
//! same pieces are drawn as shapes, in the theme's colours, from the window's own
//! size: lines stay one weight at any resolution, and what retail ended at the 4:3
//! picture's edge reaches the edge of a wide window instead (the frame lines, the side
//! windows, the bands, the columns).
//!
//! Places are of two kinds: canvas units of retail's 640x480 page for what the
//! entries stand on (the ring, the windows' inner ends, the bar), through
//! [`Placement`]; and the window's own edges for what frames the page.

use super::layout::{CANVAS, Placement};
use crate::menu::sjk::color;
use crate::menu_widgets::MenuCanvas;
use sjk_ui::{Color, DrawCommand, Gradient, Rect};

/// The ring's centre and radius on the canvas: round the emblem
/// ([`crate::menu::emblem::CLASSIC_RING`]).
const RING_CENTRE: [f32; 2] = [320.5, 273.0];
const RING_RADIUS: f32 = 112.0;
/// The side windows' top and bottom on the canvas, where retail's stood.
const WINDOW: [f32; 2] = [190.0, 356.0];
/// The page bar's band on the canvas (the row of PLAY, PROFILE, SETTINGS, SJK).
const BAR: [f32; 2] = [122.0, 151.0];

fn line(canvas: &mut MenuCanvas, from: [f32; 2], to: [f32; 2], width: f32, colour: Color) {
    let _ = canvas.draw_list_mut().push(DrawCommand::Line {
        from,
        to,
        width,
        color: colour,
    });
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

/// The ground every page stands on: the theme's ground over the map, deeper at the
/// top and the foot, a breath of the lit glass behind the middle.
fn ground(canvas: &mut MenuCanvas, viewport: [f32; 2]) {
    let [width, height] = viewport;
    let space = color::space();
    let draw = canvas.draw_list_mut();
    let _ = draw.push(DrawCommand::SolidRect {
        rect: Rect::new(0.0, 0.0, width, height),
        color: color::alpha(space, 0.86),
    });
    let _ = draw.push(DrawCommand::GradientRect {
        rect: Rect::new(0.0, height * 0.18, width, height * 0.36),
        radius: 0.0,
        gradient: Gradient {
            start: color::alpha(color::glass_lit(), 0.0),
            end: color::alpha(color::glass_lit(), 0.55),
            vertical: true,
        },
    });
    let _ = draw.push(DrawCommand::GradientRect {
        rect: Rect::new(0.0, height * 0.54, width, height * 0.36),
        radius: 0.0,
        gradient: Gradient {
            start: color::alpha(color::glass_lit(), 0.55),
            end: color::alpha(color::glass_lit(), 0.0),
            vertical: true,
        },
    });
    for (top, start, end) in [(0.0, 0.7, 0.0), (0.84, 0.0, 0.8)] {
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
}

/// The angular frame lines behind every page, mirrored left and right: from each
/// top corner a line runs in, drops on a slant and runs on towards the middle, and
/// the same from the foot, as the retail ground's large outlines do. They start at
/// the window's edges, so a wide window has longer runs, not empty sides.
fn frame_lines(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement) {
    let [width, height] = viewport;
    let s = place.scale;
    let faint = color::alpha(color::holo(), 0.16);
    let fainter = color::alpha(color::holo(), 0.09);
    let middle = width * 0.5;
    for side in [-1.0_f32, 1.0] {
        let at = |x: f32| middle + side * x;
        let reach = middle;
        // Top: in from the edge, down a slant, on to the middle's gap.
        let top = [
            [at(reach), 62.0 * s],
            [at(reach - 70.0 * s), 62.0 * s],
            [at(reach - 150.0 * s), 150.0 * s],
            [at(96.0 * s), 150.0 * s],
        ];
        // A second, inner outline closer to the title.
        let inner = [
            [at(reach - 40.0 * s), 18.0 * s],
            [at(reach - 150.0 * s), 18.0 * s],
            [at(reach - 205.0 * s), 78.0 * s],
        ];
        // Foot: the mirror of the top, lower and shorter.
        let foot = [
            [at(reach), height - 40.0 * s],
            [at(reach - 96.0 * s), height - 40.0 * s],
            [at(reach - 150.0 * s), height - 100.0 * s],
            [at(130.0 * s), height - 100.0 * s],
            [at(112.0 * s), height - 60.0 * s],
        ];
        for path in [&top[..], &foot[..]] {
            for pair in path.windows(2) {
                line(canvas, pair[0], pair[1], 2.5 * s, faint);
            }
        }
        for pair in inner.windows(2) {
            line(canvas, pair[0], pair[1], 2.0 * s, fainter);
        }
        // A spine down from the top run to the foot's, behind the windows.
        line(
            canvas,
            [at(reach - 150.0 * s), 150.0 * s],
            [at(reach - 150.0 * s), height - 100.0 * s],
            1.5 * s,
            fainter,
        );
    }
}

/// The columns of small writing down each side, as dashes: no alphabet is drawn,
/// only the rhythm of one, from a fixed pattern so it does not shimmer.
fn side_columns(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement) {
    let [width, height] = viewport;
    let s = place.scale;
    let colour = color::alpha(color::holo(), 0.10);
    let rows = ((height - 160.0 * s) / (9.0 * s)) as usize;
    for side in 0..2 {
        for row in 0..rows {
            // A small fixed scramble of the row's number.
            let seed = (row * 7 + side * 13) % 11;
            if seed == 3 || seed == 8 {
                continue;
            }
            let length = (10.0 + (seed as f32) * 2.6) * s;
            let y = 86.0 * s + row as f32 * 9.0 * s;
            let x = if side == 0 {
                10.0 * s
            } else {
                width - 10.0 * s - length
            };
            fill(canvas, Rect::new(x, y, length, 3.0 * s), 1.0 * s, colour);
            if seed % 3 == 0 {
                let x = if side == 0 {
                    10.0 * s + length + 4.0 * s
                } else {
                    width - 10.0 * s - length - 10.0 * s
                };
                fill(canvas, Rect::new(x, y, 6.0 * s, 3.0 * s), 1.0 * s, colour);
            }
        }
    }
}

/// One of the main page's side windows: a glass panel from the window's edge margin
/// to the ring, framed in the line-work, with the slot across its middle that
/// retail's windows have between their two entries.
fn side_window(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement, left: bool) {
    let s = place.scale;
    let ring = place.rect([RING_CENTRE[0] - RING_RADIUS, 0.0, RING_RADIUS * 2.0, 0.0]);
    let top = place.rect([0.0, WINDOW[0], 0.0, 0.0]).y;
    let bottom = place.rect([0.0, WINDOW[1], 0.0, 0.0]).y;
    let margin = 28.0 * s;
    // The window overlaps the ring a little; the ring's disc is drawn over it.
    let (x, right) = if left {
        (margin, ring.x + 26.0 * s)
    } else {
        (ring.x + ring.width - 26.0 * s, viewport[0] - margin)
    };
    let rect = Rect::new(x, top, right - x, bottom - top);
    let radius = 5.0 * s;
    fill(canvas, rect, radius, color::alpha(color::glass(), 0.78));
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
        color::alpha(color::holo(), 0.10),
    );
    edge(
        canvas,
        rect,
        radius,
        2.5 * s,
        color::alpha(color::holo(), 0.62),
    );
    // The slot: a framed bar from the outer side to the ring, between the entries.
    let slot_height = 28.0 * s;
    let slot_y = rect.y + (rect.height - slot_height) * 0.5;
    let inset = 38.0 * s;
    let slot = if left {
        Rect::new(rect.x + inset, slot_y, rect.width - inset, slot_height)
    } else {
        Rect::new(rect.x, slot_y, rect.width - inset, slot_height)
    };
    fill(canvas, slot, 4.0 * s, color::alpha(color::space(), 0.55));
    edge(
        canvas,
        slot,
        4.0 * s,
        2.0 * s,
        color::alpha(color::holo(), 0.42),
    );
}

/// The main page's ring round the emblem: a dark disc, two rings of the line-work,
/// heavier arcs at the shoulders, and the run of ticks under it.
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
    fill(canvas, disc, radius, color::alpha(color::space(), 0.94));
    let holo = color::holo();
    arc(
        canvas,
        centre,
        radius,
        3.0 * s,
        [0.0, TAU],
        color::alpha(holo, 0.70),
    );
    arc(
        canvas,
        centre,
        radius - 9.0 * s,
        1.5 * s,
        [0.0, TAU],
        color::alpha(holo, 0.30),
    );
    arc(
        canvas,
        centre,
        radius + 8.0 * s,
        1.5 * s,
        [0.0, TAU],
        color::alpha(holo, 0.16),
    );
    // Shoulders: a heavy arc upper left and upper right, clear of the windows.
    for start in [PI * 1.16, PI * 1.60] {
        arc(
            canvas,
            centre,
            radius + 8.0 * s,
            5.0 * s,
            [start, PI * 0.24],
            color::alpha(holo, 0.55),
        );
    }
    // The ticks under the ring, on the lower half.
    let ticks = 26;
    for tick in 0..ticks {
        let start = PI * 0.20 + (PI * 0.60) * (tick as f32 / ticks as f32);
        arc(
            canvas,
            centre,
            radius + 17.0 * s,
            5.0 * s,
            [start, PI * 0.60 / ticks as f32 * 0.55],
            color::alpha(holo, 0.45),
        );
    }
    // The cross hairs through the disc, behind the emblem.
    let reach = radius - 9.0 * s;
    for turn in 0..8 {
        let angle = turn as f32 * PI / 8.0;
        let (sin, cos) = angle.sin_cos();
        line(
            canvas,
            [centre[0] - cos * reach, centre[1] - sin * reach],
            [centre[0] + cos * reach, centre[1] + sin * reach],
            1.0 * s,
            color::alpha(holo, 0.10),
        );
    }
}

/// The main page's artwork: the ground and its frame lines, the side columns, a
/// window either side of the ring, and the ring.
pub(crate) fn main_page(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement) {
    ground(canvas, viewport);
    frame_lines(canvas, viewport, place);
    side_columns(canvas, viewport, place);
    side_window(canvas, viewport, place, true);
    side_window(canvas, viewport, place, false);
    ring(canvas, place);
}

/// A sub-page's artwork: the ground and its frame lines, the side columns, and the
/// page bar's band from edge to edge between two rules.
pub(crate) fn sub_page(canvas: &mut MenuCanvas, viewport: [f32; 2], place: &Placement) {
    ground(canvas, viewport);
    frame_lines(canvas, viewport, place);
    side_columns(canvas, viewport, place);
    let s = place.scale;
    let top = place.rect([0.0, BAR[0], 0.0, 0.0]).y;
    let bottom = place.rect([0.0, BAR[1], 0.0, 0.0]).y;
    let width = viewport[0];
    let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
        rect: Rect::new(0.0, top, width, bottom - top),
        color: color::alpha(color::glass(), 0.72),
    });
    for y in [top, bottom] {
        let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
            rect: Rect::new(0.0, y - 1.5 * s, width, 3.0 * s),
            color: color::alpha(color::holo(), 0.60),
        });
    }
}

// The ring sits between the windows' top and bottom, on the canvas.
const _: () =
    assert!(RING_CENTRE[1] - RING_RADIUS < WINDOW[0] && RING_CENTRE[1] + RING_RADIUS > WINDOW[1]);
const _: () = assert!(BAR[1] < WINDOW[0] && RING_CENTRE[0] < CANVAS[0]);
