use super::*;
use frame::{Request, parse};
use sjk_client::pmove::{MovementCollision, MovementConfig, MovementTrace, Predictor};
use sjk_client::{AnimationLengths, AnimationTiming};

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn the_command_takes_a_player_a_delay_and_a_side() {
    assert_eq!(parse(&[]), Ok(Request::Status));
    assert_eq!(parse(&words("off")), Ok(Request::Off));
    assert_eq!(
        parse(&words("Sol 0.5")),
        Ok(Request::Follow {
            player: "Sol".into(),
            delay: 500,
            mirror: true,
            walk: false
        })
    );
    assert_eq!(
        parse(&words("3 copy 2s")),
        Ok(Request::Follow {
            player: "3".into(),
            delay: 2_000,
            mirror: false,
            walk: false
        })
    );
    assert_eq!(
        parse(&words("Sol walk 0.3")),
        Ok(Request::Follow {
            player: "Sol".into(),
            delay: 300,
            mirror: true,
            walk: true
        })
    );
    assert!(parse(&words("Sol 11")).is_err());
    assert!(parse(&words("Sol -1")).is_err());
    assert!(parse(&words("Sol sideways")).is_err());
}

#[test]
fn facing_each_other_a_step_toward_is_a_step_toward_and_the_mirror_keeps_the_side() {
    // The leader faces +x (yaw 0); the mimic faces them (yaw 180).
    let toward = to_frame(from_frame([10.0, 0.0, 0.0], 0.0, -1.0), 180.0);
    assert!((toward[0] + 10.0).abs() < 1e-4 && toward[1].abs() < 1e-4);
    // The leader steps to their left (+y): in a mirror the mimic steps to its right,
    // which is +y too, so they stay face to face.
    let mirrored = to_frame(from_frame([0.0, 10.0, 0.0], 0.0, -1.0), 180.0);
    assert!(mirrored[0].abs() < 1e-4 && (mirrored[1] - 10.0).abs() < 1e-4);
    // Copied, the mimic steps to its own left, -y.
    let copied = to_frame(from_frame([0.0, 10.0, 0.0], 0.0, 1.0), 180.0);
    assert!((copied[1] + 10.0).abs() < 1e-4);
}

#[test]
fn samples_interpolate_and_extrapolate_a_little() {
    let mut samples = VecDeque::new();
    samples.push_back(Sample {
        time: 100,
        origin: [0.0; 3],
        velocity: [100.0, 0.0, 0.0],
        angles: [0.0, 350.0, 0.0],
        saber_move: 4,
        ..Sample::default()
    });
    samples.push_back(Sample {
        time: 150,
        origin: [5.0, 0.0, 0.0],
        velocity: [100.0, 0.0, 0.0],
        angles: [0.0, 10.0, 0.0],
        saber_move: 8,
        ..Sample::default()
    });
    assert!(sample_at(&samples, 99).is_none());
    let middle = sample_at(&samples, 125).unwrap();
    assert!((middle.origin[0] - 2.5).abs() < 1e-4);
    assert!(middle.angles[1].abs() < 1e-3, "through 0, not round 180");
    assert_eq!(
        middle.saber_move, 4,
        "discrete fields from the earlier sample"
    );
    let late = sample_at(&samples, 1_000).unwrap();
    assert!(
        (late.origin[0] - 15.0).abs() < 1e-3,
        "100 ms at most past the newest"
    );
    assert_eq!(late.saber_move, 8);
}

#[test]
fn a_held_key_stops_the_copy() {
    let mut mimic = Mimic::default();
    mimic.start(1, "Leader".into(), 0, true, false);
    mimic.push(Sample {
        time: 10,
        on_ground: true,
        ..Sample::default()
    });
    let state = MovementState::default();
    assert!(mimic.drive(&UserCommand::default(), &state).is_some());
    let pressed = UserCommand {
        forward_move: 127,
        ..UserCommand::default()
    };
    assert!(mimic.drive(&pressed, &state).is_none());
    assert!(!mimic.active());
    assert!(mimic.take_messages()[0].contains("took control"));
}

/// A flat floor at z 0.
struct Floor;

