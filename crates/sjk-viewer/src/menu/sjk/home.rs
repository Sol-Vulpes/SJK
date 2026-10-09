//! The SJK UI's main page: SJK's emblem in its turning holo ring over the live
//! map, the menu on an arc round the ring's right side, and the servers the
//! player joined last in a column on the right, each joined with one click.
//!
//! The ring's gold arc points at the chosen entry. Play, SJK and Quit open
//! pages of their own on the same ring (the arc swaps to their entries, as
//! retail's pages swapped inside one frame); Character and Settings open their
//! screens. Escape leaves a page for the main one, and on the main one asks to
//! quit.
//!
//! Under the servers, the SJK chat is docked (`docs/hub-chat.md`): its last lines,
//! who is online, a field to type in and Open chat for its page.
//!
//! Keys: Up and Down move along the arc, Enter takes the entry, Right (or Tab)
//! moves to the servers and Left (or Escape) back; Down past the last server
//! reaches the chat, where Enter starts typing (Enter sends, Escape stops). The
//! pointer chooses by hovering and acts with a click.
//!
//! The page is laid out on a 16:9 frame of 1080-line pixels centred in the
//! window: a wider window shows more map at the sides, a narrower one scales
//! the frame down to fit its width.

use super::recent::Ago;
use super::{Frame, color, fade, fade_across, key_hint, key_hint_width, text};
use crate::menu::MainDestination;
use crate::menu::emblem::{self, EmblemLayer};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{DrawCommand, FontWeight, Rect, TextAlign};
use winit::keyboard::KeyCode;

/// The JoF community's server, suggested while the player has joined none.
pub(crate) const JOF_SERVER: &str = "135.125.145.49:29070";

/// Pointer tokens: the arc's entries, then the servers, then the chat's field and
/// its Open chat.
const ENTRY_TOKEN: u16 = 0;
const SERVER_TOKEN: u16 = 20;
const CHAT_TOKEN: u16 = 40;
const OPEN_CHAT_TOKEN: u16 = 41;
/// The dock's names, one a row, and the profile card a name shows with its Mute.
const DOCK_NAME_TOKEN: u16 = 42;
const CARD_TOKEN: u16 = 50;
const CARD_MUTE_TOKEN: u16 = 51;
/// The longest message typed in the dock, as the hub takes it.
const DRAFT_MAX: usize = sjk_identity::chat::TEXT_MAX;

/// The pages the ring shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Page {
    Main,
    Play,
    Sjk,
    Quit,
}

impl Page {
    fn entries(self) -> &'static [Entry] {
        match self {
            Self::Main => &MAIN,
            Self::Play => &PLAY,
            Self::Sjk => &SJK,
            Self::Quit => &QUIT,
        }
    }

    /// The main page's entry that opens this page.
    fn parent_entry(self) -> usize {
        match self {
            Self::Main | Self::Play => 0,
            Self::Sjk => 3,
            Self::Quit => 4,
        }
    }

    /// The page's name over its entries; none on the main page.
    fn title(self) -> Option<&'static str> {
        match self {
            Self::Main => None,
            Self::Play => Some("Play"),
            Self::Sjk => Some("SJK"),
            Self::Quit => Some("Quit"),
        }
    }
}

/// What an entry does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    /// Show another page.
    Page(Page),
    /// Leave the page for the main one.
    Back,
    /// Something the menu carries out ([`Action`]).
    Act(Action),
}

/// What the page asks the menu to do.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    /// Open a screen the main menu leads to.
    Open(MainDestination),
    /// Join server `i` of the column.
    Join(usize),
    /// Send what was typed in the chat's field ([`Home::take_draft`]).
    SendChat,
    /// Open the SJK chat's page.
    OpenChat,
    /// Mute, on this PC, the player whose profile card the dock showed (its Mute,
    /// [`Home::take_mute`]).
    Mute,
    /// Exit to the desktop.
    Quit,
}

/// One entry of the arc.
#[derive(Clone, Copy, Debug)]
struct Entry {
    label: &'static str,
    hint: &'static str,
    step: Step,
}

const fn entry(label: &'static str, hint: &'static str, step: Step) -> Entry {
    Entry { label, hint, step }
}

const fn open(destination: MainDestination) -> Step {
    Step::Act(Action::Open(destination))
}

const MAIN: [Entry; 5] = [
    entry(
        "Play",
        "Join a server, or start your own",
        Step::Page(Page::Play),
    ),
    entry(
        "Character",
        "Name, model, saber and Force",
        open(MainDestination::Player),
    ),
    entry(
        "Settings",
        "Every option and key, with search",
        open(MainDestination::Settings { tab: 0 }),
    ),
    entry(
        "SJK",
        "Your profile and achievements, what's new, credits",
        Step::Page(Page::Sjk),
    ),
    entry("Quit", "Leave SJK", Step::Page(Page::Quit)),
];
const PLAY: [Entry; 3] = [
    entry(
        "Join a server",
        "Every server, favourites first",
        open(MainDestination::Browser),
    ),
    entry(
        "Create a game",
        "A match with bots on this machine",
        open(MainDestination::CreateGame),
    ),
    entry("Back", "", Step::Back),
];
const SJK: [Entry; 5] = [
    entry(
        "What's new",
        "Every release and who made it",
        open(MainDestination::Changelog),
    ),
    entry(
        "Update",
        "Check for a newer SJK",
        open(MainDestination::Update),
    ),
    entry(
        "Credits",
        "The people who make SJK",
        open(MainDestination::Credits),
    ),
    entry(
        "Profile",
        "Your medals, bio, record and achievements",
        open(MainDestination::Profile),
    ),
    entry("Back", "", Step::Back),
];
const QUIT: [Entry; 2] = [
    entry(
        "Quit to desktop",
        "Your settings are saved",
        Step::Act(Action::Quit),
    ),
    entry("Stay", "Back to the menu", Step::Back),
];

/// A server in the column, as the page shows it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ServerItem<'a> {
    /// The server's name, with its colour codes; its address when unknown.
    pub(crate) name: &'a str,
    pub(crate) map: &'a str,
    /// Players, capacity and ping, while the server list has the server.
    pub(crate) live: Option<(u16, u16, u32)>,
    /// When the player last joined it; `None` for the suggested JoF server.
    pub(crate) played: Option<Ago>,
}

/// What the page shows of the player and the servers this frame.
pub(crate) struct HomeView<'a> {
    pub(crate) name: &'a str,
    pub(crate) model: &'a str,
    pub(crate) blade_name: &'static str,
    /// The servers joined last, newest first, or the suggested JoF server.
    pub(crate) servers: &'a [ServerItem<'a>],
    /// The build's version, `2026.1007.1`.
    pub(crate) version: &'a str,
    /// A newer release the update check found.
    pub(crate) update: Option<&'a str>,
    pub(crate) seconds: f64,
    /// The SJK chat, docked under the servers; `None` while it is off.
    pub(crate) chat: Option<ChatDock<'a>>,
}

/// The SJK chat as the dock shows it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChatDock<'a> {
    /// The last messages, oldest first; the dock shows the last that fit.
    pub(crate) lines: &'a [DockLine<'a>],
    /// Keys reading the chat lately.
    pub(crate) online: u32,
    /// The hub answered the last poll.
    pub(crate) live: bool,
    /// Why the chat cannot send or read, or what became of the last message.
    pub(crate) notice: &'a str,
    /// How the body family measures, to set a line's runs one after another; without
    /// it they are placed by an estimate.
    pub(crate) measure: Option<crate::sjk_chat_look::Measure<'a>>,
}

