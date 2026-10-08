//! The hub's feed (`PROTOCOL.md`, "The feed"): a thread of its own that holds a long
//! poll open at the hub and keeps the SJK chat's last messages and online count, and
//! the emotes and looks received, so the identity worker's claims and reports never
//! wait behind it.
//!
//! The worker tells it, through [`FeedShared`], which hub to read (only while the
//! identity is on and the hub answered the registration, and then while the player
//! is on a game server or the chat is on), which game server's emotes and looks to ask
//! for, and whether the chat shows. With the chat off the feed still reads on a game
//! server, for the looks and emotes, but keeps no message: [`ChatState`] stays as an
//! idle feed leaves it. With no hub to read it makes no request.

use crate::hub::Hub;
use crate::keys::Identity;
use crate::service::{HubFactory, ReportOutcome};
use crate::wire::{ChatMessage, Emote, Feed, LookEvent};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Messages kept, as many as the hub keeps.
pub const MESSAGES_KEPT: usize = 200;
/// How long the hub may hold a poll when nothing is new, in seconds.
pub const WAIT: u64 = 25;
/// The feed's HTTP timeout: the hub's wait and room for a slow answer.
pub const TIMEOUT: Duration = Duration::from_secs(40);
/// Shortest time between the starts of two polls.
const POLL_GAP: Duration = Duration::from_secs(2);
/// First wait after a failure, doubled up to [`RETRY_MAX`].
const RETRY_MIN: Duration = Duration::from_secs(2);
const RETRY_MAX: Duration = Duration::from_secs(60);
/// How often an idle feed looks at what it is told.
pub(crate) const IDLE: Duration = Duration::from_secs(1);
/// Emotes waiting for the viewer, the newest kept.
const EMOTES_WAITING: usize = 64;
/// Looks waiting for the viewer, the newest kept.
const LOOKS_WAITING: usize = 64;

/// What the chat shows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChatState {
    /// The last messages, oldest first.
    pub messages: VecDeque<ChatMessage>,
    /// Counts changes to `messages`, `online` and `live`, so a reader that derives from
    /// them knows when to derive again.
    pub revision: u64,
    /// Keys that read the chat in the last minute, as the hub last said.
    pub online: u32,
    /// The last poll reached the hub.
    pub live: bool,
    /// What became of the last message or emote sent.
    pub outcome: Option<ReportOutcome>,
    /// `None` until the hub being read answered once; then a number that changes with
    /// each hub (or each time the reading starts again), so a reader can tell the
    /// backlog that came with that first answer from what is said afterwards.
    pub loaded: Option<u64>,
}

/// What the identity worker tells the feed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct FeedShared {
    /// The hub to read; `None` while there is none to read.
    pub(crate) url: Option<String>,
    /// The game server whose emotes and looks to ask for.
    pub(crate) server: Option<String>,
    /// Whether the SJK chat shows: without it no message is kept.
    pub(crate) chat: bool,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The feed's logic, advanced by [`FeedWorker::step`] with an explicit clock.
pub(crate) struct FeedWorker {
    identity: Identity,
    make_hub: HubFactory,
    shared: Arc<Mutex<FeedShared>>,
    state: Arc<Mutex<ChatState>>,
    emotes: Arc<Mutex<VecDeque<Emote>>>,
    looks: Arc<Mutex<VecDeque<LookEvent>>>,
    hub: Option<Box<dyn Hub>>,
    url: Option<String>,
    /// Whether the chat shows, as last told.
    chat: bool,
    after: u64,
    due: Instant,
    backoff: Duration,
    /// Counts the hubs read (`ChatState::loaded`).
    epoch: u64,
}

impl FeedWorker {
    pub(crate) fn new(
        identity: Identity,
        make_hub: HubFactory,
        shared: Arc<Mutex<FeedShared>>,
        state: Arc<Mutex<ChatState>>,
        emotes: Arc<Mutex<VecDeque<Emote>>>,
        looks: Arc<Mutex<VecDeque<LookEvent>>>,
        now: Instant,
    ) -> Self {
        Self {
            identity,
            make_hub,
            shared,
            state,
            emotes,
            looks,
            hub: None,
            url: None,
            chat: false,
            after: 0,
            due: now,
            backoff: RETRY_MIN,
            epoch: 0,
        }
    }

