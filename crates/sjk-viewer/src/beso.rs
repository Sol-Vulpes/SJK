//! `toy_beso`: spend owned credits on ten collectible packets, via the identity hub.
//! Legacy game servers do not attest positions; the donor supplies the nearby roster.
use crate::GpuState;
use glam::Vec3;
use serde_json::{Value, json};
use sjk_ui::Color;
use std::time::{Duration, Instant};

/// Opt in to seeing and automatically collecting other players' rain.
pub(crate) const SHOW_CVAR: &str = "cg_toyBesoShow";

const OFFSETS: [[i32; 2]; 10] = [
    [-72, -48],
    [-24, -72],
    [24, -72],
    [72, -48],
    [88, 0],
    [72, 48],
    [24, 72],
    [-24, 72],
    [-72, 48],
    [-88, 0],
];
const NEARBY: f32 = 256.0;
const COLLECT: f32 = 56.0;

struct Burst {
    id: i64,
    origin: Vec3,
    age: f32,
    seen: Instant,
    taken: [bool; 10],
    eligible: bool,
    owned: bool,
}

impl Burst {
    fn point(&self, index: usize) -> Vec3 {
        let [x, y] = OFFSETS[index];
        self.origin + Vec3::new(x as f32, y as f32, 0.0)
    }
    fn elapsed(&self, now: Instant) -> f32 {
        self.age + now.saturating_duration_since(self.seen).as_secs_f32()
    }
}

/// Retained local rain state; its frame drawing allocates nothing.
#[derive(Default)]
pub(crate) struct Beso {
    #[cfg(test)]
    forced: bool,
    bursts: Vec<Burst>,
    show_others: bool,
    place: Option<(String, String)>,
    due: Option<Instant>,
    pending: Option<Value>,
    funding: Option<Value>,
}

impl Beso {
    fn wants_poll(&self) -> bool {
        self.show_others
            || self.pending.is_some()
            || self.funding.is_some()
            || self.bursts.iter().any(|b| b.owned)
    }
    fn paid_pending(&self) -> bool {
        self.pending.as_ref().is_some_and(|p| p["action"] != "read")
    }
}

fn send(request: Value) -> bool {
    crate::player_identity::with_service(|s| s.beso(request)).is_some()
}

