//! Players' models loaded off the render thread. A player who joins or changes model
//! (a `CS_PLAYERS` change, a forced-model or mute change) used to load and upload the
//! whole model inside one frame: the `.glm`, its skin and animation sounds read and
//! parsed, its textures decoded, then uploaded, which players felt as a stutter. Now a
//! worker reads and parses the model, builds its mesh and decodes its textures
//! ([`crate::world_materials::warm_entity_materials`]); the render thread only uploads
//! it, one model a frame. Meanwhile the slot keeps the model it had, or shows the
//! stand-in actor until the first one is in (`actor_world_submission.rs`).

use super::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Workers reading models at once; more requests wait their turn.
const IN_FLIGHT: usize = 3;

/// What a worker prepared for one player.
pub(crate) struct Loaded {
    pub(crate) mesh: ActorMesh,
    pub(crate) scene: FlattenedScene,
    /// The skeletons the worker parsed, kept for the next loads.
    pub(crate) cache: GlaCache,
    /// Why the requested model could not be loaded; Kyle stands in.
    pub(crate) failed: Option<String>,
}

type Prepared = Result<Loaded, String>;

/// One player's model to load.
pub(crate) struct Request {
    pub(crate) client: u16,
    pub(crate) appearance: Appearance,
    pub(crate) entity_id: EntityId,
    pub(crate) saber_names: [Option<String>; 2],
    /// The model failed in this world before: Kyle at once.
    pub(crate) known_failed: bool,
}

struct Task {
    client: u16,
    appearance: Appearance,
    receiver: Receiver<Prepared>,
}

/// The loads of one world.
#[derive(Default)]
pub(crate) struct Loads {
    running: Vec<Task>,
    waiting: Vec<Request>,
}

/// What a worker needs from the world.
pub(crate) struct Sources {
    pub(crate) vfs: Arc<VirtualFileSystem>,
    pub(crate) shaders: Arc<ShaderCatalog>,
    pub(crate) lightmap: wgpu::TextureView,
    pub(crate) cache: GlaCache,
}

impl Loads {
    /// Whether `client`'s model `appearance` is being loaded or waits.
    pub(crate) fn pending(&self, client: u16, appearance: &Appearance) -> bool {
        self.running
            .iter()
            .any(|task| task.client == client && task.appearance == *appearance)
            || self
                .waiting
                .iter()
                .any(|request| request.client == client && request.appearance == *appearance)
    }

    /// Forget `client`'s load: the player wants another model, the one they wear, or
    /// left. A worker already reading finishes and its result is dropped.
    pub(crate) fn cancel(&mut self, client: u16) {
        self.running.retain(|task| task.client != client);
        self.waiting.retain(|request| request.client != client);
    }

    /// Queue `request` in place of any earlier one for its player.
    pub(crate) fn request(&mut self, request: Request) {
        if self.pending(request.client, &request.appearance) {
            return;
        }
        self.cancel(request.client);
        self.waiting.push(request);
    }

    /// Start waiting requests while fewer than [`IN_FLIGHT`] run.
    fn start(&mut self, sources: impl Fn() -> Option<Sources>) {
        while self.running.len() < IN_FLIGHT && !self.waiting.is_empty() {
            let Some(sources) = sources() else {
                return;
            };
            let request = self.waiting.remove(0);
            let (sender, receiver) = mpsc::channel();
            self.running.push(Task {
                client: request.client,
                appearance: request.appearance.clone(),
                receiver,
            });
            let spawned = std::thread::Builder::new()
                .name("sjk-player-load".into())
                .spawn(move || {
                    let _ = sender.send(prepare(request, sources));
                });
            if let Err(error) = spawned {
                log::progress(format_args!("player model loader: {error}"));
                self.running.pop();
                return;
            }
        }
    }

    /// The first finished load, if any, with its player; failed workers are dropped.
    fn take_finished(&mut self) -> Option<(u16, Appearance, Prepared)> {
        let mut finished = None;
        self.running.retain(|task| {
            if finished.is_some() {
                return true;
            }
            match task.receiver.try_recv() {
                Ok(prepared) => {
                    finished = Some((task.client, task.appearance.clone(), prepared));
                    false
                }
                Err(TryRecvError::Empty) => true,
                Err(TryRecvError::Disconnected) => {
                    finished = Some((
                        task.client,
                        task.appearance.clone(),
                        Err("player model loader stopped".to_owned()),
                    ));
                    false
                }
            }
        });
        finished
    }
}

