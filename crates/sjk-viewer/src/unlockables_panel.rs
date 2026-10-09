//! The Unlockables page (`docs/unlockables.md`, "Unlockables page"): every unlockable
//! of the catalogue ([`crate::unlockables`]) as a card with a live swatch, its name and
//! what it is, owned (since when, with the SJK team's note) or locked (with how to get
//! it), and Equip or Unequip for an owned blade skin, which sets `cg_saberSkin`.
//!
//! In the SJK UI it is the Profile screen's Collection tab ([`crate::profile_hub`]);
//! in the classic menus the Profile page's See unlockables or the `unlockables` command
//! open it on its own. Like the Staff page it lives in the console and has the SJK UI's
//! look in every menu style ([`view`]). The cards that do not fit scroll: the list follows the card chosen, and the
//! mouse wheel scrolls it (choosing a card it brings into view). The arrows move between
//! the cards (Up and Down by rows), Tab and Shift+Tab walk them, Enter or Space equips or
//! unequips the one chosen, Escape goes back; a click on Equip or Unequip acts, on a card
//! it only chooses it.

use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use crate::unlockables::{self, Holdings};
use sjk_ui::{InputEvent, UiEventKind};
use std::time::Instant;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "unlockables_panel_view.rs"]
mod view;

/// The cards, one token each, and their Equip or Unequip buttons.
const CARD_BASE: u16 = 1_200;
const EQUIP_BASE: u16 = 1_240;
/// Cards in a row: one column, the page's right side telling what is worn.
const COLUMNS: usize = 1;

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Set `cg_saberSkin` to this id (`""` for the stock blade).
    Wear(&'static str),
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// The unlocks the player's own hub profile lists, or why they are not known.
    pub(crate) holdings: Holdings<'a>,
    /// `cg_saberSkin`.
    pub(crate) setting: &'a str,
}

/// What the last frame showed of a card, for keys and clicks.
#[derive(Clone, Copy, Debug, Default)]
struct Card {
    /// What Enter does on it: `None` for a locked one.
    wear: Option<&'static str>,
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    /// Shown as the Profile screen's Collection tab: its title and row of tabs at the
    /// top, the page moved down under them.
    hub: bool,
    /// What the Profile screen's tabs put at the top.
    hub_header: crate::profile_hub::Header,
    ui: MenuCanvas,
    /// The card the keyboard is on, by its token.
    focus: u16,
    /// Every card in the order Tab visits them, shown or scrolled away, as the last frame
    /// laid them out.
    order: Vec<u16>,
    cards: [Card; unlockables::ALL.len()],
    /// The first card shown; the cards above it are scrolled away.
    first: usize,
    epoch: Instant,
    /// The blade skins the hub's packs brought, the swatches' looks (`sjk_packs.rs`);
    /// taken again only when they change ([`Panel::follow_skins`]).
    pub(super) skins: std::sync::Arc<crate::saber_skins::LoadedSkins>,
    /// What a world shot shows in place of the own profile's unlocks.
    #[cfg(test)]
    pub(crate) preview: Option<Vec<sjk_identity::Unlock>>,
    /// The time a world shot draws the swatches at, instead of the page's.
    #[cfg(test)]
    pub(crate) shot_seconds: Option<f32>,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            hub: false,
            hub_header: crate::profile_hub::Header::default(),
            ui: MenuCanvas::with_capacities(96, 160, 900),
            focus: CARD_BASE,
            order: Vec::with_capacity(unlockables::ALL.len()),
            cards: [Card::default(); unlockables::ALL.len()],
            first: 0,
            epoch: Instant::now(),
            skins: Default::default(),
            #[cfg(test)]
            preview: None,
            #[cfg(test)]
            shot_seconds: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Take the loaded blade skins when a pack brought new ones (one atomic read a
    /// frame otherwise).
    pub(crate) fn follow_skins(&mut self) {
        if self.skins.generation() != crate::sjk_packs::generation() {
            self.skins = crate::sjk_packs::skins();
        }
    }

    /// Show the page on its own; `owns_console` when the console was closed before it.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.hub = false;
        self.focus = CARD_BASE;
        self.first = 0;
    }

    /// Show the page as the Profile screen's Collection tab (`true`), or on its own;
    /// [`Self::open`] puts it back on its own.
    pub(crate) fn set_hub(&mut self, hub: bool) {
        self.hub = hub;
    }

    /// Whether the page is the Profile screen's Collection tab.
    pub(crate) fn is_hub(&self) -> bool {
        self.hub
    }

    /// What the Profile screen's tabs put at the top.
    pub(crate) fn hub_header(&self) -> &crate::profile_hub::Header {
        &self.hub_header
    }

    /// Put `header` at the top on the Profile screen.
    pub(crate) fn set_hub_header(&mut self, header: crate::profile_hub::Header) {
        self.hub_header = header;
    }

