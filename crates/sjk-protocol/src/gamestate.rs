use crate::{
    ENTITY_CAPACITY, EntityDeltaError, EntityField, EntityState, EntityWire, LEGACY_ENTITY_FIELDS,
    MAX_LEGACY_ENTITIES, MessageError, MessageReader, ReliableServerCommand, ServiceCommand,
    read_delta_entity_on,
};
use std::error::Error;
use std::fmt;

pub const MAX_CONFIGSTRINGS: usize = 1_700;
/// Room for the configstring text of one gamestate. Stock JKA keeps 16000, which 64
/// real players' configstrings on a `jof64` server can pass; the gamestate still has
/// to fit one server message (`MAX_MSGLEN` 49152), so that is the useful ceiling.
pub const MAX_GAMESTATE_CHARS: usize = 49_152;
pub const MAX_BIG_INFO_STRING_BYTES: usize = 8_191;

/// The state carried by JKA's initial `svc_gamestate` command.
///
/// Strings remain bytes until a subsystem chooses an encoding policy. This is
/// important for compatibility with old servers containing non-UTF-8 text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameState {
    pub server_command_sequence: i32,
    config_strings: Vec<Option<Vec<u8>>>,
    baselines: Vec<Option<EntityState>>,
    /// How this connection's server messages number entities ([`EntityWire`]).
    entity_wire: EntityWire,
    pub client_num: i32,
    pub checksum_feed: i32,
    pub rmg_marker: i16,
}

impl GameState {
    /// Empty state for a local game host using the compatibility presentation schema.
    /// This does not encode, decode or change any network message.
    pub fn empty_local(client_num: i32) -> Self {
        Self {
            server_command_sequence: 0,
            config_strings: vec![None; MAX_CONFIGSTRINGS],
            baselines: vec![None; MAX_LEGACY_ENTITIES],
            entity_wire: EntityWire::Classic,
            client_num,
            checksum_feed: 0,
            rmg_marker: 0,
        }
    }

    /// How the server numbers entities in this gamestate and the snapshots after it.
    pub fn entity_wire(&self) -> EntityWire {
        self.entity_wire
    }

    pub fn config_string(&self, index: usize) -> Option<&[u8]> {
        self.config_strings.get(index)?.as_deref()
    }

    /// `CS_SERVERINFO` (config string 0) parsed as an info string.
    ///
    /// Read lossily: servers pad `sv_hostname` with bytes such as `0x80`
    /// (the KR main server does), which are not UTF-8. Read strictly, one such
    /// byte dropped the whole string, so the client saw no `mapname` and stayed
    /// on the loading screen, and a JA+ server looked unknown (#319, #325).
    pub fn server_info(&self) -> Option<crate::InfoString> {
        let bytes = self.config_string(0)?;
        crate::InfoString::parse(&String::from_utf8_lossy(bytes)).ok()
    }

    pub fn config_strings(&self) -> impl Iterator<Item = (usize, &[u8])> {
        self.config_strings
            .iter()
            .enumerate()
            .filter_map(|(index, value)| value.as_deref().map(|value| (index, value)))
    }

    pub fn config_string_count(&self) -> usize {
        self.config_strings.iter().flatten().count()
    }

    /// Replace a validated string, returning whether its bytes changed.
    pub fn replace_config_string(
        &mut self,
        index: usize,
        value: Vec<u8>,
    ) -> Result<bool, GameStateError> {
        if index >= MAX_CONFIGSTRINGS {
            return Err(GameStateError::InvalidConfigStringUpdateIndex(index));
        }
        if value.len() > MAX_BIG_INFO_STRING_BYTES {
            return Err(GameStateError::ConfigStringTooLarge {
                actual_bytes: value.len(),
                maximum_bytes: MAX_BIG_INFO_STRING_BYTES,
            });
        }
        if self.config_string(index).unwrap_or_default() == value {
            return Ok(false);
        }
        let old_bytes = self.config_strings[index]
            .as_ref()
            .map_or(0, |old| old.len() + 1);
        let current_bytes = 1 + self
            .config_strings
            .iter()
            .flatten()
            .map(|string| string.len() + 1)
            .sum::<usize>();
        let next_bytes = current_bytes - old_bytes + value.len() + 1;
        if next_bytes > MAX_GAMESTATE_CHARS {
            return Err(GameStateError::ConfigStringStorageExceeded {
                maximum_bytes: MAX_GAMESTATE_CHARS,
            });
        }
        self.config_strings[index] = Some(value);
        Ok(true)
    }