/// One message of the dock.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DockLine<'a> {
    /// The sender's name with its colour codes.
    pub(crate) name: &'a str,
    pub(crate) text: &'a str,
    pub(crate) verified: bool,
    pub(crate) staff: bool,
    /// The sender's SJK key, for their profile card.
    pub(crate) key_id: &'a str,
}

/// The profile card a dock name shows under the pointer: who, and the name it is
/// beside.
#[derive(Debug)]
struct DockCard {
    person: crate::profile_card::Person,
    anchor: Rect,
}

/// Where the keyboard is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Focus {
    Arc,
    Server(usize),
    /// The SJK chat's field.
    Chat,
}

/// The page's state: which page of the ring, its chosen entry, the keyboard's
/// place, and the gold arc's motion.
#[derive(Debug)]
pub(crate) struct Home {
    page: Page,
    entry: usize,
    focus: Focus,
    /// Where the gold arc points (radians, clockwise from the right), easing
    /// towards its target; `None` until the first frame.
    arc: Option<f32>,
    /// Menu time of the last frame, for the arc's easing.
    last: f64,
    /// Whether the last frame docked the chat.
    dock: bool,
    /// What is typed in the chat's field, while typing.
    draft: Option<String>,
    /// What Enter sent from the field, until the menu takes it.
    sent: Option<String>,
    /// The profile card on show over the dock.
    card: Option<DockCard>,
    /// The key and name of the player its Mute asked for, until the menu takes them.
    muting: Option<(String, String)>,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            page: Page::Main,
            entry: 0,
            focus: Focus::Arc,
            arc: None,
            last: 0.0,
            dock: false,
            draft: None,
            sent: None,
            card: None,
            muting: None,
        }
    }
}

impl Home {
    /// The page `page` with `entry` chosen and the keyboard on server `server`
    /// (or the arc), for the menu snapshots.
    #[cfg(test)]
    pub(crate) fn for_snapshot(page: Page, entry: usize, server: Option<usize>) -> Self {
        Self {
            page,
            entry,
            focus: server.map_or(Focus::Arc, Focus::Server),
            ..Self::default()
        }
    }

    /// Back on the main page's first entry, the keyboard on the arc.
    pub(crate) fn reset(&mut self) {
        self.page = Page::Main;
        self.entry = 0;
        self.focus = Focus::Arc;
        self.draft = None;
    }

    /// Whether the chat's field takes the keys.
    pub(crate) fn is_typing(&self) -> bool {
        self.draft.is_some()
    }

    /// What Enter sent from the chat's field.
    pub(crate) fn take_draft(&mut self) -> String {
        self.sent.take().unwrap_or_default()
    }

    /// The key and name of the player the dock's card asked to mute ([`Action::Mute`]).
    pub(crate) fn take_mute(&mut self) -> Option<(String, String)> {
        self.muting.take()
    }

    /// A key while typing in the chat's field: text goes into it, Backspace takes a
    /// character back, Enter sends, Escape stops; every other key is swallowed, so no
    /// menu key acts.
    pub(crate) fn typing_key(&mut self, key: KeyCode, text: Option<&str>) -> Option<Action> {
        let draft = self.draft.as_mut()?;
        match key {
            KeyCode::Escape => self.draft = None,
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if draft.trim().is_empty() {
                    self.draft = None;
                } else {
                    // The draft stays for `take_draft`.
                    let text = std::mem::take(draft);
                    self.draft = None;
                    self.sent = Some(text);
                    return Some(Action::SendChat);
                }
            }
            KeyCode::Backspace => {
                draft.pop();
            }
            _ => {
                for c in text.unwrap_or_default().chars().filter(|c| !c.is_control()) {
                    if draft.chars().count() >= DRAFT_MAX {
                        break;
                    }
                    draft.push(c);
                }
            }
        }
        None
    }

    fn entries(&self) -> &'static [Entry] {
        self.page.entries()
    }

    /// Show `page`, its entry `entry` chosen.
    fn show(&mut self, page: Page, entry: usize) {
        self.page = page;
        self.entry = entry.min(page.entries().len() - 1);
        self.focus = Focus::Arc;
    }

    /// Take the chosen entry.
    fn take(&mut self) -> Option<Action> {
        match self.entries().get(self.entry)?.step {
            // Quit's page opens on Stay, so a stray Enter does not quit.
            Step::Page(Page::Quit) => self.show(Page::Quit, 1),
            Step::Page(page) => self.show(page, 0),
            Step::Back => self.back(),
            Step::Act(action) => return Some(action),
        }
        None
    }

    /// Leave a page for the main one, on the entry that opened it.
    fn back(&mut self) {
        let entry = self.page.parent_entry();
        self.show(Page::Main, entry);
    }

    /// A key on the page, with `servers` in the column; the action it takes.
    pub(crate) fn key(&mut self, key: KeyCode, servers: usize) -> Option<Action> {
        let count = self.entries().len();
        match (key, self.focus) {
            (KeyCode::ArrowUp | KeyCode::KeyW, Focus::Arc) => {
                self.entry = (self.entry + count - 1) % count;
            }
            (KeyCode::ArrowDown | KeyCode::KeyS, Focus::Arc) => {
                self.entry = (self.entry + 1) % count;
            }
            (KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::Tab, Focus::Arc) if servers > 0 => {
                self.focus = Focus::Server(0);
            }
            (KeyCode::ArrowUp | KeyCode::KeyW, Focus::Server(index)) => {
                self.focus = Focus::Server((index + servers - 1) % servers);
            }
            (KeyCode::ArrowDown | KeyCode::KeyS, Focus::Server(index))
                if self.dock && index + 1 >= servers =>
            {
                self.focus = Focus::Chat;
            }
            (KeyCode::ArrowDown | KeyCode::KeyS, Focus::Server(index)) => {
                self.focus = Focus::Server((index + 1) % servers);
            }
            (KeyCode::ArrowUp | KeyCode::KeyW, Focus::Chat) => {
                self.focus = Focus::Server(servers.saturating_sub(1));
            }
            (KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::Tab | KeyCode::Escape, Focus::Chat) => {
                self.focus = Focus::Arc;
            }
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, Focus::Chat) => {
                self.draft = Some(String::new());
            }
            (
                KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::Tab | KeyCode::Escape,
                Focus::Server(_),
            ) => {
                self.focus = Focus::Arc;
            }
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, Focus::Server(index)) => {
                return (index < servers).then_some(Action::Join(index));
            }
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, Focus::Arc) => {
                return self.take();
            }
            (KeyCode::Escape | KeyCode::Backspace, Focus::Arc) if self.page != Page::Main => {
                self.back();
            }
            (KeyCode::Escape, Focus::Arc) => self.show(Page::Quit, 1),
            _ => {}
        }
        None
    }

    /// The pointer over (or clicking) `token`, with `servers` in the column:
    /// hovering chooses, a click acts.
    pub(crate) fn pointer(&mut self, token: u16, activate: bool, servers: usize) -> Option<Action> {
        let entries = ENTRY_TOKEN..ENTRY_TOKEN + self.entries().len() as u16;
        if entries.contains(&token) {
            self.entry = usize::from(token - ENTRY_TOKEN);
            self.focus = Focus::Arc;
            return if activate { self.take() } else { None };
        }
        match token {
            CHAT_TOKEN if self.dock => {
                self.focus = Focus::Chat;
                if activate && self.draft.is_none() {
                    self.draft = Some(String::new());
                }
                return None;
            }
            OPEN_CHAT_TOKEN if self.dock => {
                self.focus = Focus::Chat;
                if activate {
                    self.draft = None;
                    return Some(Action::OpenChat);
                }
                return None;
            }
            CARD_MUTE_TOKEN if self.dock => {
                // The dock shows only players not muted: its card offers Mute.
                let card = if activate { self.card.take() } else { None };
                let card = card?;
                self.muting = Some((card.person.key_id.unwrap_or_default(), card.person.name));
                return Some(Action::Mute);
            }
            CARD_TOKEN => return None,
            token if (DOCK_NAME_TOKEN..DOCK_NAME_TOKEN + DOCK_ROWS as u16).contains(&token) => {
                return None;
            }
            _ => {}
        }
        let index = usize::from(token.checked_sub(SERVER_TOKEN)?);
        if index >= servers {
            return None;
        }
        self.focus = Focus::Server(index);
        activate.then_some(Action::Join(index))
    }

    /// Where the gold arc points this frame, eased towards `target` since the
    /// last frame at menu time `seconds`.
    fn arc_towards(&mut self, target: f32, seconds: f64) -> f32 {
        let elapsed = (seconds - self.last).clamp(0.0, 0.1) as f32;
        self.last = seconds;
        let current = self.arc.unwrap_or(target);
        let mut off = (target - current).rem_euclid(std::f32::consts::TAU);
        if off > std::f32::consts::PI {
            off -= std::f32::consts::TAU;
        }
        let next = current + off * (1.0 - (-elapsed * 9.0).exp());
        self.arc = Some(next);
        next
    }
}