/// The worker: the model (Kyle when it cannot be loaded), its mesh and its textures.
fn prepare(request: Request, sources: Sources) -> Prepared {
    let Sources {
        vfs,
        shaders,
        lightmap,
        mut cache,
    } = sources;
    let mut load = |appearance: &Appearance| {
        load_player_appearance_with(
            &vfs,
            &appearance.model,
            &appearance.variant,
            [0.0; 3],
            0.0,
            &mut cache,
        )
        .map_err(|error| error.to_string())
    };
    let kyle = crate::actor_load::fallback_appearance();
    let (preview, failed) = if request.known_failed {
        (load(&kyle)?, None)
    } else {
        match load(&request.appearance) {
            Ok(preview) => (preview, None),
            Err(error) => (load(&kyle)?, Some(error)),
        }
    };
    let mut scene = FlattenedScene::default();
    let mesh = build_actor_mesh(
        &mut scene,
        preview,
        Some(request.entity_id),
        false,
        request.appearance,
        request.saber_names,
    )
    .map_err(|error| error.to_string())?;
    crate::world_materials::warm_entity_materials(&vfs, &shaders, &lightmap, &scene.materials);
    Ok(Loaded {
        mesh,
        scene,
        cache,
        failed,
    })
}

impl GpuState {
    /// What the workers read from this world.
    fn player_load_sources(&self) -> Option<Sources> {
        Some(Sources {
            vfs: self.vfs.clone()?,
            shaders: Arc::clone(&self.shaders),
            lightmap: self.world_materials.entity_lightmap(),
            cache: self.clientinfo_watch.gla_cache.clone(),
        })
    }

    /// Once a frame: start waiting loads and put one finished model on its player.
    pub(crate) fn poll_player_loads(&mut self) {
        let mut loads = std::mem::take(&mut self.clientinfo_watch.loads);
        loads.start(|| self.player_load_sources());
        let finished = loads.take_finished();
        self.clientinfo_watch.loads = loads;
        let Some((client, appearance, prepared)) = finished else {
            return;
        };
        let loaded = match prepared {
            Ok(loaded) => loaded,
            Err(error) => {
                log::progress(format_args!(
                    "client {client}: could not load {}/{}: {error}",
                    appearance.model, appearance.variant
                ));
                self.clientinfo_watch.failed.insert(appearance);
                return;
            }
        };
        self.clientinfo_watch.gla_cache.absorb(loaded.cache);
        if let Some(error) = &loaded.failed {
            log::progress(format_args!(
                "could not load {}/{}: {error}; using Kyle",
                appearance.model, appearance.variant
            ));
            self.clientinfo_watch.failed.insert(appearance.clone());
        }
        let mesh = match self.upload_built_actor(loaded.mesh, loaded.scene, &appearance) {
            Ok(mesh) => mesh,
            Err(error) => {
                log::progress(format_args!(
                    "client {client}: model upload failed: {error}"
                ));
                return;
            }
        };
        self.place_client_mesh(client, mesh, &appearance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance(model: &str) -> Appearance {
        Appearance {
            model: model.to_owned(),
            variant: "default".to_owned(),
        }
    }

    fn request(client: u16, model: &str) -> Request {
        Request {
            client,
            appearance: appearance(model),
            entity_id: EntityId::new(u64::from(client) + 1),
            saber_names: [None, None],
            known_failed: false,
        }
    }

    #[test]
    fn a_new_request_replaces_the_players_earlier_one() {
        let mut loads = Loads::default();
        loads.request(request(3, "models/players/a"));
        loads.request(request(4, "models/players/b"));
        assert!(loads.pending(3, &appearance("models/players/a")));
        loads.request(request(3, "models/players/c"));
        assert!(!loads.pending(3, &appearance("models/players/a")));
        assert!(loads.pending(3, &appearance("models/players/c")));
        assert!(loads.pending(4, &appearance("models/players/b")));
        // The same request again changes nothing.
        loads.request(request(3, "models/players/c"));
        assert_eq!(loads.waiting.len(), 2);
        loads.cancel(4);
        assert!(!loads.pending(4, &appearance("models/players/b")));
    }

    #[test]
    fn finished_loads_come_out_one_at_a_time_and_cancelled_ones_never() {
        let mut loads = Loads::default();
        let mut senders = Vec::new();
        for client in 0..3_u16 {
            let (sender, receiver) = mpsc::channel();
            senders.push(sender);
            loads.running.push(Task {
                client,
                appearance: appearance("models/players/kyle"),
                receiver,
            });
        }
        assert!(loads.take_finished().is_none());
        loads.cancel(1);
        for sender in &senders {
            let _ = sender.send(Err("test".to_owned()));
        }
        assert_eq!(loads.take_finished().map(|(client, ..)| client), Some(0));
        assert_eq!(loads.take_finished().map(|(client, ..)| client), Some(2));
        assert!(loads.take_finished().is_none());
        // A worker that stopped without an answer is reported, not waited for.
        let (sender, receiver) = mpsc::channel::<Prepared>();
        drop(sender);
        loads.running.push(Task {
            client: 5,
            appearance: appearance("models/players/kyle"),
            receiver,
        });
        assert!(matches!(loads.take_finished(), Some((5, _, Err(_)))));
    }

    #[test]
    fn no_more_than_the_limit_start_without_sources() {
        let mut loads = Loads::default();
        for client in 0..5 {
            loads.request(request(client, "models/players/kyle"));
        }
        // Without a world to read from nothing starts and nothing is lost.
        loads.start(|| None);
        assert!(loads.running.is_empty());
        assert_eq!(loads.waiting.len(), 5);
    }
}