impl MovementCollision for Floor {
    fn trace(
        &self,
        start: [f32; 3],
        mins: [f32; 3],
        _: [f32; 3],
        end: [f32; 3],
        _: u32,
    ) -> MovementTrace {
        if end[2] + mins[2] < 0.0 && start[2] + mins[2] >= 0.0 {
            let fraction = ((start[2] + mins[2]) / (start[2] - end[2])).clamp(0.0, 1.0);
            MovementTrace {
                fraction,
                end_position: std::array::from_fn(|i| start[i] + (end[i] - start[i]) * fraction),
                plane_normal: [0.0, 0.0, 1.0],
                entity_number: 1022,
                ..MovementTrace::miss(end)
            }
        } else {
            MovementTrace::miss(end)
        }
    }
}

/// Every animation ten frames of 40 ms: saber moves need lengths to be predicted.
#[derive(Debug)]
struct Lengths;

impl AnimationLengths for Lengths {
    fn length_ms(&self, _: u16) -> Option<i32> {
        Some(400)
    }
    fn timing(&self, _: u16) -> Option<AnimationTiming> {
        Some(AnimationTiming {
            frame_count: 10,
            frame_lerp_ms: 40,
        })
    }
}

/// A saber wielder in the medium stance standing at `origin` facing `yaw`, blade lit.
fn duelist(origin: [f32; 3], yaw: f32) -> Predictor {
    let mut player = PlayerState::default();
    player.stats[0] = 100;
    player.set_origin(origin);
    player.set_view_angles([0.0, yaw, 0.0]);
    // The delta keeps the view where it is under a zero command angle.
    player.set_delta_angles([0, angle_short(yaw), 0]);
    player.set_gravity(800);
    player.set_speed(250.0);
    player.set_base_speed(250);
    player.set_ground_entity_num(1022);
    player.set_raw_field(47, u32::from(WP_SABER));
    player.set_raw_field(23, 2); // SS_MEDIUM
    player.set_raw_field(81, 0); // saberHolstered: lit
    let mut predictor = Predictor::from_player_state(&player, MovementConfig::default());
    predictor.set_animation_lengths(std::sync::Arc::new(Lengths));
    predictor
}

/// The leader's script: still, walking forward while turning left 30 degrees,
/// strafing left, a jump, a crouched walk, then two chained attacks (top left, from
/// the right) and an overhead after a pause, and still again.
fn leader_command(time: i32) -> UserCommand {
    let yaw = match time {
        0..400 => 0.0,
        400..900 => 30.0 * (time - 400) as f32 / 500.0,
        _ => 30.0,
    };
    let (forward, right, up, buttons) = match time {
        400..900 => (127, 0, 0, 0),
        900..1300 => (0, -127, 0, 0),
        1300..1450 => (127, 0, 127, 0),
        1450..1900 => (127, 0, 0, 0),
        2000..2400 => (127, 0, -127, 0),
        2600..3000 => (127, 127, 0, saber::BUTTON_ATTACK),
        3000..3800 => (0, -127, 0, saber::BUTTON_ATTACK),
        // The medium stance ends a chain after two attacks (`PM_SaberKataDone`); with
        // every animation 400 ms it is back at the ready by 4.6 s, and a new press
        // starts the third.
        4700..5100 => (127, 0, 0, saber::BUTTON_ATTACK),
        _ => (0, 0, 0, 0),
    };
    UserCommand {
        server_time: time,
        angles: [0, angle_short(yaw), 0],
        forward_move: forward,
        right_move: right,
        up_move: up,
        buttons,
        weapon: WP_SABER,
        ..UserCommand::default()
    }
}

/// The walking choreography: the same moves as [`leader_command`] at walking pace (the
/// walk key's 46, `BUTTON_WALKING` held), without the jump, attacks included.
fn walking_leader_command(time: i32) -> UserCommand {
    let mut command = leader_command(time);
    let walk = |key: i8| (i32::from(key) * 46 / 127) as i8;
    command.forward_move = walk(command.forward_move);
    command.right_move = walk(command.right_move);
    if command.up_move > 0 {
        command.up_move = 0;
    }
    command.buttons |= BUTTON_WALKING;
    command
}

fn sample_of(time: i32, state: &MovementState) -> Sample {
    Sample {
        time,
        origin: state.origin,
        velocity: state.velocity,
        angles: state.view_angles,
        on_ground: state.ground_entity_number != NO_GROUND,
        crouched: state.movement_flags & 1 != 0, // PMF_DUCKED
        saber_move: state.saber_move,
        style: state.saber_anim_level,
        holstered: state.saber_holstered == ALL_BLADES_OFF,
        weapon: state.weapon,
    }
}

