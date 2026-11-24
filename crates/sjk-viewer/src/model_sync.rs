//! Model sync in the client (`docs/model-sync.md`): JoF- and Sith-verified players
//! share the model they wear through the Sith cloud, and others fetch it.
//!
//! - `sharemodel` gathers the model the player wears with its voice, shaders and the
//!   textures it uses from elsewhere ([`sjk_model_sync::gather`]), packs it, asks the
//!   hub for an upload ticket and sends the pack to the Sith cloud; it says what went
//!   in first. `modelsync check` does the same without sending.
//! - `getmodel <name>` fetches a synced model, `getmodel all` those of the players on
//!   the server this game lacks; `cl_autoModels` (off by default) fetches those of the
//!   players met, and new versions of fetched ones every hour.
//! - `modelsync share|get|check|rollback ...` are the same commands under one name;
//!   `modelsync` alone opens the page.
//! - `modelrollback <name> <version>` (staff) makes an earlier version current.
//!
//! Every download is checked again here before it is written ([`sjk_model_sync::client`]),
//! and goes where the Sith cloud's files go ([`crate::sith_cloud::folder`]); in a game,
//! the files are mounted again at once and the players wearing it load it on this map
//! ([`take_downloaded`], `rig_reload.rs`), else it loads with the next map. A model the game already has is never fetched. Network calls
//! run on a thread of their own; the frame only hands jobs over and reads results.

use crate::console::ViewerConsole;
use sjk_model_sync::Limits;
use sjk_model_sync::client::{Cloud, Index, Record};
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Fetch the synced models of the players met, and keep fetched ones up to date.
pub(crate) const AUTO: &str = "cl_autoModels";
pub(crate) const GET: &str = "getmodel";
pub(crate) const SHARE: &str = "sharemodel";
pub(crate) const ROLLBACK: &str = "modelrollback";
/// Opens the Model sync page (`console_model_sync.rs`); with a word, the command it
/// names ([`subcommand`]).
pub(crate) const PAGE: &str = "modelsync";
/// What `sharemodel` would send, said without sending (`modelsync check`).
pub(crate) const CHECK: &str = "modelcheck";
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    (
        PAGE,
        "Open the Model sync page: share your models (with their voice) through the Sith cloud and download the ones others shared; modelsync share [name], modelsync get <name|all>, modelsync check [name]",
    ),
    (
        GET,
        "Download a player model other players shared through the Sith cloud: getmodel <name>, or getmodel all for every player here whose model you lack",
    ),
    (
        SHARE,
        "Share the model you wear through the Sith cloud, so others can see it (JoF and Sith verified players)",
    ),
    (
        ROLLBACK,
        "Make an earlier version of a shared model current again: modelrollback <name> <version> (staff)",
    ),
];

/// How long the hub may take to give a ticket.
const TICKET_WAIT: Duration = Duration::from_secs(30);
/// How often the players met are looked at, with `cl_autoModels` on.
const MET_EVERY: Duration = Duration::from_secs(20);
/// How often the fetched models are checked for new versions.
const UPDATE_EVERY: Duration = Duration::from_secs(3600);
/// How long an index is used before it is read again.
const INDEX_FOR: Duration = Duration::from_secs(300);

pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        AUTO,
        false,
        CvarFlags::ARCHIVE,
        "Download by themselves the player models other players shared through the Sith cloud, for the players you meet, and their new versions every hour; off by default (getmodel <name> fetches one)",
    ))?;
    Ok(())
}

/// Where a job talks to and writes.
#[derive(Clone)]
struct Place {
    url: String,
    hub: String,
    game_data: PathBuf,
    base: PathBuf,
}

impl Place {
    fn of(console: &ViewerConsole, game_data: &Path) -> Self {
        Self {
            url: console
                .text_value(crate::sith_cloud::URL)
                .unwrap_or("")
                .to_owned(),
            hub: console.text_value("cl_hubUrl").unwrap_or("").to_owned(),
            game_data: game_data.to_owned(),
            base: crate::sith_cloud::folder(game_data),
        }
    }
}

enum Job {
    /// Fetch these models, each with whether the game already has it (`quiet`: met
    /// players', said only when fetched, and one the game has only when it overrides
    /// a host pack's).
    Fetch(Vec<(String, bool)>, bool),
    /// Fetch the shared ones of these models (the players here), and say which were
    /// not shared.
    FetchAll(Vec<String>),
    /// Read the index again (the Model sync page).
    Index,
    /// Fetch what changed of the models fetched before.
    Update,
    Upload {
        name: String,
        pk3: Vec<u8>,
        ticket: String,
    },
    Rollback {
        name: String,
        ticket: String,
    },
}

/// What the network thread does: one job at a time, with the index it last read.
struct Worker {
    agent: ureq::Agent,
    index: Option<(Instant, String, Arc<Index>)>,
    /// The models written during the current job.
    installed: std::cell::RefCell<Vec<String>>,
}