    pub fn baseline(&self, number: usize) -> Option<&EntityState> {
        self.baselines.get(number)?.as_ref()
    }

    pub fn baselines(&self) -> impl Iterator<Item = &EntityState> {
        self.baselines.iter().flatten()
    }

    pub fn baseline_count(&self) -> usize {
        self.baselines.iter().flatten().count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialGameStateMessage {
    pub reliable_acknowledge: i32,
    pub game_state: GameState,
    pub consumed_bits: usize,
    /// The reliable commands the server flushed ahead of the gamestate.
    ///
    /// `SV_CreateClientGameStateMessage` sends everything still pending
    /// before `svc_gamestate` precisely because the gamestate then sets the
    /// client's command sequence (`codemp/server/sv_client.cpp:429-433`).
    /// Dropping them leaves the client acknowledging commands whose text it
    /// never saw, and that text is the key both sides encode packets with
    /// (`cl_net_chan.cpp:64`), so everything reliable it sends afterwards —
    /// `cp` included — reaches the server as garbage.
    pub server_commands: Vec<ReliableServerCommand>,
}

/// A gamestate on a connection that did not ask for wide entity numbers: protocol 26.
pub fn decode_initial_gamestate(payload: &[u8]) -> Result<InitialGameStateMessage, GameStateError> {
    decode_initial_gamestate_with_schema(payload, &LEGACY_ENTITY_FIELDS)
}

/// A gamestate on a connection that asked for wide entity numbers (`jofents`) when
/// `offered`: it is read wide when its own serverinfo says the server gives them
/// ([`EntityWire::negotiated`]), as stock otherwise.
pub fn decode_initial_gamestate_offering(
    payload: &[u8],
    offered: bool,
) -> Result<InitialGameStateMessage, GameStateError> {
    decode_initial_gamestate_inner(payload, &LEGACY_ENTITY_FIELDS, offered)
}

pub fn decode_initial_gamestate_with_schema(
    payload: &[u8],
    entity_schema: &[EntityField],
) -> Result<InitialGameStateMessage, GameStateError> {
    decode_initial_gamestate_inner(payload, entity_schema, false)
}

fn decode_initial_gamestate_inner(
    payload: &[u8],
    entity_schema: &[EntityField],
    offered: bool,
) -> Result<InitialGameStateMessage, GameStateError> {
    let mut message = MessageReader::new(payload);
    let reliable_acknowledge = message.read_i32()?;
    let mut server_commands = Vec::new();
    loop {
        match message.read_service_command()? {
            ServiceCommand::GameState => break,
            ServiceCommand::ServerCommand => {
                let sequence = message.read_i32()?;
                let command = message.read_c_string(MAX_BIG_INFO_STRING_BYTES)?;
                server_commands.push(ReliableServerCommand { sequence, command });
            }
            ServiceCommand::Nop | ServiceCommand::MapChange => {}
            actual => {
                return Err(GameStateError::ExpectedCommand {
                    expected: ServiceCommand::GameState,
                    actual,
                });
            }
        }
    }
    let game_state = decode_gamestate_command(&mut message, entity_schema, offered)?;
    expect_command(&mut message, ServiceCommand::End)?;

    Ok(InitialGameStateMessage {
        reliable_acknowledge,
        game_state,
        consumed_bits: message.bit_position(),
        server_commands,
    })
}

fn decode_gamestate_command(
    message: &mut MessageReader<'_>,
    entity_schema: &[EntityField],
    offered: bool,
) -> Result<GameState, GameStateError> {
    let server_command_sequence = message.read_i32()?;
    let mut config_strings: Vec<Option<Vec<u8>>> = vec![None; MAX_CONFIGSTRINGS];
    let mut baselines = vec![None; MAX_LEGACY_ENTITIES];
    // Settled by the serverinfo, which a server sends before any baseline
    // (`SV_CreateClientGameStateMessage`: every configstring, then the baselines).
    let mut entity_wire = None;
    let negotiated = |config_strings: &[Option<Vec<u8>>]| {
        EntityWire::negotiated(offered, config_strings[0].as_deref())
    };
    // The legacy string table reserves byte zero for the empty string.
    let mut string_storage_bytes = 1_usize;

    loop {
        match message.read_service_command()? {
            ServiceCommand::ConfigString => {
                let wire_index = message.read_i16()?;
                let index = usize::try_from(wire_index)
                    .ok()
                    .filter(|index| *index < MAX_CONFIGSTRINGS)
                    .ok_or(GameStateError::InvalidConfigStringIndex(wire_index))?;
                let mut value = message.read_c_string(MAX_BIG_INFO_STRING_BYTES)?;
                // MSG_ReadBigString neutralizes printf format markers before
                // placing the value in the legacy gamestate table.
                value.iter_mut().for_each(|byte| {
                    if *byte == b'%' {
                        *byte = b'.';
                    }
                });
                string_storage_bytes = string_storage_bytes
                    .checked_add(value.len() + 1)
                    .filter(|bytes| *bytes <= MAX_GAMESTATE_CHARS)
                    .ok_or(GameStateError::ConfigStringStorageExceeded {
                        maximum_bytes: MAX_GAMESTATE_CHARS,
                    })?;
                config_strings[index] = Some(value);
            }
            ServiceCommand::Baseline => {
                let wire = *entity_wire.get_or_insert_with(|| negotiated(&config_strings));
                if baselines.len() < wire.capacity() {
                    baselines.resize(wire.capacity(), None);
                }
                let number = message.read_bits(wire.number_bits())? as u16;
                let zero = EntityState::zero(number, entity_schema);
                let baseline = read_delta_entity_on(message, &zero, number, entity_schema, wire)?;
                baselines[usize::from(number)] = Some(baseline);
            }
            ServiceCommand::End => break,
            actual => {
                return Err(GameStateError::UnexpectedInnerCommand { actual });
            }
        }
    }

    let entity_wire = entity_wire.unwrap_or_else(|| negotiated(&config_strings));
    if baselines.len() < entity_wire.capacity() {
        baselines.resize(entity_wire.capacity(), None);
    }
    debug_assert!(baselines.len() <= ENTITY_CAPACITY);
    Ok(GameState {
        server_command_sequence,
        config_strings,
        baselines,
        entity_wire,
        client_num: message.read_i32()?,
        checksum_feed: message.read_i32()?,
        rmg_marker: message.read_i16()?,
    })
}

fn expect_command(
    message: &mut MessageReader<'_>,
    expected: ServiceCommand,
) -> Result<(), GameStateError> {
    let actual = message.read_service_command()?;
    if actual != expected {
        return Err(GameStateError::ExpectedCommand { expected, actual });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameStateError {
    Message(MessageError),
    EntityDelta(EntityDeltaError),
    ExpectedCommand {
        expected: ServiceCommand,
        actual: ServiceCommand,
    },
    UnexpectedInnerCommand {
        actual: ServiceCommand,
    },
    InvalidConfigStringIndex(i16),
    InvalidConfigStringUpdateIndex(usize),
    ConfigStringTooLarge {
        actual_bytes: usize,
        maximum_bytes: usize,
    },
    ConfigStringStorageExceeded {
        maximum_bytes: usize,
    },
}

impl fmt::Display for GameStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(error) => error.fmt(formatter),
            Self::EntityDelta(error) => error.fmt(formatter),
            Self::ExpectedCommand { expected, actual } => {
                write!(formatter, "expected {expected:?}, received {actual:?}")
            }
            Self::UnexpectedInnerCommand { actual } => {
                write!(formatter, "unexpected {actual:?} inside gamestate")
            }
            Self::InvalidConfigStringIndex(index) => {
                write!(formatter, "invalid configstring index {index}")
            }
            Self::InvalidConfigStringUpdateIndex(index) => {
                write!(formatter, "invalid configstring update index {index}")
            }
            Self::ConfigStringTooLarge {
                actual_bytes,
                maximum_bytes,
            } => write!(
                formatter,
                "configstring has {actual_bytes} bytes; maximum is {maximum_bytes}"
            ),
            Self::ConfigStringStorageExceeded { maximum_bytes } => write!(
                formatter,
                "gamestate configstrings exceed the {maximum_bytes}-byte legacy limit"
            ),
        }
    }
}

impl Error for GameStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Message(error) => Some(error),
            Self::EntityDelta(error) => Some(error),
            _ => None,
        }
    }
}

impl From<MessageError> for GameStateError {
    fn from(value: MessageError) -> Self {
        Self::Message(value)
    }
}

impl From<EntityDeltaError> for GameStateError {
    fn from(value: EntityDeltaError) -> Self {
        Self::EntityDelta(value)
    }
}
