//! The Collection's Log book (20/11/2026, Sol's pick when the Fishing and Mining toys
//! were removed): both toys' logs, read only, on a page of their own for a player whose
//! logs hold a catch (a tile in the hall, a tab on the row; [`crate::catch_book::shown`]).
//! Each book lists every fish and junk, or every ore and rubble, two to a row: the ones
//! never caught as "???", the others with their count and the day the first came up, the
//! rare ones in gold; its Share button says its rarest catch in the SJK chat
//! ([`crate::catch_book::share_book`], once a minute).

use super::view::{LEFT_X, RIGHT_X};
use super::*;
use crate::menu::sjk::{Frame, color, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// A book's head, a row of two cells, the head over its junk (or rubble), its Share row.
const BOOK_HEAD: f32 = 28.0;
const BOOK_ROW: f32 = 23.0;
const BOOK_SUBHEAD: f32 = 22.0;
const SHARE_ROW: f32 = 44.0;
/// Where the books stand under the page's lead line, and the room between them.
const BOOKS_TOP: f32 = 200.0;
const BOOKS_GAP: f32 = 48.0;

/// The player's text size (`ui_textScale`).
fn text_scale() -> f32 {
    crate::text::style::current().scale.max(0.5)
}

impl Panel {
    /// The Log book tab: the fish book on the left, the mining book on the right.
    pub(super) fn log_book_tab(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let Some(books) = inputs.log_books else {
            self.lead(frame, "No log books", "");
            return;
        };
        let found = |book: &crate::catch_book::Book| {
            book.book.iter().filter(|entry| entry.count > 0).count()
        };
        let kinds = books.fishing.book.len() + books.mining.book.len();
        self.lead(
            frame,
            &format!(
                "{} of {kinds} kinds found",
                found(&books.fishing) + found(&books.mining)
            ),
            "From the Fishing and Mining toys, before they went (20/11/2026)",
        );
        let width = (RIGHT_X - LEFT_X - BOOKS_GAP) * 0.5;
        for (column, (book, share)) in [
            (&books.fishing, BRAG_TOKEN),
            (&books.mining, BRAG_MINING_TOKEN),
        ]
        .into_iter()
        .enumerate()
        {
            let x = LEFT_X + column as f32 * (width + BOOKS_GAP);
            let _ = self.log_book(frame, book, share, [x, BOOKS_TOP, width]);
        }
    }

    /// A log book from (`x`, `y`) ([`crate::catch_book`]): every kind of `card`'s
    /// log two to a row, the ones never caught as "???", the others with their count and
    /// first day, the rare ones in gold; then the Share button. Returns where the next
    /// line stands.
    fn log_book(
        &mut self,
        frame: &Frame,
        card: &crate::catch_book::Book,
        share: u16,
        [x, mut y, width]: [f32; 3],
    ) -> f32 {
        let s = frame.s;
        let k = text_scale();
        let found = card.book.iter().filter(|entry| entry.count > 0).count();
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", card.title),
            frame.rect(x, y, width * 0.5, 22.0),
            16.0 * s,
            color::holo(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{found} of {} found", card.book.len()),
            frame.rect(x + width * 0.5, y + 2.0, width * 0.5, 20.0),
            14.0 * s,
            color::muted(),
            FontWeight::Regular,
            TextAlign::End,
        );
        y += BOOK_HEAD;
        let column = (width - 16.0) * 0.5;
        let split = card.book_kinds.min(card.book.len());
        for (part, entries) in [&card.book[..split], &card.book[split..]]
            .into_iter()
            .enumerate()
        {
            if entries.is_empty() {
                continue;
            }
            if part == 1 {
                let mut head = card.goods.to_owned();
                if let Some(first) = head.get_mut(..1) {
                    first.make_ascii_uppercase();
                }
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{head}"),
                    frame.rect(x, y + 2.0, width, 18.0),
                    13.0 * s,
                    color::quiet(),
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
                y += BOOK_SUBHEAD;
            }
            for (index, entry) in entries.iter().enumerate() {
                let cell_x = x + (index % 2) as f32 * (column + 16.0);
                let cell_y = y + (index / 2) as f32 * BOOK_ROW;
                let caught = entry.count > 0;
                let colour = match (caught, entry.rare) {
                    (false, _) => color::quiet(),
                    (true, true) => color::gold_bright(),
                    (true, false) => color::text(),
                };
                let note = entry.note();
                let note_width = crate::text::display_width(&note, 12.0) * k;
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{}", entry.name),
                    frame.rect(
                        cell_x,
                        cell_y + 2.0,
                        (column - note_width - 6.0).max(0.0),
                        20.0,
                    ),
                    14.0 * s,
                    colour,
                    if entry.rare && caught {
                        FontWeight::Semibold
                    } else {
                        FontWeight::Regular
                    },
                    TextAlign::Start,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{note}"),
                    frame.rect(cell_x, cell_y + 4.0, column, 18.0),
                    12.0 * s,
                    color::muted(),
                    FontWeight::Regular,
                    TextAlign::End,
                );
            }
            y += entries.len().div_ceil(2) as f32 * BOOK_ROW;
        }
        // Share: the player's own line in the SJK chat, only when pressed.
        y += 6.0;
        let live = card.brag.is_some();
        let shown = "Share in SJK chat";
        let button_width = crate::text::display_width(shown, 15.0) * k + 28.0;
        let rect = frame.rect(x, y, button_width, 32.0);
        let hovered = live && self.ui.token_hovered(share);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * s,
            color: color::alpha(
                color::gold(),
                match (live, hovered) {
                    (true, true) => 0.22,
                    (true, false) => 0.12,
                    (false, _) => 0.04,
                },
            ),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 8.0 * s,
            width: 1.2 * s,
            color: if live {
                color::gold()
            } else {
                color::alpha(color::text(), 0.3)
            },
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{shown}"),
            frame.rect(x, y + 6.0, button_width, 22.0),
            15.0 * s,
            if live {
                color::gold_bright()
            } else {
                color::quiet()
            },
            FontWeight::Semibold,
            TextAlign::Center,
        );
        if live {
            self.ui.hit_region(share, rect);
        }
        let note_x = x + button_width + 12.0;
        let note = if live {
            "Your rarest catch"
        } else {
            "Nothing to share yet"
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{note}"),
            frame.rect(note_x, y + 8.0, (x + width - note_x).max(0.0), 20.0),
            13.0 * s,
            color::quiet(),
            FontWeight::Regular,
            TextAlign::Start,
        );
        y + SHARE_ROW - 6.0
    }
}