/// The distinct directional attacks (`LS_A_TL2BR..LS_A_T2B`) in order.
fn attacks(moves: &[u32]) -> Vec<u32> {
    let mut seen: Vec<u32> = Vec::new();
    let mut last = 0;
    for &saber_move in moves {
        if saber_move != last && (4..=10).contains(&saber_move) {
            seen.push(saber_move);
        }
        last = saber_move;
    }
    seen
}

struct Showcase {
    leader_moves: Vec<u32>,
    mimic_moves: Vec<u32>,
    leader: MovementState,
    mimic: MovementState,
    mimic_jumped: bool,
    mimic_crouched: bool,
    messages: Vec<String>,
    /// The largest distance from where the copied leader stood, during the run.
    worst_tracking: f32,
    /// How much later than intended each attack began.
    attack_lag: Vec<i32>,
    /// Commands the mimic sent with the walk held but a key past half (Pmove would
    /// clear the walk), and ground commands without the walk when walking always.
    walk_broken: usize,
    walk_missing: usize,
}

/// Run the leader's script and a mimic facing them 128 units away, copying after
/// `delay` ms, with snapshots every 25 ms that reach the mimic 60 ms late.
fn showcase(delay: i32, mirror: bool) -> Showcase {
    showcase_with(delay, mirror, false, leader_command)
}

/// [`showcase`] with the mimic walking always or not, and the leader's script.
fn showcase_with(delay: i32, mirror: bool, walk: bool, script: fn(i32) -> UserCommand) -> Showcase {
    let mut leader = duelist([0.0, 0.0, 24.0], 0.0);
    // The mimic faces the leader.
    let mut follower = duelist([128.0, 0.0, 24.0], 180.0);
    let mut mimic = Mimic::default();
    mimic.start(1, "Leader".into(), delay, mirror, walk);
    let mut in_flight: VecDeque<Sample> = VecDeque::new();
    let mut run = Showcase {
        leader_moves: Vec::new(),
        mimic_moves: Vec::new(),
        leader: MovementState::default(),
        mimic: MovementState::default(),
        mimic_jumped: false,
        mimic_crouched: false,
        messages: Vec::new(),
        worst_tracking: 0.0,
        attack_lag: Vec::new(),
        walk_broken: 0,
        walk_missing: 0,
    };
    let mut leader_path: Vec<[f32; 3]> = Vec::new();
    let side = if mirror { -1.0 } else { 1.0 };
    for stamp in (8..=6_000 + delay).step_by(8) {
        if stamp <= 6_000 {
            leader.predict_command(script(stamp), &Floor);
            run.leader_moves.push(leader.state().saber_move);
            leader_path.push(leader.state().origin);
            if stamp % 24 == 0 {
                in_flight.push_back(sample_of(stamp, leader.state()));
            }
        }
        while in_flight
            .front()
            .is_some_and(|sample| sample.time + 60 <= stamp)
        {
            mimic.push(in_flight.pop_front().unwrap());
        }
        let input = UserCommand {
            server_time: stamp,
            weapon: WP_SABER,
            ..UserCommand::default()
        };
        let drive = mimic
            .drive(&input, follower.state())
            .expect("still copying");
        let sent = drive.command;
        if sent.buttons & BUTTON_WALKING != 0
            && (sent.forward_move.unsigned_abs() > 64 || sent.right_move.unsigned_abs() > 64)
        {
            run.walk_broken += 1;
        }
        if walk
            && follower.state().ground_entity_number != NO_GROUND
            && sent.buttons & BUTTON_WALKING == 0
        {
            run.walk_missing += 1;
        }
        follower.predict_command(drive.command, &Floor);
        let state = follower.state();
        run.mimic_moves.push(state.saber_move);
        // The copy runs 60 ms (the first sample's latency) plus the delay behind.
        let copied = stamp - 60 - delay;
        if copied >= 8 && (copied as usize / 8) <= leader_path.len() {
            let at = leader_path[copied as usize / 8 - 1];
            let moved = from_frame(sub(at, [0.0, 0.0, 24.0]), 0.0, side);
            let expected = add([128.0, 0.0, 24.0], to_frame(moved, 180.0));
            let off = (expected[0] - state.origin[0]).hypot(expected[1] - state.origin[1]);
            run.worst_tracking = run.worst_tracking.max(off);
        }
        run.mimic_jumped |= state.origin[2] > 40.0;
        run.mimic_crouched |= state.movement_flags & 1 != 0;
    }
    // When each directional attack began, leader and mimic, in commands of 8 ms.
    let starts = |moves: &[u32]| -> Vec<i32> {
        let mut starts = Vec::new();
        for (index, pair) in moves.windows(2).enumerate() {
            if pair[0] != pair[1] && (4..=10).contains(&pair[1]) {
                starts.push((index as i32 + 2) * 8);
            }
        }
        starts
    };
    run.attack_lag = starts(&run.leader_moves)
        .iter()
        .zip(starts(&run.mimic_moves))
        .map(|(leader, mimic)| mimic - leader - 60 - delay)
        .collect();
    run.leader = leader.state().clone();
    run.mimic = follower.state().clone();
    run.messages = mimic.take_messages();
    run
}