    /// Closing the page closes the console too.
    pub(crate) fn own_console(&mut self) {
        self.owns_console = true;
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Whether the last frame ran out of room on the canvas.
    #[cfg(test)]
    pub(crate) fn overflowed(&self) -> bool {
        self.ui.overflowed()
    }

    /// What Enter (or a click on its button) does on card `token`.
    fn activate(&mut self, token: u16) -> PanelAction {
        self.focus = token;
        let wear = token
            .checked_sub(CARD_BASE)
            .and_then(|index| self.cards.get(usize::from(index)))
            .and_then(|card| card.wear);
        wear.map_or(PanelAction::None, PanelAction::Wear)
    }

    /// Move the keyboard `by` cards along the order the last frame laid out: wrapping
    /// round for Tab, stopping at the ends for the arrows.
    fn step(&mut self, by: isize, wrap: bool) {
        let count = self.order.len() as isize;
        if count == 0 {
            return;
        }
        let at = self
            .order
            .iter()
            .position(|token| *token == self.focus)
            .map_or(0, |at| at as isize);
        let next = if wrap {
            (at + by).rem_euclid(count)
        } else {
            let next = at + by;
            if !(0..count).contains(&next) {
                return;
            }
            next
        };
        self.focus = self.order[next as usize];
    }

    /// A key.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent, shift: bool) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        self.key(key, shift)
    }

