//! The background service that keeps the player registered with the hub, repeats
//! their game-server claim while they play, tells the hub the look they wear there,
//! and reads who else on that server is known. Nothing here blocks a frame: the viewer sends [`Command`]s through
//! [`Service`] and reads a [`Snapshot`].
//!
//! The service is inert while it is disabled or has no hub address: it makes no
//! request of any kind.

use crate::feed::{ChatState, FeedShared, FeedWorker, QueuedLook, ReceivedLooks};
use crate::hub::{Hub, HubError};
use crate::keys::Identity;
use crate::report::{BugReport, PlayerReport, WorldNote};
use crate::staff::{StaffRequest, StaffState};
use crate::wire::{Achievement, Emote, Look, Presence, Profile, Unlock, names_match};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How often a claim is repeated; a claim lives 90 seconds at the hub.
const CLAIM_EVERY: Duration = Duration::from_secs(45);
/// How often the roster of known players is read.
const POLL_EVERY: Duration = Duration::from_secs(15);
/// How often the player's own profile is read again while registered, so a medal
/// the SJK team gives during a session shows without a restart.
const PROFILE_EVERY: Duration = Duration::from_secs(600);
/// Shortest time between two readings of the own profile brought forward because its
/// unlocks look out of date (a skin refused, or taken back at the hub).
const PROFILE_SOON: Duration = Duration::from_secs(30);
/// Shortest time between two sendings of the achievement counts.
const ACHIEVEMENTS_EVERY: Duration = Duration::from_secs(60);
/// How long counts the hub held back (over an hourly allowance) wait before they are
/// sent again unchanged, and how long a refusal waits.
const ACHIEVEMENTS_AGAIN: Duration = Duration::from_secs(3_600);
/// First wait after a failure, doubled up to [`RETRY_MAX`].
const RETRY_MIN: Duration = Duration::from_secs(10);
const RETRY_MAX: Duration = Duration::from_secs(120);
/// Longest the worker sleeps without looking at its commands.
const IDLE_MAX: Duration = Duration::from_secs(60);
/// Shortest time between two looks sent: the hub takes one a second.
const LOOK_EVERY: Duration = Duration::from_secs(1);
/// How long a look waits after the hub said too many (429: `look_quota`, or the
/// address's `rate_limited`), refused the signature (401) or failed.
const LOOK_AGAIN: Duration = Duration::from_secs(10);

/// What the player has configured.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Settings {
    /// Whether the identity features are on.
    pub enabled: bool,
    /// The hub's address; empty means no hub, so nothing is sent.
    pub hub_url: String,
}

/// Where the player is: the game server they are connected to and how the game
/// shows them there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    /// The server's address as connected.
    pub server: SocketAddr,
    /// The player's own client number there.
    pub slot: u8,
    /// The player's name as the game shows it.
    pub name: String,
}

/// How the service is doing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    /// Turned off in the settings.
    Disabled,
    /// On, but there is no hub address.
    NoHub,
    /// Registering with the hub.
    Registering,
    /// Registered; claims and reads work.
    Online,
    /// The last hub request failed (the text says why); the service retries.
    Failed(String),
}

/// What the viewer may read.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// The service's status.
    pub status: Status,
    /// The player's own key id.
    pub key_id: String,
    /// The player's profile at the hub, once registered.
    pub me: Option<Profile>,
    /// The game server being claimed on, if any.
    pub server: Option<SocketAddr>,
    /// Known players on that server, by slot.
    pub players: Vec<Presence>,
    /// Profiles fetched with [`Service::look_up`].
    pub profiles: HashMap<String, Profile>,
    /// The outcome of the last profile change or lookup, for the page to show.
    pub notice: Option<String>,
    /// Counts changes to [`Snapshot::players`], so a reader that caches what it
    /// derived from them (scoreboard tags) knows when to derive again.
    pub revision: u64,
    /// The outcome of the last bug report sent with [`Service::report`].
    pub report: Option<ReportOutcome>,
    /// The outcome of the last world note sent with [`Service::note`].
    pub note: Option<ReportOutcome>,
    /// The outcome of the last player report sent with [`Service::player_report`].
    pub player_report: Option<ReportOutcome>,
    /// What became of the last look sent ([`Service::set_look`]).
    pub look_outcome: Option<ReportOutcome>,
}

/// What became of a bug report or a world note.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportOutcome {
    /// Counts reports sent, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// The hub stored it.
    pub sent: bool,
    /// The hub's report number, or why it was not stored.
    pub message: String,
}

impl Snapshot {
    fn new(key_id: String) -> Self {
        Self {
            status: Status::Disabled,
            key_id,
            me: None,
            server: None,
            players: Vec::new(),
            profiles: HashMap::new(),
            notice: None,
            revision: 0,
            report: None,
            note: None,
            player_report: None,
            look_outcome: None,
        }
    }

    /// The known player in `slot` whose claimed name matches `shown`, the name the
    /// game shows there. A claim for another name is somebody else's and is ignored.
    pub fn badge(&self, slot: u8, shown: &str) -> Option<&Presence> {
        self.players
            .iter()
            .find(|player| player.slot == slot && names_match(&player.claimed_name, shown))
    }
}

/// A request to the worker.
enum Command {
    Configure(Settings),
    Enter(Location),
    Leave,
    /// The in-game name the player now wears.
    Name(String),
    SetBio(String),
    /// The counts the client keeps for its achievements, by id.
    Achievements(BTreeMap<String, u64>),
    /// A staff request.
    Staff(StaffRequest),
    LookUp(String),
    Report(BugReport),
    PlayerReport(PlayerReport),
    /// A world note and the tag its picture will come with.
    Note(u64, WorldNote),
    /// The picture of the note tagged so.
    NoteImage(u64, Vec<u8>),
    /// An SJK chat message.
    Chat(String),
    /// An emote, for the slot the player claims.
    Emote(String),
    /// Whether the SJK chat is on.
    SetChat(bool),
    /// The look the player wears.
    Look(Look),
    /// The feed relayed a look event of the player's own key: its id and blade skin.
    OwnLook(u64, String),
    Stop,
}

fn lock_chat(chat: &Mutex<ChatState>) -> MutexGuard<'_, ChatState> {
    crate::feed::lock(chat)
}

fn lock_feed(feed: &Mutex<FeedShared>) -> MutexGuard<'_, FeedShared> {
    crate::feed::lock(feed)
}

/// Notes whose pictures may still come, kept by tag: the newest few only.
const NOTES_AWAITING_PICTURES: usize = 8;

/// Builds the hub client for an address; `Err` for an address that cannot be used.
pub type HubFactory = Box<dyn Fn(&str) -> Result<Box<dyn Hub>, HubError> + Send>;

/// The worker's logic, advanced by [`Worker::handle`] and [`Worker::tick`] with
/// an explicit clock so it can be tested without waiting.
struct Worker {
    identity: Identity,
    make_hub: HubFactory,
    settings: Settings,
    hub: Option<Box<dyn Hub>>,
    location: Option<Location>,
    /// The server address the hub holds a claim for, to withdraw it.
    claimed: Option<String>,
    registered: bool,
    /// The in-game name the player wears, and the one the hub was last told.
    name: Option<String>,
    name_sent: Option<String>,
    snapshot: Arc<Mutex<Snapshot>>,
    due_register: Instant,
    due_name: Instant,
    due_claim: Instant,
    due_poll: Instant,
    due_profile: Instant,
    backoff: Duration,
    lookups: Vec<String>,
    /// Notes the hub took, as (tag, hub id), while their pictures may come.
    notes_sent: Vec<(u64, i64)>,
    /// The achievement counts the client keeps, the last ones the hub was sent, and
    /// when they may be sent next.
    counts: BTreeMap<String, u64>,
    counts_sent: Option<BTreeMap<String, u64>>,
    due_counts: Instant,
    /// What staff requests brought back.
    staff: Arc<Mutex<StaffState>>,
    /// Whether the SJK chat is on (`cl_sjkChat`).
    chat_on: bool,
    /// What the feed thread is told to read.
    feed: Arc<Mutex<FeedShared>>,
    /// What the chat shows; the worker writes the outcome of what it sends.
    chat: Arc<Mutex<ChatState>>,
    /// The look the viewer last gave.
    look: Look,
    /// The claim (server, slot and name) the hub accepted last, while it holds it: a
    /// look lives on it.
    look_claim: Option<(String, u8, String)>,
    /// The look the hub holds for the key: on the accepted claim, or, with none
    /// accepted now, on a claim that may still be live.
    look_held: Held,
    /// When the next look may go.
    due_look: Instant,
    /// Blade skins the hub said the key does not hold, and the unlocks the profile
    /// listed then: they are not sent again until those change.
    look_refused: Vec<String>,
    look_refused_for: Vec<Unlock>,
    /// The feed id of the last look the hub took from this worker.
    look_id: Option<u64>,
    /// When the own profile may next be read early ([`Worker::profile_soon`]).
    profile_soon_after: Instant,
}

/// What the hub holds as the player's look.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Held {
    /// Not known: the worker just started (a client that stopped without releasing may
    /// have left a live claim with a look), a claim failed (the last one may still be
    /// live, its look with it) or a release failed. The look goes on the next accepted
    /// claim whatever it is.
    Unknown,
    /// This look; [`Look::default`] for none, as a new claim starts.
    Look(Look),
}

impl Held {
    /// No look: what a new claim starts with.
    fn none() -> Self {
        Self::Look(Look::default())
    }
}

/// The refusal of anything sent before the hub answered the registration.
fn not_connected() -> HubError {
    HubError::Protocol("not connected to the hub (is identity on, cl_identity 1?)".to_owned())
}

/// What the player is told of a report or a note: what the hub stored it as, or why
/// it did not.
fn outcome_of(serial: u64, outcome: Result<String, HubError>) -> ReportOutcome {
    match outcome {
        Ok(message) => ReportOutcome {
            serial,
            sent: true,
            message,
        },
        Err(HubError::Rejected { message, .. }) => ReportOutcome {
            serial,
            sent: false,
            message,
        },
        Err(error) => ReportOutcome {
            serial,
            sent: false,
            message: error.to_string(),
        },
    }
}