impl Worker {
    fn cloud(&self, place: &Place) -> Result<Cloud, String> {
        Cloud::find(&self.agent, &place.url, &place.hub).map_err(|error| error.to_string())
    }

    fn index(&mut self, cloud: &Cloud, place: &Place, fresh: bool) -> Result<Arc<Index>, String> {
        let key = format!("{}|{}", place.url, place.hub);
        if !fresh
            && let Some((at, known, index)) = &self.index
            && *known == key
            && at.elapsed() < INDEX_FOR
        {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(
            cloud
                .index(&self.agent)
                .map_err(|error| error.to_string())?,
        );
        self.index = Some((Instant::now(), key, Arc::clone(&index)));
        Ok(index)
    }

    fn run(&mut self, job: Job, place: &Place) -> Done {
        let lines = match self.try_run(job, place) {
            Ok(lines) => lines,
            Err(why) => vec![format!("^3Model sync: {why}.")],
        };
        Done {
            lines,
            index: self.index.as_ref().map(|(_, _, index)| Arc::clone(index)),
            installed: self.installed.take(),
        }
    }

    fn try_run(&mut self, job: Job, place: &Place) -> Result<Vec<String>, String> {
        let cloud = self.cloud(place)?;
        match job {
            Job::Fetch(names, quiet) => {
                let index = self.index(&cloud, place, !quiet)?;
                Ok(names
                    .iter()
                    .filter(|(name, installed)| {
                        !(quiet && *installed && !index.packs.contains_key(name))
                    })
                    .filter_map(|(name, _)| self.fetch(&cloud, &index, place, name, quiet))
                    .collect())
            }
            Job::FetchAll(names) => {
                let index = self.index(&cloud, place, true)?;
                let (shared, unshared): (Vec<&String>, Vec<&String>) = names
                    .iter()
                    .partition(|name| index.models.contains_key(*name));
                let mut lines: Vec<String> = shared
                    .iter()
                    .filter_map(|name| self.fetch(&cloud, &index, place, name, false))
                    .collect();
                if !unshared.is_empty() {
                    let listed: Vec<&str> = unshared.iter().map(|name| name.as_str()).collect();
                    lines.push(format!(
                        "Model sync: not shared yet: {}.",
                        listed.join(", ")
                    ));
                }
                Ok(lines)
            }
            Job::Index => {
                self.index(&cloud, place, true)?;
                Ok(Vec::new())
            }
            Job::Update => {
                let index = self.index(&cloud, place, true)?;
                let record = Record::load(&place.game_data);
                let names: Vec<String> = record
                    .models
                    .keys()
                    .filter(|name| {
                        index
                            .models
                            .get(*name)
                            .is_some_and(|entry| record.wants(name, entry))
                    })
                    .cloned()
                    .collect();
                Ok(names
                    .iter()
                    .filter_map(|name| self.fetch(&cloud, &index, place, name, true))
                    .collect())
            }
            Job::Upload { name, pk3, ticket } => {
                let uploaded = cloud
                    .upload(&self.agent, &name, &pk3, &ticket)
                    .map_err(|error| error.to_string())?;
                let _ = self.index(&cloud, place, true);
                Ok(vec![if uploaded.linked {
                    format!(
                        "Model sync: {name} is already on the Sith cloud as it is (version {}).",
                        uploaded.version
                    )
                } else {
                    format!(
                        "Model sync: {name} shared, version {}. Players with it download it from their next map.",
                        uploaded.version
                    )
                }])
            }
            Job::Rollback { name, ticket } => {
                let version = cloud
                    .rollback(&self.agent, &name, &ticket)
                    .map_err(|error| error.to_string())?;
                let _ = self.index(&cloud, place, true);
                Ok(vec![format!(
                    "Model sync: {name} is back to version {version}."
                )])
            }
        }
    }

    /// Fetches one model, or says why not (nothing in `quiet` but a success).
    fn fetch(
        &self,
        cloud: &Cloud,
        index: &Index,
        place: &Place,
        name: &str,
        quiet: bool,
    ) -> Option<String> {
        let loud = |text: String| (!quiet).then_some(text);
        let Some(entry) = index.models.get(name) else {
            if index.packs.contains_key(name) {
                return loud(format!(
                    "Model sync: nobody shared a newer {name}; the Sith cloud card (sith.downloads) has the hosts' pack."
                ));
            }
            return loud(format!(
                "Model sync: nobody shared {name} on the Sith cloud."
            ));
        };
        let mut record = Record::load(&place.game_data);
        if !record.wants(name, entry) {
            return loud(format!("Model sync: you have the latest {name}."));
        }
        let result = cloud
            .download(&self.agent, name, entry, &Limits::DEFAULT)
            .map_err(|error| error.to_string())
            .and_then(|(bytes, _)| {
                record
                    .install(&place.game_data, &place.base, name, entry, &bytes)
                    .map_err(|error| error.to_string())
            });
        Some(match result {
            Ok(_) => {
                self.installed.borrow_mut().push(name.to_owned());
                format!("Model sync: {name} version {} downloaded.", entry.version)
            }
            Err(why) => format!("^3Model sync: {name}: {why}."),
        })
    }
}

/// A pack waiting for its ticket.
struct Waiting {
    name: String,
    pk3: Vec<u8>,
    /// The ticket outcome's serial when it was asked for.
    serial: u64,
    at: Instant,
    rollback: bool,
}

/// What the thread sends back after a job: console lines and the index it holds.
struct Done {
    lines: Vec<String>,
    index: Option<Arc<Index>>,
    /// The models it wrote.
    installed: Vec<String>,
}

/// The jobs sent to the thread and what it sends back.
type Channels = (Sender<(Job, Place)>, Receiver<Done>);

/// Console lines the Model sync page shows, at most.
const RECENT: usize = 6;

struct State {
    jobs: Option<Channels>,
    waiting: Option<Waiting>,
    /// Models asked for this session, so a met player's is asked once.
    asked: BTreeSet<String>,
    /// Models the Inspect card offered this session.
    offered: BTreeSet<String>,
    /// A rig model waiting for its share to be confirmed, since when.
    confirm: Option<(String, Instant)>,
    met_at: Option<Instant>,
    /// The models the other players on the server wear, read every [`MET_EVERY`].
    here: Vec<String>,
    update_at: Option<Instant>,
    /// The index the thread last read, for the Model sync page.
    index: Option<Arc<Index>>,
    /// The last lines said, newest last, for the page.
    recent: Vec<String>,
    /// Models downloaded and not yet shown on the map ([`take_downloaded`]).
    downloaded: BTreeSet<String>,
    /// Jobs sent and not answered yet.
    pending: usize,
    /// Bumped whenever something the page shows may have changed (a line said, a job
    /// answered), so it reads its rows again only then.
    changes: u64,
}

static STATE: Mutex<State> = Mutex::new(State {
    jobs: None,
    waiting: None,
    asked: BTreeSet::new(),
    offered: BTreeSet::new(),
    confirm: None,
    met_at: None,
    here: Vec::new(),
    update_at: None,
    index: None,
    recent: Vec::new(),
    downloaded: BTreeSet::new(),
    pending: 0,
    changes: 0,
});

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|error| error.into_inner())
}

