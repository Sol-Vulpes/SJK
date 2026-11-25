//! New on every kind (15/11/2026, the ideas page's "New on every kind"): the Collection's
//! cards, tags and finishers carry New as its kybers do (`crate::collection_seen`). Each
//! kind's page shows "N new" and Mark all seen (N) beside its headline, a New sticker on
//! each New thing, and sees the thing chosen once it has stayed chosen a second (the
//! finishers, all shown at once, once their page has stayed open a second). The hall's
//! tiles show "N new" pills, and the Inventory a sticker on each New row.
//!
//! Drawing stays in the pages; this file holds what they share.

use super::view::{LEAD_Y, label_width};
use super::*;
use crate::collection_seen::{Kind, News, Watch};
use crate::menu::sjk::{Frame, color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// Mark all seen beside a kind's headline (cards, tags, finishers).
pub(super) const KIND_SEEN_ALL_TOKEN: u16 = 8_600;
/// The dark ink on a gold fill.
const INK: Color = Color::new(0.078, 0.063, 0.02, 1.0);
/// The finishers page's choice, all of them shown at once ([`Panel::watch_new`]).
const FINISHERS_PAGE: &str = "finishers page";

/// The New things the pages show, and what they were read from.
#[derive(Debug, Default)]
pub(super) struct NewsState {
    pub(super) news: News,
    /// The seen-list's revision and whether anything can be New (the identity on, the
    /// debug collectibles off).
    from: Option<(u64, bool)>,
    watch: Watch,
    /// What a world shot or a test shows in place of the live ones.
    #[cfg(test)]
    pub(super) preview: Option<News>,
}

impl Panel {
    /// The New things for `inputs` this frame, read again only when the list changed.
    pub(super) fn follow_news(&mut self, inputs: &Inputs<'_>) {
        #[cfg(test)]
        if let Some(news) = &self.news.preview {
            self.news.news = news.clone();
            return;
        }
        let live = inputs.enabled && !inputs.holdings.debug();
        let from = (crate::collection_seen::revision(), live);
        if self.news.from == Some(from) {
            return;
        }
        self.news.news = if live {
            crate::collection_seen::news()
        } else {
            News::default()
        };
        self.news.from = Some(from);
    }

    /// Whether anything of `kind` with `id` is New.
    pub(super) fn is_new(&self, kind: Kind, id: &str) -> bool {
        self.news.news.is_new(kind, id)
    }

    /// How many things of `kind` are New.
    pub(super) fn kind_new(&self, kind: Kind) -> usize {
        self.news.news.count(kind)
    }

    /// The page's choice rests on `chosen` this frame (`None`: nothing chosen): its New
    /// things are seen once it has stayed a second.
    pub(super) fn watch_new(&mut self, chosen: Option<(Kind, &str)>) {
        let now = self.look_clock();
        let NewsState { news, watch, .. } = &mut self.news;
        let key = chosen.map(|(kind, id)| crate::collection_seen::key_of(kind, id));
        let seen = watch.observe(key, now, || {
            chosen.map_or_else(Vec::new, |(kind, id)| {
                if id == FINISHERS_PAGE {
                    news.all_of(kind)
                } else {
                    news.things_of(kind, id)
                }
            })
        });
        if !seen.is_empty() {
            // The moment they are met: their first look (only New things are watched).
            self.reveal_things(&seen);
            crate::collection_seen::see_all(seen);
        }
    }

    /// The finishers page is on show: its New finishers are seen once it has stayed a
    /// second.
    pub(super) fn watch_finishers(&mut self) {
        self.watch_new(Some((Kind::Finisher, FINISHERS_PAGE)));
    }

    /// The choice left the page: what it rested on is seen if it had stayed a second.
    pub(super) fn leave_news(&mut self) {
        let seen = self.news.watch.leave(self.look_clock());
        if !seen.is_empty() {
            crate::collection_seen::see_all(seen);
        }
    }

    /// The kind of the page on show, when it carries New ([`Kind::Kyber`]'s page has its
    /// own, `crate::shader_marks`).
    pub(super) fn page_kind(&self) -> Option<Kind> {
        match self.tab {
            Tab::Cards => Some(Kind::Card),
            Tab::Tags => Some(Kind::Tag),
            Tab::Shaders if self.finishers_shown => Some(Kind::Finisher),
            _ => None,
        }
    }

    /// Mark every New thing of the page's kind seen (N, or Mark all seen).
    pub(super) fn see_kind_all(&mut self) -> PanelAction {
        if let Some(kind) = self.page_kind() {
            let things = self.news.news.all_of(kind);
            if !things.is_empty() {
                crate::collection_seen::see_all(things);
                ui_cues_click();
            }
        }
        PanelAction::None
    }

    /// After a kind's headline (`headline`'s words, at the lead's line; its start when there
    /// is none): "N new" on a gold
    /// pill and Mark all seen (N), while something of `kind` is New.
    pub(super) fn kind_new_count(&mut self, frame: &Frame, headline: &str, kind: Kind) {
        let count = self.kind_new(kind);
        if count == 0 {
            return;
        }
        let s = frame.s;
        let x = super::view::after_headline(headline, self.widen);
        let y = LEAD_Y + 5.0;
        let pill = count_pill(&mut self.ui, frame, [x, y], 26.0, count);
        let seen_x = x + pill + 8.0;
        let label = "Mark all seen  (N)";
        let width = label_width(label, 15.0, self.widen) + 30.0;
        let rect = frame.rect(seen_x, y - 1.0, width, 28.0);
        let hovered = self.ui.token_hovered(KIND_SEEN_ALL_TOKEN);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 14.0 * s,
            color: color::alpha(color::gold(), if hovered { 0.18 } else { 0.06 }),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 14.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::gold(), if hovered { 1.0 } else { 0.55 }),
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{label}"),
            frame.rect(seen_x, y + 3.0, width, 22.0),
            15.0 * s,
            color::gold_bright(),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        self.ui.hit_region(KIND_SEEN_ALL_TOKEN, rect);
    }

    /// Show these New things in place of the live ones, for a world shot or a test.
    #[cfg(test)]
    pub(crate) fn news_for_shot(&mut self, news: Option<News>) {
        self.news.preview = news;
    }
}