fn lock(snapshot: &Mutex<Snapshot>) -> MutexGuard<'_, Snapshot> {
    snapshot
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Worker {
    fn new(
        identity: Identity,
        make_hub: HubFactory,
        snapshot: Arc<Mutex<Snapshot>>,
        staff: Arc<Mutex<StaffState>>,
        now: Instant,
    ) -> Self {
        Self {
            identity,
            make_hub,
            settings: Settings::default(),
            hub: None,
            location: None,
            claimed: None,
            registered: false,
            name: None,
            name_sent: None,
            snapshot,
            due_register: now,
            due_name: now,
            due_claim: now,
            due_poll: now,
            due_profile: now,
            backoff: RETRY_MIN,
            lookups: Vec::new(),
            notes_sent: Vec::new(),
            counts: BTreeMap::new(),
            counts_sent: None,
            due_counts: now,
            staff,
            chat_on: false,
            feed: Arc::default(),
            chat: Arc::default(),
            look: Look::default(),
            look_claim: None,
            look_held: Held::Unknown,
            due_look: now,
            look_refused: Vec::new(),
            look_refused_for: Vec::new(),
            look_id: None,
            profile_soon_after: now,
        }
    }

    /// Read the own profile soon: its unlocks look out of date. At most once every
    /// [`PROFILE_SOON`], and never later than it was due anyway.
    fn profile_soon(&mut self, now: Instant) {
        let at = self.profile_soon_after.max(now);
        if at < self.due_profile {
            self.due_profile = at;
            self.profile_soon_after = at + PROFILE_SOON;
        }
    }

    /// The feed relayed look `id` of the player's own key, with blade skin `saber`.
    /// One the hub made after the last this worker sent, wearing another skin than
    /// the one to send, is the hub taking the skin back (staff relocked it): the
    /// profile's unlocks are out of date.
    fn own_look_seen(&mut self, id: u64, saber: &str, now: Instant) {
        if self.look_id.is_none_or(|sent| id > sent) && saber != self.look_to_send().saber {
            self.profile_soon(now);
        }
    }

    /// Tell the feed thread what to read: the hub, only while the identity is on and
    /// the hub answered the registration, and then on a game server (for its looks
    /// and emotes, whatever the chat says) or with the chat on; the server played on;
    /// and whether the chat shows. Another hub or server, or none, is a new
    /// generation: the looks read before are dropped.
    fn publish_feed(&self) {
        let reading = self.chat_on || self.location.is_some();
        let url = (self.settings.enabled && self.registered && reading && self.hub.is_some())
            .then(|| self.settings.hub_url.trim().to_owned());
        let wanted = FeedShared {
            url,
            server: self
                .location
                .as_ref()
                .map(|location| location.server.to_string()),
            chat: self.chat_on,
            generation: 0,
        };
        let mut shared = lock_feed(&self.feed);
        let generation = if (&shared.url, &shared.server) == (&wanted.url, &wanted.server) {
            shared.generation
        } else {
            shared.generation + 1
        };
        let wanted = FeedShared {
            generation,
            ..wanted
        };
        if *shared != wanted {
            *shared = wanted;
        }
    }

    /// Put what became of a message or an emote where the chat shows it.
    fn chat_outcome(&self, outcome: Result<String, HubError>) {
        let mut chat = lock_chat(&self.chat);
        let serial = chat.outcome.as_ref().map_or(1, |last| last.serial + 1);
        chat.outcome = Some(outcome_of(serial, outcome));
    }

    /// Send an SJK chat message under the rules, once registered.
    fn send_chat(&mut self, text: &str) {
        let outcome = match crate::chat::check(text) {
            Err(error) => Err(HubError::Rejected {
                status: 400,
                code: error.code().to_owned(),
                message: error.message().to_owned(),
            }),
            Ok(text) => match (self.hub.as_mut(), self.registered) {
                (Some(hub), true) => {
                    let name = self.name.clone().unwrap_or_default();
                    hub.chat(&self.identity, &text, &name)
                        .map(|_| String::new())
                }
                _ => Err(not_connected()),
            },
        };
        self.chat_outcome(outcome);
    }

    /// Send an emote for the slot the player claims on their server.
    fn send_emote(&mut self, emote: &str) {
        let well_formed = (1..=32).contains(&emote.len())
            && emote
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        let server = self
            .location
            .as_ref()
            .map(|location| location.server.to_string());
        let outcome = match (self.hub.as_mut(), self.registered, server) {
            _ if !well_formed => Err(HubError::Rejected {
                status: 400,
                code: "bad_emote".to_owned(),
                message: "an emote is 1 to 32 of a to z, 0 to 9 and _".to_owned(),
            }),
            (Some(_), true, None) => Err(HubError::Rejected {
                status: 403,
                code: "not_on_server".to_owned(),
                message: "join a server to emote".to_owned(),
            }),
            (Some(hub), true, Some(server)) => hub
                .emote(&self.identity, &server, emote)
                .map(|_| String::new()),
            _ => Err(not_connected()),
        };
        self.chat_outcome(outcome);
    }

    fn update(&self, change: impl FnOnce(&mut Snapshot)) {
        change(&mut lock(&self.snapshot));
    }

    fn failed(&mut self, error: &HubError, now: Instant) -> Instant {
        self.update(|snapshot| snapshot.status = Status::Failed(error.to_string()));
        let due = now + self.backoff;
        self.backoff = (self.backoff * 2).min(RETRY_MAX);
        due
    }

    /// Withdraw the claim the hub holds, if any, ignoring a failure: it expires. The
    /// look on it goes with it; after a failure the hub may hold both a while longer.
    fn release(&mut self) {
        match (self.claimed.take(), self.hub.as_mut()) {
            (Some(server), Some(hub)) => {
                self.look_held = match hub.release(&self.identity, &server) {
                    Ok(()) => Held::none(),
                    Err(_) => Held::Unknown,
                };
            }
            (Some(_), None) => self.look_held = Held::Unknown,
            (None, _) => {}
        }
        self.look_claim = None;
        self.update(|snapshot| {
            snapshot.server = None;
            snapshot.players.clear();
            snapshot.revision += 1;
        });
    }

    fn handle(&mut self, command: Command, now: Instant) {
        match command {
            Command::Configure(settings) => self.configure(settings, now),
            Command::Enter(location) => {
                if self.location.as_ref() == Some(&location) {
                    return;
                }
                if self.location.as_ref().map(|old| old.server) != Some(location.server) {
                    self.release();
                }
                self.update(|snapshot| snapshot.server = Some(location.server));
                self.location = Some(location);
                self.due_claim = now;
                self.due_poll = now;
            }
            Command::Leave => {
                self.release();
                self.location = None;
            }
            Command::Name(name) => {
                let name = Some(name).filter(|name| !name.trim().is_empty());
                if name != self.name {
                    self.name = name;
                    self.due_name = now;
                }
            }
            Command::SetBio(bio) => self.set_bio(&bio),
            Command::Achievements(counts) => self.counts = counts,
            Command::Staff(request) => self.staff(&request),
            Command::Report(mut report) => {
                if report.name.is_empty() {
                    report.name = self.name.clone().unwrap_or_default();
                }
                self.report(&report);
            }
            Command::PlayerReport(mut report) => {
                if report.name.is_empty() {
                    report.name = self.name.clone().unwrap_or_default();
                }
                self.player_report(&report);
            }
            Command::Note(tag, mut note) => {
                if note.name.is_empty() {
                    note.name = self.name.clone().unwrap_or_default();
                }
                self.note(tag, &note);
            }
            Command::NoteImage(tag, jpeg) => self.note_image(tag, &jpeg),
            Command::LookUp(key_id) => self.lookups.push(key_id),
            Command::Chat(text) => self.send_chat(&text),
            Command::Emote(emote) => self.send_emote(&emote),
            Command::SetChat(on) => self.chat_on = on,
            Command::Look(look) => self.look = look,
            Command::OwnLook(id, saber) => self.own_look_seen(id, &saber, now),
            Command::Stop => self.release(),
        }
        self.publish_feed();
    }

    fn configure(&mut self, settings: Settings, now: Instant) {
        if settings == self.settings {
            return;
        }
        // Anything that changes the hub or switches the feature off ends the claim
        // first, on the hub that holds it.
        self.release();
        self.hub = None;
        self.registered = false;
        self.name_sent = None;
        self.update(|snapshot| snapshot.me = None);
        if settings.enabled && !settings.hub_url.trim().is_empty() {
            match (self.make_hub)(settings.hub_url.trim()) {
                Ok(hub) => self.hub = Some(hub),
                Err(error) => {
                    self.update(|snapshot| snapshot.status = Status::Failed(error.to_string()))
                }
            }
        }
        self.settings = settings;
        self.backoff = RETRY_MIN;
        self.due_register = now;
        self.due_claim = now;
        self.due_poll = now;
    }

    fn note(&mut self, tag: u64, note: &WorldNote) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.note(&self.identity, note),
            _ => Err(HubError::Protocol(
                "not connected to the hub (is identity on, cl_identity 1?)".to_owned(),
            )),
        };
        if let Ok(id) = outcome {
            if self.notes_sent.len() >= NOTES_AWAITING_PICTURES {
                self.notes_sent.remove(0);
            }
            self.notes_sent.push((tag, id));
        }
        self.update(|snapshot| {
            let serial = snapshot.note.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.note = Some(outcome_of(serial, outcome.map(|id| format!("note #{id}"))));
        });
    }

    /// Attach `jpeg` to the note tagged `tag`, if the hub took that note. A picture
    /// that fails leaves the note as it is.
    fn note_image(&mut self, tag: u64, jpeg: &[u8]) {
        let Some(index) = self.notes_sent.iter().position(|(sent, _)| *sent == tag) else {
            return;
        };
        let (_, id) = self.notes_sent.remove(index);
        if let Some(hub) = self.hub.as_mut() {
            let _ = hub.note_image(&self.identity, id, jpeg);
        }
    }

    fn report(&mut self, report: &BugReport) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.report(&self.identity, report),
            _ => Err(HubError::Protocol(
                "not connected to the hub (is identity on, cl_identity 1?)".to_owned(),
            )),
        };
        self.update(|snapshot| {
            let serial = snapshot.report.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.report = Some(outcome_of(
                serial,
                outcome.map(|id| format!("report #{id}")),
            ));
        });
    }

    /// Send a player report: only once registered, and only with a key the hub's
    /// operator verified (the hub refuses the others too).
    fn player_report(&mut self, report: &PlayerReport) {
        let verified = lock(&self.snapshot)
            .me
            .as_ref()
            .is_some_and(|me| me.verified);
        let outcome = match (self.hub.as_mut(), self.registered, verified) {
            (Some(hub), true, true) => hub.player_report(&self.identity, report),
            (Some(_), true, false) => Err(HubError::Rejected {
                status: 403,
                code: "not_verified".to_owned(),
                message: "only verified SJK players can report players".to_owned(),
            }),
            _ => Err(HubError::Protocol(
                "not connected to the hub (is identity on, cl_identity 1?)".to_owned(),
            )),
        };
        self.update(|snapshot| {
            let serial = snapshot
                .player_report
                .as_ref()
                .map_or(1, |last| last.serial + 1);
            snapshot.player_report = Some(outcome_of(
                serial,
                outcome.map(|id| format!("player report #{id}")),
            ));
        });
    }

    fn set_bio(&mut self, bio: &str) {
        // The hub's rules, checked here first so the player hears why at once.
        let bio = match crate::bio::check(bio) {
            Ok(bio) => bio,
            Err(error) => {
                self.update(|snapshot| snapshot.notice = Some(error.to_string()));
                return;
            }
        };
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.set_bio(&self.identity, &bio),
            _ => {
                self.update(|snapshot| {
                    snapshot.notice = Some("not connected to the hub".to_owned())
                });
                return;
            }
        };
        self.update(|snapshot| match outcome {
            Ok(profile) => {
                snapshot.me = Some(profile);
                snapshot.notice = Some("saved".to_owned());
            }
            Err(error) => snapshot.notice = Some(error.to_string()),
        });
    }

    /// Send a staff request: only once registered, and only for a key whose profile
    /// says staff (the hub refuses the others too).
    fn staff(&mut self, request: &StaffRequest) {
        let me = lock(&self.snapshot).me.clone();
        let outcome = match (self.hub.as_mut(), self.registered, &me) {
            (Some(hub), true, Some(me)) if me.staff => hub.staff(&self.identity, request),
            (Some(_), true, _) => Err(HubError::Rejected {
                status: 403,
                code: "not_staff".to_owned(),
                message: "only SJK staff can do this".to_owned(),
            }),
            _ => Err(HubError::Protocol(
                "not connected to the hub (is identity on, cl_identity 1?)".to_owned(),
            )),
        };
        let mut staff = self
            .staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        staff.serial += 1;
        staff.busy = false;
        match outcome {
            Ok(profiles) => {
                staff.failed = false;
                staff.message = crate::staff::done(request, profiles.len());
                if matches!(request, StaffRequest::Search(_)) {
                    staff.players = profiles;
                } else if let Some(profile) = profiles.first() {
                    staff.replace(profile);
                    // A change to the player's own key shows on their pages at once.
                    if Some(profile.key_id.as_str()) == me.as_ref().map(|me| me.key_id.as_str()) {
                        lock(&self.snapshot).me = Some(profile.clone());
                    }
                }
            }
            Err(HubError::Rejected { message, .. }) => {
                staff.failed = true;
                staff.message = message;
            }
            Err(error) => {
                staff.failed = true;
                staff.message = error.to_string();
            }
        }
    }

    /// Do whatever is due at `now`; return how long to wait for the next thing.
    fn tick(&mut self, now: Instant) -> Duration {
        let wait = self.tick_due(now);
        self.publish_feed();
        wait
    }

    fn tick_due(&mut self, now: Instant) -> Duration {
        if !self.settings.enabled {
            self.update(|snapshot| snapshot.status = Status::Disabled);
            return IDLE_MAX;
        }
        if self.hub.is_none() {
            if self.settings.hub_url.trim().is_empty() {
                self.update(|snapshot| snapshot.status = Status::NoHub);
            }
            return IDLE_MAX;
        }
        if !self.registered {
            if now < self.due_register {
                return self.due_register - now;
            }
            self.update(|snapshot| {
                if !matches!(snapshot.status, Status::Failed(_)) {
                    snapshot.status = Status::Registering;
                }
            });
            let name = self.name.clone();
            let outcome = self
                .hub
                .as_mut()
                .map(|hub| hub.register(&self.identity, name.as_deref()));
            match outcome {
                Some(Ok(profile)) => {
                    self.registered = true;
                    self.name_sent = name;
                    self.backoff = RETRY_MIN;
                    self.due_profile = now + PROFILE_EVERY;
                    self.update(|snapshot| {
                        snapshot.me = Some(profile);
                        snapshot.status = Status::Online;
                    });
                }
                Some(Err(error)) => {
                    self.due_register = self.failed(&error, now);
                    return self.due_register - now;
                }
                None => {}
            }
        }
        self.run_due(now);
        let mut wait = IDLE_MAX;
        if self.location.is_some() {
            wait = wait
                .min(self.due_claim.saturating_duration_since(now))
                .min(self.due_poll.saturating_duration_since(now));
        }
        if self.name != self.name_sent {
            wait = wait.min(self.due_name.saturating_duration_since(now));
        }
        wait = wait.min(self.due_profile.saturating_duration_since(now));
        if self.counts_waiting() {
            wait = wait.min(self.due_counts.saturating_duration_since(now));
        }
        if !self.lookups.is_empty() {
            wait = Duration::ZERO;
        }
        if self.look_waiting() {
            wait = wait.min(self.due_look.saturating_duration_since(now));
        }
        wait
    }

    /// The look to send: the one the viewer gave, without a blade skin the hub said
    /// the key does not hold while the profile's unlocks are the same.
    fn look_to_send(&mut self) -> Look {
        if !self.look_refused.is_empty() {
            let same = lock(&self.snapshot)
                .me
                .as_ref()
                .is_some_and(|me| me.unlocks == self.look_refused_for);
            if !same {
                self.look_refused.clear();
            }
        }
        let mut look = self.look.clone();
        if self.look_refused.contains(&look.saber) {
            look.saber.clear();
        }
        look
    }

    /// Whether the hub's claim should get another look than the one it holds.
    fn look_waiting(&mut self) -> bool {
        self.look_claim.is_some() && Held::Look(self.look_to_send()) != self.look_held
    }

    /// Send the look for the accepted claim if it changed and one may go.
    fn send_look(&mut self, now: Instant) {
        if now < self.due_look || !self.look_waiting() {
            return;
        }
        let look = self.look_to_send();
        let (Some((server, _, _)), Some(hub)) = (self.look_claim.clone(), self.hub.as_mut()) else {
            return;
        };
        let outcome = hub.look(&self.identity, &server, &look);
        self.due_look = now + LOOK_EVERY;
        match &outcome {
            Ok(id) => {
                self.look_held = Held::Look(look.clone());
                self.look_id = Some(*id);
            }
            // The skin is not the key's (or not the hub's): the rest of the look still
            // goes, without it, until the profile changes. Not the key's while its
            // profile said so: the profile is out of date.
            Err(HubError::Rejected { code, .. })
                if (code == "not_unlocked" || code == "bad_look") && !look.saber.is_empty() =>
            {
                if code == "not_unlocked" {
                    self.profile_soon(now);
                }
                self.look_refused_for = lock(&self.snapshot)
                    .me
                    .as_ref()
                    .map(|me| me.unlocks.clone())
                    .unwrap_or_default();
                self.look_refused.push(look.saber.clone());
            }
            // The claim lapsed at the hub: the next accepted claim sends the look again.
            Err(HubError::Rejected { code, .. }) if code == "not_on_server" => {
                self.look_claim = None;
                self.look_held = Held::none();
            }
            // Too many: the key's look quota, or the address's allowance (429
            // `rate_limited`) shared with the chat and emotes. A signature refused
            // (401, the clock still off after the one retry) is not final either.
            Err(HubError::Rejected {
                status: 401 | 429, ..
            }) => {
                self.due_look = now + LOOK_AGAIN;
            }
            // Any other refusal (an older hub without looks) is not repeated until the
            // look or the claim changes.
            Err(HubError::Rejected { status, .. }) if (400..500).contains(status) => {
                self.look_held = Held::Look(look.clone());
            }
            Err(_) => self.due_look = now + LOOK_AGAIN,
        }
        self.update(|snapshot| {
            let serial = snapshot
                .look_outcome
                .as_ref()
                .map_or(1, |last| last.serial + 1);
            snapshot.look_outcome = Some(outcome_of(serial, outcome.map(|_| String::new())));
        });
    }

    /// Whether the hub should hear the achievement counts: they changed since they
    /// were last sent, or one is above what the hub holds (it was held back by its
    /// hourly allowance) and the last sending is an hour old.
    fn counts_waiting(&self) -> bool {
        if self.counts.is_empty() {
            return false;
        }
        if self.counts_sent.as_ref() != Some(&self.counts) {
            return true;
        }
        let held = lock(&self.snapshot).me.as_ref().map(|me| {
            self.counts.iter().any(|(id, &count)| {
                match me.achievements.iter().find(|held| &held.id == id) {
                    Some(held) => count > held.progress && held.unlocked == 0,
                    None => count > 0,
                }
            })
        });
        held.unwrap_or(false)
    }

    /// Send the achievement counts if they are waiting and due.
    fn send_counts(&mut self, now: Instant) -> Option<HubError> {
        if now < self.due_counts || !self.counts_waiting() {
            return None;
        }
        let hub = self.hub.as_mut()?;
        match hub.set_achievements(&self.identity, &self.counts) {
            Ok(achievements) => {
                set_achievements(&mut lock(&self.snapshot), achievements);
                self.counts_sent = Some(self.counts.clone());
                self.due_counts = now + ACHIEVEMENTS_EVERY;
                // Counts the hub held back wait an hour unless they change.
                if self.counts_waiting() {
                    self.due_counts = now + ACHIEVEMENTS_AGAIN;
                }
                None
            }
            Err(HubError::Rejected { status, .. }) if (400..500).contains(&status) => {
                // A refusal (a quota, an older hub without achievements) is not
                // retried soon; it is not the hub being down either.
                self.counts_sent = Some(self.counts.clone());
                self.due_counts = now + ACHIEVEMENTS_AGAIN;
                None
            }
            Err(failure) => {
                self.due_counts = now + self.backoff;
                Some(failure)
            }
        }
    }

    /// A new name, the claim, the roster read and the pending lookups, where due.
    fn run_due(&mut self, now: Instant) {
        let counts_error = self.send_counts(now);
        let Some(hub) = self.hub.as_mut() else { return };
        let mut error = None;
        // A name worn since registering joins the key's history at the hub (a
        // registration of a known key only adds the name). A failure waits and retries.
        if self.name != self.name_sent && now >= self.due_name {
            match hub.register(&self.identity, self.name.as_deref()) {
                Ok(profile) => {
                    self.name_sent = self.name.clone();
                    self.due_profile = now + PROFILE_EVERY;
                    lock(&self.snapshot).me = Some(profile);
                }
                Err(failure) => {
                    self.due_name = now + self.backoff;
                    error = Some(failure);
                }
            }
        }
        // The own profile again from time to time: medals and the verified flag change
        // at the hub, not here.
        if now >= self.due_profile {
            match hub.profile(&self.identity.key_id()) {
                Ok(profile) => {
                    self.due_profile = now + PROFILE_EVERY;
                    lock(&self.snapshot).me = Some(profile);
                }
                Err(failure) => {
                    self.due_profile = now + self.backoff;
                    error = Some(failure);
                }
            }
        }
        if let Some(location) = self.location.clone() {
            let server = location.server.to_string();
            if now >= self.due_claim {
                match hub.claim(&self.identity, &server, location.slot, &location.name) {
                    Ok(()) => {
                        self.claimed = Some(server.clone());
                        self.due_claim = now + CLAIM_EVERY;
                        // Renewing the same claim keeps its look; another server,
                        // slot or name starts with none. With no claim accepted before
                        // (released, lapsed, failed or none yet), what the hub holds
                        // stays as it was known, or unknown.
                        let claim = (server.clone(), location.slot, location.name.clone());
                        if self.look_claim.as_ref() != Some(&claim)
                            && self.look_claim.replace(claim).is_some()
                        {
                            self.look_held = Held::none();
                        }
                    }
                    Err(failure) => {
                        self.due_claim = now + self.backoff;
                        // The claim may lapse meanwhile, and its look with it, or
                        // still be live with it: what the hub holds is not known.
                        self.look_claim = None;
                        self.look_held = Held::Unknown;
                        error = Some(failure);
                    }
                }
            }
            if now >= self.due_poll {
                match hub.presence(&server) {
                    Ok(players) => {
                        self.due_poll = now + POLL_EVERY;
                        let mut snapshot = lock(&self.snapshot);
                        if snapshot.players != players {
                            snapshot.players = players;
                            snapshot.revision += 1;
                        }
                    }
                    Err(failure) => {
                        self.due_poll = now + self.backoff;
                        error = Some(failure);
                    }
                }
            }
        }
        for key_id in std::mem::take(&mut self.lookups) {
            match hub.profile(&key_id) {
                Ok(profile) => {
                    lock(&self.snapshot)
                        .profiles
                        .insert(profile.key_id.clone(), profile);
                }
                Err(failure) => {
                    lock(&self.snapshot).notice = Some(failure.to_string());
                }
            }
        }
        // A look is cosmetic: what became of it is in its outcome, not the status.
        self.send_look(now);
        match error.or(counts_error) {
            Some(failure) => {
                self.backoff = (self.backoff * 2).min(RETRY_MAX);
                self.update(|snapshot| snapshot.status = Status::Failed(failure.to_string()));
            }
            None => {
                self.backoff = RETRY_MIN;
                self.update(|snapshot| {
                    if matches!(snapshot.status, Status::Failed(_)) {
                        snapshot.status = Status::Online;
                    }
                });
            }
        }
    }
}