impl State {
    fn send(&mut self, job: Job, place: Place) {
        let (jobs, _) = self.jobs.get_or_insert_with(|| {
            let (jobs, incoming) = channel::<(Job, Place)>();
            let (answers, results) = channel();
            std::thread::Builder::new()
                .name("model-sync".into())
                .spawn(move || {
                    let mut worker = Worker {
                        agent: sjk_model_sync::client::agent(crate::build_info::VERSION),
                        index: None,
                        installed: Default::default(),
                    };
                    for (job, place) in incoming {
                        if answers.send(worker.run(job, &place)).is_err() {
                            break;
                        }
                    }
                })
                .expect("the model sync thread starts");
            (jobs, results)
        });
        if jobs.send((job, place)).is_ok() {
            self.pending += 1;
            self.changes += 1;
        }
    }

    fn say(&mut self, line: &str) {
        if self.recent.len() == RECENT {
            self.recent.remove(0);
        }
        self.recent.push(line.to_owned());
        self.changes += 1;
    }
}

/// The command `modelsync <word> ...` names, with its arguments: `share`, `get`,
/// `check`, `rollback`; `None` for another word (the page opens).
pub(crate) fn subcommand(args: &[String]) -> Option<(&'static str, &[String])> {
    let (word, rest) = args.split_first()?;
    let name = match word.to_ascii_lowercase().as_str() {
        "share" | "update" => SHARE,
        "get" | "download" => GET,
        "check" => CHECK,
        "rollback" => ROLLBACK,
        _ => return None,
    };
    Some((name, rest))
}

/// `getmodel`, `sharemodel`, `modelsync check` and `modelrollback`; lines for the
/// console, which the Model sync page shows too ([`recent_lines`]).
pub(crate) fn command(
    name: &str,
    args: &[String],
    console: &ViewerConsole,
    game_data: &Path,
    vfs: Option<&sjk_vfs::VirtualFileSystem>,
    worn: Option<String>,
) -> Result<Vec<String>, String> {
    let result = run_command(name, args, console, game_data, vfs, worn);
    let mut state = lock();
    match &result {
        Ok(lines) => lines.iter().for_each(|line| state.say(line)),
        Err(why) => state.say(&format!("^3Model sync: {why}.")),
    }
    result
}