#[test]
fn the_mirror_copies_steps_turns_jumps_crouches_and_a_saber_chain() {
    for delay in [0, 300, 1_000] {
        let run = showcase(delay, true);
        assert!(run.messages.is_empty(), "{:?}", run.messages);
        let leader = attacks(&run.leader_moves);
        assert_eq!(leader, [4, 8, 10], "the script's attacks: TL2BR, R2L, T2B");
        // Mirrored: top right, from the left, overhead.
        assert_eq!(attacks(&run.mimic_moves), [9, 5, 10], "{delay} ms");
        assert!(
            run.mimic_jumped && run.mimic_crouched,
            "{delay} ms: jumped {} crouched {}",
            run.mimic_jumped,
            run.mimic_crouched
        );
        // Where the leader ended, in their frame, mirrored into the mimic's.
        let moved = from_frame(sub(run.leader.origin, [0.0, 0.0, 24.0]), 0.0, -1.0);
        let expected = add([128.0, 0.0, 24.0], to_frame(moved, 180.0));
        let off = (expected[0] - run.mimic.origin[0]).hypot(expected[1] - run.mimic.origin[1]);
        assert!(off < 3.0, "{delay} ms: {off} units from {expected:?}");
        // Throughout the run (measured: 31 units with no delay, where nothing ahead is
        // known and a full-speed leader cannot be caught up; 10 with a delay to read
        // the leader's turns ahead).
        let worst = if delay == 0 { 40.0 } else { 15.0 };
        assert!(
            run.worst_tracking < worst,
            "{delay} ms: {} units off during the run",
            run.worst_tracking
        );
        // Each attack starts within a few commands of when it should (measured 4-24).
        assert!(
            run.attack_lag.iter().all(|lag| (-16..=40).contains(lag)),
            "{delay} ms: attacks off by {:?} ms",
            run.attack_lag
        );
        // The leader turned 30 degrees left; the mirror turns 30 right.
        let yaw = angle_delta(run.mimic.view_angles[1], 150.0);
        assert!(yaw.abs() < 1.0, "{delay} ms: yaw off by {yaw}");
    }
}

#[test]
fn the_copy_keeps_the_leaders_sides() {
    let run = showcase(300, false);
    assert_eq!(attacks(&run.mimic_moves), [4, 8, 10]);
    let moved = from_frame(sub(run.leader.origin, [0.0, 0.0, 24.0]), 0.0, 1.0);
    let expected = add([128.0, 0.0, 24.0], to_frame(moved, 180.0));
    let off = (expected[0] - run.mimic.origin[0]).hypot(expected[1] - run.mimic.origin[1]);
    assert!(off < 3.0, "{off} units from {expected:?}");
    assert!(
        run.worst_tracking < 15.0,
        "{} units off",
        run.worst_tracking
    );
}

#[test]
fn walking_keeps_every_key_within_half_so_the_walk_holds() {
    // Pmove clears the walk past a key of 64: the automatic walk caps its keys too.
    let run = showcase(300, true);
    assert_eq!(run.walk_broken, 0, "commands whose walk Pmove would clear");
    assert_eq!(cap_keys(127, -127, 64), (64, -64));
    assert_eq!(cap_keys(127, 10, 64), (64, 5));
    assert_eq!(cap_keys(127, 1, 64), (64, 1), "a key does not vanish");
    assert_eq!(cap_keys(40, -30, 64), (40, -30));
}

