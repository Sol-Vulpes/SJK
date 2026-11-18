//! One tier frame everywhere (`docs/sjk-ui.md`, "Tier frame"; Sol's pick of idea 2 of the
//! Collection's ideas round, 15/11/2026): a thing's rarity told the same way on every page
//! that lists things, at three sizes ([`TierSize`]: a tile, a list row, a short line), each
//! step up the tiers adding one touch to the one under it, so the tier reads at a glance,
//! for a colour-blind player too:
//!
//! - **Common:** plain glass and a holo hairline.
//! - **Uncommon:** the tier's bar (along a tile's top, down a row's start).
//! - **Rare:** the edge tinted the tier's colour and a soft glow from the bar.
//! - **Legendary:** corner brackets and a faint halo round the frame.
//! - **Mythical:** the full edge in the tier's colour, lit inside too, the bar along the
//!   whole top and, only on the thing chosen, a slow sheen crossing it now and then (off
//!   with `ui_rarityEffects 0`, [`crate::rarity_fx`]).
//! - **Unique:** outside the ladder (hand-given, never dropped): a rose edge and an inner
//!   line of its own, and its own mark.
//!
//! [`tier_mark`] draws the step as diamonds (one for Uncommon to four for Mythical, the
//! holocron tiers' order; none for Common) and Unique's SJK Symbols mark, so the tier
//! survives a colour-blind player, a greyed thing not held and a black-and-white picture.
//! The thing chosen takes a gold wash over its glass (the caller rings it gold), so a
//! Mythical frame, gold itself, never passes for the choice. Everything is the UI's own
//! shapes; a frame is at most a couple of dozen commands.

use super::{Frame, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::unlockables::Rarity;
use sjk_ui::{Color, DrawCommand, FontWeight, Gradient, TextAlign};

/// How large a frame is drawn: what the tier adds keeps its meaning, scaled to the room.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TierSize {
    /// A tile or a card (the Cards, Tags and Forge grids, the Holocrons tab's tiles, the
    /// Inventory's item card): the bar along its top.
    Tile,
    /// A list's row (Inventory, History, the Forge's list): the bar down its start.
    Row,
    /// A short line (a summary's line): a bar, the tint and the mark only.
    Line,
}

impl TierSize {
    /// The frame's corner radius, frame pixels.
    pub(crate) const fn radius(self) -> f32 {
        match self {
            Self::Tile => 12.0,
            Self::Row => 8.0,
            Self::Line => 6.0,
        }
    }
}

/// How a frame is drawn this frame, besides its tier and size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TierLook {
    /// The thing is the one chosen: its glass takes a gold wash, and Mythical's sheen runs
    /// on it alone (the caller draws its gold ring round it).
    pub(crate) chosen: bool,
    /// Mythical's sheen runs though it is not a choice among others: the thing shown on its
    /// own (the Inventory's item card).
    pub(crate) shine: bool,
    /// The pointer rests on it: the glass a touch lighter.
    pub(crate) hovered: bool,
    /// Not held: everything faint (the tier still reads).
    pub(crate) dim: bool,
    /// Fill it with the dark glass; `false` over a picture that fills it already.
    pub(crate) glass: bool,
    /// Never move (a page whose [`crate::rarity_fx`] effects already animate it).
    pub(crate) still: bool,
    /// The UI's clock, for the sheen.
    pub(crate) seconds: f32,
    /// A first look's flash over the frame, 0 (none) to 1, in the tier's colour
    /// (`collection_reveal.rs`; the caller passes 0 with `ui_rarityEffects 0`).
    pub(crate) flash: f32,
}

impl TierLook {
    /// A frame of glass at `seconds`, not chosen, held.
    pub(crate) const fn at(seconds: f32) -> Self {
        Self {
            chosen: false,
            shine: false,
            hovered: false,
            dim: false,
            glass: true,
            still: false,
            seconds,
            flash: 0.0,
        }
    }
}

/// The tier's step on the ladder, what [`tier_mark`] counts: 0 for Common, 1 (Uncommon)
/// to 4 (Mythical); Unique, outside the ladder, has none (its own mark).
pub(crate) const fn tier_step(tier: Rarity) -> u8 {
    match tier {
        Rarity::Common | Rarity::Unique => 0,
        Rarity::Uncommon => 1,
        Rarity::Rare => 2,
        Rarity::Legendary => 3,
        Rarity::Mythical => 4,
    }
}