fn run_command(
    name: &str,
    args: &[String],
    console: &ViewerConsole,
    game_data: &Path,
    vfs: Option<&sjk_vfs::VirtualFileSystem>,
    worn: Option<String>,
) -> Result<Vec<String>, String> {
    let place = Place::of(console, game_data);
    match name {
        GET if args
            .first()
            .is_some_and(|word| word.eq_ignore_ascii_case("all")) =>
        {
            let record = Record::load(game_data);
            let mut state = lock();
            let wanted: Vec<String> = state
                .here
                .iter()
                .filter(|model| !installed(model) && !record.models.contains_key(*model))
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if wanted.is_empty() {
                return Ok(vec![
                    "Model sync: you have the model of every player here.".to_owned(),
                ]);
            }
            state.asked.extend(wanted.iter().cloned());
            let count = wanted.len();
            state.send(Job::FetchAll(wanted), place);
            Ok(vec![format!(
                "Model sync: looking for the {count} model{} you lack on the Sith cloud...",
                if count == 1 { "" } else { "s" }
            )])
        }
        GET => {
            let model = args
                .first()
                .map(|model| model.trim().to_ascii_lowercase())
                .filter(|model| sjk_model_sync::valid_name(model))
                .ok_or("usage: getmodel <name> (a-z 0-9 _ -), or getmodel all")?;
            let mut state = lock();
            state.asked.insert(model.clone());
            let has = installed(&model);
            state.send(Job::Fetch(vec![(model.clone(), has)], false), place);
            Ok(vec![format!(
                "Model sync: looking for {model} on the Sith cloud..."
            )])
        }
        SHARE | CHECK => {
            let model = match args.first() {
                Some(model) => model.trim().to_ascii_lowercase(),
                None => {
                    worn.ok_or("you wear no model to share (sharemodel <name> shares another)")?
                }
            };
            if crate::installed_models::models()
                .is_some_and(|models| models.retail.contains(&model))
            {
                return Err(format!(
                    "{model} is one of the game's own models: everyone has it"
                ));
            }
            let vfs = vfs.ok_or("the game data is not loaded yet")?;
            let gathered = gather(vfs, &model);
            let notes = notes(&gathered);
            let (pk3, checked) =
                sjk_model_sync::pack::pack(&model, gathered.files, &Limits::DEFAULT)
                    .map_err(|why| format!("{model} cannot be shared: {why}.{}", hint(&why)))?;
            let packed = format!(
                "Model sync: {model} packs into {} files, {}{notes}",
                checked.files.len(),
                sjk_jof_content::human_size(pk3.len() as u64)
            );
            if name == CHECK {
                let mut lines = vec![packed];
                lines.extend(missing_line(&gathered.missing));
                lines.push(format!(
                    "Model sync: {model} can be shared (sharemodel {model}, or Share on the Model sync page)."
                ));
                return Ok(lines);
            }
            // A rig model is most often taken from another game: share one only with
            // its author's leave (Sol, 11/11/2026). The second press within a minute,
            // or `sharemodel <name> mine`, confirms it.
            if checked.rig && !confirmed(&model, args.get(1).map(String::as_str)) {
                return Ok(vec![format!(
                    "Model sync: {model} is a rig model. Share only a model you made, or one its author lets you share: press Share again to confirm (or sharemodel {model} mine)."
                )]);
            }
            if !crate::player_identity::ask_model_ticket(model.clone(), checked.content_md5) {
                return Err("the SJK hub is not connected".into());
            }
            let serial = crate::player_identity::model_ticket().map_or(0, |outcome| outcome.serial);
            lock().waiting = Some(Waiting {
                name: model.clone(),
                pk3,
                serial,
                at: Instant::now(),
                rollback: false,
            });
            let mut lines = vec![packed];
            lines.extend(missing_line(&gathered.missing));
            lines.push("Model sync: asking the SJK hub...".to_owned());
            Ok(lines)
        }
        ROLLBACK => {
            if !crate::player_identity::is_staff() {
                return Err("only SJK staff can roll a model back".into());
            }
            let (Some(model), Some(version)) = (
                args.first().map(|model| model.to_ascii_lowercase()),
                args.get(1).and_then(|version| version.parse::<u32>().ok()),
            ) else {
                return Err("usage: modelrollback <name> <version>".into());
            };
            if !sjk_model_sync::valid_name(&model) || version == 0 {
                return Err("usage: modelrollback <name> <version>".into());
            }
            if !crate::player_identity::ask_model_rollback_ticket(model.clone(), version) {
                return Err("the SJK hub is not connected".into());
            }
            let serial = crate::player_identity::model_ticket().map_or(0, |outcome| outcome.serial);
            lock().waiting = Some(Waiting {
                name: model.clone(),
                pk3: Vec::new(),
                serial,
                at: Instant::now(),
                rollback: true,
            });
            Ok(vec![format!(
                "Model sync: rolling {model} back to version {version}..."
            )])
        }
        _ => Err(format!("unknown model sync command {name}")),
    }
}

