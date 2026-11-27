//! A `jofents` stream end to end: a gamestate with baselines past 1023, the demo
//! gamestate written back, and a snapshot whose entity numbers and entity-number
//! fields are 11 bits wide.

use crate::gamestate_write::write_gamestate_block_on;
use crate::{
    EntityState, EntityWire, GameState, LEGACY_ENTITY_FIELDS, MessageWriter, ServiceCommand,
    decode_initial_gamestate, decode_initial_gamestate_offering, decode_snapshot,
    write_delta_entity_on, write_initial_gamestate,
};

const WIDE_SERVERINFO: &[u8] = br"\mapname\mp/ffa3\sv_maxclients\32\jofents\2048";

fn field(name: &str) -> usize {
    LEGACY_ENTITY_FIELDS
        .iter()
        .position(|field| field.name == name)
        .unwrap()
}

/// Entity 1500, owned by 1900 and standing on 2000, its `event` (10 bits, not an
/// entity number) 1000.
fn far_entity() -> EntityState {
    let mut entity = EntityState::zero(1_500, &LEGACY_ENTITY_FIELDS);
    entity.set_raw_field(field("eType"), 3);
    entity.set_raw_field(field("owner"), 1_900);
    entity.set_raw_field(field("groundEntityNum"), 2_000);
    entity.set_raw_field(field("event"), 1_000);
    entity
}

fn gamestate_payload(server_info: &[u8], baselines: &[EntityState], wire: EntityWire) -> Vec<u8> {
    let mut message = MessageWriter::new(crate::MAX_LEGACY_MESSAGE_BYTES);
    message.write_i32(0).unwrap();
    write_gamestate_block_on(
        &mut message,
        7,
        [(0, server_info), (1, br"\sv_serverid\5".as_slice())],
        baselines,
        0,
        0,
        wire,
    )
    .unwrap();
    message.write_u8(ServiceCommand::End as u8).unwrap();
    message.finish().unwrap()
}

fn wide_gamestate() -> GameState {
    let payload = gamestate_payload(WIDE_SERVERINFO, &[far_entity()], EntityWire::Wide);
    decode_initial_gamestate_offering(&payload, true)
        .unwrap()
        .game_state
}

#[test]
fn a_negotiated_gamestate_reads_baselines_past_1023() {
    let game = wide_gamestate();
    assert_eq!(game.entity_wire(), EntityWire::Wide);
    let baseline = game.baseline(1_500).expect("baseline 1500");
    assert_eq!(baseline, &far_entity());
    assert_eq!(baseline.owner(), 1_900);
    assert_eq!(baseline.ground_entity_num(), 2_000);
    assert_eq!(baseline.event(), 1_000);
}

#[test]
fn a_classic_gamestate_is_unchanged_whatever_was_offered() {
    let mut near = far_entity();
    near.set_number(900).unwrap();
    near.set_raw_field(field("owner"), 900);
    near.set_raw_field(field("groundEntityNum"), 1_000);
    let classic = gamestate_payload(
        br"\mapname\mp/ffa3",
        std::slice::from_ref(&near),
        EntityWire::Classic,
    );
    for offered in [false, true] {
        let game = decode_initial_gamestate_offering(&classic, offered)
            .unwrap()
            .game_state;
        assert_eq!(game.entity_wire(), EntityWire::Classic);
        assert_eq!(game.baseline(900), Some(&near));
    }
    assert_eq!(
        decode_initial_gamestate(&classic).unwrap().game_state,
        decode_initial_gamestate_offering(&classic, true)
            .unwrap()
            .game_state
    );
}

#[test]
fn a_recorded_demo_gamestate_keeps_the_wide_format() {
    let game = wide_gamestate();
    let payload = write_initial_gamestate(3, &game).unwrap();
    // A demo has no connect: its serverinfo alone says how it was recorded.
    let replayed = decode_initial_gamestate_offering(&payload, true)
        .unwrap()
        .game_state;
    assert_eq!(replayed.entity_wire(), EntityWire::Wide);
    assert_eq!(replayed.baseline(1_500), game.baseline(1_500));
}

