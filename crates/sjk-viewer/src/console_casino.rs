//! SJK casino page, in every menu style. All wagers run on the identity worker.
use super::*;
use crate::menu::sjk::{Frame, TextTarget, color, kit, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use serde_json::{Value, json};
use sjk_ui::{AbstractAction, Color, DrawCommand, FontWeight, InputEvent, TextAlign, UiEventKind};
use std::time::{Duration, Instant};

const GAMES: [&str; 4] = ["Texas Hold'em", "Roulette", "Slots", "Blackjack"];
const CHOICES: [&str; 6] = ["red", "black", "odd", "even", "low", "high"];

/// Retained page and one outstanding request. An uncertain mutation keeps its token
/// until explicitly retried, so a network error cannot repeat a wager.
pub(super) struct Casino {
    pub open: bool,
    pub(super) ui: MenuCanvas,
    game: usize,
    bet: i64,
    table: u8,
    choice: usize,
    number: u8,
    straight: bool,
    data: Value,
    message: String,
    pending: Option<Value>,
    failed: bool,
    due: Instant,
    dirty: bool,
    viewport: [f32; 2],
}

impl Casino {
    pub fn new() -> Self {
        Self {
            open: false,
            ui: MenuCanvas::new(),
            game: 0,
            bet: 10,
            table: 0,
            choice: 0,
            number: 0,
            straight: false,
            data: Value::Null,
            message: String::new(),
            pending: None,
            failed: false,
            due: Instant::now(),
            dirty: true,
            viewport: [0.0; 2],
        }
    }

    fn request(&mut self, action: &str) {
        if self.pending.is_some() {
            return;
        }
        let Ok(token) = sjk_identity::economy_request_token() else {
            self.message = "System randomness is unavailable.".into();
            self.dirty = true;
            return;
        };
        let table_revision = self.data["tables"]
            .as_array()
            .and_then(|tables| {
                tables
                    .iter()
                    .find(|t| t["id"].as_u64() == Some(u64::from(self.table)))
            })
            .and_then(|t| t["revision"].as_u64())
            .unwrap_or(0);
        let request = json!({"token":token,"action":action,"bet":self.bet,"table":self.table,
            "table_revision":table_revision,"revision":self.data["revision"].as_u64().unwrap_or(0),
            "choice":if self.straight {self.number.to_string()} else {CHOICES[self.choice].into()}});
        if crate::player_identity::with_service(|s| s.casino(request.clone())).is_some() {
            self.pending = Some(request);
            self.failed = false;
            self.message = "Waiting for the hub...".into();
        } else {
            self.message = "Enable SJK identity in Settings to use your credits.".into();
        }
        self.due = Instant::now() + Duration::from_secs(1);
        self.dirty = true;
    }

    fn refresh(&mut self) {
        let now = Instant::now();
        if now < self.due {
            return;
        }
        self.due = now + Duration::from_secs(1);
        if let Some(pending) = &self.pending {
            let answer =
                crate::player_identity::with_service(|s| s.with_snapshot(|v| v.casino.clone()))
                    .flatten();
            if let Some((token, result)) = answer
                && pending["token"].as_str() == Some(token.as_str())
            {
                match result {
                    Ok(data) => {
                        // Polls carry no result: keep the most recent roll visible.
                        let last = self.data["result"].take();
                        self.data = data;
                        if self.data["result"].is_null() {
                            self.data["result"] = last;
                        }
                        self.pending = None;
                        self.failed = false;
                        self.message.clear();
                    }
                    Err(error) => {
                        self.message = error;
                        self.failed = true;
                    }
                }
                self.dirty = true;
            }
        } else {
            self.request("read");
        }
    }

    fn activate(&mut self, token: u16) {
        match token {
            0 => self.open = false,
            1..=4 => self.game = usize::from(token - 1),
            5 => {
                self.bet = (self.bet - if self.game == 3 { 2 } else { 1 }).max(if self.game == 3 {
                    2
                } else {
                    1
                })
            }
            6 | 22 | 23 => {
                self.bet = self
                    .bet
                    .saturating_add(match token {
                        22 => 10,
                        23 => 100,
                        _ => {
                            if self.game == 3 {
                                2
                            } else {
                                1
                            }
                        }
                    })
                    .min(if self.game == 0 { 100_000 } else { 500 })
            }
            7 => self.table = (self.table + 15) % 16,
            8 => self.table = (self.table + 1) % 16,
            9 => {
                self.choice = (self.choice + 1) % 6;
                self.straight = false;
            }
            10 => {
                self.number = (self.number + 1) % 37;
                self.straight = true;
            }
            11 => self.request(["join", "roulette", "slots", "blackjack"][self.game]),
            12 => self.request("start"),
            13 => self.request("leave"),
            14 => self.request("fold"),
            15 => self.request("call"),
            16 => self.request("raise"),
            17 => self.request("hit"),
            18 => self.request("stand"),
            19 => {
                if let Some(request) = &self.pending {
                    let _ = crate::player_identity::with_service(|s| s.casino(request.clone()));
                    self.failed = false;
                    self.message = "Retrying the same request...".into();
                } else {
                    self.request("read");
                }
            }
            21 => {
                let table = self.data["tables"].as_array().and_then(|t| {
                    t.iter()
                        .find(|t| t["id"].as_u64() == Some(u64::from(self.table)))
                });
                if let Some(table) = table
                    && let Some(seat) = table["seats"]
                        .as_array()
                        .and_then(|seats| seats.iter().find(|s| s["person"] == self.data["person"]))
                {
                    self.bet =
                        seat["round"].as_i64().unwrap_or(0) + seat["stack"].as_i64().unwrap_or(0);
                    let call = self.bet <= table["current"].as_i64().unwrap_or(0);
                    self.request(if call { "call" } else { "raise" });
                }
            }
            20 => {
                // Explicit refresh after a definite rejection; no new wager.
                self.pending = None;
                self.failed = false;
                self.request("read");
            }
            _ => {}
        }
        if self.game != 0 {
            self.bet = self.bet.min(500);
        }
        if self.game == 3 {
            self.bet = ((self.bet + 1) / 2 * 2).min(500);
        }
        self.dirty = true;
    }

    fn words(&mut self, frame: &Frame, words: &str, x: f32, y: f32, size: f32, colour: Color) {
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{words}"),
            frame.rect(x, y, 1700.0, 42.0),
            size * frame.s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn button(
        &mut self,
        f: &Frame,
        token: u16,
        words: &str,
        x: f32,
        y: f32,
        width: f32,
        enabled: bool,
    ) {
        kit::button(
            &mut self.ui,
            f,
            [x, y, width, 48.0],
            words,
            false,
            enabled,
            false,
            token,
        );
    }

    fn build(&mut self, viewport: [f32; 2]) {
        self.ui.begin_transparent(viewport);
        let f = Frame::new(viewport);
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]),
            color: Color::new(0.015, 0.025, 0.04, 0.94),
        });
        self.words(&f, "SJK CASINO", 100.0, 55.0, 46.0, color::GOLD);
        self.words(
            &f,
            &format!("{} credits", self.data["credits"].as_i64().unwrap_or(0)),
            1390.0,
            65.0,
            28.0,
            color::GOLD,
        );
        self.button(&f, 0, "Close", 1670.0, 55.0, 150.0, true);
        for (i, name) in GAMES.iter().enumerate() {
            self.button(
                &f,
                i as u16 + 1,
                name,
                100.0 + i as f32 * 330.0,
                145.0,
                300.0,
                true,
            );
        }
        let ready = self.pending.is_none() && !self.data.is_null();
        self.words(
            &f,
            &format!(
                "{}  |  wager / raise to: {} credits",
                GAMES[self.game], self.bet
            ),
            100.0,
            230.0,
            30.0,
            color::GOLD,
        );
        self.button(
            &f,
            5,
            if self.game == 3 { "-2" } else { "-1" },
            1210.0,
            230.0,
            100.0,
            true,
        );
        self.button(
            &f,
            6,
            if self.game == 3 { "+2" } else { "+1" },
            1320.0,
            230.0,
            100.0,
            true,
        );
        self.button(&f, 22, "+10", 1430.0, 230.0, 100.0, true);
        self.button(&f, 23, "+100", 1540.0, 230.0, 100.0, true);
        match self.game {
            0 => {
                self.words(
                    &f,
                    &format!(
                        "Table {} / 15  |  100-credit buy-in, blinds 1 / 2, no rake",
                        self.table
                    ),
                    100.0,
                    300.0,
                    23.0,
                    color::QUIET,
                );
                self.button(&f, 7, "Previous", 1300.0, 300.0, 160.0, true);
                self.button(&f, 8, "Next", 1470.0, 300.0, 160.0, true);
                let table = self.data["tables"]
                    .as_array()
                    .and_then(|t| {
                        t.iter()
                            .find(|t| t["id"].as_u64() == Some(u64::from(self.table)))
                    })
                    .cloned()
                    .unwrap_or(Value::Null);
                self.words(
                    &f,
                    &format!("Board: {}", cards(&table["board"])),
                    100.0,
                    370.0,
                    30.0,
                    color::GOLD,
                );
                if let Some(seats) = table["seats"].as_array() {
                    for (i, seat) in seats.iter().enumerate() {
                        let person = seat["person"].as_str().unwrap_or("?");
                        let me = self.data["person"].as_str() == Some(person);
                        let name = if me {
                            "You"
                        } else {
                            seat["name"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| person.get(..12).unwrap_or(person))
                        };
                        self.words(
                            &f,
                            &format!(
                                "{}{}   {} credits   bet {}   {}{}",
                                if table["turn"].as_u64() == Some(i as u64)
                                    && table["street"]
                                        .as_u64()
                                        .is_some_and(|n| (1..=4).contains(&n))
                                {
                                    "> "
                                } else {
                                    "  "
                                },
                                name,
                                seat["stack"],
                                seat["round"],
                                cards(&seat["hole"]),
                                if seat["folded"] == true {
                                    "  folded"
                                } else {
                                    ""
                                }
                            ),
                            100.0,
                            435.0 + i as f32 * 52.0,
                            24.0,
                            color::QUIET,
                        );
                    }
                }
                self.words(
                    &f,
                    &format!(
                        "Street {}   current bet {}   minimum raise {}   30 seconds per turn",
                        table["street"].as_u64().unwrap_or(0),
                        table["current"].as_i64().unwrap_or(0),
                        table["min_raise"].as_i64().unwrap_or(2)
                    ),
                    100.0,
                    770.0,
                    22.0,
                    color::QUIET,
                );
                for (i, (token, name)) in [
                    (11, "Join"),
                    (12, "Ready for hand"),
                    (13, "Cash out"),
                    (14, "Fold"),
                    (15, "Check / call"),
                    (16, "Raise to wager"),
                    (21, "All in"),
                ]
                .iter()
                .enumerate()
                {
                    self.button(
                        &f,
                        *token,
                        name,
                        100.0 + i as f32 * 245.0,
                        835.0,
                        230.0,
                        ready,
                    );
                }
            }
            1 => {
                self.words(
                    &f,
                    "European wheel: 0..36. Zero loses all even-money bets.",
                    100.0,
                    330.0,
                    27.0,
                    color::QUIET,
                );
                self.words(
                    &f,
                    "Even money returns 2x; a single number returns 36x (stake included).",
                    100.0,
                    390.0,
                    24.0,
                    color::QUIET,
                );
                self.button(
                    &f,
                    9,
                    &format!("Even money: {}", CHOICES[self.choice]),
                    100.0,
                    475.0,
                    420.0,
                    true,
                );
                self.button(
                    &f,
                    10,
                    &format!("Single number: {}", self.number),
                    550.0,
                    475.0,
                    420.0,
                    true,
                );
                self.words(
                    &f,
                    &format!(
                        "Selected: {}",
                        if self.straight {
                            self.number.to_string()
                        } else {
                            CHOICES[self.choice].into()
                        }
                    ),
                    100.0,
                    550.0,
                    28.0,
                    color::GOLD,
                );
                self.button(&f, 11, "Place wager & spin", 100.0, 635.0, 420.0, ready);
            }
            2 => {
                self.words(
                    &f,
                    "Three reels. Six equally likely symbols per reel.",
                    100.0,
                    330.0,
                    28.0,
                    color::QUIET,
                );
                self.words(
                    &f,
                    "Three matching symbols pay 5 / 10 / 20 / 30 / 50 / 90x. Other spins pay zero.",
                    100.0,
                    395.0,
                    24.0,
                    color::QUIET,
                );
                self.words(
                    &f,
                    "Returns include the stake. Theoretical return: 205 / 216 (94.91%).",
                    100.0,
                    450.0,
                    24.0,
                    color::QUIET,
                );
                self.button(&f, 11, "Place wager & spin", 100.0, 570.0, 420.0, ready);
            }
            _ => {
                self.words(
                    &f,
                    "Single deck. Natural pays 3:2; dealer stands on soft 17. Even wagers only.",
                    100.0,
                    330.0,
                    24.0,
                    color::QUIET,
                );
                self.words(
                    &f,
                    "No split, double or insurance. Close and reopen to resume an unfinished hand.",
                    100.0,
                    385.0,
                    23.0,
                    color::QUIET,
                );
                let g = &self.data["blackjack"];
                let player = format!("Your cards: {}", cards(&g["player"]));
                let dealer = format!(
                    "Dealer: {}{}",
                    cards(&g["dealer"]),
                    if g["done"] == false {
                        "   [hidden]"
                    } else {
                        ""
                    }
                );
                let result = format!("Returned: {} credits", g["returned"].as_i64().unwrap_or(0));
                self.words(&f, &dealer, 100.0, 470.0, 32.0, color::GOLD);
                self.words(&f, &player, 100.0, 535.0, 32.0, color::GOLD);
                self.words(&f, &result, 100.0, 600.0, 24.0, color::QUIET);
                self.button(
                    &f,
                    11,
                    "Place wager & deal",
                    100.0,
                    690.0,
                    400.0,
                    ready && self.data["blackjack"]["done"] != false,
                );
                self.button(
                    &f,
                    17,
                    "Hit",
                    540.0,
                    690.0,
                    200.0,
                    ready && self.data["blackjack"]["done"] == false,
                );
                self.button(
                    &f,
                    18,
                    "Stand",
                    780.0,
                    690.0,
                    200.0,
                    ready && self.data["blackjack"]["done"] == false,
                );
            }
        }
        if self.game == 1 || self.game == 2 {
            let result = &self.data["result"];
            let words = if self.game == 1 && result["game"] == "roulette" {
                format!(
                    "Last spin: {}. Wager {} / returned {} credits",
                    result["number"], result["bet"], result["returned"]
                )
            } else if self.game == 2 && result["game"] == "slots" {
                format!(
                    "Reels: {}. Wager {} / returned {} credits",
                    result["reels"], result["bet"], result["returned"]
                )
            } else {
                String::new()
            };
            self.words(&f, &words, 100.0, 770.0, 28.0, color::GOLD);
        }
        self.words(&f, &self.message.clone(), 100.0, 930.0, 20.0, color::QUIET);
        self.button(
            &f,
            19,
            if self.failed {
                "Retry same request"
            } else {
                "Refresh"
            },
            1260.0,
            970.0,
            280.0,
            self.failed || self.pending.is_none(),
        );
        self.button(
            &f,
            20,
            "Read current state",
            1560.0,
            970.0,
            260.0,
            self.failed,
        );
        self.words(&f,"Credits are real SJK credits. Esc closes this page; it keeps your seat and active hand.",100.0,1000.0,18.0,color::QUIET);
        self.viewport = viewport;
        self.dirty = false;
    }
}