/// How long a rig model's confirmation waits for the second press.
const CONFIRM_FOR: Duration = Duration::from_secs(60);

/// Whether sharing rig model `model` is confirmed: `mine` given, or a second
/// `sharemodel` for it within [`CONFIRM_FOR`] of the first (which arms it).
fn confirmed(model: &str, word: Option<&str>) -> bool {
    let mut state = lock();
    let now = Instant::now();
    let armed = state
        .confirm
        .take()
        .is_some_and(|(name, at)| name == model && now.duration_since(at) < CONFIRM_FOR);
    if armed || word.is_some_and(|word| word.eq_ignore_ascii_case("mine")) {
        return true;
    }
    state.confirm = Some((model.to_owned(), now));
    false
}

/// The game's files, for [`sjk_model_sync::gather`]: what the retail game has is
/// stock (its `assets*.pk3`), never packed or moved.
struct GameFiles<'a> {
    vfs: &'a sjk_vfs::VirtualFileSystem,
    retail: Option<sjk_vfs::VirtualFileSystem>,
}

impl sjk_model_sync::gather::Source for GameFiles<'_> {
    fn list(&self, dir: &str, extension: &str) -> Vec<String> {
        self.vfs
            .list_files(dir, extension)
            .into_iter()
            .map(|path| format!("{dir}/{path}"))
            .collect()
    }

    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.vfs.read(path).ok().flatten().map(|asset| asset.bytes)
    }

    fn has(&self, path: &str) -> bool {
        self.vfs.contains(path).unwrap_or(false)
    }

    fn stock(&self, path: &str) -> bool {
        self.retail
            .as_ref()
            .is_some_and(|retail| retail.contains(path).unwrap_or(false))
    }
}

/// Everything model `name` needs, from wherever this game has it: its folder, its
/// voice, its shaders and what it uses from elsewhere (docs/model-sync.md).
fn gather(vfs: &sjk_vfs::VirtualFileSystem, name: &str) -> sjk_model_sync::gather::Gathered {
    let files = GameFiles {
        vfs,
        retail: sjk_client::retail_view(vfs),
    };
    sjk_model_sync::gather::gather(name, &files)
}

/// What a share took beyond the folder, for the line that says it was packed: the
/// voice, and what came from elsewhere.
fn notes(gathered: &sjk_model_sync::gather::Gathered) -> String {
    let mut notes = Vec::new();
    if gathered.sounds > 0 {
        notes.push(format!("{} sounds (its voice)", gathered.sounds));
    }
    if let Some(voice) = &gathered.other_voice {
        notes.push(format!(
            "it speaks with the {voice} voice, not shared (the game's own, or not on this PC)"
        ));
    } else if gathered.sounds == 0 {
        notes.push("no voice of its own: others hear the game's generic voice".to_owned());
    }
    if gathered.pulled_in > 0 {
        notes.push(format!(
            "{} file{} it uses from outside its folder",
            gathered.pulled_in,
            if gathered.pulled_in == 1 { "" } else { "s" }
        ));
    }
    format!(": {}.", notes.join("; "))
}

/// The textures and shaders the model names that this game has nowhere: they show
/// plain for everyone, the sender too.
fn missing_line(missing: &[String]) -> Option<String> {
    const SHOWN: usize = 4;
    (!missing.is_empty()).then(|| {
        let more = missing.len().saturating_sub(SHOWN);
        format!(
            "^3Model sync: not found on this PC, so plain for others too: {}{}.",
            missing[..missing.len().min(SHOWN)].join(", "),
            if more > 0 {
                format!(" and {more} more")
            } else {
                String::new()
            }
        )
    })
}

/// How to fix a refused pack, after the reason.
fn hint(why: &sjk_model_sync::Rejection) -> &'static str {
    use sjk_model_sync::Rejection;
    match why {
        Rejection::TooLarge { what, .. } if what.contains("sound") => {
            " Drop the long sounds (songs, long taunts) from its voice folder, or save them as .mp3"
        }
        Rejection::TooLarge { what, .. } if what.contains("texture") => {
            " Make its largest textures smaller (2048 pixels is plenty) or save them as .jpg"
        }
        Rejection::TooLarge { what, .. } if what.contains("vertex") => {
            " The mesh is too detailed: share a lighter version of it"
        }
        Rejection::TooLarge { .. } => {
            " Remove the skins and textures it does not use from its folder, or save textures as .jpg"
        }
        Rejection::FileType(_) => " Remove that file from the model's folder",
        Rejection::Broken { .. } => " Open that file in an image or sound editor and save it again",
        Rejection::BadName(_) => {
            " Rename the model's folder (and its .skin and shader names) to a-z 0-9 _ -"
        }
        _ => "",
    }
}

