//! The Collection screen (`docs/sjk-ui.md`, Collection): what an SJK player collects,
//! under one row of tabs. Medals (given by the SJK team, hanging from a rail),
//! Achievements (a wall of medallions by category), Shaders (the saber's looks: the
//! stock blade and every blade skin, owned or locked), Toys (the Illuminate holocron)
//! and Nameplates (ornaments for the plate over the player's head; none yet). Every
//! tab shows its things on the left, each in its place from the start (the ones the
//! player lacks dimmed), and the one chosen up close on the right; on Shaders, Toys and
//! Nameplates that is the player's own model, on the menu map's stage or in a live
//! preview over a match ([`Backstage`]), wearing the shader chosen, a locked one too
//! (only on this screen: nothing is sent).
//!
//! In the SJK UI it is the main page's and the game menu's Collection, a screen of its
//! own ([`crate::profile_hub`]); in the classic menus the `collection`, `achievements`
//! and `unlockables` commands open it on its own. Like the Profile page it lives in the
//! console and has the SJK UI's look in every menu style ([`view`]).
//!
//! Keys: the arrows choose (on Achievements Up and Down move between the categories),
//! Tab and Shift+Tab walk the things, Enter or Space equips or unequips the shader
//! chosen or lights and puts out the Illuminate holocron, Ctrl+Tab changes tab (the
//! switch's, [`crate::profile_hub`]), Escape goes back. The pointer chooses by
//! hovering; a click on a button acts, on a tab shows it.

use crate::achievements::Standing;
use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;
use crate::unlockables::{self, Holdings};
use sjk_identity::Snapshot;
use sjk_ui::{InputEvent, UiEventKind};
use std::time::Instant;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "collection_achievements.rs"]
mod achievements;
#[path = "collection_medals.rs"]
mod medals;
#[path = "collection_nameplates.rs"]
mod nameplates;
#[path = "collection_shaders.rs"]
mod shaders;
#[path = "collection_swatch.rs"]
pub(crate) mod swatch;
#[path = "collection_toys.rs"]
mod toys;
#[path = "collection_view.rs"]
mod view;

pub(crate) use view::{MODEL_AREA, row_labels};

/// The things' tokens: the medals, the achievements, the shaders' rows, the shader's
/// Equip or Unequip, the toy and its switch.
const MEDAL_BASE: u16 = 1_200;
const ACHIEVEMENT_BASE: u16 = 1_240;
const SHADER_BASE: u16 = 1_280;
const WEAR_TOKEN: u16 = 1_300;
const TOY_TOKEN: u16 = 1_310;
const TOY_SWITCH_TOKEN: u16 = 1_311;
/// The Shaders tab's rows: the stock blade, then every blade skin.
const SHADER_ROWS: usize = 1 + unlockables::ALL.len();
/// Shader rows the rack shows at once; it scrolls to the rest.
const RACK_SHOWN: usize = 6;

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Set `cg_saberSkin` to this id (`""` for the stock blade).
    Wear(&'static str),
    /// Light the Illuminate holocron (`true`) or put it out: the same as `toy_illuminate`,
    /// set rather than toggled.
    Illuminate(bool),
    /// Show a tab another page draws: the Holocrons page.
    Hub(Tab),
}

/// Where the player's model stands behind Shaders, Toys and Nameplates this frame, as
/// the renderer says ([`crate::menu_stage`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Backstage {
    /// Nowhere: the page draws the thing itself.
    #[default]
    None,
    /// On the menu map's stage, right of the page; `head` is where the top of its head
    /// is in the window, when known.
    World { head: Option<[f32; 2]> },
    /// In the live preview the page shows at [`MODEL_AREA`], once a frame of it is drawn.
    Preview { ready: bool },
}

/// The blade the model on the stage holds while the page shows it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PreviewSkin {
    /// What the player wears ([`crate::GpuState::local_saber_skin`]).
    Worn,
    /// The stock blade.
    Stock,
    /// This blade skin, owned or not: only drawn here, never sent.
    Skin(&'static str),
}