/// Put the hub's answer about the player's achievements into their profile.
fn set_achievements(snapshot: &mut Snapshot, achievements: Vec<Achievement>) {
    if let Some(me) = snapshot.me.as_mut() {
        me.achievements = achievements;
    }
}

/// The running service: commands go in, a [`Snapshot`] comes out.
pub struct Service {
    commands: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    staff: Arc<Mutex<StaffState>>,
    chat: Arc<Mutex<ChatState>>,
    emotes: Arc<Mutex<VecDeque<Emote>>>,
    looks: Arc<Mutex<VecDeque<QueuedLook>>>,
    /// What the feed is told to read, for the looks' generation.
    feed: Arc<Mutex<FeedShared>>,
    /// The player's own key id, to recognise its looks in the feed.
    key_id: String,
    /// Set to end the feed thread, which ends after its poll at the latest.
    feed_stop: Arc<std::sync::atomic::AtomicBool>,
    finished: Mutex<Receiver<()>>,
    /// The last tag given to a note ([`Service::note`]).
    note_tags: std::sync::atomic::AtomicU64,
}

impl Service {
    /// Start the worker thread for `identity`. `make_hub` builds a hub client for
    /// the configured address. Without a feed, the SJK chat sends but never reads.
    pub fn start(identity: Identity, make_hub: HubFactory) -> Self {
        Self::start_with_feed(identity, make_hub, None)
    }