/// The inspected player's `(name, model)`: when this game lacks the model, says once
/// how to fetch it (the Inspect card's "Download this model").
pub(crate) fn offer(
    console: &mut ViewerConsole,
    game_data: &Path,
    inspected: Option<(String, String)>,
) {
    let Some((name, model)) = inspected else {
        return;
    };
    if !sjk_model_sync::valid_name(&model)
        || installed(&model)
        || Record::load(game_data).models.contains_key(&model)
    {
        return;
    }
    let mut state = lock();
    if !state.offered.insert(model.clone()) {
        return;
    }
    drop(state);
    console.push_log(format!(
        "{name}^7 wears {model}, which you don't have: getmodel {model} downloads it if they shared it (or turn on {AUTO})."
    ));
}

fn installed(model: &str) -> bool {
    crate::installed_models::models()
        .is_some_and(|models| models.all.binary_search(&model.to_owned()).is_ok())
}

/// Each frame: hand a ticket that came to its upload, print what the thread finished,
/// and with `cl_autoModels` on, ask for the models of the players met and, hourly,
/// for new versions. A few comparisons while nothing is due.
pub(crate) fn update(
    console: &mut ViewerConsole,
    game_data: &Path,
    met: impl FnOnce() -> Vec<String>,
) {
    let now = Instant::now();
    let mut state = lock();
    let mut lines = Vec::new();
    let mut answers = Vec::new();
    if let Some((_, results)) = &state.jobs {
        while let Ok(done) = results.try_recv() {
            answers.push(done);
        }
    }
    for done in answers {
        state.pending = state.pending.saturating_sub(1);
        state.changes += 1;
        if done.index.is_some() {
            state.index = done.index;
        }
        state.downloaded.extend(done.installed);
        lines.extend(done.lines);
    }
    if let Some(waiting) = &state.waiting {
        let outcome = crate::player_identity::model_ticket()
            .filter(|outcome| outcome.serial > waiting.serial);
        if let Some(outcome) = outcome {
            let waiting = state.waiting.take().expect("a pack waits");
            if outcome.sent {
                let place = Place::of(console, game_data);
                let job = if waiting.rollback {
                    Job::Rollback {
                        name: waiting.name,
                        ticket: outcome.message,
                    }
                } else {
                    Job::Upload {
                        name: waiting.name,
                        pk3: waiting.pk3,
                        ticket: outcome.message,
                    }
                };
                state.send(job, place);
            } else {
                lines.push(format!(
                    "^3Model sync: the SJK hub said no: {}.",
                    outcome.message
                ));
            }
        } else if now.duration_since(waiting.at) > TICKET_WAIT {
            state.waiting = None;
            lines.push("^3Model sync: the SJK hub did not answer; try again.".to_owned());
        }
    }
    if state
        .met_at
        .is_none_or(|at| now.duration_since(at) >= MET_EVERY)
    {
        state.met_at = Some(now);
        state.here = met()
            .into_iter()
            .filter(|model| sjk_model_sync::valid_name(model))
            .collect();
        if console.bool_cvar(AUTO) == Some(true) {
            let record = Record::load(game_data);
            let wanted: Vec<(String, bool)> = state
                .here
                .clone()
                .into_iter()
                .filter(|model| !record.models.contains_key(model))
                .filter(|model| !state.asked.contains(model))
                .map(|model| {
                    let has = installed(&model);
                    (model, has)
                })
                .collect();
            if !wanted.is_empty() {
                state
                    .asked
                    .extend(wanted.iter().map(|(model, _)| model.clone()));
                state.send(Job::Fetch(wanted, true), Place::of(console, game_data));
            }
        }
    }
    if console.bool_cvar(AUTO) == Some(true) {
        let first = state.update_at.is_none();
        if first
            || state
                .update_at
                .is_some_and(|at| now.duration_since(at) >= UPDATE_EVERY)
        {
            state.update_at = Some(now);
            if !first || !Record::load(game_data).models.is_empty() {
                state.send(Job::Update, Place::of(console, game_data));
            }
        }
    }
    for line in &lines {
        state.say(line);
    }
    drop(state);
    for line in lines {
        // The console's log line is the log's too: one line each (it was written twice).
        console.push_log(line);
    }
}

/// Where a model stands, for the Model sync page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageRow {
    pub(crate) name: String,
    /// The version on the Sith cloud, when someone shared it.
    pub(crate) cloud: Option<u32>,
    /// The version this game fetched through model sync, if any.
    pub(crate) fetched: Option<u32>,
    /// One of the hosts' own packs holds it too (an upload is a newer version of it).
    pub(crate) host_pack: bool,
    /// The game has it (from anywhere), so it can be shared from here.
    pub(crate) installed: bool,
    /// The model the player wears.
    pub(crate) worn: bool,
}