#[test]
fn a_negotiated_snapshot_reads_11_bit_entities_and_fields() {
    let game = wide_gamestate();
    let mut message = MessageWriter::new(4_096);
    message.write_i32(0).unwrap(); // reliable acknowledge
    message.write_u8(ServiceCommand::Snapshot as u8).unwrap();
    message.write_i32(1_000).unwrap(); // server time
    message.write_u8(0).unwrap(); // not a delta
    message.write_u8(0).unwrap(); // flags
    message.write_u8(0).unwrap(); // no area mask
    // Player state, ordinary schema: fields 0..=16, only groundEntityNum (16) set,
    // in 11 bits; then no arrays.
    message.write_bits(0, 1).unwrap();
    message.write_u8(17).unwrap();
    for _ in 0..16 {
        message.write_bits(0, 1).unwrap();
    }
    message.write_bits(1, 1).unwrap();
    message.write_bits(1_800, 11).unwrap();
    message.write_bits(0, 1).unwrap();
    // In number order: 1024, new against nothing, owned by 1500; then 1500 against
    // its baseline, now standing on 2046; then the 11-bit end of the list.
    let mut first_wide = EntityState::zero(1_024, &LEGACY_ENTITY_FIELDS);
    first_wide.set_raw_field(field("owner"), 1_500);
    let zero = EntityState::zero(1_024, &LEGACY_ENTITY_FIELDS);
    write_delta_entity_on(
        &mut message,
        &zero,
        Some(&first_wide),
        true,
        &LEGACY_ENTITY_FIELDS,
        EntityWire::Wide,
    )
    .unwrap();
    let mut moved = far_entity();
    moved.set_raw_field(field("groundEntityNum"), 2_046);
    write_delta_entity_on(
        &mut message,
        game.baseline(1_500).unwrap(),
        Some(&moved),
        true,
        &LEGACY_ENTITY_FIELDS,
        EntityWire::Wide,
    )
    .unwrap();
    message.write_bits(2_047, 11).unwrap();
    let payload = message.finish().unwrap();

    let snapshot = decode_snapshot(&payload, 1, &game, None).unwrap();
    assert_eq!(snapshot.player.ground_entity_num(), 1_800);
    assert_eq!(snapshot.entities.len(), 2);
    assert_eq!(snapshot.entities[0], first_wide);
    let entity = &snapshot.entities[1];
    assert_eq!(entity.number(), 1_500);
    assert_eq!(entity.ground_entity_num(), 2_046);
    assert_eq!(entity.owner(), 1_900);
    assert_eq!(entity.event(), 1_000);
}

#[test]
fn a_snapshot_before_the_gamestate_is_skipped_unread() {
    // A server sends a connecting client snapshots before its gamestate, already 11
    // bits wide on a `jofents` connection. The join skips any message that is not a
    // gamestate at its first command, before any entity bit, so their width never
    // matters.
    let mut message = MessageWriter::new(4_096);
    message.write_i32(0).unwrap();
    message
        .write_u8(ServiceCommand::ServerCommand as u8)
        .unwrap();
    message.write_i32(1).unwrap();
    message.write_c_string(b"print hello").unwrap();
    message.write_u8(ServiceCommand::Snapshot as u8).unwrap();
    message.write_i32(1_000).unwrap();
    for _ in 0..64 {
        message.write_bits(0x7ff, 11).unwrap();
    }
    let payload = message.finish().unwrap();
    for offered in [false, true] {
        assert_eq!(
            decode_initial_gamestate_offering(&payload, offered),
            Err(crate::GameStateError::ExpectedCommand {
                expected: ServiceCommand::GameState,
                actual: ServiceCommand::Snapshot,
            })
        );
    }
}
