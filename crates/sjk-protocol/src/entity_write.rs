//! Protocol-26 entity delta encoding.

use crate::{
    ENTITY_NUMBER_NONE, EntityField, EntityFieldEncoding, EntityState, EntityWire, MessageError,
    MessageWriter,
};
use std::error::Error;
use std::fmt;

const FLOAT_INT_BITS: u8 = 13;
const FLOAT_INT_BIAS: i32 = 1 << (FLOAT_INT_BITS - 1);

/// Write one entity delta using the protocol field schema.
///
/// This is the inverse of [`crate::read_delta_entity`] and follows
/// `MSG_WriteDeltaEntity` in `codemp/qcommon/msg.cpp:1040-1146`. The entity
/// number is part of this encoding. A `None` target emits a removal; an
/// unchanged target is omitted unless `force` is true.
pub fn write_delta_entity(
    message: &mut MessageWriter,
    from: &EntityState,
    to: Option<&EntityState>,
    force: bool,
    schema: &[EntityField],
) -> Result<bool, EntityWriteError> {
    write_delta_entity_on(message, from, to, force, schema, EntityWire::Classic)
}

/// [`write_delta_entity`] for a stream whose entity numbers are `wire` wide: a demo
/// recorded on a `jofents` connection writes its gamestate's baselines this way.
pub fn write_delta_entity_on(
    message: &mut MessageWriter,
    from: &EntityState,
    to: Option<&EntityState>,
    force: bool,
    schema: &[EntityField],
    wire: EntityWire,
) -> Result<bool, EntityWriteError> {
    validate_state(from, schema)?;
    let Some(to) = to else {
        if from.number() == ENTITY_NUMBER_NONE {
            return Ok(false);
        }
        validate_number(from.number(), wire)?;
        message.write_bits(u32::from(from.number()), wire.number_bits())?;
        message.write_bits(1, 1)?;
        return Ok(true);
    };
    validate_state(to, schema)?;
    validate_number(to.number(), wire)?;

    let changed = last_changed_field(from, to, schema);
    if changed == 0 && !force {
        return Ok(false);
    }
    message.write_bits(u32::from(to.number()), wire.number_bits())?;
    message.write_bits(0, 1)?;
    if changed == 0 {
        message.write_bits(0, 1)?;
        return Ok(true);
    }
    message.write_bits(1, 1)?;
    message.write_u8(changed as u8)?;
    for (index, field) in schema.iter().take(changed).enumerate() {
        let before = from.raw_field(index).expect("validated state field");
        let after = to.raw_field(index).expect("validated state field");
        if before == after {
            message.write_bits(0, 1)?;
            continue;
        }
        message.write_bits(1, 1)?;
        write_field(message, field, after, wire)?;
    }
    Ok(true)
}

fn validate_state(state: &EntityState, schema: &[EntityField]) -> Result<(), EntityWriteError> {
    if state.field_count() != schema.len() {
        return Err(EntityWriteError::SchemaLengthMismatch {
            state_fields: state.field_count(),
            schema_fields: schema.len(),
        });
    }
    Ok(())
}

fn validate_number(number: u16, wire: EntityWire) -> Result<(), EntityWriteError> {
    if usize::from(number) >= wire.capacity() {
        return Err(EntityWriteError::InvalidEntityNumber(number));
    }
    Ok(())
}

fn last_changed_field(from: &EntityState, to: &EntityState, schema: &[EntityField]) -> usize {
    schema
        .iter()
        .enumerate()
        .filter(|(index, _)| from.raw_field(*index) != to.raw_field(*index))
        .map(|(index, _)| index + 1)
        .last()
        .unwrap_or(0)
}

fn write_field(
    message: &mut MessageWriter,
    field: &EntityField,
    value: u32,
    wire: EntityWire,
) -> Result<(), MessageError> {
    let width = match field.encoding {
        EntityFieldEncoding::Float => return write_float(message, value),
        EntityFieldEncoding::Integer { width, .. } => width,
        EntityFieldEncoding::EntityNumber => wire.number_bits(),
    };
    if value == 0 {
        message.write_bits(0, 1)
    } else {
        message.write_bits(1, 1)?;
        message.write_bits(value, width)
    }
}

fn write_float(message: &mut MessageWriter, bits: u32) -> Result<(), MessageError> {
    let value = f32::from_bits(bits);
    if value == 0.0 {
        return message.write_bits(0, 1);
    }
    message.write_bits(1, 1)?;
    let integral = value as i32;
    if integral as f32 == value && (0..(1 << FLOAT_INT_BITS)).contains(&(integral + FLOAT_INT_BIAS))
    {
        message.write_bits(0, 1)?;
        message.write_bits((integral + FLOAT_INT_BIAS) as u32, FLOAT_INT_BITS)
    } else {
        message.write_bits(1, 1)?;
        message.write_bits(bits, 32)
    }
}

/// Failure while serializing an entity delta.
#[derive(Debug)]
pub enum EntityWriteError {
    /// Compressed message writer failure.
    Message(MessageError),
    /// Entity number lies outside the schema's legacy range.
    InvalidEntityNumber(u16),
    /// State and schema contain different field counts.
    SchemaLengthMismatch {
        /// Number of fields in the state.
        state_fields: usize,
        /// Number of fields in the schema.
        schema_fields: usize,
    },
}

impl fmt::Display for EntityWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(error) => error.fmt(formatter),
            Self::InvalidEntityNumber(number) => {
                write!(formatter, "invalid entity number {number}")
            }
            Self::SchemaLengthMismatch {
                state_fields,
                schema_fields,
            } => write!(
                formatter,
                "entity has {state_fields} fields but schema has {schema_fields}"
            ),
        }
    }
}

impl Error for EntityWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Message(error) => Some(error),
            _ => None,
        }
    }
}

impl From<MessageError> for EntityWriteError {
    fn from(value: MessageError) -> Self {
        Self::Message(value)
    }
}