    /// Poll the hub if it is time to; return how long to wait before the next step.
    /// A poll blocks for as long as the hub holds it (at most [`WAIT`] seconds).
    pub(crate) fn step(&mut self, now: Instant) -> Duration {
        let shared = lock(&self.shared).clone();
        if shared.url != self.url {
            self.chat = shared.chat;
            self.switch(shared.url.clone(), now);
        } else if shared.chat != self.chat {
            self.show_chat(shared.chat, now);
        }
        if self.url.is_none() {
            return IDLE;
        }
        if now < self.due {
            return self.due - now;
        }
        let Some(hub) = self.hub.as_mut() else {
            return IDLE;
        };
        match hub.feed(&self.identity, self.after, shared.server.as_deref(), WAIT) {
            Ok(feed) => {
                self.apply(feed);
                self.backoff = RETRY_MIN;
                self.due = now + POLL_GAP;
            }
            Err(_) => {
                self.set_live(false);
                self.due = now + self.backoff;
                self.backoff = (self.backoff * 2).min(RETRY_MAX);
            }
        }
        self.due.saturating_duration_since(now)
    }

    /// Read another hub, or none: start again from its backlog.
    fn switch(&mut self, url: Option<String>, now: Instant) {
        self.hub = url.as_deref().and_then(|url| (self.make_hub)(url).ok());
        self.url = url;
        self.after = 0;
        self.due = now;
        self.backoff = RETRY_MIN;
        self.clear_chat();
    }

    /// The chat was turned on or off while the same hub is read (on a game server).
    /// Off, the messages go; on, the reading starts again from the hub's backlog, as
    /// it would have had the feed been idle.
    fn show_chat(&mut self, on: bool, now: Instant) {
        self.chat = on;
        if on {
            self.after = 0;
            self.due = now;
        }
        self.clear_chat();
    }

    /// Forget the messages: a new backlog, under a new epoch, is due.
    fn clear_chat(&mut self) {
        self.epoch += 1;
        let mut state = lock(&self.state);
        state.loaded = None;
        state.messages.clear();
        state.live = false;
        state.online = 0;
        state.revision += 1;
    }

    fn set_live(&self, live: bool) {
        if !self.chat {
            return;
        }
        let mut state = lock(&self.state);
        if state.live != live {
            state.live = live;
            state.revision += 1;
        }
    }

    /// Take an answer into the state: its messages only while the chat shows.
    fn apply(&mut self, mut feed: Feed) {
        if self.chat {
            self.apply_chat(&mut feed);
        }
        queue(&self.emotes, feed.emotes, EMOTES_WAITING);
        queue(&self.looks, feed.looks, LOOKS_WAITING);
        self.after = feed.next;
    }

    /// Take an answer's messages, deletions and online count into the chat.
    fn apply_chat(&self, feed: &mut Feed) {
        let mut state = lock(&self.state);
        let mut changed =
            !state.live || state.online != feed.online || state.loaded != Some(self.epoch);
        state.live = true;
        state.online = feed.online;
        // Ids that go backwards mean the hub restarted: its backlog starts afresh.
        if feed.next < self.after {
            changed |= !state.messages.is_empty();
            state.messages.clear();
        }
        if !feed.deleted.is_empty() {
            let before = state.messages.len();
            state
                .messages
                .retain(|message| !feed.deleted.contains(&message.id));
            changed |= state.messages.len() != before;
        }
        for message in feed.chat.drain(..) {
            if state
                .messages
                .back()
                .is_some_and(|last| message.id <= last.id)
            {
                continue;
            }
            if state.messages.len() == MESSAGES_KEPT {
                state.messages.pop_front();
            }
            state.messages.push_back(message);
            changed = true;
        }
        state.loaded = Some(self.epoch);
        if changed {
            state.revision += 1;
        }
    }
}

/// Add `received` to `waiting`, keeping the newest `kept`.
fn queue<T>(waiting: &Mutex<VecDeque<T>>, received: Vec<T>, kept: usize) {
    if received.is_empty() {
        return;
    }
    let mut waiting = lock(waiting);
    for item in received {
        if waiting.len() == kept {
            waiting.pop_front();
        }
        waiting.push_back(item);
    }
}