/// How tall a log book of `first` kinds then `rest` stands, its Share row included.
#[cfg(test)]
fn book_height(first: usize, rest: usize) -> f32 {
    BOOK_HEAD
        + first.div_ceil(2) as f32 * BOOK_ROW
        + if rest > 0 {
            BOOK_SUBHEAD + rest.div_ceil(2) as f32 * BOOK_ROW
        } else {
            0.0
        }
        + SHARE_ROW
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A player who caught something sees both books: every kind, the caught ones by
    /// name, a Share button for the book with a catch, none for the empty one; the books
    /// fit over the keys at 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn the_log_book_shows_both_books_and_shares_the_one_with_a_catch() {
        let books = crate::catch_book::Books::of(
            "kyber_minnow 2
boot 1
",
            "",
        );
        assert!(books.fishing.any() && !books.mining.any() && books.any());
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            let inputs = Inputs {
                log_books: Some(&books),
                ..super::super::tests::inputs(unlockables::Holdings::Known(&[]), "")
            };
            let mut panel = Panel::new();
            panel.open(Tab::LogBook, true, true, ReturnTarget::InGame);
            panel.build(&inputs, &font, viewport);
            assert!(!panel.ui.overflowed(), "{viewport:?}");
            let all = panel.ui.text_runs().collect::<Vec<_>>().join(
                "
",
            );
            assert!(all.contains("Fishing") && all.contains("Mining"), "{all}");
            assert!(all.contains("Kyber minnow") && all.contains("???"), "{all}");
            assert!(panel.ui.rect_for(BRAG_TOKEN).is_some());
            assert!(panel.ui.rect_for(BRAG_MINING_TOKEN).is_none());
            let frame = Frame::new(viewport);
            let keys = frame.point(0.0, super::super::view::KEYS_Y)[1];
            let share = panel.ui.rect_for(BRAG_TOKEN).unwrap();
            assert!(share.bottom() <= keys, "{viewport:?}");
        }
        let tallest =
            book_height(crate::fishing::FISH_KINDS, crate::fishing::JUNK_KINDS).max(book_height(
                crate::mining::ORE_KINDS,
                crate::mining::FINDS.len() - crate::mining::ORE_KINDS,
            ));
        assert!(BOOKS_TOP + tallest <= super::super::view::KEYS_Y);
    }
}