#[test]
fn walking_always_copies_a_walking_choreography_attacks_and_all() {
    for delay in [0, 300] {
        let run = showcase_with(delay, true, true, walking_leader_command);
        assert!(run.messages.is_empty(), "{:?}", run.messages);
        assert_eq!(
            run.walk_missing, 0,
            "{delay} ms: ground commands without the walk"
        );
        assert_eq!(run.walk_broken, 0, "{delay} ms: walks Pmove would clear");
        assert_eq!(attacks(&run.leader_moves), [4, 8, 10]);
        assert_eq!(attacks(&run.mimic_moves), [9, 5, 10], "{delay} ms");
        assert!(run.mimic_crouched, "{delay} ms");
        let moved = from_frame(sub(run.leader.origin, [0.0, 0.0, 24.0]), 0.0, -1.0);
        let expected = add([128.0, 0.0, 24.0], to_frame(moved, 180.0));
        let off = (expected[0] - run.mimic.origin[0]).hypot(expected[1] - run.mimic.origin[1]);
        assert!(off < 3.0, "{delay} ms: {off} units from {expected:?}");
        // Walking pace is easier to match (measured: 8 units with no delay, 5 with).
        assert!(
            run.worst_tracking < 12.0,
            "{delay} ms: {} units off during the run",
            run.worst_tracking
        );
        assert!(
            run.attack_lag.iter().all(|lag| (-16..=40).contains(lag)),
            "{delay} ms: attacks off by {:?} ms",
            run.attack_lag
        );
    }
}

/// The server's side of the view for one command: `PM_UpdateViewAngles` (the command's
/// angles plus `delta_angles`, the pitch clamped at 16000 by rewriting its delta), or a
/// held view (`PM_SetPMViewAngle`: the delta cancels the command's angles).
fn serve_view(state: &mut MovementState, angles: [i32; 3], held: bool) {
    for axis in 0..2 {
        if held {
            state.delta_angles[axis] =
                angle_short(state.view_angles[axis]).wrapping_sub(angles[axis]) & 0xFFFF;
            continue;
        }
        let mut short = angles[axis].wrapping_add(state.delta_angles[axis]) as i16;
        if axis == 0 && short.unsigned_abs() > 16_000 {
            short = 16_000 * short.signum();
            state.delta_angles[0] = i32::from(short).wrapping_sub(angles[0]) & 0xFFFF;
        }
        state.view_angles[axis] = f32::from(short) * (360.0 / 65_536.0);
    }
}

#[test]
fn the_view_turns_to_the_wanted_one_and_stays_put_through_a_held_view() {
    let mut state = MovementState {
        view_angles: [0.0, 90.0, 0.0],
        delta_angles: [1_234, 40_000, 0],
        ..MovementState::default()
    };
    // Free: there within a few commands, six degrees at a time, and exactly.
    let wanted = [20.0, 120.0];
    let mut last = None;
    for step in 0..6 {
        let (angles, asked) = steer(last, &state, wanted);
        let before = state.view_angles;
        serve_view(&mut state, angles, false);
        last = Some(angles);
        for ((now, before), asked) in state.view_angles.iter().zip(before).zip(asked) {
            assert!(
                angle_delta(*now, before).abs() <= MAX_TURN + 0.01,
                "step {step}"
            );
            assert!(angle_delta(*now, asked).abs() < 0.01);
        }
    }
    assert!(angle_delta(state.view_angles[0], 20.0).abs() < 0.01);
    assert!(angle_delta(state.view_angles[1], 120.0).abs() < 0.01);

    // Held for a second while the leader looks elsewhere, and the mimic knows a
    // `delta_angles` three commands old (a prediction the server's hold got ahead of):
    // the view stays through the hold and, released, goes on from where it was held, a
    // turn at a time, never thrown down or round.
    let wanted = [-30.0, -100.0];
    let mut known = std::collections::VecDeque::from([state.delta_angles; 3]);
    for step in 0..250 {
        let held = step < 125;
        let stale = MovementState {
            delta_angles: known.pop_front().unwrap(),
            ..state.clone()
        };
        let (angles, _) = steer(last, &stale, wanted);
        let before = state.view_angles;
        serve_view(&mut state, angles, held);
        known.push_back(state.delta_angles);
        last = Some(angles);
        for (now, before) in state.view_angles.iter().zip(before).take(2) {
            let turned = angle_delta(*now, before).abs();
            assert!(
                turned <= if held { 0.01 } else { MAX_TURN + 0.01 },
                "step {step}: turned {turned}"
            );
        }
    }
    assert!(angle_delta(state.view_angles[0], -30.0).abs() < 0.01);
    assert!(angle_delta(state.view_angles[1], -100.0).abs() < 0.01);
}