/// Run the feed on this thread until `stop` is set.
pub(crate) fn run(mut worker: FeedWorker, stop: &std::sync::atomic::AtomicBool) {
    use std::sync::atomic::Ordering;
    while !stop.load(Ordering::Relaxed) {
        let wait = worker.step(Instant::now());
        // Short sleeps, so a change of hub or a stop is seen within a second.
        std::thread::sleep(wait.min(IDLE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::HubError;
    use crate::wire::Profile;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A hub whose feed answers come from a script.
    #[derive(Clone, Default)]
    struct Scripted {
        answers: Arc<Mutex<VecDeque<Result<Feed, HubError>>>>,
        asked: Arc<Mutex<Vec<String>>>,
        down: Arc<AtomicBool>,
    }

    impl Hub for Scripted {
        fn register(&mut self, _: &Identity, _: Option<&str>) -> Result<Profile, HubError> {
            unreachable!()
        }
        fn set_bio(&mut self, _: &Identity, _: &str) -> Result<Profile, HubError> {
            unreachable!()
        }
        fn profile(&mut self, _: &str) -> Result<Profile, HubError> {
            unreachable!()
        }
        fn claim(&mut self, _: &Identity, _: &str, _: u8, _: &str) -> Result<(), HubError> {
            unreachable!()
        }
        fn release(&mut self, _: &Identity, _: &str) -> Result<(), HubError> {
            unreachable!()
        }
        fn presence(&mut self, _: &str) -> Result<Vec<crate::Presence>, HubError> {
            unreachable!()
        }
        fn feed(
            &mut self,
            _: &Identity,
            after: u64,
            server: Option<&str>,
            wait: u64,
        ) -> Result<Feed, HubError> {
            self.asked
                .lock()
                .unwrap()
                .push(format!("after={after} server={server:?} wait={wait}"));
            if self.down.load(Ordering::SeqCst) {
                return Err(HubError::Network("down".to_owned()));
            }
            self.answers
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(Feed::default()))
        }
    }

    fn message(id: u64, text: &str) -> ChatMessage {
        ChatMessage {
            id,
            at: 0,
            key_id: "aa".to_owned(),
            name: "Sol".to_owned(),
            verified: false,
            staff: false,
            text: text.to_owned(),
        }
    }

    fn answer(next: u64, chat: Vec<ChatMessage>) -> Result<Feed, HubError> {
        Ok(Feed {
            next,
            chat,
            online: 3,
            ..Feed::default()
        })
    }

    struct Rig {
        hub: Scripted,
        shared: Arc<Mutex<FeedShared>>,
        state: Arc<Mutex<ChatState>>,
        emotes: Arc<Mutex<VecDeque<Emote>>>,
        looks: Arc<Mutex<VecDeque<LookEvent>>>,
        worker: FeedWorker,
        made: Arc<Mutex<Vec<String>>>,
    }

    fn rig(now: Instant) -> Rig {
        let hub = Scripted::default();
        let shared = Arc::new(Mutex::new(FeedShared::default()));
        let state = Arc::new(Mutex::new(ChatState::default()));
        let emotes = Arc::new(Mutex::new(VecDeque::new()));
        let looks = Arc::new(Mutex::new(VecDeque::new()));
        let made = Arc::new(Mutex::new(Vec::new()));
        let (factory, log) = (hub.clone(), Arc::clone(&made));
        let make: HubFactory = Box::new(move |url| {
            log.lock().unwrap().push(url.to_owned());
            Ok(Box::new(factory.clone()))
        });
        let worker = FeedWorker::new(
            Identity::from_seed([1; 32]),
            make,
            Arc::clone(&shared),
            Arc::clone(&state),
            Arc::clone(&emotes),
            Arc::clone(&looks),
            now,
        );
        Rig {
            hub,
            shared,
            state,
            emotes,
            looks,
            worker,
            made,
        }
    }

    impl Rig {
        fn script(&self, answer: Result<Feed, HubError>) {
            self.hub.answers.lock().unwrap().push_back(answer);
        }
        fn asked(&self) -> Vec<String> {
            self.hub.asked.lock().unwrap().clone()
        }
        fn texts(&self) -> Vec<String> {
            lock(&self.state)
                .messages
                .iter()
                .map(|m| m.text.clone())
                .collect()
        }
        fn read(&self, url: &str, server: Option<&str>) {
            *lock(&self.shared) = FeedShared {
                url: Some(url.to_owned()),
                server: server.map(str::to_owned),
                chat: true,
            };
        }
        fn show_chat(&self, on: bool) {
            lock(&self.shared).chat = on;
        }
    }

    #[test]
    fn nothing_is_asked_while_there_is_no_hub_to_read() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        assert_eq!(rig.worker.step(t0), IDLE);
        assert!(rig.asked().is_empty());
        assert!(rig.made.lock().unwrap().is_empty());
        assert!(!lock(&rig.state).live);
    }

    #[test]
    fn the_feed_appends_in_order_and_skips_repeats() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        rig.script(answer(2, vec![message(1, "a"), message(2, "b")]));
        rig.script(answer(3, vec![message(2, "b"), message(3, "c")]));
        rig.worker.step(t0);
        let revision = lock(&rig.state).revision;
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(rig.texts(), ["a", "b", "c"]);
        assert_eq!(
            rig.asked(),
            [
                "after=0 server=Some(\"1.2.3.4:29070\") wait=25",
                "after=2 server=Some(\"1.2.3.4:29070\") wait=25"
            ]
        );
        let state = lock(&rig.state).clone();
        assert!(state.live);
        assert_eq!(state.online, 3);
        assert!(state.revision > revision);
        // An answer with nothing new changes nothing.
        rig.script(answer(3, Vec::new()));
        rig.worker.step(t0 + Duration::from_secs(4));
        assert_eq!(lock(&rig.state).revision, state.revision);
    }

    #[test]
    fn the_last_200_messages_are_kept() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        let batch: Vec<ChatMessage> = (1..=250).map(|id| message(id, &id.to_string())).collect();
        rig.script(answer(250, batch));
        rig.worker.step(t0);
        let texts = rig.texts();
        assert_eq!(texts.len(), MESSAGES_KEPT);
        assert_eq!(texts[0], "51");
    }

    #[test]
    fn deleted_ids_leave_the_state() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(answer(2, vec![message(1, "spam"), message(2, "hi")]));
        rig.script(Ok(Feed {
            next: 3,
            deleted: vec![1],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(rig.texts(), ["hi"]);
    }

    #[test]
    fn emotes_wait_for_the_viewer() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        let wave = Emote {
            id: 4,
            at: 0,
            slot: 3,
            claimed_name: "Sol".to_owned(),
            key_id: "aa".to_owned(),
            emote: "wave".to_owned(),
        };
        rig.script(Ok(Feed {
            next: 4,
            emotes: vec![wave.clone()],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        assert_eq!(
            lock(&rig.emotes).iter().cloned().collect::<Vec<_>>(),
            [wave]
        );
    }

    fn look_event(id: u64, illuminate: bool) -> LookEvent {
        LookEvent {
            id,
            at: 0,
            slot: 3,
            claimed_name: "Sol".to_owned(),
            key_id: "aa".to_owned(),
            saber: "saber_sun".to_owned(),
            illuminate,
        }
    }

    #[test]
    fn looks_wait_for_the_viewer_and_the_newest_are_kept() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        let batch: Vec<LookEvent> = (1..=70).map(|id| look_event(id, id % 2 == 0)).collect();
        rig.script(Ok(Feed {
            next: 70,
            looks: batch,
            ..Feed::default()
        }));
        rig.worker.step(t0);
        let waiting = lock(&rig.looks);
        assert_eq!(waiting.len(), LOOKS_WAITING);
        assert_eq!(waiting.front().map(|look| look.id), Some(7));
        assert_eq!(waiting.back(), Some(&look_event(70, true)));
    }

    #[test]
    fn with_the_chat_off_the_feed_keeps_looks_and_emotes_but_no_message() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        rig.show_chat(false);
        rig.script(Ok(Feed {
            next: 5,
            chat: vec![message(4, "hidden")],
            looks: vec![look_event(5, true)],
            online: 9,
            ..Feed::default()
        }));
        rig.worker.step(t0);
        assert_eq!(
            rig.asked(),
            ["after=0 server=Some(\"1.2.3.4:29070\") wait=25"]
        );
        assert_eq!(lock(&rig.looks).len(), 1);
        // The chat is as an idle feed leaves it: nothing the page or the game shows.
        let idle = |state: &ChatState| ChatState {
            revision: state.revision,
            ..ChatState::default()
        };
        let state = lock(&rig.state).clone();
        assert_eq!(state, idle(&state));
        // A failure says nothing either.
        rig.hub.down.store(true, Ordering::SeqCst);
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(*lock(&rig.state), state);
        rig.hub.down.store(false, Ordering::SeqCst);
        // On again: the backlog is read from the start, under a new epoch.
        rig.show_chat(true);
        rig.script(answer(6, vec![message(4, "hidden"), message(6, "hello")]));
        rig.worker.step(t0 + POLL_GAP * 2);
        assert_eq!(
            rig.asked()[2],
            "after=0 server=Some(\"1.2.3.4:29070\") wait=25"
        );
        assert_eq!(rig.texts(), ["hidden", "hello"]);
        let first = lock(&rig.state).loaded.expect("loaded");
        // Off once more: the messages go, and the reading goes on from where it was.
        rig.show_chat(false);
        rig.worker.step(t0 + POLL_GAP * 3);
        assert!(rig.texts().is_empty());
        assert_eq!(lock(&rig.state).loaded, None);
        assert_eq!(
            rig.asked()[3],
            "after=6 server=Some(\"1.2.3.4:29070\") wait=25"
        );
        rig.show_chat(true);
        rig.worker.step(t0 + POLL_GAP * 4);
        assert_ne!(lock(&rig.state).loaded, Some(first));
    }

    #[test]
    fn a_new_hub_starts_from_the_backlog() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://one", None);
        rig.script(answer(9, vec![message(9, "old hub")]));
        rig.worker.step(t0);
        rig.read("https://two", None);
        rig.script(answer(1, vec![message(1, "new hub")]));
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(*rig.made.lock().unwrap(), ["https://one", "https://two"]);
        assert_eq!(rig.asked()[1], "after=0 server=None wait=25");
        assert_eq!(rig.texts(), ["new hub"]);
    }

    #[test]
    fn the_backlog_is_marked_loaded_once_the_hub_answered() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        assert_eq!(lock(&rig.state).loaded, None);
        rig.read("https://one", None);
        rig.hub.down.store(true, Ordering::SeqCst);
        rig.worker.step(t0);
        assert_eq!(lock(&rig.state).loaded, None, "no answer yet");
        rig.hub.down.store(false, Ordering::SeqCst);
        rig.script(answer(2, vec![message(1, "a"), message(2, "b")]));
        rig.worker.step(t0 + RETRY_MIN);
        let first = lock(&rig.state).loaded.expect("loaded");
        // Another hub: not loaded until it answers, then under another epoch.
        rig.read("https://two", None);
        rig.hub.down.store(true, Ordering::SeqCst);
        rig.worker.step(t0 + RETRY_MIN * 2);
        assert_eq!(lock(&rig.state).loaded, None);
        rig.hub.down.store(false, Ordering::SeqCst);
        rig.worker.step(t0 + RETRY_MIN * 4);
        let second = lock(&rig.state).loaded.expect("loaded again");
        assert_ne!(first, second);
    }

    #[test]
    fn a_hub_that_restarted_starts_from_its_backlog() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(answer(90, vec![message(90, "before")]));
        // The hub restarted: its ids begin again, below what was asked.
        rig.script(answer(
            2,
            vec![message(1, "after a"), message(2, "after b")],
        ));
        rig.worker.step(t0);
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(rig.texts(), ["after a", "after b"]);
        rig.worker.step(t0 + Duration::from_secs(4));
        assert_eq!(rig.asked()[2], "after=2 server=None wait=25");
    }

    #[test]
    fn feed_stops_when_chat_turns_off() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(answer(1, vec![message(1, "a")]));
        rig.worker.step(t0);
        *lock(&rig.shared) = FeedShared::default();
        assert_eq!(rig.worker.step(t0 + Duration::from_secs(2)), IDLE);
        assert_eq!(rig.asked().len(), 1);
        let state = lock(&rig.state).clone();
        assert!(!state.live);
        assert!(
            state.messages.is_empty(),
            "the chat is off: nothing to show"
        );
    }

    #[test]
    fn polls_are_two_seconds_apart() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        assert_eq!(rig.worker.step(t0), POLL_GAP);
        assert_eq!(
            rig.worker.step(t0 + Duration::from_millis(500)),
            Duration::from_millis(1_500)
        );
        assert_eq!(rig.asked().len(), 1);
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(rig.asked().len(), 2);
    }

    #[test]
    fn failures_back_off() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.hub.down.store(true, Ordering::SeqCst);
        assert_eq!(rig.worker.step(t0), RETRY_MIN);
        assert!(!lock(&rig.state).live);
        assert_eq!(rig.worker.step(t0 + RETRY_MIN), RETRY_MIN * 2);
        rig.hub.down.store(false, Ordering::SeqCst);
        rig.worker.step(t0 + RETRY_MIN * 3);
        assert!(lock(&rig.state).live);
        assert_eq!(rig.asked().len(), 3);
    }
}