/// The tier of a holocron or dormant kyber of tier `index` (0 Uncommon to 3 Mythical,
/// [`crate::holocrons::TIERS`] and [`crate::kyber::TIERS`]).
pub(crate) fn holocron_tier(index: usize) -> Rarity {
    [
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Legendary,
        Rarity::Mythical,
    ][index.min(3)]
}

/// The tier of a card of rarity `index` ([`crate::cards::RARITIES`]: 0 Common to 4
/// Mythical).
pub(crate) fn card_tier(index: usize) -> Rarity {
    [
        Rarity::Common,
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Legendary,
        Rarity::Mythical,
    ][index.min(4)]
}

/// One sheen's period and the share of it the band takes to cross.
const SHEEN_SECONDS: f32 = 6.0;
const SHEEN_CROSS: f32 = 0.25;

fn push(canvas: &mut MenuCanvas, command: DrawCommand) {
    let _ = canvas.draw_list_mut().push(command);
}

/// `colour` at `alpha` times its own.
fn faded(colour: Color, alpha: f32) -> Color {
    Color::new(colour.r, colour.g, colour.b, colour.a * alpha)
}

/// Tier `tier`'s frame over `rect` (frame pixels) at `size`, as `look` says. The caller
/// draws the thing over it and, when chosen, its own gold ring round it.
pub(crate) fn tier_frame(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    tier: Rarity,
    size: TierSize,
    look: TierLook,
) {
    let [x, y, width, height] = rect;
    let s = frame.s;
    let radius = size.radius();
    let colour = tier.colour();
    let step = tier_step(tier);
    let ink = if look.dim { 0.42 } else { 1.0 };
    let area = frame.rect(x, y, width, height);
    if look.glass {
        push(
            canvas,
            DrawCommand::RoundedRect {
                rect: area,
                radius: radius * s,
                color: super::color::alpha(
                    super::color::space(),
                    if look.hovered { 0.8 } else { 0.62 },
                ),
            },
        );
        // The thing chosen: a gold wash, the kit's colour for what is chosen, so a
        // Mythical frame (gold too) never passes for the choice.
        if look.chosen {
            push(
                canvas,
                DrawCommand::RoundedRect {
                    rect: area,
                    radius: radius * s,
                    color: super::color::alpha(super::color::gold(), 0.1),
                },
            );
        }
    }
    // The glow from the bar (Rare on), under everything else.
    if step >= 2 && size != TierSize::Line {
        let glow = faded(colour, 0.16 * ink);
        let (rect, vertical) = match size {
            TierSize::Tile => (
                frame.rect(x + 1.0, y + 1.0, width - 2.0, (height * 0.42).min(64.0)),
                true,
            ),
            _ => (
                frame.rect(x + 1.0, y + 1.0, (width * 0.32).min(220.0), height - 2.0),
                false,
            ),
        };
        push(
            canvas,
            DrawCommand::GradientRect {
                rect,
                radius: radius * s,
                gradient: Gradient {
                    start: glow,
                    end: faded(colour, 0.0),
                    vertical,
                },
            },
        );
    }
    // The halo round it (Legendary on).
    if step >= 3 && size != TierSize::Line {
        let out = if size == TierSize::Tile { 3.0 } else { 2.0 };
        push(
            canvas,
            DrawCommand::Border {
                rect: frame.rect(x - out, y - out, width + out * 2.0, height + out * 2.0),
                radius: (radius + out) * s,
                width: 2.0 * s,
                color: faded(colour, 0.2 * ink),
            },
        );
    }
    // The edge: holo below Rare, the tier's colour from Rare on, full at Mythical.
    let (edge, edge_width) = match tier {
        Rarity::Common | Rarity::Uncommon => (
            super::color::alpha(super::color::holo(), if look.hovered { 0.34 } else { 0.18 }),
            1.0,
        ),
        Rarity::Rare | Rarity::Legendary => (faded(colour, 0.5 * ink), 1.2),
        Rarity::Mythical => (faded(colour, 0.95 * ink), 1.6),
        Rarity::Unique => (faded(colour, 0.75 * ink), 1.3),
    };
    push(
        canvas,
        DrawCommand::Border {
            rect: area,
            radius: radius * s,
            width: edge_width * s,
            color: edge,
        },
    );
    // Mythical's edge is lit inside too: a hairline of its colour just within.
    if step == 4 {
        push(
            canvas,
            DrawCommand::Border {
                rect: frame.rect(x + 2.0, y + 2.0, width - 4.0, height - 4.0),
                radius: (radius - 2.0) * s,
                width: s,
                color: faded(colour, 0.35 * ink),
            },
        );
    }
    // The bar (Uncommon on): along a tile's top, down a row's or a line's start; at
    // Mythical the tile's runs its whole top.
    if step >= 1 {
        let bar = match size {
            TierSize::Tile if step == 4 => frame.rect(x + radius, y, width - radius * 2.0, 3.0),
            TierSize::Tile => frame.rect(x + 16.0, y, width - 32.0, 3.0),
            TierSize::Row => frame.rect(x + 7.0, y + height * 0.22, 4.0, height * 0.56),
            TierSize::Line => frame.rect(x + 4.0, y + height * 0.25, 3.0, height * 0.5),
        };
        push(
            canvas,
            DrawCommand::RoundedRect {
                rect: bar,
                radius: bar.width.min(bar.height) * 0.5,
                color: faded(colour, ink),
            },
        );
    }
    // The corner brackets (Legendary on).
    if step >= 3 && size != TierSize::Line {
        let (arm, thick, inset) = match size {
            TierSize::Tile => (16.0, 2.0, 5.0),
            _ => (9.0, 1.6, 4.0),
        };
        let bracket = faded(colour, 0.9 * ink);
        let left = x + inset;
        let right = x + width - inset;
        let top = y + inset;
        let bottom = y + height - inset;
        for (corner, across, down) in [
            ([left, top], 1.0, 1.0),
            ([right, top], -1.0, 1.0),
            ([left, bottom], 1.0, -1.0),
            ([right, bottom], -1.0, -1.0),
        ] {
            for to in [
                [corner[0] + arm * across, corner[1]],
                [corner[0], corner[1] + arm * down],
            ] {
                push(
                    canvas,
                    DrawCommand::Line {
                        from: frame.point(corner[0], corner[1]),
                        to: frame.point(to[0], to[1]),
                        width: thick * s,
                        color: bracket,
                    },
                );
            }
        }
    }
    // Unique's inner line, its own.
    if tier == Rarity::Unique {
        let inset = if size == TierSize::Tile { 4.0 } else { 3.0 };
        push(
            canvas,
            DrawCommand::Border {
                rect: frame.rect(
                    x + inset,
                    y + inset,
                    width - inset * 2.0,
                    height - inset * 2.0,
                ),
                radius: (radius - inset).max(2.0) * s,
                width: s,
                color: faded(colour, 0.5 * ink),
            },
        );
    }
    // A first look's flash: the edge lit in the tier's colour (holo for Common) and a
    // halo round it, fading out.
    if look.flash > 0.0 {
        let flash = look.flash.min(1.0);
        let lit = if step == 0 {
            super::color::holo()
        } else {
            colour
        };
        push(
            canvas,
            DrawCommand::RoundedRect {
                rect: area,
                radius: radius * s,
                color: faded(lit, 0.16 * flash),
            },
        );
        push(
            canvas,
            DrawCommand::Border {
                rect: area,
                radius: radius * s,
                width: 2.4 * s,
                color: faded(lit, flash),
            },
        );
        let out = 4.0 + 6.0 * (1.0 - flash);
        push(
            canvas,
            DrawCommand::Border {
                rect: frame.rect(x - out, y - out, width + out * 2.0, height + out * 2.0),
                radius: (radius + out) * s,
                width: 3.0 * s,
                color: faded(lit, 0.45 * flash),
            },
        );
    }
    // Mythical's sheen, on the thing chosen alone, now and then.
    if step == 4
        && (look.chosen || look.shine)
        && !look.still
        && !look.dim
        && crate::rarity_fx::enabled()
        && let Some(along) = sheen_at(look.seconds)
    {
        let band = (width * 0.22).clamp(24.0, 140.0);
        let left = x - band + (width + band) * along;
        push(
            canvas,
            DrawCommand::PushRoundedClip {
                rect: area,
                radius: radius * s,
            },
        );
        let light = Color::new(1.0, 1.0, 1.0, 0.09);
        for (start, end, from) in [
            (faded(light, 0.0), light, left),
            (light, faded(light, 0.0), left + band * 0.5),
        ] {
            push(
                canvas,
                DrawCommand::GradientRect {
                    rect: frame.rect(from, y, band * 0.5, height),
                    radius: 0.0,
                    gradient: Gradient {
                        start,
                        end,
                        vertical: false,
                    },
                },
            );
        }
        push(canvas, DrawCommand::PopClip);
    }
}