/// The New sticker after an Inventory row's name (its left at `x`, `middle` its middle
/// line), when what it stands for is New: a card in its finish, the rest in any.
/// Holocrons, dormant kybers and GIF tags carry none.
pub(super) fn row_sticker(
    canvas: &mut MenuCanvas,
    news: &NewsState,
    frame: &Frame,
    item: &super::inventory::Item,
    [x, middle]: [f32; 2],
) {
    use super::inventory::Thing as Row;
    let news = &news.news;
    let new = match &item.thing {
        Row::Kyber(id) => news.is_new(Kind::Kyber, id),
        Row::Finisher(id) => news.is_new(Kind::Finisher, id),
        Row::Tag(id) => news.is_new(Kind::Tag, id),
        Row::Card {
            face: Some(face),
            finish,
            ..
        } => crate::cards::faces::current()
            .list
            .get(*face)
            .is_some_and(|face| news.is_new_in(Kind::Card, &face.id, *finish)),
        _ => false,
    };
    if new {
        let name =
            crate::text::display_width(&item.name, 18.0) * crate::text::style::current().scale;
        let _ =
            super::marks_view::new_sticker(canvas, frame, [x + name + 12.0, middle - 10.0], 20.0);
    }
}

/// "N new" on a gold pill `height` tall, its left at `[x, y]` (frame pixels), the words
/// in dark ink. Returns its width.
pub(super) fn count_pill(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    height: f32,
    count: usize,
) -> f32 {
    let s = frame.s;
    let size = height * 0.66;
    let words = format!("{} new", crate::cards::grouped(count as u64));
    let width = crate::text::display_width(&words, size) * crate::text::style::current().scale
        + height * 0.9;
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: frame.rect(x, y, width, height),
        radius: height * 0.5 * s,
        color: color::gold_bright(),
    });
    text(
        canvas,
        TextFamily::Display,
        format_args!("{words}"),
        frame.rect(x, y + (height - size * 1.25) * 0.5, width, size * 1.25),
        size * s,
        INK,
        FontWeight::Semibold,
        TextAlign::Center,
    );
    width
}

/// The width [`count_pill`] takes for `count` at `height`.
pub(super) fn count_pill_width(height: f32, count: usize) -> f32 {
    let words = format!("{} new", crate::cards::grouped(count as u64));
    crate::text::display_width(&words, height * 0.66) * crate::text::style::current().scale
        + height * 0.9
}

#[cfg(test)]
mod tests {
    use super::super::tests::inputs;
    use super::*;
    use crate::collection_seen::Kind;
    use crate::player_menu::ReturnTarget;

    fn built(panel: &mut Panel, inputs: &Inputs<'_>) {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(inputs, &fonts.font, [1920.0, 1080.0]);
        assert!(!panel.overflowed());
    }