/// The ring's centre, its radius and the arc's radius, in frame pixels.
const RING: [f32; 2] = [620.0, 540.0];
const RING_RADIUS: f32 = 312.0;
const ARC_RADIUS: f32 = 440.0;
/// Degrees between entries along the arc.
const ARC_STEP: f32 = 16.0;
/// The servers' column: its rule's x, its text's x and width, its top and the
/// height of one server.
const COLUMN_RULE: f32 = 1470.0;
const COLUMN_X: f32 = 1500.0;
const COLUMN_WIDTH: f32 = 324.0;
const COLUMN_TOP: f32 = 236.0;
const SERVER_HEIGHT: f32 = 96.0;
/// The docked chat: its top, a message row's height and how many rows show.
const DOCK_TOP: f32 = 700.0;
const DOCK_ROW: f32 = 26.0;
const DOCK_ROWS: usize = 5;
/// The field's top, under the rows.
const DOCK_FIELD: f32 = DOCK_TOP + 40.0 + DOCK_ROWS as f32 * DOCK_ROW + 8.0;

/// Angle (radians) of entry `index` of `count` round the ring.
fn entry_angle(index: usize, count: usize) -> f32 {
    ((index as f32 - (count as f32 - 1.0) * 0.5) * ARC_STEP).to_radians()
}

/// Where entry `index` of `count` starts, in frame pixels: on the arc, at its
/// vertical middle.
fn entry_point(index: usize, count: usize) -> [f32; 2] {
    let angle = entry_angle(index, count);
    [
        RING[0] + ARC_RADIUS * angle.cos(),
        RING[1] + ARC_RADIUS * angle.sin(),
    ]
}

/// Build the page into `canvas` at `reveal` opacity.
pub(crate) fn build(
    canvas: &mut MenuCanvas,
    viewport: [f32; 2],
    home: &mut Home,
    view: &HomeView<'_>,
    reveal: f32,
) {
    let frame = Frame::new(viewport);
    let s = frame.s;
    let seconds = view.seconds;
    canvas.begin_transparent(viewport);
    canvas.push_opacity(reveal);
    scrims(canvas, viewport, &frame);

    // The emblem in its ring, the sunburst's warmth behind it.
    let centre = frame.point(RING[0], RING[1]);
    let turn = (seconds * std::f64::consts::TAU / 240.0) as f32;
    emblem::rays(
        canvas,
        EmblemLayer::Sunburst,
        centre,
        420.0 * s,
        -turn * 0.6,
        color::alpha(color::GOLD, 0.18),
    );
    emblem::rays(
        canvas,
        EmblemLayer::Ring,
        centre,
        RING_RADIUS * s,
        turn,
        color::alpha(color::HOLO, 0.9),
    );
    emblem::draw(
        canvas,
        Rect::new(
            centre[0] - 190.0 * s,
            centre[1] - 190.0 * s,
            380.0 * s,
            380.0 * s,
        ),
        seconds,
    );

    // The gold arc points at the chosen entry, or at the servers.
    home.dock = view.chat.is_some();
    if !home.dock {
        home.draft = None;
        if home.focus == Focus::Chat {
            home.focus = Focus::Arc;
        }
    }
    let entries = home.entries();
    let target = match home.focus {
        Focus::Arc => entry_angle(home.entry, entries.len()),
        Focus::Server(index) => {
            let y = COLUMN_TOP + 64.0 + (index as f32 + 0.5) * SERVER_HEIGHT;
            (y - RING[1]).atan2(COLUMN_X - RING[0])
        }
        Focus::Chat => (DOCK_FIELD + 16.0 - RING[1]).atan2(COLUMN_X - RING[0]),
    };
    let arc = home.arc_towards(target, seconds);
    let sweep = 0.42;
    let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
        center: centre,
        radius: (RING_RADIUS + 6.0) * s,
        width: 6.0 * s,
        start: arc - sweep * 0.5,
        sweep,
        color: color::GOLD_BRIGHT,
        knockout: None,
    });
    // A short beam from the arc towards its entry.
    if home.focus == Focus::Arc {
        let inner = RING_RADIUS + 12.0;
        let outer = ARC_RADIUS - 18.0;
        let steps = 12;
        for step in 0..steps {
            let t = step as f32 / steps as f32;
            let r = inner + (outer - inner) * t;
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    centre[0] + r * arc.cos() * s - 1.2 * s,
                    centre[1] + r * arc.sin() * s - 1.2 * s,
                    2.4 * s,
                    2.4 * s,
                ),
                radius: 1.2 * s,
                color: color::alpha(color::GOLD_BRIGHT, 0.85 * (1.0 - t)),
            });
        }
    }

    page_title(canvas, &frame, home);
    for (index, entry) in entries.iter().enumerate() {
        let chosen = index == home.entry;
        let lit = chosen && home.focus == Focus::Arc;
        let size = if chosen { 68.0 } else { 44.0 };
        let colour = match (lit, chosen, entry.step) {
            // Leaving is the one thing in ember.
            (true, _, Step::Act(Action::Quit)) => color::EMBER,
            (true, _, _) => color::GOLD_BRIGHT,
            (false, true, _) => color::TEXT,
            (false, false, Step::Back) => color::QUIET,
            (false, false, Step::Page(Page::Quit)) => color::QUIET,
            (false, false, _) => color::MUTED,
        };
        let [x, y] = entry_point(index, entries.len());
        text(
            canvas,
            TextFamily::Display,
            format_args!("{}", entry.label),
            frame.rect(x, y - size * 0.62, 640.0, size * 1.2),
            size * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        if chosen && !entry.hint.is_empty() {
            text(
                canvas,
                TextFamily::Body,
                format_args!("{}", entry.hint),
                frame.rect(x + 2.0, y + 32.0, 520.0, 24.0),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let pitch = ARC_RADIUS * ARC_STEP.to_radians();
        canvas.hit_region(
            ENTRY_TOKEN + index as u16,
            frame.rect(x - 16.0, y - pitch * 0.5, COLUMN_RULE - x - 40.0, pitch),
        );
    }

    servers(canvas, &frame, home, view);
    if let Some(dock) = &view.chat {
        chat_dock(canvas, &frame, home, dock);
    }
    player(canvas, &frame, view);
    hints(canvas, &frame, home, !view.servers.is_empty());
    version(canvas, &frame, view);
    // The card goes over everything, its targets last.
    match &view.chat {
        Some(dock) => dock_card(canvas, &frame, viewport, home, dock),
        None => home.card = None,
    }
    canvas.pop_opacity();
    let selected = match home.focus {
        Focus::Arc => ENTRY_TOKEN + home.entry as u16,
        Focus::Server(index) => SERVER_TOKEN + index as u16,
        Focus::Chat => CHAT_TOKEN,
    };
    canvas.finish(selected);
}

/// Fades keeping text readable over any map: dark on the left behind the ring
/// and the arc, lighter in the middle, dark again behind the servers, and at
/// the top and bottom.
fn scrims(canvas: &mut MenuCanvas, viewport: [f32; 2], frame: &Frame) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::SPACE, alpha);
    // The frame's stops, stretched to the window's edges.
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    let stops = [
        (0.0, 0.92),
        (x(730.0), 0.78),
        (x(1190.0), 0.35),
        (x(1400.0), 0.62),
        (width, 0.78),
    ];
    for pair in stops.windows(2) {
        let ((left, from), (right, to)) = (pair[0], pair[1]);
        if right > left {
            fade_across(
                canvas,
                Rect::new(left, 0.0, right - left, height),
                space(from),
                space(to),
            );
        }
    }
    fade(
        canvas,
        Rect::new(0.0, 0.0, width, height * 0.22),
        space(0.55),
        space(0.0),
    );
    fade(
        canvas,
        Rect::new(0.0, height * 0.72, width, height * 0.28),
        space(0.0),
        space(0.85),
    );
}