/// What the Model sync page shows: the models this game has that are not the game's
/// own, then those shared on the Sith cloud it lacks; the worn model first, then by
/// name. `None` for the cloud's fields until the index has been read.
pub(crate) fn page_rows(game_data: &Path, worn: Option<&str>) -> Vec<PageRow> {
    let record = Record::load(game_data);
    let index = lock().index.clone();
    let installed_models = crate::installed_models::models();
    let mut names: BTreeSet<String> = installed_models
        .as_ref()
        .map(|models| {
            models
                .all
                .iter()
                .filter(|name| !models.retail.contains(*name))
                .filter(|name| sjk_model_sync::valid_name(name))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if let Some(index) = &index {
        names.extend(index.models.keys().cloned());
    }
    names.extend(record.models.keys().cloned());
    let mut rows: Vec<PageRow> = names
        .into_iter()
        .map(|name| PageRow {
            cloud: index
                .as_ref()
                .and_then(|index| index.models.get(&name))
                .map(|entry| entry.version),
            fetched: record.models.get(&name).map(|fetched| fetched.version),
            host_pack: index
                .as_ref()
                .is_some_and(|index| index.packs.contains_key(&name)),
            installed: installed(&name) || record.models.contains_key(&name),
            worn: worn == Some(name.as_str()),
            name,
        })
        .collect();
    rows.sort_by(|a, b| b.worn.cmp(&a.worn).then_with(|| a.name.cmp(&b.name)));
    rows
}

/// What the Model sync page offers for a row: a button, and the console command it
/// queues (`<command> <name>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowAction {
    /// Upload a model nobody shared yet.
    Share,
    /// Upload a newer version of one already on the Sith cloud, or over the hosts'
    /// pack that holds its name.
    Update,
    /// Fetch the cloud's version.
    Download,
}

impl RowAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Share => "Share",
            Self::Update => "Update",
            Self::Download => "Download",
        }
    }

    /// The console command that does it.
    pub(crate) fn command(self) -> &'static str {
        match self {
            Self::Share | Self::Update => SHARE,
            Self::Download => GET,
        }
    }
}

impl PageRow {
    /// The row's buttons: Share or Update for a model the game has, when the player may
    /// share (`can_share`, [`can_share`]); Download when the cloud has a version this
    /// game did not fetch (none fetched, or an older one), installed from elsewhere or not.
    pub(crate) fn actions(&self, can_share: bool) -> [Option<RowAction>; 2] {
        let upload =
            (can_share && self.installed).then_some(if self.cloud.is_some() || self.host_pack {
                RowAction::Update
            } else {
                RowAction::Share
            });
        // Whenever the cloud has a version this game did not fetch, even of a model it
        // has from elsewhere: the shared one then wins over the local copy, as the
        // player chose (Sol, 11/11/2026: `rollem`, installed, could not be fetched).
        let download = self
            .cloud
            .is_some_and(|cloud| self.fetched.is_none_or(|fetched| fetched < cloud));
        [upload, download.then_some(RowAction::Download)]
    }

    /// Where the model stands, in a few words (`has_index`: the cloud's list was read).
    pub(crate) fn status(&self, has_index: bool) -> String {
        match (self.cloud, self.fetched) {
            (Some(cloud), Some(fetched)) if fetched >= cloud => format!("Downloaded v{fetched}"),
            (Some(cloud), Some(fetched)) => format!("Downloaded v{fetched}, v{cloud} on the cloud"),
            (Some(cloud), None) if self.host_pack => {
                format!("On the cloud: v{cloud}, over the hosts' pack")
            }
            (Some(cloud), None) => format!("On the cloud: v{cloud}"),
            (None, _) if self.host_pack => "In the Sith cloud's packs".to_owned(),
            (None, Some(fetched)) => format!("Downloaded v{fetched}"),
            (None, None) if has_index => "Not shared".to_owned(),
            (None, None) => "Not checked yet".to_owned(),
        }
    }
}

/// Whether the player may share models: the SJK identity on (`identity_on`, its
/// `cl_identity`) and online, verified with a JoF or Sith tick, or staff. The same test
/// as the Profile page's Share my model.
pub(crate) fn can_share(identity_on: bool) -> bool {
    identity_on
        && crate::player_identity::with_snapshot(|snapshot| {
            snapshot.status == sjk_identity::Status::Online
                && snapshot.me.as_ref().is_some_and(|me| {
                    me.verified
                        && (me.staff
                            || matches!(
                                me.tick,
                                sjk_identity::Tick::Jof | sjk_identity::Tick::Sith
                            ))
                })
        })
        .unwrap_or(false)
}