fn cards(value: &Value) -> String {
    const RANKS: [&str; 13] = [
        "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K", "A",
    ];
    const SUITS: [&str; 4] = ["C", "D", "H", "S"];
    value
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .filter_map(|c| c.as_u64())
                .filter(|c| *c < 52)
                .map(|c| format!("{}{}", RANKS[c as usize % 13], SUITS[c as usize / 13]))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .unwrap_or_default()
}

impl ViewerConsole {
    #[cfg(test)]
    pub(crate) fn preview_casino(&mut self, game: usize) {
        if !self.casino.open {
            self.toggle_casino();
        }
        self.casino.open = true;
        self.casino.game = game;
        self.casino.pending = None;
        self.casino.due = Instant::now() + Duration::from_secs(3600);
        self.casino.data = json!({"revision":1,"credits":12500,"person":"you",
            "tables":[{"id":0,"revision":4,"street":2,"current":10,"min_raise":8,"turn":0,
                "board":[12,24,33],"seats":[{"person":"you","stack":90,"round":0,"hole":[11,25],"folded":false},
                {"person":"player-two","name":"Player Two","stack":90,"round":10,"hole":[],"folded":false}]}],
            "blackjack":{"bet":10,"player":[12,8],"dealer":[5,9],"done":true,"returned":25},
            "result":{"game":if game==1 {"roulette"} else {"slots"},"number":17,"reels":[4,4,4],"bet":10,"returned":if game==1 {20} else {500}}});
        self.casino.dirty = true;
    }
    pub(crate) fn toggle_casino(&mut self) {
        if self.casino.open {
            self.casino.open = false;
            self.set_open(false);
            return;
        }
        self.set_open(false);
        self.set_open(true);
        self.casino.open = true;
        self.casino.due = Instant::now();
        self.casino.dirty = true;
    }