    /// A pressed key, by its code.
    fn key(&mut self, key: KeyCode, shift: bool) -> PanelAction {
        let row = COLUMNS as isize;
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(if shift { -1 } else { 1 }, true),
            KeyCode::ArrowLeft => self.step(-1, false),
            KeyCode::ArrowRight => self.step(1, false),
            KeyCode::ArrowUp => self.step(-row, false),
            KeyCode::ArrowDown => self.step(row, false),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                return self.activate(self.focus);
            }
            _ => {}
        }
        PanelAction::None
    }

    /// Scroll the cards a row down (`by` 1) or up (-1), choosing the nearest card shown
    /// when the chosen one leaves the view.
    fn scroll(&mut self, by: isize) {
        let rows = unlockables::ALL.len().div_ceil(COLUMNS);
        let last = rows.saturating_sub(view::ROWS_SHOWN);
        self.first = self.first.saturating_add_signed(by).min(last);
        let shown = self.first * COLUMNS..(self.first + view::ROWS_SHOWN) * COLUMNS;
        let index = usize::from(self.focus.saturating_sub(CARD_BASE));
        if !shown.contains(&index) {
            let index = index.clamp(shown.start, shown.end.min(unlockables::ALL.len()) - 1);
            self.focus = CARD_BASE + index as u16;
        }
    }

    /// A pointer event.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        if let InputEvent::PointerWheel { delta, .. } = event {
            // Up (positive) shows the cards above.
            if delta.y > 0.0 {
                self.scroll(-1);
            } else if delta.y < 0.0 {
                self.scroll(1);
            }
            return PanelAction::None;
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            Some(token) if (EQUIP_BASE..EQUIP_BASE + self.cards.len() as u16).contains(&token) => {
                self.activate(CARD_BASE + (token - EQUIP_BASE))
            }
            // A click on a card only chooses it.
            Some(token) if self.order.contains(&token) => {
                self.focus = token;
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::Unlock;

    fn sun() -> Unlock {
        Unlock {
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "Thanks for testing the blade".into(),
        }
    }

    /// The panel after one frame of `inputs`, so its cards are known.
    fn drawn(inputs: &Inputs<'_>) -> Panel {
        let mut panel = Panel::new();
        panel.open(true);
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(inputs, &fonts.font, [1920.0, 1080.0]);
        panel
    }

    #[test]
    fn enter_equips_and_unequips_an_owned_blade_skin() {
        let owned = [sun()];
        let mut panel = drawn(&Inputs {
            holdings: Holdings::Known(&owned),
            setting: "",
        });
        // Every card is in the order, the ones scrolled away too.
        let all: Vec<u16> = (0..unlockables::ALL.len() as u16)
            .map(|index| CARD_BASE + index)
            .collect();
        assert_eq!(panel.order, all);
        assert_eq!(
            panel.key(KeyCode::Enter, false),
            PanelAction::Wear("saber_sun")
        );
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let worn = Inputs {
            holdings: Holdings::Known(&owned),
            setting: "saber_sun",
        };
        panel.build(&worn, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.key(KeyCode::Space, false), PanelAction::Wear(""));
        assert_eq!(panel.key(KeyCode::Escape, false), PanelAction::Close);
    }

    #[test]
    fn a_locked_or_unknown_unlockable_cannot_be_equipped() {
        for holdings in [
            Holdings::Known(&[]),
            Holdings::IdentityOff,
            Holdings::Waiting,
        ] {
            let mut panel = drawn(&Inputs {
                holdings,
                setting: "saber_sun",
            });
            assert_eq!(
                panel.key(KeyCode::Enter, false),
                PanelAction::None,
                "{holdings:?}"
            );
            assert!(panel.ui.rect_for(EQUIP_BASE).is_none(), "no Equip");
            assert!(panel.ui.rect_for(CARD_BASE).is_some(), "still chosen");
        }
    }

    #[test]
    fn the_keys_stay_on_the_cards_and_tab_comes_round() {
        let owned = [sun()];
        let mut panel = drawn(&Inputs {
            holdings: Holdings::Known(&owned),
            setting: "",
        });
        // Synthetic order of five cards, one to a row.
        panel.order = (0..5).map(|index| CARD_BASE + index).collect();
        panel.focus = CARD_BASE;
        let _ = panel.key(KeyCode::ArrowUp, false);
        assert_eq!(panel.focus, CARD_BASE, "the arrows stop at the ends");
        let _ = panel.key(KeyCode::ArrowDown, false);
        let _ = panel.key(KeyCode::ArrowRight, false);
        assert_eq!(panel.focus, CARD_BASE + 2);
        for _ in 0..4 {
            let _ = panel.key(KeyCode::ArrowDown, false);
        }
        assert_eq!(panel.focus, CARD_BASE + 4, "no card under the last");
        let _ = panel.key(KeyCode::ArrowUp, false);
        let _ = panel.key(KeyCode::ArrowLeft, false);
        let _ = panel.key(KeyCode::ArrowUp, false);
        assert_eq!(panel.focus, CARD_BASE + 1);
        for _ in 0..5 {
            let _ = panel.key(KeyCode::Tab, false);
        }
        assert_eq!(panel.focus, CARD_BASE + 1, "Tab comes round");
        let _ = panel.key(KeyCode::Tab, true);
        assert_eq!(panel.focus, CARD_BASE);
    }

    #[test]
    fn the_cards_scroll_to_the_one_chosen_and_with_the_wheel() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let storm = Unlock {
            id: "saber_storm".into(),
            ..sun()
        };
        let prism = Unlock {
            id: "saber_prism".into(),
            ..sun()
        };
        let owned = [sun(), storm, prism];
        let inputs = Inputs {
            holdings: Holdings::Known(&owned),
            setting: "",
        };
        let mut panel = drawn(&inputs);
        let count = unlockables::ALL.len();
        assert!(count > view::ROWS_SHOWN, "more cards than fit");
        let shown = |panel: &Panel| -> Vec<u16> {
            (0..count as u16)
                .map(|index| CARD_BASE + index)
                .filter(|token| panel.ui.rect_for(*token).is_some())
                .collect()
        };
        assert_eq!(shown(&panel), [CARD_BASE, CARD_BASE + 1]);
        // Down to the last card: the list follows, and Enter equips it.
        for _ in 0..count {
            let _ = panel.key(KeyCode::ArrowDown, false);
            panel.build(&inputs, &fonts.font, [1920.0, 1080.0]);
        }
        let last = CARD_BASE + count as u16 - 1;
        assert_eq!(panel.focus, last);
        assert!(shown(&panel).contains(&last), "{:?}", shown(&panel));
        assert_eq!(shown(&panel).len(), view::ROWS_SHOWN);
        assert_eq!(
            panel.key(KeyCode::Enter, false),
            PanelAction::Wear("saber_prism")
        );
        // The wheel up scrolls back a row at a time, the choice following into view.
        let wheel = |panel: &mut Panel, y: f32| {
            panel.handle_pointer(InputEvent::PointerWheel {
                position: sjk_ui::Vec2::new(400.0, 500.0),
                delta: sjk_ui::Vec2::new(0.0, y),
            })
        };
        for _ in 0..count {
            assert_eq!(wheel(&mut panel, 1.0), PanelAction::None);
            panel.build(&inputs, &fonts.font, [1920.0, 1080.0]);
        }
        assert_eq!(shown(&panel), [CARD_BASE, CARD_BASE + 1]);
        assert_eq!(panel.focus, CARD_BASE + 1, "the nearest card shown");
        // And down again; it stops at the end.
        for _ in 0..count {
            let _ = wheel(&mut panel, -1.0);
            panel.build(&inputs, &fonts.font, [1920.0, 1080.0]);
        }
        assert!(shown(&panel).contains(&last));
        // A card scrolled away is still equipped by Enter once chosen: Storm, owned.
        panel.focus = CARD_BASE + 1;
        assert_eq!(
            panel.key(KeyCode::Enter, false),
            PanelAction::Wear("saber_storm")
        );
        // Opening again starts at the top.
        panel.open(true);
        panel.build(&inputs, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(shown(&panel), [CARD_BASE, CARD_BASE + 1]);
    }

    #[test]
    fn a_click_on_the_button_acts_and_on_the_card_only_chooses() {
        let owned = [sun()];
        let mut panel = drawn(&Inputs {
            holdings: Holdings::Known(&owned),
            setting: "",
        });
        let click = |panel: &mut Panel, token: u16| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + 4.0, rect.y + 4.0);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                action = panel.handle_pointer(event);
            }
            action
        };
        assert_eq!(click(&mut panel, CARD_BASE), PanelAction::None);
        assert_eq!(panel.focus, CARD_BASE);
        assert_eq!(
            click(&mut panel, EQUIP_BASE),
            PanelAction::Wear("saber_sun")
        );
        assert_eq!(click(&mut panel, BACK_TOKEN), PanelAction::Close);
    }
}