/// A sub-page's name over its first entry.
fn page_title(canvas: &mut MenuCanvas, frame: &Frame, home: &Home) {
    let Some(title) = home.page.title() else {
        return;
    };
    let count = home.entries().len();
    let angle = entry_angle(0, count) - (ARC_STEP * 1.1).to_radians();
    let x = RING[0] + ARC_RADIUS * angle.cos();
    let y = RING[1] + ARC_RADIUS * angle.sin();
    text(
        canvas,
        TextFamily::Display,
        format_args!("{title}"),
        frame.rect(x, y - 16.0, 400.0, 30.0),
        24.0 * frame.s,
        color::alpha(color::HOLO, 0.85),
        FontWeight::Semibold,
        TextAlign::Start,
    );
}

/// The servers' column: the ones joined last, or the suggested JoF server.
fn servers(canvas: &mut MenuCanvas, frame: &Frame, home: &Home, view: &HomeView<'_>) {
    let s = frame.s;
    let count = view.servers.len();
    let suggested = view
        .servers
        .first()
        .is_some_and(|server| server.played.is_none());
    let bottom = COLUMN_TOP + 64.0 + count.max(1) as f32 * SERVER_HEIGHT;
    let rule = color::alpha(color::HOLO, 0.6);
    let clear = color::alpha(color::HOLO, 0.0);
    let rule_top = COLUMN_TOP - 10.0;
    let height = bottom - rule_top;
    fade(
        canvas,
        frame.rect(COLUMN_RULE, rule_top, 2.0, height * 0.18),
        clear,
        rule,
    );
    let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
        rect: frame.rect(COLUMN_RULE, rule_top + height * 0.18, 2.0, height * 0.64),
        color: rule,
    });
    fade(
        canvas,
        frame.rect(COLUMN_RULE, rule_top + height * 0.82, 2.0, height * 0.18),
        rule,
        clear,
    );
    text(
        canvas,
        TextFamily::Display,
        format_args!(
            "{}",
            if suggested {
                "Start here"
            } else {
                "Recent servers"
            }
        ),
        frame.rect(COLUMN_X, COLUMN_TOP, COLUMN_WIDTH, 34.0),
        28.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    for (index, server) in view.servers.iter().enumerate() {
        let top = COLUMN_TOP + 64.0 + index as f32 * SERVER_HEIGHT;
        let token = SERVER_TOKEN + index as u16;
        let focused = home.focus == Focus::Server(index) || canvas.token_hovered(token);
        let target = frame.rect(
            COLUMN_X - 14.0,
            top - 8.0,
            COLUMN_WIDTH + 24.0,
            SERVER_HEIGHT - 8.0,
        );
        if focused {
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: target,
                radius: 10.0 * s,
                color: color::alpha(color::HOLO, 0.1),
            });
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(COLUMN_X - 14.0, top - 2.0, 3.0, SERVER_HEIGHT - 20.0),
                radius: 1.5 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        text(
            canvas,
            TextFamily::Display,
            format_args!("{}", server.name),
            frame.rect(COLUMN_X, top, COLUMN_WIDTH, 32.0),
            27.0 * s,
            if focused {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        let detail = frame.rect(COLUMN_X, top + 34.0, COLUMN_WIDTH, 22.0);
        let quiet = frame.rect(COLUMN_X, top + 56.0, COLUMN_WIDTH, 20.0);
        let body = |canvas: &mut MenuCanvas, rect, colour, value: std::fmt::Arguments<'_>| {
            text(
                canvas,
                TextFamily::Body,
                value,
                rect,
                16.0 * s,
                colour,
                FontWeight::Regular,
                TextAlign::Start,
            );
        };
        match server.live {
            Some((players, capacity, ping)) => body(
                canvas,
                detail,
                color::MUTED,
                format_args!("{players} of {capacity} on {}, {ping} ms", server.map),
            ),
            None if server.map.is_empty() => {
                body(
                    canvas,
                    detail,
                    color::MUTED,
                    format_args!("Not in the server list"),
                );
            }
            None => body(canvas, detail, color::MUTED, format_args!("{}", server.map)),
        }
        match server.played {
            Some(ago) => body(canvas, quiet, color::QUIET, format_args!("Played {ago}")),
            None => body(
                canvas,
                quiet,
                color::QUIET,
                format_args!("The JoF community's server"),
            ),
        }
        canvas.hit_region(token, target);
    }
}