    pub(super) fn casino_key(&mut self, event: &KeyEvent) -> bool {
        if !self.casino.open {
            return false;
        }
        if event.state != ElementState::Pressed {
            return true;
        }
        let action = match event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) => {
                self.casino.open = false;
                self.set_open(false);
                return true;
            }
            PhysicalKey::Code(KeyCode::Tab | KeyCode::ArrowDown | KeyCode::ArrowRight) => {
                Some(if self.shift {
                    AbstractAction::Previous
                } else {
                    AbstractAction::Next
                })
            }
            PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::ArrowLeft) => {
                Some(AbstractAction::Previous)
            }
            PhysicalKey::Code(KeyCode::Enter | KeyCode::Space) if !event.repeat => {
                Some(AbstractAction::Accept)
            }
            _ => None,
        };
        if let Some(token) = action.and_then(|a| self.casino.ui.action(a)) {
            self.casino.activate(token);
        }
        if !self.casino.open {
            self.set_open(false);
        }
        self.casino.dirty = true;
        true
    }

    pub(super) fn casino_pointer(&mut self, event: InputEvent) -> bool {
        if !self.casino.open {
            return false;
        }
        if let Some(e) = self.casino.ui.pointer(event)
            && matches!(e.kind, UiEventKind::Click | UiEventKind::Activate)
            && let Some(t) = e.token
        {
            self.casino.activate(t);
        }
        if !self.casino.open {
            self.set_open(false);
        }
        self.casino.dirty = true;
        true
    }

    pub(super) fn append_casino(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        self.casino.refresh();
        if self.casino.dirty || viewport != self.casino.viewport {
            self.casino.build(viewport);
        }
        target.append(&self.casino.ui, viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_four_pages_fit_and_open_in_sjk_style() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        for game in 0..4 {
            console.preview_casino(game);
            for viewport in [[1920.0, 1080.0], [1024.0, 768.0], [3840.0, 2160.0]] {
                console.casino.build(viewport);
                assert!(!console.casino.ui.overflowed());
                assert!(console.casino.ui.widget_count() >= 9);
                assert!(console.sjk_page_open());
                assert!(console.covers_frame());
            }
        }
    }
    #[test]
    fn whole_credit_wagers_and_blackjack_even_bets_are_selectable() {
        let mut page = Casino::new();
        page.bet = 2;
        page.activate(5);
        assert_eq!(page.bet, 1);
        page.activate(6);
        assert_eq!(page.bet, 2);
        page.activate(22);
        assert_eq!(page.bet, 12);
        page.bet = 499;
        page.activate(4);
        assert_eq!(page.bet, 500);
        page.activate(5);
        assert_eq!(page.bet, 498);
        page.activate(23);
        assert_eq!(page.bet, 500);
    }
    #[test]
    fn closing_preserves_an_uncertain_wager_token() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.toggle_casino();
        console.casino.pending = Some(json!({"token":"abc","action":"slots"}));
        console.set_open(false);
        assert!(!console.casino.open);
        assert_eq!(console.casino.pending.as_ref().unwrap()["token"], "abc");
        console.toggle_casino();
        assert!(console.casino.open);
    }
}