#[test]
fn a_steep_look_stays_short_of_the_pitch_clamp() {
    let mut mimic = Mimic::default();
    mimic.start(1, "Leader".into(), 0, false, false);
    mimic.push(Sample {
        time: 10,
        on_ground: true,
        ..Sample::default()
    });
    let mut follower = duelist([128.0, 0.0, 24.0], 180.0);
    for stamp in (8..2_000).step_by(8) {
        if stamp % 24 == 0 {
            // The leader looks straight down.
            mimic.push(Sample {
                time: stamp,
                on_ground: true,
                angles: [89.0, 0.0, 0.0],
                ..Sample::default()
            });
        }
        let input = UserCommand {
            server_time: stamp,
            weapon: WP_SABER,
            ..UserCommand::default()
        };
        let drive = mimic.drive(&input, follower.state()).unwrap();
        follower.predict_command(drive.command, &Floor);
    }
    let pitch = angle_delta(follower.state().view_angles[0], 0.0);
    assert!((pitch - PITCH_LIMIT).abs() < 0.1, "pitch {pitch}");
    assert!(angle_delta(follower.state().view_angles[1], 180.0).abs() < 0.1);
}

#[test]
fn the_stance_is_cycled_only_between_a_single_sabers_stances_and_never_in_a_swing() {
    let mut mimic = Mimic::default();
    mimic.start(1, "Leader".into(), 0, false, false);
    let following = mimic.following.as_mut().unwrap();
    let leader = |style: u8, saber_move: u32| Sample {
        style,
        saber_move,
        weapon: WP_SABER,
        ..Sample::default()
    };
    let own = |style: u8, saber_move: u32, holstered: u8| MovementState {
        weapon: WP_SABER,
        saber_anim_level: style,
        saber_move,
        saber_holstered: holstered,
        ..MovementState::default()
    };
    // A staff (7) or two sabers (6) copying a single saber's strong stance: no press,
    // which would put a blade away.
    for held in [6, 7] {
        assert_eq!(
            following.saber_upkeep(&leader(3, 1), &own(held, 1, 0), 1_000),
            0
        );
    }
    // A single saber copying a staff: no press either.
    assert_eq!(
        following.saber_upkeep(&leader(7, 1), &own(2, 1, 0), 2_000),
        0
    );
    // One blade of the staff off is not a holstered saber.
    assert_eq!(
        following.saber_upkeep(&leader(7, 1), &own(7, 1, 1), 3_000),
        0
    );
    // Single to single: a press, but not while either swings (move 10, the overhead).
    assert_eq!(
        following.saber_upkeep(&leader(3, 10), &own(2, 1, 0), 4_000),
        0
    );
    assert_eq!(
        following.saber_upkeep(&leader(3, 1), &own(2, 10, 0), 5_000),
        0
    );
    assert_eq!(
        following.saber_upkeep(&leader(3, 1), &own(2, 1, 0), 6_000),
        GENCMD_SABER_ATTACK_CYCLE
    );
    // Every blade off on one side only: the switch.
    assert_eq!(
        following.saber_upkeep(&leader(3, 1), &own(3, 1, 2), 7_000),
        GENCMD_SABER_SWITCH
    );
}

#[test]
fn the_leaders_view_is_read_from_their_angular_trajectory() {
    // `apos.trBase` (netfields 5, 3, 33) holds a player's view; `angles` (25, 9, 24)
    // is where they spawned or were teleported.
    let mut entity = EntityState::from_raw_fields(1, vec![0_u32; 132]).unwrap();
    for (field, value) in [(5, 10.0_f32), (3, 135.0), (25, 0.0), (9, -90.0)] {
        assert!(entity.set_raw_field(field, value.to_bits()));
    }
    let sample = Sample::of_entity(100, &entity);
    assert_eq!(sample.angles, [10.0, 135.0, 0.0]);
}