/// How far across the frame Mythical's sheen is at `seconds` (0 its left edge, 1 past its
/// right), `None` while it rests between crossings.
pub(crate) fn sheen_at(seconds: f32) -> Option<f32> {
    let phase = (seconds / SHEEN_SECONDS).rem_euclid(1.0);
    (phase < SHEEN_CROSS).then(|| phase / SHEEN_CROSS)
}

/// How wide [`tier_mark`] draws tier `tier`'s mark at `size` (frame pixels), 0 for none:
/// for laying a mark out before it is drawn (a centred line).
pub(crate) fn tier_mark_width(tier: Rarity, size: f32) -> f32 {
    if tier == Rarity::Unique {
        return size * 1.3;
    }
    let steps = tier_step(tier);
    if steps == 0 {
        0.0
    } else {
        f32::from(steps) * size + f32::from(steps - 1) * size * 0.28
    }
}

/// Tier `tier`'s mark from `x` (its left edge), centred on `y`, `size` frame pixels high:
/// a diamond for each step ([`tier_step`]: none for Common), or Unique's SJK Symbols mark,
/// in the tier's colour (faint when `dim`). Returns how wide it is, 0 for none.
pub(crate) fn tier_mark(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    tier: Rarity,
    size: f32,
    dim: bool,
) -> f32 {
    let colour = faded(tier.colour(), if dim { 0.5 } else { 1.0 });
    if tier == Rarity::Unique {
        let side = size * 1.3;
        text(
            canvas,
            TextFamily::Body,
            format_args!("{}", tier.symbol()),
            frame.rect(x, y - side * 0.5, side, side),
            side * frame.s,
            colour,
            FontWeight::Regular,
            TextAlign::Center,
        );
        return side;
    }
    let steps = tier_step(tier);
    let half = size * 0.5;
    let gap = size * 0.28;
    for step in 0..steps {
        let centre = x + half + f32::from(step) * (size + gap);
        let [cx, cy] = frame.point(centre, y);
        let h = half * frame.s;
        let mut points = [[0.0; 2]; sjk_ui::POLYGON_POINTS];
        points[..4].copy_from_slice(&[
            [cx, cy - h],
            [cx + h * 0.78, cy],
            [cx, cy + h],
            [cx - h * 0.78, cy],
        ]);
        push(
            canvas,
            DrawCommand::Polygon {
                points,
                count: 4,
                color: colour,
                texture: None,
            },
        );
    }
    tier_mark_width(tier, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIERS: [Rarity; 6] = [
        Rarity::Common,
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Legendary,
        Rarity::Mythical,
        Rarity::Unique,
    ];

    fn commands(tier: Rarity, size: TierSize, look: TierLook) -> Vec<DrawCommand> {
        let mut canvas = MenuCanvas::new();
        let frame = Frame::new([1920.0, 1080.0]);
        tier_frame(
            &mut canvas,
            &frame,
            [100.0, 100.0, 300.0, 200.0],
            tier,
            size,
            look,
        );
        canvas.draw_list().commands().to_vec()
    }

    fn count(list: &[DrawCommand], kind: fn(&DrawCommand) -> bool) -> usize {
        list.iter().filter(|command| kind(command)).count()
    }

    /// Each step up the ladder adds its touch to the one under it: the bar from Uncommon,
    /// the glow from Rare, the brackets and halo from Legendary, the full edge at
    /// Mythical; Unique has its inner line and no brackets.
    #[test]
    fn each_tier_adds_one_thing() {
        let look = TierLook::at(0.0);
        let sizes: Vec<usize> = TIERS[..5]
            .iter()
            .map(|tier| commands(*tier, TierSize::Tile, look).len())
            .collect();
        assert!(sizes.windows(2).all(|pair| pair[0] < pair[1]), "{sizes:?}");
        let gradients = |tier| {
            count(&commands(tier, TierSize::Tile, look), |c| {
                matches!(c, DrawCommand::GradientRect { .. })
            })
        };
        let lines = |tier| {
            count(&commands(tier, TierSize::Tile, look), |c| {
                matches!(c, DrawCommand::Line { .. })
            })
        };
        assert_eq!(gradients(Rarity::Uncommon), 0);
        assert_eq!(gradients(Rarity::Rare), 1, "the top glow");
        assert_eq!(lines(Rarity::Rare), 0);
        assert_eq!(lines(Rarity::Legendary), 8, "four corners, two arms each");
        assert_eq!(lines(Rarity::Unique), 0, "Unique stands outside the ladder");
        let borders = |tier| {
            count(&commands(tier, TierSize::Tile, look), |c| {
                matches!(c, DrawCommand::Border { .. })
            })
        };
        assert_eq!(borders(Rarity::Common), 1, "the hairline");
        assert_eq!(borders(Rarity::Legendary), 2, "the edge and the halo");
        assert_eq!(borders(Rarity::Unique), 2, "the edge and its inner line");
        // Over a picture, no glass.
        let bare = TierLook {
            glass: false,
            ..look
        };
        assert_eq!(
            commands(Rarity::Common, TierSize::Row, bare).len() + 1,
            commands(Rarity::Common, TierSize::Row, look).len()
        );
    }

    /// Mythical's sheen runs on the thing chosen alone, while crossing, and never on a still
    /// or dim frame; no other tier has one.
    #[test]
    fn only_a_chosen_mythical_shines() {
        let crossing = TierLook {
            chosen: true,
            ..TierLook::at(0.5)
        };
        let clips = |tier, look| {
            count(&commands(tier, TierSize::Tile, look), |c| {
                matches!(c, DrawCommand::PushRoundedClip { .. })
            })
        };
        assert!(sheen_at(0.5).is_some());
        assert!(sheen_at(SHEEN_SECONDS * 0.9).is_none(), "it rests");
        assert_eq!(clips(Rarity::Mythical, crossing), 1);
        assert_eq!(clips(Rarity::Legendary, crossing), 0);
        let alone = TierLook {
            chosen: false,
            shine: true,
            ..crossing
        };
        assert_eq!(
            clips(Rarity::Mythical, alone),
            1,
            "a thing shown on its own"
        );
        assert_eq!(
            clips(
                Rarity::Mythical,
                TierLook {
                    chosen: false,
                    ..crossing
                }
            ),
            0
        );
        for look in [
            TierLook {
                still: true,
                ..crossing
            },
            TierLook {
                dim: true,
                ..crossing
            },
            TierLook {
                seconds: SHEEN_SECONDS * 0.9,
                ..crossing
            },
        ] {
            assert_eq!(clips(Rarity::Mythical, look), 0, "{look:?}");
        }
    }

    /// The mark counts the step in diamonds (none for Common) and gives Unique its glyph;
    /// the holocron and card tiers land on the ladder's.
    #[test]
    fn the_mark_counts_the_step() {
        let frame = Frame::new([1920.0, 1080.0]);
        for tier in TIERS {
            let mut canvas = MenuCanvas::new();
            let width = tier_mark(&mut canvas, &frame, [0.0, 10.0], tier, 12.0, false);
            let diamonds = count(canvas.draw_list().commands(), |c| {
                matches!(c, DrawCommand::Polygon { count: 4, .. })
            });
            assert_eq!(diamonds, usize::from(tier_step(tier)), "{tier:?}");
            assert_eq!(width > 0.0, tier != Rarity::Common, "{tier:?}");
            // Its width is known before it is drawn, for a centred line.
            assert_eq!(width, tier_mark_width(tier, 12.0), "{tier:?}");
            if tier == Rarity::Unique {
                let words: Vec<String> = canvas.text_runs().map(str::to_owned).collect();
                assert_eq!(words, [crate::symbols::RARITY[5].to_string()]);
            }
        }
        assert_eq!(holocron_tier(0), Rarity::Uncommon);
        assert_eq!(holocron_tier(3), Rarity::Mythical);
        assert_eq!(card_tier(0), Rarity::Common);
        assert_eq!(card_tier(9), Rarity::Mythical);
        assert_eq!(tier_step(Rarity::Mythical), 4);
    }
}