/// The SJK chat under the servers: its name and who is online, its last lines, the
/// field and Open chat.
fn chat_dock(canvas: &mut MenuCanvas, frame: &Frame, home: &Home, dock: &ChatDock<'_>) {
    let s = frame.s;
    text(
        canvas,
        TextFamily::Display,
        format_args!("SJK chat"),
        frame.rect(COLUMN_X, DOCK_TOP, COLUMN_WIDTH, 30.0),
        24.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    let status = frame.rect(COLUMN_X, DOCK_TOP + 4.0, COLUMN_WIDTH, 24.0);
    let body = |canvas: &mut MenuCanvas, rect, colour, align, value: std::fmt::Arguments<'_>| {
        text(
            canvas,
            TextFamily::Body,
            value,
            rect,
            15.0 * s,
            colour,
            FontWeight::Regular,
            align,
        );
    };
    if dock.live {
        body(
            canvas,
            status,
            color::MUTED,
            TextAlign::End,
            format_args!("{} online", dock.online),
        );
    } else {
        body(
            canvas,
            status,
            color::QUIET,
            TextAlign::End,
            format_args!("Not connected"),
        );
    }
    // The lines sit on the field, the newest last, as chats do.
    let shown = dock.lines.len().min(DOCK_ROWS);
    let first_row = DOCK_ROWS - shown;
    for (row, line) in dock.lines[dock.lines.len() - shown..].iter().enumerate() {
        let top = DOCK_TOP + 40.0 + (first_row + row) as f32 * DOCK_ROW;
        let name = dock_line(
            canvas,
            frame.rect(COLUMN_X, top, COLUMN_WIDTH, DOCK_ROW - 4.0),
            line,
            dock.measure,
            15.0 * s,
        );
        // Resting the pointer on the name shows the sender's profile card.
        canvas.hit_region(DOCK_NAME_TOKEN + row as u16, name);
    }
    if shown == 0 {
        body(
            canvas,
            frame.rect(
                COLUMN_X,
                DOCK_TOP + 40.0 + (DOCK_ROWS - 1) as f32 * DOCK_ROW,
                COLUMN_WIDTH,
                DOCK_ROW - 4.0,
            ),
            color::QUIET,
            TextAlign::Start,
            format_args!("Nobody has said anything yet"),
        );
    }
    let field = frame.rect(COLUMN_X - 14.0, DOCK_FIELD, COLUMN_WIDTH + 24.0, 34.0);
    let focused = home.focus == Focus::Chat || canvas.token_hovered(CHAT_TOKEN);
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: field,
        radius: 8.0 * s,
        color: color::alpha(color::HOLO, if focused { 0.14 } else { 0.07 }),
    });
    if focused {
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(COLUMN_X - 14.0, DOCK_FIELD + 6.0, 3.0, 22.0),
            radius: 1.5 * s,
            color: color::GOLD_BRIGHT,
        });
    }
    let inside = frame.rect(COLUMN_X, DOCK_FIELD + 5.0, COLUMN_WIDTH - 4.0, 24.0);
    match home.draft.as_deref() {
        // The end of a long draft shows, as in a field that scrolls.
        Some(draft) => {
            let start = draft
                .char_indices()
                .rev()
                .nth(34)
                .map_or(0, |(index, _)| index);
            body(
                canvas,
                inside,
                color::TEXT,
                TextAlign::Start,
                format_args!("{}_", &draft[start..]),
            );
        }
        None if focused => body(
            canvas,
            inside,
            color::MUTED,
            TextAlign::Start,
            format_args!("Enter to talk to every SJK player"),
        ),
        None => body(
            canvas,
            inside,
            color::QUIET,
            TextAlign::Start,
            format_args!("Say something to every SJK player"),
        ),
    }
    canvas.hit_region(CHAT_TOKEN, field);
    let below = DOCK_FIELD + 42.0;
    if !dock.notice.is_empty() {
        body(
            canvas,
            frame.rect(COLUMN_X, below, COLUMN_WIDTH - 96.0, 20.0),
            color::QUIET,
            TextAlign::Start,
            format_args!("{}", dock.notice),
        );
    }
    let open = frame.rect(COLUMN_X + COLUMN_WIDTH - 96.0, below - 2.0, 96.0, 24.0);
    let lit = canvas.token_hovered(OPEN_CHAT_TOKEN);
    body(
        canvas,
        open,
        if lit { color::GOLD_BRIGHT } else { color::GOLD },
        TextAlign::End,
        format_args!("Open chat"),
    );
    canvas.hit_region(OPEN_CHAT_TOKEN, open);
}

/// One message of the dock on its one row, as SJK chat lines look everywhere
/// ([`crate::sjk_chat_look`]): the name in its colours, the verified tick alone for
/// a verified sender, a colon, then the message in the SJK chat's gold, cut at the
/// column's edge. `size` is the text size in window pixels.
fn dock_line(
    canvas: &mut MenuCanvas,
    rect: Rect,
    line: &DockLine<'_>,
    measure: Option<crate::sjk_chat_look::Measure<'_>>,
    size: f32,
) -> Rect {
    use crate::sjk_chat_look;
    use crate::text::TextFace;
    let width = |value: &str, face| match measure {
        Some(measure) => measure.width(value, size, face),
        // About the body family's mean advance; colour codes take no room.
        None => {
            let codes = value
                .as_bytes()
                .windows(2)
                .filter(|pair| pair[0] == b'^' && pair[1].is_ascii_digit())
                .count();
            value.chars().count().saturating_sub(codes * 2) as f32 * size * 0.52
        }
    };
    let run = |canvas: &mut MenuCanvas, rect, colour, value: std::fmt::Arguments<'_>| {
        text(
            canvas,
            TextFamily::Body,
            value,
            rect,
            size,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    };
    let name_width = width(line.name, TextFace::Regular).min(rect.width * 0.55);
    run(
        canvas,
        Rect::new(rect.x, rect.y, name_width + 1.0, rect.height),
        color::TEXT,
        format_args!("{}", line.name),
    );
    let mut x = rect.x + name_width;
    if line.verified {
        sjk_chat_look::tick(
            canvas.draw_list_mut(),
            x,
            rect.y + rect.height * 0.5,
            size,
            1.0,
        );
        x += sjk_chat_look::tick_room(size);
    }
    let colon = width(": ", TextFace::Regular);
    run(
        canvas,
        Rect::new(x, rect.y, colon, rect.height),
        color::TEXT,
        format_args!(":"),
    );
    x += colon;
    run(
        canvas,
        Rect::new(x, rect.y, (rect.right() - x).max(1.0), rect.height),
        sjk_chat_look::GOLD,
        format_args!("{}", line.text),
    );
    Rect::new(rect.x, rect.y, name_width + 1.0, rect.height)
}

/// Follow the pointer over the dock's names (the last frame's): the profile card of
/// the sender under it, kept while the pointer is on the card, else none; then draw it
/// beside the name, over the page.
fn dock_card(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    viewport: [f32; 2],
    home: &mut Home,
    dock: &ChatDock<'_>,
) {
    let shown = dock.lines.len().min(DOCK_ROWS);
    let first = dock.lines.len() - shown;
    let hovered = (0..shown).find(|row| canvas.token_hovered(DOCK_NAME_TOKEN + *row as u16));
    match hovered {
        Some(row) => {
            let line = &dock.lines[first + row];
            let anchor = canvas
                .rect_for(DOCK_NAME_TOKEN + row as u16)
                .unwrap_or_default();
            match &mut home.card {
                Some(card)
                    if card.person.name == line.name
                        && card.person.key_id.as_deref() == Some(line.key_id) =>
                {
                    card.anchor = anchor;
                }
                _ => {
                    home.card = Some(DockCard {
                        person: dock_person(line),
                        anchor,
                    });
                }
            }
        }
        None if canvas.token_hovered(CARD_TOKEN) || canvas.token_hovered(CARD_MUTE_TOKEN) => {}
        None => home.card = None,
    }
    let Some(card) = home.card.as_ref() else {
        return;
    };
    let size = crate::profile_card::size(&card.person, frame.s);
    let origin = crate::profile_card::beside(card.anchor, size, viewport, 12.0 * frame.s);
    crate::profile_card::draw(
        canvas,
        &crate::profile_card::Card {
            person: &card.person,
            muted: false,
            measure: dock.measure.as_ref(),
        },
        origin,
        frame.s,
        crate::profile_card::Tokens {
            card: CARD_TOKEN,
            mute: CARD_MUTE_TOKEN,
        },
    );
}