/// What the page wants the stage to show.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StageWish {
    pub(crate) skin: PreviewSkin,
    /// The Illuminate holocron floats by the model, lit.
    pub(crate) holocron: bool,
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// `cl_identity`.
    pub(crate) enabled: bool,
    pub(crate) snapshot: Option<&'a Snapshot>,
    /// The unlocks the player's own hub profile lists, or why they are not known.
    pub(crate) holdings: Holdings<'a>,
    /// `cg_saberSkin`.
    pub(crate) setting: &'a str,
    /// Every achievement with the player's progress.
    pub(crate) standings: &'a [Standing],
    /// The Illuminate holocron is lit now (the client's own state, kept in the console
    /// by the viewer each frame).
    pub(crate) illuminate: bool,
    /// The `name` cvar, for the nameplate.
    pub(crate) name: &'a str,
    /// The stock blade's colour (`color1`), for its swatch.
    pub(crate) stock: sjk_ui::Color,
}

/// What the last frame showed of a shader's row, for keys and clicks.
#[derive(Clone, Copy, Debug, Default)]
struct ShaderRow {
    /// What Enter does on it: `None` for a locked one or one already worn by the stock
    /// row.
    wear: Option<&'static str>,
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    /// Shown as the SJK UI's Collection screen: its way back names where it returns.
    hub: bool,
    back: ReturnTarget,
    tab: Tab,
    ui: MenuCanvas,
    /// The thing chosen on each tab: a medal in the order shown, an achievement in the
    /// catalogue's order, a shader's row.
    medal: usize,
    achievement: usize,
    shader: usize,
    /// The first shader row the rack shows (it scrolls, [`RACK_SHOWN`] at a time).
    shader_first: usize,
    /// How many medals the last frame showed.
    medals_shown: usize,
    shader_rows: [ShaderRow; SHADER_ROWS],
    /// Where the model stands this frame.
    backstage: Backstage,
    /// How much wider than the display family the page's text face is drawn.
    widen: f32,
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
            back: ReturnTarget::MainMenu,
            tab: Tab::Medals,
            ui: MenuCanvas::with_capacities(160, 220, 1_400),
            medal: 0,
            // The first frame chooses the one nearest to unlocking.
            achievement: usize::MAX,
            shader: 0,
            shader_first: 0,
            medals_shown: 0,
            shader_rows: [ShaderRow::default(); SHADER_ROWS],
            backstage: Backstage::None,
            widen: 1.0,
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

    /// Show the page on `tab` (one of the Collection's), on its own (`hub` false: the
    /// classic menus' commands) or as the SJK UI's Collection screen returning to
    /// `back`; `owns_console` when the console was closed before it. The things chosen
    /// on each tab stay as they were.
    pub(crate) fn open(&mut self, tab: Tab, owns_console: bool, hub: bool, back: ReturnTarget) {
        if !self.open {
            // Shaders opens on what the player wears.
            self.shader = usize::MAX;
        }
        self.open = true;
        self.owns_console = owns_console;
        self.hub = hub;
        self.back = back;
        self.show(tab);
    }

    /// Turn to `tab`, if it is one of the Collection's this page draws (Holocrons is
    /// the Holocrons page).
    pub(crate) fn show(&mut self, tab: Tab) {
        if tab.screen() == crate::profile_hub::Screen::Collection && tab != Tab::Holocrons {
            self.tab = tab;
        }
    }

    /// The tab on show (or shown last).
    pub(crate) fn tab(&self) -> Tab {
        self.tab
    }

    /// Whether the page is the SJK UI's Collection screen.
    pub(crate) fn is_hub(&self) -> bool {
        self.hub
    }

    /// Where the Collection screen returns.
    pub(crate) fn back(&self) -> ReturnTarget {
        self.back
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        self.backstage = Backstage::None;
        owned
    }

    /// Where the model stands this frame.
    pub(crate) fn set_backstage(&mut self, backstage: Backstage) {
        self.backstage = backstage;
    }