impl GpuState {
    /// Fund one burst. Every invocation spends once; `retry` repeats the prior token.
    pub(crate) fn beso_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        if args.first().is_some_and(|a| a == "forget") {
            if self.beso.paid_pending() {
                return Err("Wait for the funding request to finish".into());
            }
            self.beso.funding = None;
            return Ok(vec!["Forgot the uncertain retry token. Any existing rain and refunds remain at the hub.".into()]);
        }
        if args.is_empty() && self.beso.funding.is_some() {
            return self.beso_command(&["retry".into()]);
        }
        if args.first().is_some_and(|a| a == "retry") {
            if args.len() != 1 {
                return Err("toy_beso retry".into());
            }
            let request = self
                .beso
                .funding
                .clone()
                .ok_or("No uncertain credit rain to retry")?;
            if self.beso.paid_pending() {
                return Err("Wait for the current hub request".into());
            }
            if !send(request.clone()) {
                return Err("Enable SJK identity first".into());
            }
            self.beso.pending = Some(request);
            self.beso.due = None;
            return Ok(vec![
                "Retrying the same funded rain; this cannot spend twice.".into(),
            ]);
        }
        if args.len() > 1 {
            return Err("toy_beso [100..100000 credits, multiple of 10] | retry".into());
        }
        let amount = args
            .first()
            .map_or(Ok(100), |s| s.parse::<i64>())
            .map_err(|_| "Choose a whole credit amount")?;
        if !(100..=100_000).contains(&amount) || amount % 10 != 0 {
            return Err("Rain 100..100000 credits, in multiples of ten".into());
        }
        if self.beso.paid_pending() || self.beso.funding.is_some() {
            return Err("Wait for the hub, or use toy_beso retry after a failed request".into());
        }
        let session = self
            .live_session
            .as_ref()
            .filter(|s| !s.is_local())
            .ok_or("Join a game server first")?;
        if session.latest_snapshot().player.health() <= 0 {
            return Err("Be alive to make it rain".into());
        }
        let origin = Vec3::from_array(session.latest_snapshot().player.origin());
        let snapshot = session.latest_snapshot();
        let eligible = crate::player_identity::with_snapshot(|identity| {
            identity
                .players
                .iter()
                .filter_map(|p| {
                    let entity = snapshot.entities.iter().find(|e| {
                        e.number() == u16::from(p.slot)
                            && e.entity_type() == 1
                            && e.e_flags() & ((1 << 1) | (1 << 8)) == 0
                    })?;
                    let live = self
                        .live_world
                        .entity(sjk_runtime::EntityId::new(u64::from(entity.number()) + 1))?;
                    let point =
                        Vec3::from_array(live.sample(i64::from(snapshot.server_time)).translation);
                    let shown = crate::player_identity::shown_name(
                        session.game_state(),
                        usize::from(p.slot),
                    )?;
                    (point.distance_squared(origin) <= NEARBY * NEARBY
                        && sjk_identity::names_match(&p.claimed_name, &shown))
                    .then(|| p.key_id.clone())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        if eligible.is_empty() {
            return Err("No nearby SJK players to collect your credits".into());
        }
        let token = sjk_identity::beso_request_token().map_err(|e| e.to_string())?;
        let request = json!({"action":"rain","server":session.server().to_string(),"map":crate::remap_blocked_maps::map_name(&self.world_load_map),"token":token,"amount":amount,"origin":origin.to_array(),"eligible":eligible});
        if !send(request.clone()) {
            return Err("Enable SJK identity first".into());
        }
        self.beso.funding = Some(request.clone());
        self.beso.pending = Some(request);
        self.beso.due = None;
        Ok(vec![format!(
            "Funding ten packets with {amount} of your credits..."
        )])
    }

    /// Read the hub at one-second intervals; collect only a packet actually in reach.
    pub(crate) fn update_beso(&mut self, events: &[sjk_identity::ToyEvent]) {
        let now = Instant::now();
        if events.iter().any(|e| e.toy == "beso" && e.event == "rain") {
            self.beso.due = None;
        }
        if self.beso.due.is_some_and(|due| now < due) {
            return;
        }
        self.beso.due = Some(now + Duration::from_millis(500));
        self.beso.show_others = self
            .console
            .as_ref()
            .is_some_and(|c| c.bool_cvar(SHOW_CVAR) == Some(true));
        if !self.beso.wants_poll() {
            self.beso.due = Some(now + Duration::from_millis(500));
            return;
        }
        let Some(session) = self.live_session.as_ref().filter(|s| !s.is_local()) else {
            self.beso.bursts.clear();
            self.beso.place = None;
            return;
        };
        let server = session.server().to_string();
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let origin = Vec3::from_array(session.latest_snapshot().player.origin());
        let alive = session.latest_snapshot().player.health() > 0;
        if self.beso.place.as_ref() != Some(&(server.clone(), map.clone())) {
            self.beso.bursts.clear();
            self.beso.place = Some((server.clone(), map.clone()));
        }
        if let Some(pending) = &self.beso.pending {
            let outcome = crate::player_identity::with_snapshot(|s| s.beso.clone()).flatten();
            let Some((token, result)) =
                outcome.filter(|(token, _)| pending["token"].as_str() == Some(token.as_str()))
            else {
                return;
            };
            let was_funding = pending["action"] == "rain";
            let read_failed = pending["action"] == "read" && result.is_err();
            let same_place = pending["server"] == server && pending["map"] == map;
            match result {
                Ok(answer) => {
                    if was_funding {
                        self.beso.funding = None;
                    }
                    if same_place {
                        self.beso.bursts.clear();
                        let own = crate::player_identity::with_snapshot(|s| s.key_id.clone())
                            .unwrap_or_default();
                        if let Some(bursts) = answer["bursts"].as_array() {
                            for row in bursts.iter().take(4) {
                                let Some(point) = row["origin"].as_array().filter(|p| p.len() == 3)
                                else {
                                    continue;
                                };
                                let point = Vec3::new(
                                    point[0].as_f64().unwrap_or(0.0) as f32,
                                    point[1].as_f64().unwrap_or(0.0) as f32,
                                    point[2].as_f64().unwrap_or(0.0) as f32,
                                );
                                if !point.is_finite() {
                                    continue;
                                }
                                let mut taken = [false; 10];
                                if let Some(list) = row["taken"].as_array() {
                                    for n in list {
                                        if let Some(i) = n.as_u64().filter(|i| *i < 10) {
                                            taken[i as usize] = true;
                                        }
                                    }
                                }
                                self.beso.bursts.push(Burst {
                                    id: row["id"].as_i64().unwrap_or(0),
                                    origin: point,
                                    age: (answer["server_time"].as_i64().unwrap_or(0)
                                        - row["at"].as_i64().unwrap_or(0))
                                    .max(0) as f32,
                                    seen: now,
                                    taken,
                                    eligible: row["eligible"] == true,
                                    owned: row["owner"].as_str() == Some(own.as_str()),
                                });
                            }
                        }
                    }
                    let won = answer["won"].as_i64().unwrap_or(0);
                    if won > 0 || was_funding {
                        let words = if was_funding {
                            "Credit rain funded! Uncollected credits return after it ends.".into()
                        } else {
                            format!("You collected {won} credits. One packet per person per rain.")
                        };
                        self.chat.receive(
                            sjk_client::ServerEventKind::CenterPrint,
                            words,
                            None,
                            now,
                        );
                    }
                }
                Err(error) => {
                    // Read errors are quiet; paid operations are reported exactly once.
                    if pending["action"] != "read" {
                        self.chat.receive(
                            sjk_client::ServerEventKind::CenterPrint,
                            format!(
                                "Credit rain: {error}{}",
                                if was_funding {
                                    " Use toy_beso retry."
                                } else {
                                    ""
                                }
                            ),
                            None,
                            now,
                        );
                    }
                }
            }
            let _ = token;
            self.beso.pending = None;
            self.beso.due = Some(
                now + Duration::from_secs(if read_failed {
                    60
                } else if self.beso.bursts.is_empty() {
                    10
                } else {
                    1
                }),
            );
            return;
        }
        let Ok(token) = sjk_identity::beso_request_token() else {
            return;
        };
        let mut request = json!({"action":"read","server":server,"map":map,"token":token});
        if alive
            && self.beso.show_others
            && let Some((id, index)) = self
                .beso
                .bursts
                .iter()
                .filter(|b| b.eligible && (2.0..30.0).contains(&b.elapsed(now)))
                .find_map(|b| {
                    (0..10)
                        .find(|i| {
                            !b.taken[*i] && b.point(*i).distance_squared(origin) < COLLECT * COLLECT
                        })
                        .map(|i| (b.id, i))
                })
        {
            request["action"] = json!("collect");
            request["id"] = json!(id);
            request["packet"] = json!(index);
            request["origin"] = json!(origin.to_array());
        }
        if send(request.clone()) {
            self.beso.pending = Some(request);
        }
    }

    /// Draw retained packets into the existing world-marker list; no new frame storage.
    pub(crate) fn append_beso_marks(&mut self, shown: bool) {
        #[cfg(test)]
        let shown = shown || self.beso.forced;
        let Some(camera) = self.frame_view.filter(|_| shown) else {
            return;
        };
        let now = Instant::now();
        for burst in &self.beso.bursts {
            if !burst.owned && !self.beso.show_others {
                continue;
            }
            let elapsed = burst.elapsed(now);
            if elapsed >= 30.0 {
                continue;
            }
            for index in 0..10 {
                if burst.taken[index] {
                    continue;
                }
                let age = (elapsed - index as f32 * 0.09).max(0.0);
                let point = burst.point(index) + Vec3::Z * (180.0 - 80.0 * age * age).max(0.0);
                let Some([x, y]) = camera.project(point) else {
                    continue;
                };
                let size = (1500.0 / camera.eye.distance(point).max(40.0)).clamp(8.0, 24.0)
                    * crate::ui_scale::height_scale(camera.viewport[1]);
                let _ = crate::cards::mark::draw(
                    &mut self.world_pings.list,
                    [x - size * 0.5, y - size * 0.5],
                    size,
                    Color::new(1.0, 0.78, 0.22, ((30.0 - elapsed) / 2.0).min(1.0)),
                );
            }
        }
    }
}

#[cfg(test)]
impl GpuState {
    pub(crate) fn preview_beso_rain(&mut self) {
        let camera = self.frame_view.unwrap();
        let origin =
            camera.eye + (camera.target - camera.eye).normalize_or_zero() * 250.0 - Vec3::Z * 70.0;
        self.beso.bursts = vec![Burst {
            id: 1,
            origin,
            age: 0.8,
            seen: Instant::now(),
            taken: [false; 10],
            eligible: false,
            owned: true,
        }];
        self.beso.forced = true;
        self.world_pings.forced = true;
        self.beso.due = Some(Instant::now() + Duration::from_secs(3600));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_opted_out_idle_client_does_not_poll_a_staff_only_feature() {
        let mut state = Beso::default();
        assert!(!state.wants_poll());
        state.show_others = true;
        assert!(state.wants_poll());
        state.show_others = false;
        state.funding = Some(json!({"action":"rain"}));
        assert!(state.wants_poll());
        state.funding = None;
        state.pending = Some(json!({"action":"read"}));
        assert!(state.wants_poll());
    }
    #[test]
    fn credit_rain_collection_requires_an_explicit_opt_in() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.bool_cvar(SHOW_CVAR), Some(false));
        console.set_cvar(SHOW_CVAR, "1");
        assert_eq!(console.bool_cvar(SHOW_CVAR), Some(true));
    }
    #[test]
    fn idle_reads_do_not_block_a_paid_request_but_paid_operations_do() {
        let mut beso = Beso::default();
        beso.pending = Some(json!({"action":"read"}));
        assert!(!beso.paid_pending());
        beso.pending = Some(json!({"action":"rain"}));
        assert!(beso.paid_pending());
        beso.pending = Some(json!({"action":"collect"}));
        assert!(beso.paid_pending());
    }
    #[test]
    fn packet_positions_match_the_hubs_integer_offsets() {
        let b = Burst {
            id: 1,
            origin: Vec3::new(10.0, 20.0, 24.0),
            age: 0.0,
            seen: Instant::now(),
            taken: [false; 10],
            eligible: true,
            owned: false,
        };
        assert_eq!(b.point(0), Vec3::new(-62.0, -28.0, 24.0));
        assert_eq!(b.point(9), Vec3::new(-78.0, 20.0, 24.0));
        for i in 0..10 {
            assert!(b.point(i).distance(b.origin) <= 100.0);
            for j in i + 1..10 {
                assert_ne!(b.point(i), b.point(j));
            }
        }
    }
}