    /// The Cards tab with no card held points the way: Open holocrons while some wait,
    /// Find a server without; Enter (or its button) does it. A card held shows the binder,
    /// with nothing offered.
    #[test]
    fn an_empty_cards_tab_offers_the_way_to_the_first_card() {
        let mut snapshot = super::super::forge::sample_snapshot(Vec::new());
        let mut panel = Panel::new();
        panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
        panel.preview_cards = Some((Vec::new(), 0, 0));
        built(&mut panel, &inputs(Holdings::Known(&[]), ""));
        assert_eq!(panel.empty_action, Some(view::EmptyAction::Servers));
        assert!(panel.ui.text_runs().any(|run| run == "No cards yet"));
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("Find a server"))
        );
        assert_eq!(panel.key(KeyCode::Enter, false, true), PanelAction::Servers);
        if let Some(me) = snapshot.me.as_mut() {
            me.holocron_counts.rare = 3;
        }
        let waiting = Inputs {
            snapshot: Some(&snapshot),
            ..inputs(Holdings::Known(&[]), "")
        };
        built(&mut panel, &waiting);
        assert_eq!(panel.empty_action, Some(view::EmptyAction::Holocrons));
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("Open Holocrons") && run.contains("3 waiting"))
        );
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Hub(Tab::Holocrons)
        );
        let stack = sjk_identity::CardStack {
            card: Some("sjk_alpha_000".to_owned()),
            rarity: "rare".to_owned(),
            finish: "normal".to_owned(),
            edition: "alpha".to_owned(),
            count: 1,
            first: 0,
            last: 0,
        };
        panel.preview_cards = Some((vec![stack], 0, 0));
        built(&mut panel, &waiting);
        assert_eq!(panel.empty_action, None);
        assert!(!panel.ui.text_runs().any(|run| run == "No cards yet"));
    }

    /// Each kind's page says how many of its kind are New with Mark all seen, and the
    /// hall's tiles carry each kind's count; nothing New, neither shows.
    #[test]
    fn each_kind_shows_its_new_count_and_mark_all_seen() {
        let stack = |finish: &str| sjk_identity::CardStack {
            card: Some("sjk_alpha_002".to_owned()),
            rarity: "legendary".to_owned(),
            finish: finish.to_owned(),
            edition: "alpha".to_owned(),
            count: 1,
            first: 0,
            last: 0,
        };
        let tag = crate::tags::CATALOGUE.iter().find(|tag| !tag.free).unwrap();
        let news = News::shown(&[
            (Kind::Card, "sjk_alpha_002", 1),
            (Kind::Card, "sjk_alpha_002", 2),
            (Kind::Tag, tag.id, 0),
        ]);
        let shown = inputs(Holdings::Known(&[]), "");
        let mut panel = Panel::new();
        panel.news_for_shot(Some(news));
        panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
        panel.preview_cards = Some((vec![stack("holo"), stack("solar")], 0, 0));
        built(&mut panel, &shown);
        assert!(panel.ui.text_runs().any(|run| run == "2 new"));
        assert!(panel.ui.text_runs().any(|run| run == "Mark all seen  (N)"));
        assert!(panel.ui.rect_for(KIND_SEEN_ALL_TOKEN).is_some());
        assert_eq!(panel.page_kind(), Some(Kind::Card));
        panel.show(Tab::Tags);
        built(&mut panel, &shown);
        assert!(panel.ui.text_runs().any(|run| run == "1 new"));
        assert_eq!(panel.page_kind(), Some(Kind::Tag));
        panel.enter_hall();
        let summaries = panel.summaries(&shown, &view::counts(&shown));
        let new_of = |tile: hall::Tile| {
            hall::tiles()
                .iter()
                .position(|each| *each == tile)
                .map(|index| summaries[index].new)
        };
        assert_eq!(new_of(hall::Tile::Cards), Some(2));
        assert_eq!(new_of(hall::Tile::Tags), Some(1));
        assert_eq!(new_of(hall::Tile::Finishers), Some(0));
        panel.news_for_shot(Some(News::default()));
        panel.show(Tab::Cards);
        built(&mut panel, &shown);
        assert!(panel.ui.rect_for(KIND_SEEN_ALL_TOKEN).is_none());
    }

    /// The first look plays when a New thing has stayed chosen its second, once; a thing
    /// already seen stays quiet however long it is chosen.
    #[test]
    fn a_new_thing_reveals_itself_once_when_first_seen() {
        use crate::audio::ui_cues::{Cue, take_posted};
        use std::time::Duration;
        const NEW: &str = "reveal_test_card";
        let mut panel = Panel::new();
        panel.news.news = News::shown(&[(Kind::Card, NEW, 1)]);
        let _ = take_posted();
        panel.watch_new(Some((Kind::Card, "reveal_test_seen")));
        panel.epoch -= Duration::from_millis(1_500);
        panel.watch_new(Some((Kind::Card, "reveal_test_seen")));
        assert!(take_posted().is_empty(), "nothing New, nothing revealed");
        panel.watch_new(Some((Kind::Card, NEW)));
        assert!(take_posted().is_empty(), "not before its second");
        assert!(!panel.reveals.revealed(Kind::Card, NEW));
        panel.epoch -= Duration::from_millis(1_000);
        panel.watch_new(Some((Kind::Card, NEW)));
        assert_eq!(take_posted(), [Cue::Reveal]);
        assert!(panel.reveals.revealed(Kind::Card, NEW));
        panel.epoch -= Duration::from_millis(1_000);
        panel.watch_new(Some((Kind::Card, NEW)));
        panel.leave_news();
        assert!(take_posted().is_empty(), "once");
    }
}