    /// What the stage should show while the page is open on Shaders, Toys or
    /// Nameplates.
    pub(crate) fn stage_wish(&self) -> Option<StageWish> {
        if !self.open {
            return None;
        }
        match self.tab {
            Tab::Shaders => Some(StageWish {
                skin: match self.shader {
                    0 => PreviewSkin::Stock,
                    row => unlockables::ALL
                        .get(row - 1)
                        .map_or(PreviewSkin::Worn, |skin| PreviewSkin::Skin(skin.id)),
                },
                holocron: false,
            }),
            Tab::Toys => Some(StageWish {
                skin: PreviewSkin::Worn,
                holocron: true,
            }),
            Tab::Nameplates => Some(StageWish {
                skin: PreviewSkin::Worn,
                holocron: false,
            }),
            _ => None,
        }
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Whether the last frame ran out of room on the canvas.
    #[cfg(test)]
    pub(crate) fn overflowed(&self) -> bool {
        self.ui.overflowed()
    }

    /// What Enter (or a click on the button) does on the tab on show.
    fn activate(&mut self, illuminate: bool) -> PanelAction {
        match self.tab {
            Tab::Shaders => self
                .shader_rows
                .get(self.shader)
                .and_then(|row| row.wear)
                .map_or(PanelAction::None, PanelAction::Wear),
            Tab::Toys => PanelAction::Illuminate(!illuminate),
            _ => PanelAction::None,
        }
    }

    /// How many things the tab on show offers to choose.
    fn things(&self) -> usize {
        match self.tab {
            Tab::Medals => self.medals_shown,
            Tab::Achievements => crate::achievements::ALL.len(),
            Tab::Shaders => SHADER_ROWS,
            _ => 0,
        }
    }

    /// The chosen thing of the tab on show, by its index.
    fn chosen_mut(&mut self) -> Option<&mut usize> {
        match self.tab {
            Tab::Medals => Some(&mut self.medal),
            Tab::Achievements => Some(&mut self.achievement),
            Tab::Shaders => Some(&mut self.shader),
            _ => None,
        }
    }

    /// Move the choice `by` things: wrapping round for Tab, stopping at the ends for
    /// the arrows.
    fn step(&mut self, by: isize, wrap: bool) {
        let count = self.things() as isize;
        let Some(chosen) = self.chosen_mut() else {
            return;
        };
        if count == 0 {
            return;
        }
        let at = (*chosen as isize).min(count - 1);
        let next = if wrap {
            (at + by).rem_euclid(count)
        } else {
            (at + by).clamp(0, count - 1)
        };
        *chosen = next as usize;
    }

    /// A key; `illuminate` is whether the holocron is lit.
    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        shift: bool,
        illuminate: bool,
    ) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        self.key(key, shift, illuminate)
    }

    /// A pressed key, by its code.
    fn key(&mut self, key: KeyCode, shift: bool, illuminate: bool) -> PanelAction {
        let vertical = self.tab == Tab::Shaders;
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(if shift { -1 } else { 1 }, true),
            KeyCode::ArrowLeft if !vertical => self.step(-1, false),
            KeyCode::ArrowRight if !vertical => self.step(1, false),
            KeyCode::ArrowUp | KeyCode::ArrowDown if self.tab == Tab::Achievements => {
                self.achievement =
                    achievements::vertical(self.achievement, key == KeyCode::ArrowDown);
            }
            KeyCode::ArrowUp | KeyCode::ArrowLeft => self.step(-1, false),
            KeyCode::ArrowDown | KeyCode::ArrowRight => self.step(1, false),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                return self.activate(illuminate);
            }
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event; `illuminate` is whether the holocron is lit.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent, illuminate: bool) -> PanelAction {
        if let InputEvent::PointerWheel { delta, .. } = event
            && self.tab == Tab::Shaders
        {
            // The wheel scrolls the rack a row a notch; the chosen row stays in view.
            let last = SHADER_ROWS.saturating_sub(RACK_SHOWN);
            self.shader_first = if delta.y > 0.0 {
                self.shader_first.saturating_sub(1)
            } else if delta.y < 0.0 {
                (self.shader_first + 1).min(last)
            } else {
                self.shader_first
            };
            self.shader = self
                .shader
                .clamp(self.shader_first, self.shader_first + RACK_SHOWN - 1);
            return PanelAction::None;
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        let Some(token) = event.token else {
            return PanelAction::None;
        };
        let choose = matches!(
            event.kind,
            UiEventKind::HoverEnter | UiEventKind::Hover | UiEventKind::Activate
        );
        if choose && self.choose_token(token) {
            return PanelAction::None;
        }
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match token {
            BACK_TOKEN => PanelAction::Close,
            WEAR_TOKEN | TOY_TOKEN | TOY_SWITCH_TOKEN => self.activate(illuminate),
            token => match crate::profile_hub::collection_tab_of(token) {
                Some(Tab::Holocrons) => PanelAction::Hub(Tab::Holocrons),
                Some(tab) => {
                    self.tab = tab;
                    PanelAction::None
                }
                None => PanelAction::None,
            },
        }
    }

    /// Choose the thing `token` stands for; whether it is one.
    fn choose_token(&mut self, token: u16) -> bool {
        let within = |base: u16, count: usize| {
            token
                .checked_sub(base)
                .map(usize::from)
                .filter(|index| *index < count)
        };
        if let Some(index) = within(MEDAL_BASE, self.medals_shown) {
            self.medal = index;
        } else if let Some(index) = within(ACHIEVEMENT_BASE, crate::achievements::ALL.len()) {
            self.achievement = index;
        } else if let Some(index) = within(SHADER_BASE, SHADER_ROWS) {
            self.shader = index;
        } else {
            return false;
        }
        true
    }

    /// Choose shader row `row` (0 the stock blade), for a world shot.
    #[cfg(test)]
    pub(crate) fn choose_shader_for_shot(&mut self, row: usize) {
        self.shader = row.min(SHADER_ROWS - 1);
    }

    /// The token of the thing chosen on the tab on show, for the canvas's focus.
    fn focus_token(&self) -> u16 {
        match self.tab {
            Tab::Medals => MEDAL_BASE + self.medal as u16,
            Tab::Achievements => {
                ACHIEVEMENT_BASE + self.achievement.min(crate::achievements::ALL.len() - 1) as u16
            }
            Tab::Shaders => SHADER_BASE + self.shader as u16,
            Tab::Toys => TOY_TOKEN,
            _ => BACK_TOKEN,
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
            medal: None,
        }
    }

    pub(super) fn inputs<'a>(holdings: Holdings<'a>, setting: &'a str) -> Inputs<'a> {
        Inputs {
            enabled: true,
            snapshot: None,
            holdings,
            setting,
            standings: &[],
            illuminate: true,
            name: "^1Sol^7Vulpes",
            stock: sjk_ui::Color::new(0.16, 0.48, 1.0, 1.0),
        }
    }

    /// The panel open on `tab` after one frame of `inputs`, so its things are known.
    pub(super) fn drawn(tab: Tab, inputs: &Inputs<'_>) -> Panel {
        let mut panel = Panel::new();
        panel.open(tab, true, true, ReturnTarget::MainMenu);
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(inputs, &fonts.font, [1920.0, 1080.0]);
        panel
    }

    #[test]
    fn shaders_open_on_what_is_worn_and_enter_equips_and_unequips() {
        let owned = [sun()];
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&owned), "saber_sun"));
        let sun_row = 1 + unlockables::ALL
            .iter()
            .position(|skin| skin.id == "saber_sun")
            .unwrap();
        assert_eq!(panel.shader, sun_row, "the worn one is chosen");
        assert_eq!(
            panel.stage_wish().map(|wish| wish.skin),
            Some(PreviewSkin::Skin("saber_sun"))
        );
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Wear("")
        );
        // The stock row wears the stock blade; the Sun row, once not worn, the Sun.
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Wear("saber_sun")
        );
        let _ = panel.key(KeyCode::ArrowUp, false, true);
        assert_eq!(panel.shader, 0);
        assert_eq!(
            panel.stage_wish().map(|wish| wish.skin),
            Some(PreviewSkin::Stock)
        );
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::None,
            "worn already"
        );
        assert_eq!(panel.key(KeyCode::Escape, false, true), PanelAction::Close);
    }

    #[test]
    fn a_locked_shader_is_previewed_but_never_equipped() {
        for holdings in [
            Holdings::Known(&[]),
            Holdings::IdentityOff,
            Holdings::Waiting,
        ] {
            let mut panel = drawn(Tab::Shaders, &inputs(holdings, ""));
            let _ = panel.key(KeyCode::ArrowDown, false, true);
            let id = unlockables::ALL[0].id;
            assert_eq!(
                panel.stage_wish().map(|wish| wish.skin),
                Some(PreviewSkin::Skin(id)),
                "{holdings:?}"
            );
            assert_eq!(panel.key(KeyCode::Enter, false, true), PanelAction::None);
            assert!(panel.ui.rect_for(WEAR_TOKEN).is_none(), "no Equip");
        }
    }

    #[test]
    fn the_toy_lights_and_puts_out_illuminate_and_only_the_kit_tabs_want_the_stage() {
        let mut panel = drawn(Tab::Toys, &inputs(Holdings::Known(&[]), ""));
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Illuminate(false)
        );
        assert_eq!(
            panel.key(KeyCode::Space, false, false),
            PanelAction::Illuminate(true)
        );
        assert!(panel.stage_wish().is_some_and(|wish| wish.holocron));
        for (tab, wants) in [
            (Tab::Medals, false),
            (Tab::Achievements, false),
            (Tab::Shaders, true),
            (Tab::Toys, true),
            (Tab::Nameplates, true),
        ] {
            panel.show(tab);
            assert_eq!(panel.stage_wish().is_some(), wants, "{tab:?}");
        }
        // A tab of the Profile screen is not the page's.
        panel.show(Tab::Saber);
        assert_eq!(panel.tab(), Tab::Nameplates);
        let _ = panel.close();
        assert_eq!(panel.stage_wish(), None);
    }

    #[test]
    fn the_keys_stay_on_the_things_and_tab_comes_round() {
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&[]), ""));
        assert_eq!(panel.shader, 0, "the stock blade, worn");
        let _ = panel.key(KeyCode::ArrowUp, false, true);
        assert_eq!(panel.shader, 0, "the arrows stop at the ends");
        for _ in 0..SHADER_ROWS + 2 {
            let _ = panel.key(KeyCode::ArrowDown, false, true);
        }
        assert_eq!(panel.shader, SHADER_ROWS - 1);
        let _ = panel.key(KeyCode::Tab, false, true);
        assert_eq!(panel.shader, 0, "Tab comes round");
        let _ = panel.key(KeyCode::Tab, true, true);
        assert_eq!(panel.shader, SHADER_ROWS - 1);
    }

    /// The Toys tab's switch and plinth light the holocron when it is out and put it out
    /// when it is lit, whichever state the page was told, and its words name the new
    /// command, not the Force wheel.
    #[test]
    fn the_toys_switch_lights_or_puts_out_by_the_state_it_is_told() {
        let click = |panel: &mut Panel, token: u16, lit: bool| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
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
                let next = panel.handle_pointer(event, lit);
                if next != PanelAction::None {
                    action = next;
                }
            }
            action
        };
        for (token, lit, wanted) in [
            (TOY_SWITCH_TOKEN, false, true),
            (TOY_SWITCH_TOKEN, true, false),
            (TOY_TOKEN, false, true),
        ] {
            let mut shown = inputs(Holdings::Known(&[]), "");
            shown.illuminate = lit;
            let mut panel = drawn(Tab::Toys, &shown);
            assert_eq!(
                click(&mut panel, token, lit),
                PanelAction::Illuminate(wanted),
                "token {token}, lit {lit}"
            );
            let all = panel.ui.text_runs().collect::<Vec<_>>().join(
                "
",
            );
            assert!(all.contains("toy_illuminate"), "{all}");
            assert!(all.contains("Holocron lit"), "{all}");
            assert!(
                !all.contains("Force wheel") && !all.contains("force_illuminate"),
                "{all}"
            );
        }
    }

    #[test]
    fn a_click_chooses_acts_and_turns_the_tab() {
        let owned = [sun()];
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&owned), ""));
        let click = |panel: &mut Panel, token: u16| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
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
                let next = panel.handle_pointer(event, true);
                if next != PanelAction::None {
                    action = next;
                }
            }
            action
        };
        let sun_row = 1 + unlockables::ALL
            .iter()
            .position(|skin| skin.id == "saber_sun")
            .unwrap() as u16;
        assert_eq!(click(&mut panel, SHADER_BASE + sun_row), PanelAction::None);
        assert_eq!(panel.shader, usize::from(sun_row));
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(
            click(&mut panel, WEAR_TOKEN),
            PanelAction::Wear("saber_sun")
        );
        assert_eq!(
            click(&mut panel, crate::profile_hub::TOKEN + 1),
            PanelAction::None
        );
        assert_eq!(panel.tab(), Tab::Achievements);
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(click(&mut panel, BACK_TOKEN), PanelAction::Close);
    }
}