    /// [`Service::start`] with the SJK chat's feed on a thread of its own, reading the
    /// hub through clients `make_feed_hub` builds (with a timeout longer than the
    /// hub's wait, [`crate::feed::TIMEOUT`]).
    pub fn start_with_feed(
        identity: Identity,
        make_hub: HubFactory,
        make_feed_hub: Option<HubFactory>,
    ) -> Self {
        let key_id = identity.key_id();
        let snapshot = Arc::new(Mutex::new(Snapshot::new(key_id.clone())));
        let staff = Arc::new(Mutex::new(StaffState::default()));
        let (commands, inbox) = channel();
        let (done, finished) = channel();
        let feed_identity = identity.clone();
        let mut worker = Worker::new(
            identity,
            make_hub,
            Arc::clone(&snapshot),
            Arc::clone(&staff),
            Instant::now(),
        );
        let chat = Arc::clone(&worker.chat);
        let feed = Arc::clone(&worker.feed);
        let emotes = Arc::new(Mutex::new(VecDeque::new()));
        let looks = Arc::new(Mutex::new(VecDeque::new()));
        let feed_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        if let Some(make_feed_hub) = make_feed_hub {
            let feed = FeedWorker::new(
                feed_identity,
                make_feed_hub,
                Arc::clone(&worker.feed),
                Arc::clone(&chat),
                Arc::clone(&emotes),
                Arc::clone(&looks),
                Instant::now(),
            );
            let stop = Arc::clone(&feed_stop);
            let spawned = std::thread::Builder::new()
                .name("sjk-hub-feed".to_owned())
                .spawn(move || crate::feed::run(feed, &stop));
            if let Err(error) = spawned {
                lock(&snapshot).status =
                    Status::Failed(format!("cannot start the chat thread: {error}"));
            }
        }
        let spawned = std::thread::Builder::new()
            .name("sjk-identity".to_owned())
            .spawn(move || {
                loop {
                    let wait = worker.tick(Instant::now());
                    match inbox.recv_timeout(wait) {
                        Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => {
                            worker.handle(Command::Stop, Instant::now());
                            break;
                        }
                        Ok(command) => worker.handle(command, Instant::now()),
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                }
                let _ = done.send(());
            });
        if let Err(error) = spawned {
            lock(&snapshot).status =
                Status::Failed(format!("cannot start the identity thread: {error}"));
        }
        Self {
            commands,
            snapshot,
            staff,
            chat,
            emotes,
            looks,
            feed,
            key_id,
            feed_stop,
            finished: Mutex::new(finished),
            note_tags: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Apply the player's settings; the service starts or stops talking to the hub.
    pub fn configure(&self, settings: Settings) {
        let _ = self.commands.send(Command::Configure(settings));
    }

    /// The player is on a game server (call again if `slot` or `name` changes).
    pub fn enter(&self, location: Location) {
        let _ = self.commands.send(Command::Enter(location));
    }

    /// The player left their game server.
    pub fn leave(&self) {
        let _ = self.commands.send(Command::Leave);
    }

    /// The in-game name the player wears: sent with the registration and again
    /// whenever it changes, for the key's name history at the hub.
    pub fn set_name(&self, name: String) {
        let _ = self.commands.send(Command::Name(name));
    }

    /// Change the player's bio at the hub.
    pub fn set_bio(&self, bio: String) {
        let _ = self.commands.send(Command::SetBio(bio));
    }

    /// The counts the client keeps for the achievements it counts, by id: sent to the
    /// hub once registered, at most once a minute, and again when the hub held part
    /// back. The hub's answer arrives in the profile's `achievements`.
    pub fn set_achievement_counts(&self, counts: BTreeMap<String, u64>) {
        let _ = self.commands.send(Command::Achievements(counts));
    }

    /// Send a staff request; its answer arrives in [`Service::staff_state`].
    pub fn staff(&self, request: StaffRequest) {
        self.staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .busy = true;
        let _ = self.commands.send(Command::Staff(request));
    }

    /// What staff requests brought back.
    pub fn staff_state(&self) -> StaffState {
        self.staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Send a bug report; its outcome arrives in [`Snapshot::report`].
    pub fn report(&self, report: BugReport) {
        let _ = self.commands.send(Command::Report(report));
    }

    /// Send a report about another player; its outcome arrives in
    /// [`Snapshot::player_report`]. Only a verified key may.
    pub fn player_report(&self, report: PlayerReport) {
        let _ = self.commands.send(Command::PlayerReport(report));
    }

    /// Send a world note; its outcome arrives in [`Snapshot::note`]. The answer tags
    /// the note for [`Service::note_image`].
    pub fn note(&self, note: WorldNote) -> u64 {
        let tag = self
            .note_tags
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let _ = self.commands.send(Command::Note(tag, note));
        tag
    }

    /// The picture of the note tagged `tag`, a JPEG; sent once the hub took the note.
    pub fn note_image(&self, tag: u64, jpeg: Vec<u8>) {
        let _ = self.commands.send(Command::NoteImage(tag, jpeg));
    }

    /// Fetch a player's profile (their bio) into [`Snapshot::profiles`].
    pub fn look_up(&self, key_id: String) {
        let _ = self.commands.send(Command::LookUp(key_id));
    }

    /// A copy of the current state.
    pub fn snapshot(&self) -> Snapshot {
        lock(&self.snapshot).clone()
    }

    /// Read the current state without copying it; keep `read` short, the worker
    /// waits for the lock.
    pub fn with_snapshot<R>(&self, read: impl FnOnce(&Snapshot) -> R) -> R {
        read(&lock(&self.snapshot))
    }

    /// Turn the SJK chat on or off. Off, the feed still reads the hub while the
    /// player is on a game server (for its looks and emotes) but keeps no message; in
    /// the menus it reads only with the chat on.
    pub fn set_chat(&self, on: bool) {
        let _ = self.commands.send(Command::SetChat(on));
    }

    /// Send an SJK chat message; what became of it arrives in [`ChatState::outcome`].
    pub fn chat(&self, text: String) {
        let _ = self.commands.send(Command::Chat(text));
    }

    /// Play an emote for the player's slot on their server; what became of it arrives in
    /// [`ChatState::outcome`].
    pub fn emote(&self, emote: String) {
        let _ = self.commands.send(Command::Emote(emote));
    }

    /// Read the chat without copying it; keep `read` short.
    pub fn with_chat<R>(&self, read: impl FnOnce(&ChatState) -> R) -> R {
        read(&lock_chat(&self.chat))
    }

    /// The emotes received since the last call, oldest first.
    pub fn take_emotes(&self) -> Vec<Emote> {
        crate::feed::lock(&self.emotes).drain(..).collect()
    }

    /// The look the player wears (`PROTOCOL.md`, "Looks"). The worker keeps the
    /// latest and sends it once its claim on a server is accepted, again when it
    /// changes or the claim does (another server, slot or name), at most once a
    /// second; a blade skin the hub says the key does not hold is left out until the
    /// profile changes. What became of it arrives in [`Snapshot::look_outcome`].
    pub fn set_look(&self, look: Look) {
        let _ = self.commands.send(Command::Look(look));
    }

    /// The looks the feed received since the last call, oldest first: those it read
    /// for `server`, the game server the player is on, under the current reading. A
    /// poll still under way when the server changed or the identity went off brings
    /// none, and [`ReceivedLooks::generation`] tells when every look had before is out
    /// of date. The newest look of the player's own key goes to the worker, which reads
    /// the own profile again soon when the hub took a skin back.
    pub fn take_looks(&self, server: Option<SocketAddr>) -> ReceivedLooks {
        let received = crate::feed::take_looks(&self.feed, &self.looks, server);
        if let Some(own) = received
            .events
            .iter()
            .rev()
            .find(|event| event.key_id == self.key_id)
        {
            let _ = self
                .commands
                .send(Command::OwnLook(own.id, own.saber.clone()));
        }
        received
    }

    /// Withdraw the claim and stop, waiting at most `timeout` for it. The feed thread
    /// is told to stop and ends after its poll, without being waited for.
    pub fn shutdown(&self, timeout: Duration) {
        self.feed_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = self.commands.send(Command::Stop);
        let finished = self
            .finished
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = finished.recv_timeout(timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A hub that records what it is asked.
    #[derive(Clone, Default)]
    struct Fake {
        log: Arc<Mutex<Vec<String>>>,
        fail: Arc<AtomicBool>,
        roster: Arc<Mutex<Vec<Presence>>>,
        /// The most of any count the fake hub takes, as its hourly allowance would.
        cap: Arc<Mutex<Option<u64>>>,
        /// The achievements the fake hub holds, as its profiles list them.
        held: Arc<Mutex<Vec<Achievement>>>,
        /// The unlocks the fake hub holds for the player's key.
        owned: Arc<Mutex<Vec<String>>>,
        /// Refusals the next looks get, in order.
        look_refusals: Arc<Mutex<VecDeque<HubError>>>,
    }

    impl Fake {
        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }

        fn unlocks(&self) -> Vec<Unlock> {
            self.owned
                .lock()
                .unwrap()
                .iter()
                .map(|id| Unlock {
                    id: id.clone(),
                    granted: 1,
                    note: String::new(),
                })
                .collect()
        }

        fn looks(&self) -> Vec<String> {
            self.log()
                .into_iter()
                .filter(|line| line.starts_with("look "))
                .collect()
        }

        fn record(&self, line: String) -> Result<(), HubError> {
            self.log.lock().unwrap().push(line);
            if self.fail.load(Ordering::SeqCst) {
                Err(HubError::Network("down".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    fn profile(name: &str) -> Profile {
        Profile {
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: name.to_owned(),
            bio: String::new(),
            verified: false,
            staff: true,
            created: 0,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
            unlocks: Vec::new(),
        }
    }

    impl Hub for Fake {
        fn register(&mut self, _: &Identity, name: Option<&str>) -> Result<Profile, HubError> {
            let unlocks = self.unlocks();
            match name {
                Some(name) => self.record(format!("register {name}")).map(|()| Profile {
                    unlocks,
                    ..profile(name)
                }),
                None => self.record("register".to_owned()).map(|()| Profile {
                    unlocks,
                    ..profile("")
                }),
            }
        }
        fn set_bio(&mut self, _: &Identity, bio: &str) -> Result<Profile, HubError> {
            self.record(format!("bio {bio}")).map(|()| profile(""))
        }
        fn profile(&mut self, key_id: &str) -> Result<Profile, HubError> {
            self.record(format!("lookup {key_id}")).map(|()| Profile {
                achievements: self.held.lock().unwrap().clone(),
                unlocks: self.unlocks(),
                ..profile("Other")
            })
        }
        fn claim(
            &mut self,
            _: &Identity,
            server: &str,
            slot: u8,
            name: &str,
        ) -> Result<(), HubError> {
            self.record(format!("claim {server} {slot} {name}"))
        }
        fn release(&mut self, _: &Identity, server: &str) -> Result<(), HubError> {
            self.record(format!("release {server}"))
        }
        fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError> {
            self.record(format!("presence {server}"))
                .map(|()| self.roster.lock().unwrap().clone())
        }
        fn report(&mut self, _: &Identity, report: &BugReport) -> Result<i64, HubError> {
            self.record(format!("report {}", report.text))?;
            self.record(format!("report name {}", report.name))
                .map(|()| 7)
        }
        fn player_report(&mut self, _: &Identity, report: &PlayerReport) -> Result<i64, HubError> {
            Fake::record(
                self,
                format!(
                    "player {} {} {} by {}",
                    report.category.code(),
                    report.slot,
                    report.target_name,
                    report.name
                ),
            )
            .map(|()| 11)
        }
        fn note(&mut self, _: &Identity, note: &WorldNote) -> Result<i64, HubError> {
            Fake::record(self, format!("note {} {}", note.text, note.shader))?;
            Fake::record(self, format!("note name {}", note.name)).map(|()| 9)
        }
        fn note_image(&mut self, _: &Identity, id: i64, jpeg: &[u8]) -> Result<(), HubError> {
            Fake::record(self, format!("image {id} {} bytes", jpeg.len()))
        }
        fn chat(&mut self, _: &Identity, text: &str, name: &str) -> Result<u64, HubError> {
            Fake::record(self, format!("chat {text} as {name}")).map(|()| 12)
        }
        fn emote(&mut self, _: &Identity, server: &str, emote: &str) -> Result<u64, HubError> {
            Fake::record(self, format!("emote {emote} on {server}")).map(|()| 13)
        }
        fn look(&mut self, _: &Identity, server: &str, look: &Look) -> Result<u64, HubError> {
            Fake::record(
                self,
                format!("look {server} {:?} {}", look.saber, look.illuminate),
            )?;
            if let Some(refusal) = self.look_refusals.lock().unwrap().pop_front() {
                return Err(refusal);
            }
            if !look.saber.is_empty() && !self.owned.lock().unwrap().contains(&look.saber) {
                return Err(HubError::Rejected {
                    status: 403,
                    code: "not_unlocked".to_owned(),
                    message: "this key does not hold that unlock".to_owned(),
                });
            }
            Ok(15)
        }
        fn set_achievements(
            &mut self,
            _: &Identity,
            progress: &BTreeMap<String, u64>,
        ) -> Result<Vec<Achievement>, HubError> {
            let line: Vec<String> = progress.iter().map(|(id, n)| format!("{id}={n}")).collect();
            Fake::record(self, format!("achievements {}", line.join(",")))?;
            let cap = *self.cap.lock().unwrap();
            let held: Vec<Achievement> = progress
                .iter()
                .map(|(id, &count)| Achievement {
                    id: id.clone(),
                    progress: cap.map_or(count, |cap| count.min(cap)),
                    goal: 100,
                    unlocked: 0,
                })
                .collect();
            *self.held.lock().unwrap() = held.clone();
            Ok(held)
        }
        fn staff(
            &mut self,
            _: &Identity,
            request: &StaffRequest,
        ) -> Result<Vec<Profile>, HubError> {
            Fake::record(self, format!("staff {request:?}"))?;
            Ok(match request {
                StaffRequest::Search(_) => vec![profile("Found"), profile("Other")],
                StaffRequest::Award { key_id, medal, .. } => vec![Profile {
                    key_id: key_id.clone(),
                    medals: vec![crate::wire::Medal {
                        id: medal.clone(),
                        count: 1,
                        awarded: 1,
                        note: String::new(),
                    }],
                    ..profile("Target")
                }],
                StaffRequest::Unlock { key_id, unlock, .. } => {
                    self.owned.lock().unwrap().push(unlock.clone());
                    vec![Profile {
                        key_id: key_id.clone(),
                        unlocks: self.unlocks(),
                        ..profile("Target")
                    }]
                }
                StaffRequest::Relock { key_id, unlock } => {
                    self.owned.lock().unwrap().retain(|id| id != unlock);
                    vec![Profile {
                        key_id: key_id.clone(),
                        unlocks: self.unlocks(),
                        ..profile("Target")
                    }]
                }
                StaffRequest::Unaward { key_id, .. }
                | StaffRequest::ClearAchievements { key_id, .. } => vec![Profile {
                    key_id: key_id.clone(),
                    ..profile("Target")
                }],
                StaffRequest::ChatDelete { .. } | StaffRequest::ChatMute { .. } => Vec::new(),
            })
        }
    }

    fn worker(fake: &Fake, now: Instant) -> (Worker, Arc<Mutex<Snapshot>>) {
        let identity = Identity::from_seed([1; 32]);
        let snapshot = Arc::new(Mutex::new(Snapshot::new(identity.key_id())));
        let factory = fake.clone();
        let make: HubFactory = Box::new(move |url| {
            if url.starts_with("https://") {
                Ok(Box::new(factory.clone()))
            } else {
                Err(HubError::Network("bad address".to_owned()))
            }
        });
        (
            Worker::new(
                identity,
                make,
                Arc::clone(&snapshot),
                Arc::new(Mutex::new(StaffState::default())),
                now,
            ),
            snapshot,
        )
    }

    fn on(url: &str) -> Settings {
        Settings {
            enabled: true,
            hub_url: url.to_owned(),
        }
    }

    fn here(slot: u8, name: &str) -> Location {
        Location {
            server: "1.2.3.4:29070".parse().unwrap(),
            slot,
            name: name.to_owned(),
        }
    }

    fn counts(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
        pairs.iter().map(|(id, n)| ((*id).to_owned(), *n)).collect()
    }

    fn sent_counts(fake: &Fake) -> Vec<String> {
        fake.log()
            .into_iter()
            .filter(|line| line.starts_with("achievements"))
            .collect()
    }

    fn chat_outcome(worker: &Worker) -> ReportOutcome {
        lock_chat(&worker.chat).outcome.clone().unwrap()
    }

    #[test]
    fn chat_needs_the_rules_and_a_registration() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Chat("hello".into()), t0);
        let first = chat_outcome(&worker);
        assert!(!first.sent && first.message.contains("not connected"));
        worker.handle(Command::Name("^2Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        // The rules are checked here first; nothing goes.
        worker.handle(Command::Chat("hi \u{1F600}".into()), t0);
        let refused = chat_outcome(&worker);
        assert_eq!(
            (refused.serial, refused.sent, refused.message.as_str()),
            (2, false, crate::chat::ChatError::Characters.message())
        );
        worker.handle(Command::Chat("  gg   wp ".into()), t0);
        assert_eq!(fake.log().last().unwrap(), "chat gg wp as ^2Sol");
        let sent = chat_outcome(&worker);
        assert!(sent.sent);
        assert_eq!(sent.serial, 3);
    }

    #[test]
    fn emotes_need_a_server_and_a_well_formed_id() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Emote("wave".into()), t0);
        let nowhere = chat_outcome(&worker);
        assert!(!nowhere.sent && nowhere.message.contains("server"));
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Emote("Wave!".into()), t0);
        assert!(!chat_outcome(&worker).sent);
        assert!(!fake.log().iter().any(|line| line.starts_with("emote")));
        worker.handle(Command::Emote("wave".into()), t0);
        assert_eq!(fake.log().last().unwrap(), "emote wave on 1.2.3.4:29070");
        assert!(chat_outcome(&worker).sent);
    }

    #[test]
    fn the_feed_runs_when_registered_with_chat_on_or_on_a_server() {
        let fake = Fake::default();
        fake.fail.store(true, Ordering::SeqCst);
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        let shared = Arc::clone(&worker.feed);
        let url = || lock_feed(&shared).url.clone();
        worker.handle(Command::SetChat(true), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(url(), None, "not registered yet");
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(30));
        assert_eq!(url().as_deref(), Some("https://hub"));
        assert_eq!(lock_feed(&shared).server, None);
        assert!(lock_feed(&shared).chat);
        // In the menus the chat off stops the reading.
        worker.handle(Command::SetChat(false), t0);
        assert_eq!(url(), None);
        // On a server it reads anyway, for the looks and emotes, the chat hidden.
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        assert_eq!(url().as_deref(), Some("https://hub"));
        assert_eq!(lock_feed(&shared).server.as_deref(), Some("1.2.3.4:29070"));
        assert!(!lock_feed(&shared).chat);
        worker.handle(Command::SetChat(true), t0);
        assert!(url().is_some() && lock_feed(&shared).chat);
        worker.handle(Command::SetChat(false), t0);
        worker.handle(Command::Leave, t0);
        assert_eq!(url(), None, "back in the menus with the chat off");
        worker.handle(Command::SetChat(true), t0);
        assert!(url().is_some());
        worker.handle(Command::Configure(Settings::default()), t0);
        assert_eq!(url(), None, "identity off");
    }

    #[test]
    fn another_server_or_the_identity_off_is_a_new_reading_for_the_looks() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        let shared = Arc::clone(&worker.feed);
        let generation = || lock_feed(&shared).generation;
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        let first = generation();
        // The chat, a slot or a name change nothing of what is read.
        worker.handle(Command::SetChat(true), t0);
        worker.handle(Command::Enter(here(4, "Sol")), t0);
        assert_eq!(generation(), first);
        let elsewhere = Location {
            server: "5.6.7.8:29070".parse().unwrap(),
            ..here(4, "Sol")
        };
        worker.handle(Command::Enter(elsewhere), t0);
        let second = generation();
        assert!(second > first);
        worker.handle(Command::Configure(Settings::default()), t0);
        assert!(generation() > second, "identity off");
    }

    fn lit(saber: &str) -> Look {
        Look {
            saber: saber.to_owned(),
            illuminate: true,
        }
    }

    #[test]
    fn the_look_goes_once_the_claim_is_accepted_and_at_most_once_a_second() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        // Without a claim there is nowhere to wear it.
        worker.handle(Command::Look(lit("")), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert!(fake.looks().is_empty());
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(
            fake.log()[1..],
            [
                "claim 1.2.3.4:29070 3 Sol",
                "presence 1.2.3.4:29070",
                "look 1.2.3.4:29070 \"\" true",
            ]
        );
        assert!(lock(&snapshot).look_outcome.as_ref().unwrap().sent);
        // Changes within the second wait, and only the latest goes.
        let ms = |ms| t0 + Duration::from_millis(ms);
        worker.handle(Command::Look(lit("saber_sun")), ms(200));
        assert_eq!(worker.tick(ms(200)), Duration::from_millis(800));
        worker.handle(Command::Look(Look::default()), ms(500));
        worker.tick(ms(500));
        worker.handle(Command::Look(lit("saber_sun")), ms(700));
        worker.tick(ms(700));
        assert_eq!(fake.looks().len(), 1);
        worker.tick(ms(1_000));
        assert_eq!(
            fake.looks(),
            [
                "look 1.2.3.4:29070 \"\" true",
                "look 1.2.3.4:29070 \"saber_sun\" true"
            ]
        );
        // The same look is not sent again, nor with the claim renewed.
        worker.handle(Command::Look(lit("saber_sun")), ms(2_000));
        worker.tick(ms(2_000));
        worker.tick(t0 + Duration::from_secs(46));
        assert_eq!(fake.looks().len(), 2);
    }

    #[test]
    fn a_new_claim_gets_the_look_again_and_no_look_sends_nothing() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        // A client that stopped without releasing may have left a live claim with a
        // look: the first accepted claim gets the look, even none.
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"\" false"]);
        // Another slot: a new claim, which starts with no look, so none goes.
        let t1 = t0 + Duration::from_secs(5);
        worker.handle(Command::Enter(here(4, "Sol")), t1);
        worker.tick(t1);
        assert_eq!(fake.log().last().unwrap(), "presence 1.2.3.4:29070");
        assert_eq!(fake.looks().len(), 1);
        worker.handle(Command::Look(lit("")), t1);
        worker.tick(t1);
        assert_eq!(fake.looks().len(), 2);
        // Another slot again: the new claim gets the look again.
        let t2 = t1 + Duration::from_secs(5);
        worker.handle(Command::Enter(here(5, "Sol")), t2);
        worker.tick(t2);
        assert_eq!(fake.log().last().unwrap(), "look 1.2.3.4:29070 \"\" true");
        assert_eq!(fake.looks().len(), 3);
        // Another server: the old claim is withdrawn and the new one gets it.
        let elsewhere = Location {
            server: "5.6.7.8:29070".parse().unwrap(),
            ..here(5, "Sol")
        };
        let t3 = t2 + Duration::from_secs(5);
        worker.handle(Command::Enter(elsewhere), t3);
        worker.tick(t3);
        assert_eq!(fake.log().last().unwrap(), "look 5.6.7.8:29070 \"\" true");
        // A claim that failed may have lapsed: the next accepted one sends it again.
        fake.fail.store(true, Ordering::SeqCst);
        let t4 = t3 + Duration::from_secs(46);
        worker.tick(t4);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t4 + Duration::from_secs(30));
        assert_eq!(fake.looks().len(), 5);
        // Leaving needs nothing: the release drops it.
        worker.handle(Command::Leave, t4 + Duration::from_secs(31));
        assert_eq!(fake.log().last().unwrap(), "release 5.6.7.8:29070");
        // Back on the server: a new claim after a release starts with no look.
        let t5 = t4 + Duration::from_secs(40);
        worker.handle(Command::Look(Look::default()), t5);
        worker.handle(Command::Enter(here(5, "Sol")), t5);
        worker.tick(t5);
        assert_eq!(fake.log().last().unwrap(), "presence 1.2.3.4:29070");
        assert_eq!(fake.looks().len(), 5);
    }

    #[test]
    fn after_a_failed_claim_the_look_goes_whatever_it_is() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.handle(Command::Look(lit("saber_sun")), t0);
        worker.tick(t0);
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"saber_sun\" true"]);
        // The renewal fails (it timed out, say) while the hub still holds the claim and
        // its look; meanwhile the look goes back to none.
        fake.fail.store(true, Ordering::SeqCst);
        let t1 = t0 + Duration::from_secs(46);
        worker.tick(t1);
        worker.handle(Command::Look(Look::default()), t1);
        fake.fail.store(false, Ordering::SeqCst);
        // The same claim is accepted again: the hub may still hold the Sun, so none goes.
        let t2 = t1 + Duration::from_secs(30);
        worker.tick(t2);
        assert_eq!(
            fake.looks().last().unwrap(),
            "look 1.2.3.4:29070 \"\" false"
        );
        assert_eq!(fake.looks().len(), 2);
        // Known again: not repeated.
        worker.tick(t2 + Duration::from_secs(46));
        assert_eq!(fake.looks().len(), 2);
    }

    fn refused(status: u16, code: &str) -> HubError {
        HubError::Rejected {
            status,
            code: code.to_owned(),
            message: String::new(),
        }
    }

    #[test]
    fn a_look_refused_for_too_many_or_the_signature_goes_again_later() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let sent = fake.looks().len();
        for (status, code) in [(429, "rate_limited"), (429, "look_quota"), (401, "clock")] {
            fake.look_refusals
                .lock()
                .unwrap()
                .push_back(refused(status, code));
        }
        let mut at = t0 + LOOK_EVERY;
        worker.handle(Command::Look(lit("")), at);
        for (n, code) in ["rate_limited", "look_quota", "clock"].iter().enumerate() {
            worker.tick(at);
            assert_eq!(fake.looks().len(), sent + n + 1, "{code}");
            assert!(!lock(&snapshot).look_outcome.clone().unwrap().sent);
            // Not again within the wait, then again.
            worker.tick(at + LOOK_AGAIN - Duration::from_millis(1));
            assert_eq!(fake.looks().len(), sent + n + 1, "{code}");
            at += LOOK_AGAIN;
        }
        worker.tick(at);
        assert_eq!(fake.looks().len(), sent + 4);
        assert!(lock(&snapshot).look_outcome.clone().unwrap().sent);
        // Another refusal (an older hub) is not repeated.
        fake.look_refusals
            .lock()
            .unwrap()
            .push_back(refused(404, "not_found"));
        worker.handle(Command::Look(Look::default()), at);
        worker.tick(at + LOOK_AGAIN);
        worker.tick(at + LOOK_AGAIN * 3);
        assert_eq!(fake.looks().len(), sent + 5);
    }

    fn profile_reads(fake: &Fake) -> usize {
        fake.log()
            .iter()
            .filter(|line| line.starts_with("lookup"))
            .count()
    }

    #[test]
    fn a_refused_skin_reads_the_own_profile_soon_at_most_twice_a_minute() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        // The profile listed the Sun, but staff took it back: the hub refuses it.
        worker.handle(Command::Look(lit("saber_sun")), t0);
        assert_eq!(
            worker.tick(t0),
            Duration::ZERO,
            "the profile is due at once"
        );
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"saber_sun\" true"]);
        assert_eq!(profile_reads(&fake), 0);
        let ms = |ms| t0 + Duration::from_millis(ms);
        worker.tick(ms(10));
        assert_eq!(profile_reads(&fake), 1);
        // Refused again: not before 30 seconds from the last early reading.
        worker.handle(Command::Look(lit("saber_moon")), ms(2_000));
        worker.tick(ms(2_000));
        assert_eq!(
            fake.looks().last().unwrap(),
            "look 1.2.3.4:29070 \"saber_moon\" true"
        );
        worker.tick(ms(29_000));
        assert_eq!(profile_reads(&fake), 1);
        worker.tick(ms(30_010));
        assert_eq!(profile_reads(&fake), 2);
    }

    #[test]
    fn the_hub_taking_the_own_skin_back_reads_the_own_profile_soon() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.handle(Command::Look(lit("saber_sun")), t0);
        worker.tick(t0);
        // The fake hub numbers it 15. The feed brings it back: nothing to do.
        let t1 = t0 + Duration::from_secs(1);
        worker.handle(Command::OwnLook(15, "saber_sun".into()), t1);
        // An older one, or a newer one with the same skin, is not news either.
        worker.handle(Command::OwnLook(12, String::new()), t1);
        worker.handle(Command::OwnLook(16, "saber_sun".into()), t1);
        worker.tick(t1);
        assert_eq!(profile_reads(&fake), 0);
        // Staff relock it: the hub's event takes the skin off.
        worker.handle(Command::OwnLook(20, String::new()), t1);
        worker.tick(t1);
        assert_eq!(profile_reads(&fake), 1);
        // Another within the 30 seconds waits for them.
        worker.handle(Command::OwnLook(21, String::new()), t1);
        worker.tick(t1 + Duration::from_secs(29));
        assert_eq!(profile_reads(&fake), 1);
        worker.tick(t1 + Duration::from_secs(30));
        assert_eq!(profile_reads(&fake), 2);
    }

    #[test]
    fn a_blade_skin_the_key_lacks_is_left_out_until_the_profile_changes() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.handle(Command::Look(lit("saber_sun")), t0);
        worker.tick(t0);
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"saber_sun\" true"]);
        let outcome = lock(&snapshot).look_outcome.clone().unwrap();
        assert!(!outcome.sent);
        // A second later the Illuminate still goes, without the skin, and then nothing.
        let t1 = t0 + Duration::from_secs(1);
        worker.tick(t1);
        worker.tick(t1 + Duration::from_secs(5));
        assert_eq!(
            fake.looks()[1..],
            ["look 1.2.3.4:29070 \"\" true".to_owned()]
        );
        // Another skin is tried and refused; back to the first, which stays out: the
        // hub already holds the look without a skin, so nothing goes.
        worker.handle(
            Command::Look(lit("saber_moon")),
            t1 + Duration::from_secs(5),
        );
        worker.tick(t1 + Duration::from_secs(5));
        worker.handle(Command::Look(lit("saber_sun")), t1 + Duration::from_secs(7));
        worker.tick(t1 + Duration::from_secs(7));
        worker.tick(t1 + Duration::from_secs(8));
        assert_eq!(
            fake.looks()[2..],
            ["look 1.2.3.4:29070 \"saber_moon\" true".to_owned()]
        );
        // Staff unlock it on the player's own key: the profile changes, it goes.
        let me = lock(&snapshot).me.clone().unwrap().key_id;
        worker.handle(
            Command::Staff(StaffRequest::Unlock {
                key_id: me,
                unlock: "saber_sun".into(),
                note: String::new(),
            }),
            t1 + Duration::from_secs(9),
        );
        worker.tick(t1 + Duration::from_secs(9));
        assert_eq!(
            fake.looks().last().unwrap(),
            "look 1.2.3.4:29070 \"saber_sun\" true"
        );
        assert!(lock(&snapshot).look_outcome.clone().unwrap().sent);
    }

    #[test]
    fn staff_unlock_and_relock_change_the_profile_they_answer() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let staff = Arc::clone(&worker.staff);
        let me = lock(&snapshot).me.clone().unwrap().key_id;
        worker.handle(
            Command::Staff(StaffRequest::Unlock {
                key_id: me.clone(),
                unlock: "saber_sun".into(),
                note: "Thanks".into(),
            }),
            t0,
        );
        assert_eq!(staff.lock().unwrap().message, "Unlocked saber_sun");
        let unlocks = |snapshot: &Mutex<Snapshot>| lock(snapshot).me.clone().unwrap().unlocks;
        assert_eq!(unlocks(&snapshot)[0].id, "saber_sun");
        worker.handle(
            Command::Staff(StaffRequest::Relock {
                key_id: me,
                unlock: "saber_sun".into(),
            }),
            t0,
        );
        assert_eq!(staff.lock().unwrap().message, "Took back saber_sun");
        assert!(unlocks(&snapshot).is_empty());
        assert_eq!(staff.lock().unwrap().players[0].unlocks, []);
    }

    #[test]
    fn achievement_counts_go_to_the_hub_at_most_once_a_minute() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Achievements(counts(&[("kills_100", 3)])), t0);
        worker.tick(t0);
        assert_eq!(sent_counts(&fake), ["achievements kills_100=3"]);
        let me = lock(&snapshot).me.clone().unwrap();
        assert_eq!(me.achievements[0].progress, 3);
        // The same counts are not sent again; new ones wait for the minute.
        worker.tick(t0 + Duration::from_secs(30));
        worker.handle(
            Command::Achievements(counts(&[("kills_100", 4)])),
            t0 + Duration::from_secs(30),
        );
        worker.tick(t0 + Duration::from_secs(40));
        assert_eq!(sent_counts(&fake).len(), 1);
        worker.tick(t0 + Duration::from_secs(61));
        assert_eq!(
            sent_counts(&fake).last().unwrap(),
            "achievements kills_100=4"
        );
        worker.tick(t0 + Duration::from_secs(5_000));
        assert_eq!(sent_counts(&fake).len(), 2, "nothing waits");
    }

    #[test]
    fn counts_the_hub_held_back_are_sent_again_an_hour_later() {
        let fake = Fake::default();
        *fake.cap.lock().unwrap() = Some(2);
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Achievements(counts(&[("kills_100", 5)])), t0);
        worker.tick(t0);
        worker.tick(t0 + Duration::from_secs(120));
        assert_eq!(sent_counts(&fake).len(), 1, "held back: waits the hour");
        *fake.cap.lock().unwrap() = None;
        worker.tick(t0 + Duration::from_secs(3_601));
        assert_eq!(sent_counts(&fake).len(), 2);
        worker.tick(t0 + Duration::from_secs(8_000));
        assert_eq!(sent_counts(&fake).len(), 2, "all taken: nothing waits");
    }

    #[test]
    fn a_bio_breaking_the_rules_is_refused_before_the_hub_hears_it() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetBio("hi \u{1F600}".to_owned()), t0);
        assert!(!fake.log().iter().any(|line| line.starts_with("bio")));
        assert_eq!(
            lock(&snapshot).notice.as_deref(),
            Some(crate::bio::BioError::Character.message())
        );
        worker.handle(Command::SetBio("  hello   there ".to_owned()), t0);
        assert_eq!(fake.log().last().unwrap(), "bio hello there");
    }

    #[test]
    fn staff_requests_need_a_staff_profile_and_update_what_they_change() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let staff = Arc::clone(&worker.staff);
        let state = || staff.lock().unwrap().clone();
        worker.handle(Command::Staff(StaffRequest::Search("so".into())), t0);
        let found = state();
        assert_eq!(found.players.len(), 2);
        assert_eq!(found.message, "2 players found");
        assert!(!found.failed);
        // A change to another key replaces it in the list; to the player's own key,
        // their profile too.
        let me = lock(&snapshot).me.clone().unwrap().key_id;
        worker.handle(
            Command::Staff(StaffRequest::Award {
                key_id: me.clone(),
                medal: "bug_hunter".into(),
                note: String::new(),
            }),
            t0,
        );
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().medals.len(), 1);
        assert_eq!(state().message, "Gave bug_hunter");
        // A key that is not staff is refused here, before the hub hears it.
        lock(&snapshot).me.as_mut().unwrap().staff = false;
        let before = fake.log().len();
        worker.handle(Command::Staff(StaffRequest::Search(String::new())), t0);
        assert_eq!(fake.log().len(), before);
        assert!(state().failed);
        assert_eq!(state().message, "only SJK staff can do this");
    }

    #[test]
    fn nothing_is_sent_while_disabled_or_without_a_hub() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(lock(&snapshot).status, Status::Disabled);
        worker.handle(Command::Configure(on("")), t0);
        worker.tick(t0);
        assert_eq!(lock(&snapshot).status, Status::NoHub);
        worker.handle(
            Command::Configure(Settings {
                enabled: false,
                hub_url: "https://h".into(),
            }),
            t0,
        );
        worker.tick(t0);
        assert!(fake.log().is_empty());
    }

    #[test]
    fn it_registers_before_it_claims_and_repeats_the_claim() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(
            fake.log(),
            [
                "register",
                "claim 1.2.3.4:29070 3 Sol",
                "presence 1.2.3.4:29070",
                // What the hub held for the key before the start is not known.
                "look 1.2.3.4:29070 \"\" false"
            ]
        );
        assert_eq!(lock(&snapshot).status, Status::Online);
        // Nothing is due for a while, then the roster, then the claim.
        worker.tick(t0 + Duration::from_secs(10));
        assert_eq!(fake.log().len(), 4);
        worker.tick(t0 + Duration::from_secs(16));
        assert_eq!(fake.log().last().unwrap(), "presence 1.2.3.4:29070");
        worker.tick(t0 + Duration::from_secs(46));
        assert!(
            fake.log()
                .iter()
                .filter(|line| line.starts_with("claim"))
                .count()
                == 2
        );
    }

    #[test]
    fn leaving_or_switching_off_withdraws_the_claim() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Leave, t0);
        assert_eq!(fake.log().last().unwrap(), "release 1.2.3.4:29070");
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Configure(Settings::default()), t0);
        assert_eq!(fake.log().last().unwrap(), "release 1.2.3.4:29070");
        let before = fake.log().len();
        worker.tick(t0 + Duration::from_secs(500));
        assert_eq!(fake.log().len(), before);
    }

    #[test]
    fn a_new_name_or_slot_is_claimed_at_once_and_the_same_one_is_not_repeated() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let claims = |fake: &Fake| fake.log().iter().filter(|l| l.starts_with("claim")).count();
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(claims(&fake), 1);
        worker.handle(Command::Enter(here(3, "Fox")), t0);
        worker.tick(t0);
        assert_eq!(claims(&fake), 2);
        assert_eq!(
            fake.log()
                .iter()
                .filter(|l| l.starts_with("release"))
                .count(),
            0
        );
    }

    #[test]
    fn failures_back_off_and_recovery_clears_the_status() {
        let fake = Fake::default();
        fake.fail.store(true, Ordering::SeqCst);
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        let wait = worker.tick(t0);
        assert!(matches!(lock(&snapshot).status, Status::Failed(_)));
        assert_eq!(wait, RETRY_MIN);
        worker.tick(t0 + Duration::from_secs(5));
        assert_eq!(fake.log().len(), 1, "no retry before the backoff ends");
        let wait = worker.tick(t0 + Duration::from_secs(11));
        assert_eq!(wait, RETRY_MIN * 2);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(40));
        assert_eq!(lock(&snapshot).status, Status::Online);
    }

    #[test]
    fn a_bad_address_is_reported_not_used() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("http://hub.example")), t0);
        worker.tick(t0);
        assert!(matches!(lock(&snapshot).status, Status::Failed(_)));
        assert!(fake.log().is_empty());
    }

    #[test]
    fn badges_match_by_slot_and_name_only() {
        let fake = Fake::default();
        *fake.roster.lock().unwrap() = vec![Presence {
            slot: 3,
            claimed_name: "^1Sol".to_owned(),
            key_id: "0123456789abcdef".to_owned(),
            name: "Sol".to_owned(),
            verified: true,
            medals: Vec::new(),
            look: None,
        }];
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(1, "Me")), t0);
        worker.tick(t0);
        let snapshot = lock(&snapshot);
        assert!(snapshot.badge(3, "sol").unwrap().verified);
        assert!(
            snapshot.badge(3, "Fox").is_none(),
            "another name in the slot"
        );
        assert!(snapshot.badge(4, "Sol").is_none(), "another slot");
    }

    #[test]
    fn bio_changes_and_lookups_go_through_the_hub() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::SetBio("x".into()), t0);
        assert_eq!(
            lock(&snapshot).notice.as_deref(),
            Some("not connected to the hub")
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetBio("hello".into()), t0);
        assert_eq!(fake.log().last().unwrap(), "bio hello");
        assert_eq!(lock(&snapshot).notice.as_deref(), Some("saved"));
        worker.handle(Command::LookUp("0123456789abcdef".into()), t0);
        worker.tick(t0);
        assert!(lock(&snapshot).profiles.contains_key("0123456789abcdef"));
    }

    #[test]
    fn notes_go_to_the_hub_and_their_pictures_follow() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let note = WorldNote {
            text: "too shiny".to_owned(),
            shader: "textures/vjun/newfloor_vjun".to_owned(),
            ..WorldNote::default()
        };
        // Before registering, a note is refused and its picture goes nowhere.
        worker.handle(Command::Note(1, note.clone()), t0);
        worker.handle(Command::NoteImage(1, vec![0; 4]), t0);
        assert!(!lock(&snapshot).note.clone().unwrap().sent);
        assert!(fake.log().is_empty());
        worker.handle(Command::Name("^1Sol".to_owned()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Note(2, note), t0);
        let outcome = lock(&snapshot).note.clone().unwrap();
        assert!(outcome.sent);
        assert_eq!((outcome.serial, outcome.message.as_str()), (2, "note #9"));
        // A picture for an unknown tag is dropped; the note's goes to its hub id once.
        worker.handle(Command::NoteImage(5, vec![0; 4]), t0);
        worker.handle(Command::NoteImage(2, vec![0; 4]), t0);
        worker.handle(Command::NoteImage(2, vec![0; 4]), t0);
        let log = fake.log();
        assert_eq!(
            log.iter()
                .filter(|line| line.starts_with("image"))
                .collect::<Vec<_>>(),
            ["image 9 4 bytes"]
        );
        assert!(log.contains(&"note too shiny textures/vjun/newfloor_vjun".to_owned()));
        // The note carries the name the player wears.
        assert!(log.contains(&"note name ^1Sol".to_owned()), "{log:?}");
    }

    #[test]
    fn bug_reports_go_to_the_hub_once_registered() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let report = BugReport {
            text: "The door flickers".into(),
            ..BugReport::default()
        };
        // Before registering, nothing is sent and the outcome says why.
        worker.handle(Command::Report(report.clone()), t0);
        let first = lock(&snapshot).report.clone().unwrap();
        assert!(!first.sent && first.message.contains("not connected"));
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Report(report), t0);
        assert!(fake.log().contains(&"report The door flickers".to_owned()));
        let outcome = lock(&snapshot).report.clone().unwrap();
        assert_eq!((outcome.serial, outcome.sent), (first.serial + 1, true));
        assert_eq!(outcome.message, "report #7");
    }

    #[test]
    fn player_reports_go_only_from_a_verified_key() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let report = PlayerReport {
            category: crate::report::Category::Cheating,
            text: "Speed hacking all round".into(),
            slot: 5,
            target_name: "^1Troll".into(),
            ..PlayerReport::default()
        };
        worker.handle(Command::PlayerReport(report.clone()), t0);
        let first = lock(&snapshot).player_report.clone().unwrap();
        assert!(!first.sent && first.message.contains("not connected"));
        worker.handle(Command::Name("^2Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        // Registered but not verified: nothing goes, and the outcome says why.
        worker.handle(Command::PlayerReport(report.clone()), t0);
        let unverified = lock(&snapshot).player_report.clone().unwrap();
        assert!(!unverified.sent && unverified.message.contains("verified"));
        assert!(!fake.log().iter().any(|line| line.starts_with("player")));
        lock(&snapshot).me.as_mut().unwrap().verified = true;
        worker.handle(Command::PlayerReport(report), t0);
        let sent = lock(&snapshot).player_report.clone().unwrap();
        assert_eq!(
            (sent.serial, sent.sent, sent.message.as_str()),
            (3, true, "player report #11")
        );
        // It carries the name the reporter wears.
        assert!(
            fake.log()
                .contains(&"player cheating 5 ^1Troll by ^2Sol".to_owned()),
            "{:?}",
            fake.log()
        );
    }

    #[test]
    fn the_worn_name_goes_with_the_registration_and_again_when_it_changes() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Name("^1Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(fake.log(), ["register ^1Sol"]);
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().name, "^1Sol");
        // The same name again sends nothing; an empty one is no name.
        worker.handle(Command::Name("^1Sol".into()), t0);
        worker.tick(t0);
        assert_eq!(fake.log().len(), 1);
        worker.handle(Command::Name("Fox".into()), t0);
        let wait = worker.tick(t0);
        assert_eq!(fake.log().last().unwrap(), "register Fox");
        assert_eq!(wait, IDLE_MAX, "nothing else is due");
        // A failure retries after the backoff, not at once.
        fake.fail.store(true, Ordering::SeqCst);
        worker.handle(Command::Name("Wolf".into()), t0);
        worker.tick(t0);
        let count = fake.log().len();
        worker.tick(t0 + Duration::from_secs(1));
        assert_eq!(fake.log().len(), count);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(30));
        assert_eq!(fake.log().last().unwrap(), "register Wolf");
    }

    #[test]
    fn the_own_profile_is_read_again_now_and_then() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        let wait = worker.tick(t0);
        assert_eq!(fake.log(), ["register"]);
        assert!(wait <= PROFILE_EVERY);
        worker.tick(t0 + PROFILE_EVERY - Duration::from_secs(1));
        assert_eq!(fake.log().len(), 1, "not yet");
        worker.tick(t0 + PROFILE_EVERY);
        let own = format!("lookup {}", Identity::from_seed([1; 32]).key_id());
        assert_eq!(fake.log().last(), Some(&own));
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().name, "Other");
        worker.tick(t0 + PROFILE_EVERY + Duration::from_secs(1));
        assert_eq!(fake.log().len(), 2, "and then not for a while");
    }
}