/// Who a dock line's sender is. The dock leaves muted senders out, and the main page
/// is not on a server, so where they are is not known.
fn dock_person(line: &DockLine<'_>) -> crate::profile_card::Person {
    crate::profile_card::Person {
        name: line.name.to_owned(),
        key_id: (!line.key_id.is_empty()).then(|| line.key_id.to_owned()),
        hub_name: None,
        verified: line.verified,
        staff: line.staff,
        medals: crate::medals::Medals::default(),
        place: crate::profile_card::Place::Unknown,
    }
}

/// The player, bottom left: a gold ring with their initial, their name, and
/// their model and blade.
fn player(canvas: &mut MenuCanvas, frame: &Frame, view: &HomeView<'_>) {
    let s = frame.s;
    let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
        center: frame.point(122.0, 990.0),
        radius: 25.0 * s,
        width: 2.0 * s,
        start: 0.0,
        sweep: std::f32::consts::TAU,
        color: color::alpha(color::GOLD, 0.85),
        knockout: None,
    });
    let initial = initial(view.name);
    text(
        canvas,
        TextFamily::Display,
        format_args!("{initial}"),
        frame.rect(96.0, 972.0, 52.0, 36.0),
        30.0 * s,
        color::GOLD_BRIGHT,
        FontWeight::Semibold,
        TextAlign::Center,
    );
    text(
        canvas,
        TextFamily::Display,
        format_args!("{}", view.name),
        frame.rect(166.0, 962.0, 560.0, 30.0),
        26.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!(
            "{}, {} saber",
            crate::menu::classic::view::Sentence(view.model),
            view.blade_name
        ),
        frame.rect(166.0, 994.0, 560.0, 22.0),
        16.0 * s,
        color::MUTED,
        FontWeight::Regular,
        TextAlign::Start,
    );
}

/// The first letter of `name` past its colour codes and clan tags' symbols,
/// in capitals; `S` for a name with none.
fn initial(name: &str) -> char {
    let mut characters = name.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '^' && characters.peek().is_some_and(char::is_ascii_digit) {
            characters.next();
            continue;
        }
        if character.is_alphanumeric() {
            return character.to_ascii_uppercase();
        }
    }
    'S'
}

/// The keys of the page, bottom centre.
fn hints(canvas: &mut MenuCanvas, frame: &Frame, home: &Home, servers: bool) {
    let s = frame.s;
    let arc_back = if home.page == Page::Main {
        "quit"
    } else {
        "back"
    };
    let rows: [(&[&str], &str); 3] = match home.focus {
        Focus::Arc if servers => [
            (&["Up", "Down"], "turn the ring"),
            (&["Enter"], "open"),
            (&["Right"], "servers"),
        ],
        Focus::Arc => [
            (&["Up", "Down"], "turn the ring"),
            (&["Enter"], "open"),
            (&["Esc"], arc_back),
        ],
        Focus::Server(_) => [
            (&["Up", "Down"], "choose"),
            (&["Enter"], "join"),
            (&["Left"], "back"),
        ],
        Focus::Chat if home.is_typing() => [
            (&["Enter"], "send"),
            (&["Esc"], "stop typing"),
            (&["Backspace"], "erase"),
        ],
        Focus::Chat => [
            (&["Enter"], "type"),
            (&["Up"], "servers"),
            (&["Left"], "back"),
        ],
    };
    let gap = 28.0 * s;
    let total: f32 = rows
        .iter()
        .map(|(keys, action)| key_hint_width(keys, action, s))
        .sum::<f32>()
        + gap * (rows.len() - 1) as f32;
    let [centre, y] = frame.point(960.0, 1004.0);
    let mut x = centre - total * 0.5;
    for (keys, action) in rows {
        x = key_hint(canvas, keys, action, x, y, s) + gap;
    }
}

