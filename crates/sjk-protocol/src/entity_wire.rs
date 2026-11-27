//! Entity numbers past 1023 (`jofents`), negotiated per connection.
//!
//! Protocol 26 sends an entity number in 10 bits (`GENTITYNUM_BITS`): numbers 0 to
//! 1021 are entities, 1022 is the world and 1023 "none", which also ends a snapshot's
//! entity list. A server that can have more entities (Heimdall, JoF's server, and its
//! JA+) says so with `jofents\2048` in its `getinfo` answer, and a client that can
//! read them asks for them with `jofents=2048` in its connect userinfo. The server
//! takes the key at connect for the whole connection and from then on sends that
//! client, in its gamestate and snapshots:
//!
//! * every entity number (a baseline's, a packet entity's) in 11 bits, and the end of
//!   the entity list as 2047;
//! * every netfield the source declares `GENTITYNUM_BITS` in 11 bits: they are marked
//!   [`EntityFieldEncoding::EntityNumber`](crate::EntityFieldEncoding::EntityNumber)
//!   in the entity schema and by `E` in the player-state tables (`snapshot.rs`). Fields
//!   that are 10 bits wide without naming an entity (`event`, `health`, `eFlags2`,
//!   `standheight`...) keep their 10 bits.
//!
//! `ENTITYNUM_WORLD` (1022) and `ENTITYNUM_NONE` (1023) keep their values: the server
//! never gives a real entity 1022, 1023 or 2047. What the client sends is unchanged.
//!
//! Without the key on both sides nothing changes, bit for bit: [`EntityWire::Classic`].

/// The key a server advertises and a client asks with.
pub const WIDE_ENTITIES_KEY: &str = "jofents";
/// Its only value: the entity count the wide format can number.
pub const WIDE_ENTITIES_VALUE: &str = "2048";

/// The most entities any connection can number: the size of a per-entity table.
/// A classic server uses the first 1024 of them.
pub const ENTITY_CAPACITY: usize = 1 << WIDE_ENTITY_NUMBER_BITS;

const CLASSIC_ENTITY_NUMBER_BITS: u8 = 10;
const WIDE_ENTITY_NUMBER_BITS: u8 = 11;

/// How a connection's server messages number entities.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum EntityWire {
    /// Protocol 26 as stock: 10-bit entity numbers.
    #[default]
    Classic,
    /// `jofents`: 11-bit entity numbers.
    Wide,
}

impl EntityWire {
    /// Bits of an entity number and of every entity-number netfield.
    pub const fn number_bits(self) -> u8 {
        match self {
            Self::Classic => CLASSIC_ENTITY_NUMBER_BITS,
            Self::Wide => WIDE_ENTITY_NUMBER_BITS,
        }
    }

    /// How many entity numbers the format has (1024 or 2048).
    pub const fn capacity(self) -> usize {
        1 << self.number_bits()
    }

    /// The number that ends a snapshot's entity list: all bits set (1023 or 2047).
    pub const fn end_of_entities(self) -> u16 {
        (self.capacity() - 1) as u16
    }

    /// Whether an info string (a `getinfo` answer or a serverinfo) offers wide entity
    /// numbers.
    pub fn advertised(info: &[u8]) -> bool {
        crate::info_value(info, WIDE_ENTITIES_KEY.as_bytes())
            == Some(WIDE_ENTITIES_VALUE.as_bytes())
    }

    /// [`Self::advertised`] for a parsed `getinfo` answer: whether to ask this server
    /// for wide entity numbers at connect.
    pub fn offered_by(info: &crate::InfoString) -> bool {
        info.get(WIDE_ENTITIES_KEY) == Some(WIDE_ENTITIES_VALUE)
    }

    /// The format of a gamestate: wide only when this client asked for it at connect
    /// (`offered`) and the gamestate's serverinfo (configstring 0) says the server
    /// gives it, so a server that turned the feature off between `getinfo` and
    /// `connect` is still read as it sends.
    pub fn negotiated(offered: bool, server_info: Option<&[u8]>) -> Self {
        if offered && server_info.is_some_and(Self::advertised) {
            Self::Wide
        } else {
            Self::Classic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_is_protocol_26() {
        let wire = EntityWire::Classic;
        assert_eq!(wire.number_bits(), 10);
        assert_eq!(wire.capacity(), 1_024);
        assert_eq!(wire.end_of_entities(), crate::ENTITY_NUMBER_NONE);
        assert_eq!(EntityWire::default(), EntityWire::Classic);
    }

    #[test]
    fn wide_numbers_2048_and_ends_at_2047() {
        let wire = EntityWire::Wide;
        assert_eq!(wire.number_bits(), 11);
        assert_eq!(wire.capacity(), ENTITY_CAPACITY);
        assert_eq!(wire.end_of_entities(), 2_047);
    }

    #[test]
    fn wide_only_when_asked_and_given() {
        let given = br"\mapname\mp/ffa3\jofents\2048\sv_maxclients\32".as_slice();
        assert_eq!(EntityWire::negotiated(true, Some(given)), EntityWire::Wide);
        assert_eq!(
            EntityWire::negotiated(false, Some(given)),
            EntityWire::Classic
        );
        assert_eq!(
            EntityWire::negotiated(true, Some(br"\mapname\mp/ffa3")),
            EntityWire::Classic
        );
        assert_eq!(
            EntityWire::negotiated(true, Some(br"\jofents\1024")),
            EntityWire::Classic
        );
        assert_eq!(EntityWire::negotiated(true, None), EntityWire::Classic);
    }
}

#[cfg(test)]
#[path = "entity_wire_tests.rs"]
mod stream_tests;