/// A number that changes whenever what the page shows may have (a line said, a job
/// sent or answered).
pub(crate) fn changes() -> u64 {
    lock().changes
}

/// Ask the Sith cloud for its list again (the page asks when it opens).
pub(crate) fn refresh_index(console: &ViewerConsole, game_data: &Path) {
    lock().send(Job::Index, Place::of(console, game_data));
}

/// Whether the index has been read once.
pub(crate) fn has_index() -> bool {
    lock().index.is_some()
}

/// Whether a job or an upload's ticket is still on its way.
pub(crate) fn busy() -> bool {
    let state = lock();
    state.pending > 0 || state.waiting.is_some()
}

/// The models downloaded since the last call, to show on the map at once.
pub(crate) fn take_downloaded() -> BTreeSet<String> {
    std::mem::take(&mut lock().downloaded)
}

/// Give back models [`take_downloaded`] gave that could not be shown yet.
pub(crate) fn keep_downloaded(models: BTreeSet<String>) {
    lock().downloaded.extend(models);
}

/// The last lines model sync said, newest last (colour codes kept).
pub(crate) fn recent_lines() -> Vec<String> {
    lock().recent.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_downloads_are_off_until_the_player_turns_them_on() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.bool_cvar(AUTO), Some(false));
    }

    #[test]
    fn a_share_takes_the_models_folder_its_voice_and_its_shaders() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory(
            "test",
            [
                ("models/players/solfox/model.glm", b"glm".to_vec()),
                ("models/players/solfox/body.tga", b"tga".to_vec()),
                ("models/players/solfox/sub/eyes.png", b"png".to_vec()),
                ("models/players/solfox/notes.txt", b"txt".to_vec()),
                (
                    "models/players/solfox/sjk_rig.cfg",
                    b"model model.gltf".to_vec(),
                ),
                (
                    "models/players/solfox/sjk_rig_cloth.cfg",
                    b"model c.gltf".to_vec(),
                ),
                ("models/players/solfox/animation.cfg", b"x".to_vec()),
                (
                    "models/players/solfox/sounds.cfg",
                    b"SolFox
f"
                    .to_vec(),
                ),
                ("sound/chars/solfox/misc/pain25.wav", b"wav".to_vec()),
                ("models/players/solfox/model.gltf", b"{}".to_vec()),
                ("models/players/solfox/model.bin", b"bin".to_vec()),
                ("models/players/kyle/model.glm", b"glm".to_vec()),
                (
                    "shaders/community.shader",
                    b"models/players/solfox/body { }
models/players/other/x { }"
                        .to_vec(),
                ),
            ],
        )
        .unwrap();
        let gathered = gather(&vfs, "solfox");
        let paths: Vec<&str> = gathered
            .files
            .iter()
            .map(|(path, _)| path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "models/players/solfox/body.tga",
                "models/players/solfox/model.bin",
                "models/players/solfox/model.glm",
                "models/players/solfox/model.gltf",
                "models/players/solfox/sjk_rig.cfg",
                "models/players/solfox/sjk_rig_cloth.cfg",
                "models/players/solfox/sounds.cfg",
                "models/players/solfox/sub/eyes.png",
                "shaders/solfox.shader",
                "sound/chars/solfox/misc/pain25.wav",
            ]
        );
        assert_eq!(gathered.sounds, 1);
        assert_eq!(notes(&gathered), ": 1 sounds (its voice).");
    }

    #[test]
    fn modelsync_words_name_the_commands() {
        let args = |words: &[&str]| {
            words
                .iter()
                .map(|word| (*word).to_owned())
                .collect::<Vec<_>>()
        };
        let share = args(&["Share", "solfox"]);
        assert_eq!(subcommand(&share), Some((SHARE, &share[1..])));
        let get = args(&["get", "all"]);
        assert_eq!(subcommand(&get), Some((GET, &get[1..])));
        assert_eq!(
            subcommand(&args(&["check"])).map(|(name, _)| name),
            Some(CHECK)
        );
        assert_eq!(subcommand(&args(&["page"])), None);
        assert_eq!(subcommand(&[]), None);
    }

    #[test]
    fn a_rig_model_is_shared_on_the_second_press_or_with_mine() {
        lock().confirm = None;
        assert!(!confirmed("vinfox", None), "the first press only asks");
        assert!(confirmed("vinfox", None), "the second one shares");
        assert!(!confirmed("vinfox", None), "and the next asks again");
        assert!(!confirmed("solfox", None));
        assert!(
            !confirmed("vinfox", None),
            "another model's press does not count"
        );
        lock().confirm = None;
        assert!(confirmed("vinfox", Some("MINE")));
        lock().confirm = Some(("vinfox".into(), Instant::now() - CONFIRM_FOR));
        assert!(!confirmed("vinfox", None), "a confirmation goes stale");
        lock().confirm = None;
    }
}