/// The version, bottom right, and the newer release the update check found.
fn version(canvas: &mut MenuCanvas, frame: &Frame, view: &HomeView<'_>) {
    let s = frame.s;
    text(
        canvas,
        TextFamily::Display,
        format_args!("SJK {}", view.version),
        frame.rect(1224.0, 962.0, 600.0, 28.0),
        22.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::End,
    );
    if let Some(update) = view.update {
        text(
            canvas,
            TextFamily::Body,
            format_args!("SJK {update} is ready to install"),
            frame.rect(1224.0, 994.0, 600.0, 22.0),
            16.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Regular,
            TextAlign::End,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEWPORTS: [[f32; 2]; 6] = [
        [1_920.0, 1_080.0],
        [3_840.0, 2_160.0],
        [2_560.0, 1_080.0],
        [1_440.0, 1_080.0],
        [1_280.0, 1_024.0],
        [1_024.0, 768.0],
    ];

    #[test]
    fn the_frame_fits_every_window_and_keeps_its_proportions() {
        for viewport in VIEWPORTS {
            let frame = Frame::new(viewport);
            let corner = frame.point(1920.0, 1080.0);
            assert!(
                frame.origin[0] >= -0.01 && frame.origin[1] >= -0.01,
                "{viewport:?}"
            );
            assert!(corner[0] <= viewport[0] + 0.01 && corner[1] <= viewport[1] + 0.01);
            // Centred both ways.
            assert!((frame.origin[0] - (viewport[0] - corner[0])).abs() < 0.01);
        }
        // 16:9 fills the window; 4:3 scales the frame to its width.
        assert_eq!(Frame::new([3_840.0, 2_160.0]).s, 2.0);
        assert_eq!(Frame::new([1_440.0, 1_080.0]).s, 0.75);
    }

    #[test]
    fn the_arc_clears_the_ring_and_the_column() {
        for page in [Page::Main, Page::Play, Page::Sjk, Page::Quit] {
            let count = page.entries().len();
            for index in 0..count {
                let [x, y] = entry_point(index, count);
                assert!(x > RING[0] + RING_RADIUS + 60.0, "{page:?} {index}");
                // The widest label at the chosen size ("Quit to desktop", about
                // 400 pixels of Rajdhani at 68) stays short of the column.
                assert!(x + 400.0 < COLUMN_RULE, "{page:?} {index}");
                assert!(y > 120.0 && y < 900.0, "{page:?} {index}");
            }
        }
    }

    // Four servers end above the corners' text, and the dock between them.
    const _: () = assert!(COLUMN_TOP + 64.0 + 4.0 * SERVER_HEIGHT < DOCK_TOP);
    const _: () = assert!(DOCK_FIELD + 42.0 + 22.0 < 962.0);

    #[test]
    fn pages_open_and_close_on_the_ring() {
        let mut home = Home::default();
        // Play opens its page; its Back returns to Play.
        assert_eq!(home.key(KeyCode::Enter, 0), None);
        assert_eq!(home.page, Page::Play);
        assert_eq!(
            home.key(KeyCode::Enter, 0),
            Some(Action::Open(MainDestination::Browser))
        );
        home.key(KeyCode::ArrowUp, 0);
        assert_eq!(home.key(KeyCode::Enter, 0), None, "Back");
        assert_eq!((home.page, home.entry), (Page::Main, 0));
        // Escape on the main page asks to quit, on Stay; Escape again leaves.
        home.key(KeyCode::Escape, 0);
        assert_eq!((home.page, home.entry), (Page::Quit, 1));
        home.key(KeyCode::Escape, 0);
        assert_eq!((home.page, home.entry), (Page::Main, 4));
        // Quit to desktop.
        home.key(KeyCode::Enter, 0);
        home.key(KeyCode::ArrowUp, 0);
        assert_eq!(home.key(KeyCode::Enter, 0), Some(Action::Quit));
        // Settings opens its screen straight away.
        home.reset();
        home.key(KeyCode::ArrowDown, 0);
        home.key(KeyCode::ArrowDown, 0);
        assert_eq!(
            home.key(KeyCode::Enter, 0),
            Some(Action::Open(MainDestination::Settings { tab: 0 }))
        );
    }

    #[test]
    fn the_keyboard_and_pointer_reach_the_servers() {
        let mut home = Home::default();
        // No servers: Right stays on the arc.
        home.key(KeyCode::ArrowRight, 0);
        assert_eq!(home.focus, Focus::Arc);
        home.key(KeyCode::ArrowRight, 3);
        assert_eq!(home.focus, Focus::Server(0));
        home.key(KeyCode::ArrowUp, 3);
        assert_eq!(home.key(KeyCode::Enter, 3), Some(Action::Join(2)));
        home.key(KeyCode::Escape, 3);
        assert_eq!(home.focus, Focus::Arc);
        assert_eq!(
            home.page,
            Page::Main,
            "Escape left the column, not the page"
        );
        // Hovering a server chooses it, a click joins it.
        assert_eq!(home.pointer(SERVER_TOKEN + 1, false, 3), None);
        assert_eq!(home.focus, Focus::Server(1));
        assert_eq!(
            home.pointer(SERVER_TOKEN + 1, true, 3),
            Some(Action::Join(1))
        );
        assert_eq!(home.pointer(SERVER_TOKEN + 3, true, 3), None);
        // Hovering an entry chooses it; a click on SJK opens its page.
        home.pointer(ENTRY_TOKEN + 3, false, 3);
        assert_eq!((home.entry, home.focus), (3, Focus::Arc));
        assert_eq!(home.pointer(ENTRY_TOKEN + 3, true, 3), None);
        assert_eq!(home.page, Page::Sjk);
        assert_eq!(
            home.pointer(ENTRY_TOKEN + 2, true, 3),
            Some(Action::Open(MainDestination::Credits))
        );
    }

    fn server(name: &'static str) -> ServerItem<'static> {
        ServerItem {
            name,
            map: "mp/ffa3",
            live: Some((5, 16, 40)),
            played: Some(Ago::Minutes(5)),
        }
    }

    const LINES: [DockLine<'static>; 2] = [
        DockLine {
            name: "^2Sol",
            text: "gg all",
            verified: true,
            staff: true,
            key_id: "0123456789abcdef",
        },
        DockLine {
            name: "Fox",
            text: "a long message that will not fit on one row of the dock at all, cut",
            verified: false,
            staff: false,
            key_id: "fedcba9876543210",
        },
    ];

    fn view<'a>(servers: &'a [ServerItem<'a>], chat: Option<ChatDock<'a>>) -> HomeView<'a> {
        HomeView {
            name: "Sol",
            model: "kyle",
            blade_name: "blue",
            servers,
            version: "2026.1008.1",
            update: Some("2026.1009.1"),
            seconds: 1.0,
            chat,
        }
    }

    fn dock() -> ChatDock<'static> {
        ChatDock {
            lines: &LINES,
            online: 7,
            live: true,
            notice: "",
            measure: None,
        }
    }

    #[test]
    fn the_dock_fits_under_the_servers_in_every_window() {
        let servers = [server("a"), server("b"), server("c"), server("d")];
        for viewport in VIEWPORTS {
            let mut canvas = MenuCanvas::new();
            let mut home = Home::default();
            build(
                &mut canvas,
                viewport,
                &mut home,
                &view(&servers, Some(dock())),
                1.0,
            );
            let frame = Frame::new(viewport);
            let field = canvas.rect_for(CHAT_TOKEN).expect("the dock's field");
            let open = canvas.rect_for(OPEN_CHAT_TOKEN).expect("Open chat");
            let last = canvas.rect_for(SERVER_TOKEN + 3).unwrap();
            let version_top = frame.point(0.0, 962.0)[1];
            for rect in [field, open] {
                assert!(rect.y > last.bottom(), "{viewport:?}: under the servers");
                assert!(
                    rect.bottom() < version_top,
                    "{viewport:?}: above the version"
                );
                assert!(rect.x >= 0.0 && rect.right() <= viewport[0], "{viewport:?}");
            }
        }
        // Without the chat there is no dock.
        let mut canvas = MenuCanvas::new();
        build(
            &mut canvas,
            VIEWPORTS[0],
            &mut Home::default(),
            &view(&servers, None),
            1.0,
        );
        assert!(canvas.rect_for(CHAT_TOKEN).is_none());
    }

    #[test]
    fn the_dock_shows_the_last_lines_and_who_is_online() {
        let servers = [server("a")];
        let mut canvas = MenuCanvas::new();
        let mut home = Home::default();
        build(
            &mut canvas,
            VIEWPORTS[0],
            &mut home,
            &view(&servers, Some(dock())),
            1.0,
        );
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"SJK chat"), "{runs:?}");
        assert!(runs.contains(&"7 online"), "{runs:?}");
        assert!(runs.contains(&"^2Sol"), "{runs:?}");
        assert!(runs.contains(&"gg all"), "{runs:?}");
        assert!(runs.contains(&"Open chat"), "{runs:?}");
    }

    #[test]
    fn resting_on_a_dock_name_shows_the_profile_card_with_mute() {
        use sjk_ui::{InputEvent, Vec2};
        let servers = [server("a")];
        let mut canvas = MenuCanvas::new();
        let mut home = Home {
            dock: true,
            ..Home::default()
        };
        let draw = |canvas: &mut MenuCanvas, home: &mut Home| {
            build(
                canvas,
                VIEWPORTS[0],
                home,
                &view(&servers, Some(dock())),
                1.0,
            );
        };
        let centre = |rect: Rect| Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        draw(&mut canvas, &mut home);
        assert!(home.card.is_none());
        let name = canvas.rect_for(DOCK_NAME_TOKEN).expect("Sol's name");
        canvas.pointer(InputEvent::PointerMove(centre(name)));
        draw(&mut canvas, &mut home);
        let runs: Vec<&str> = canvas.text_runs().collect();
        for wanted in ["SJK player, SJK staff", "Key 0123456789abcdef", "Mute"] {
            assert!(runs.contains(&wanted), "{wanted:?} in {runs:?}");
        }
        let card = canvas.rect_for(CARD_TOKEN).expect("the card");
        assert!(
            card.x >= name.right() || card.right() <= name.x,
            "beside the name: {card:?} {name:?}"
        );
        let [width, height] = VIEWPORTS[0];
        assert!(card.x >= 0.0 && card.right() <= width && card.bottom() <= height);
        // Onto the card: it stays; Mute mutes Sol, the dock's first line.
        let mute = canvas.rect_for(CARD_MUTE_TOKEN).expect("Mute");
        canvas.pointer(InputEvent::PointerMove(centre(mute)));
        draw(&mut canvas, &mut home);
        assert!(home.card.is_some());
        // Away from both: gone.
        canvas.pointer(InputEvent::PointerMove(Vec2::new(4.0, 4.0)));
        draw(&mut canvas, &mut home);
        assert!(home.card.is_none());
        canvas.pointer(InputEvent::PointerMove(centre(name)));
        draw(&mut canvas, &mut home);
        assert_eq!(home.pointer(CARD_MUTE_TOKEN, true, 1), Some(Action::Mute));
        assert_eq!(
            home.take_mute(),
            Some(("0123456789abcdef".to_owned(), "^2Sol".to_owned()))
        );
        assert_eq!(home.pointer(CARD_MUTE_TOKEN, true, 1), None, "once");
        assert_eq!(home.take_mute(), None);
    }

    #[test]
    fn a_dock_line_flows_with_the_tick_alone_and_its_text_in_gold() {
        let font = crate::text::test_font();
        let servers = [server("a")];
        let mut canvas = MenuCanvas::new();
        let chat = ChatDock {
            measure: Some(crate::sjk_chat_look::Measure::new(
                &font,
                crate::text::TextStyle::NEUTRAL,
            )),
            ..dock()
        };
        build(
            &mut canvas,
            VIEWPORTS[0],
            &mut Home::default(),
            &view(&servers, Some(chat)),
            1.0,
        );
        let texts: Vec<(String, Rect, sjk_ui::Color)> = canvas
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    rect, text, color, ..
                } => Some((canvas.stored_text(*text).to_owned(), *rect, *color)),
                _ => None,
            })
            .collect();
        let find = |wanted: &str| {
            texts
                .iter()
                .find(|(text, ..)| text == wanted)
                .unwrap_or_else(|| panic!("{wanted:?} not in {texts:?}"))
        };
        let (_, name, name_colour) = find("^2Sol");
        let (_, body, body_colour) = find("gg all");
        assert!((name.y - body.y).abs() < 0.01 && body.x > name.right());
        assert_eq!(*body_colour, crate::sjk_chat_look::GOLD);
        assert_ne!(*name_colour, crate::sjk_chat_look::GOLD);
        let ticks: Vec<Rect> = canvas
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::TexturedQuad { rect, texture, .. }
                    if *texture == crate::ui_renderer::VERIFIED_TEXTURE =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(ticks.len(), 1, "Sol is verified, Fox is not");
        assert!(ticks[0].x >= name.right() - 1.0 && ticks[0].right() <= body.x);
    }

    #[test]
    fn the_keyboard_reaches_the_dock_and_types() {
        let mut home = Home {
            dock: true,
            ..Home::default()
        };
        home.key(KeyCode::ArrowRight, 3);
        home.key(KeyCode::ArrowDown, 3);
        home.key(KeyCode::ArrowDown, 3);
        assert_eq!(home.focus, Focus::Server(2));
        home.key(KeyCode::ArrowDown, 3);
        assert_eq!(home.focus, Focus::Chat);
        home.key(KeyCode::ArrowUp, 3);
        assert_eq!(home.focus, Focus::Server(2));
        home.key(KeyCode::ArrowDown, 3);
        assert_eq!(home.key(KeyCode::Enter, 3), None);
        assert!(home.is_typing());
        // Without the dock, Down on the last server goes round to the first.
        let mut home = Home::default();
        home.key(KeyCode::ArrowRight, 3);
        for _ in 0..3 {
            home.key(KeyCode::ArrowDown, 3);
        }
        assert_eq!(home.focus, Focus::Server(0));
    }

    #[test]
    fn dock_typing_takes_letters_not_menu_keys() {
        let mut home = Home {
            dock: true,
            ..Home::default()
        };
        home.focus = Focus::Chat;
        home.key(KeyCode::Enter, 1);
        for (key, text) in [
            (KeyCode::KeyW, "w"),
            (KeyCode::KeyS, "s"),
            (KeyCode::Space, " "),
        ] {
            assert_eq!(home.typing_key(key, Some(text)), None);
        }
        assert_eq!(home.typing_key(KeyCode::ArrowUp, None), None);
        assert_eq!(home.typing_key(KeyCode::Tab, Some("\t")), None);
        assert_eq!(home.focus, Focus::Chat);
        assert_eq!(home.draft.as_deref(), Some("ws "));
        home.typing_key(KeyCode::Backspace, Some("\u{8}"));
        assert_eq!(home.draft.as_deref(), Some("ws"));
        for _ in 0..200 {
            home.typing_key(KeyCode::KeyA, Some("a"));
        }
        assert_eq!(home.draft.as_ref().unwrap().chars().count(), 150);
    }

    #[test]
    fn enter_sends_and_escape_cancels() {
        let mut home = Home {
            dock: true,
            ..Home::default()
        };
        home.focus = Focus::Chat;
        home.key(KeyCode::Enter, 1);
        home.typing_key(KeyCode::KeyG, Some("g"));
        home.typing_key(KeyCode::KeyG, Some("g"));
        assert_eq!(
            home.typing_key(KeyCode::Enter, Some("\r")),
            Some(Action::SendChat)
        );
        assert_eq!(home.take_draft(), "gg");
        assert!(!home.is_typing());
        home.key(KeyCode::Enter, 1);
        home.typing_key(KeyCode::KeyX, Some("x"));
        assert_eq!(home.typing_key(KeyCode::Escape, None), None);
        assert!(!home.is_typing());
        assert_eq!(home.focus, Focus::Chat);
        // Enter on nothing typed only stops typing.
        home.key(KeyCode::Enter, 1);
        assert_eq!(home.typing_key(KeyCode::Enter, Some("\r")), None);
        assert!(!home.is_typing());
    }

    #[test]
    fn the_pointer_types_in_the_dock_and_opens_the_chat() {
        let mut home = Home {
            dock: true,
            ..Home::default()
        };
        assert_eq!(home.pointer(CHAT_TOKEN, false, 1), None);
        assert_eq!(home.focus, Focus::Chat);
        assert!(!home.is_typing());
        assert_eq!(home.pointer(CHAT_TOKEN, true, 1), None);
        assert!(home.is_typing());
        assert_eq!(
            home.pointer(OPEN_CHAT_TOKEN, true, 1),
            Some(Action::OpenChat)
        );
        assert!(!home.is_typing(), "the page takes the typing");
    }

    #[test]
    fn the_arc_eases_the_short_way_round() {
        let mut home = Home::default();
        assert_eq!(home.arc_towards(1.0, 10.0), 1.0);
        let next = home.arc_towards(2.0, 10.05);
        assert!(next > 1.0 && next < 2.0);
        home.arc = Some(3.0);
        assert!(home.arc_towards(-3.0, 10.1) > 3.0);
    }

    #[test]
    fn the_initial_skips_colour_codes_and_symbols() {
        assert_eq!(initial("^5JoF^7 Jedi"), 'J');
        assert_eq!(initial("{JoF}solol"), 'J');
        assert_eq!(initial("^1^2"), 'S');
        assert_eq!(initial("sol"), 'S');
    }
}
