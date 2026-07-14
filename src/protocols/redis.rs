use super::protocol::enforce_package_size;
use super::{Protocol, ProtocolError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedisArgument {
    Bulk(Vec<u8>),
    Group(Vec<RedisArgument>),
}

impl From<&str> for RedisArgument {
    fn from(value: &str) -> Self {
        Self::Bulk(value.as_bytes().to_vec())
    }
}

impl From<String> for RedisArgument {
    fn from(value: String) -> Self {
        Self::Bulk(value.into_bytes())
    }
}

impl From<Vec<u8>> for RedisArgument {
    fn from(value: Vec<u8>) -> Self {
        Self::Bulk(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedisValue {
    SimpleString(Vec<u8>),
    Error(Vec<u8>),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RedisValue>>),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Redis;

impl Protocol for Redis {
    type Outbound = [RedisArgument];
    type Inbound = RedisValue;

    fn input(buffer: &[u8], max_package_size: usize) -> Result<Option<usize>, ProtocolError> {
        match parse_value(buffer)? {
            Some((_, consumed)) => {
                enforce_package_size(consumed, max_package_size)?;
                Ok(Some(consumed))
            }
            None if buffer.len() >= max_package_size => Err(ProtocolError::PackageTooLarge {
                actual: buffer.len(),
                maximum: max_package_size,
            }),
            None => Ok(None),
        }
    }

    fn encode(data: &[RedisArgument]) -> Result<Vec<u8>, ProtocolError> {
        let count = flattened_len(data);
        let mut encoded = format!("*{count}\r\n").into_bytes();
        encode_arguments(data, &mut encoded);
        Ok(encoded)
    }

    fn decode(packet: &[u8]) -> Result<RedisValue, ProtocolError> {
        let (value, consumed) = parse_value(packet)?.ok_or(ProtocolError::UnexpectedEnd)?;
        if consumed != packet.len() {
            return Err(ProtocolError::InvalidData(
                "Redis packet contains trailing bytes",
            ));
        }
        Ok(value)
    }
}

fn flattened_len(arguments: &[RedisArgument]) -> usize {
    arguments
        .iter()
        .map(|argument| match argument {
            RedisArgument::Bulk(_) => 1,
            RedisArgument::Group(nested) => flattened_len(nested),
        })
        .sum()
}

fn encode_arguments(arguments: &[RedisArgument], output: &mut Vec<u8>) {
    for argument in arguments {
        match argument {
            RedisArgument::Bulk(bytes) => {
                output.extend_from_slice(format!("${}\r\n", bytes.len()).as_bytes());
                output.extend_from_slice(bytes);
                output.extend_from_slice(b"\r\n");
            }
            RedisArgument::Group(nested) => encode_arguments(nested, output),
        }
    }
}

fn parse_value(buffer: &[u8]) -> Result<Option<(RedisValue, usize)>, ProtocolError> {
    let Some(prefix) = buffer.first().copied() else {
        return Ok(None);
    };
    match prefix {
        b'+' => parse_line(buffer, RedisValue::SimpleString),
        b'-' => parse_line(buffer, RedisValue::Error),
        b':' => parse_integer(buffer),
        b'$' => parse_bulk_string(buffer),
        b'*' => parse_array(buffer),
        _ => Err(ProtocolError::InvalidData("unknown Redis reply type")),
    }
}

fn parse_line(
    buffer: &[u8],
    constructor: impl FnOnce(Vec<u8>) -> RedisValue,
) -> Result<Option<(RedisValue, usize)>, ProtocolError> {
    let Some(line_end) = find_crlf(buffer, 1) else {
        return Ok(None);
    };
    Ok(Some((
        constructor(buffer[1..line_end].to_vec()),
        line_end + 2,
    )))
}

fn parse_integer(buffer: &[u8]) -> Result<Option<(RedisValue, usize)>, ProtocolError> {
    let Some(line_end) = find_crlf(buffer, 1) else {
        return Ok(None);
    };
    let number = parse_i64(&buffer[1..line_end])?;
    Ok(Some((RedisValue::Integer(number), line_end + 2)))
}

fn parse_bulk_string(buffer: &[u8]) -> Result<Option<(RedisValue, usize)>, ProtocolError> {
    let Some(line_end) = find_crlf(buffer, 1) else {
        return Ok(None);
    };
    let length = parse_i64(&buffer[1..line_end])?;
    if length == -1 {
        return Ok(Some((RedisValue::BulkString(None), line_end + 2)));
    }
    let length = usize::try_from(length).map_err(|_| ProtocolError::InvalidNumber)?;
    let data_start = line_end + 2;
    let data_end = data_start
        .checked_add(length)
        .ok_or(ProtocolError::InvalidNumber)?;
    let total = data_end
        .checked_add(2)
        .ok_or(ProtocolError::InvalidNumber)?;
    let Some(ending) = buffer.get(data_end..total) else {
        return Ok(None);
    };
    if ending != b"\r\n" {
        return Err(ProtocolError::InvalidData(
            "Redis bulk string has invalid terminator",
        ));
    }
    Ok(Some((
        RedisValue::BulkString(Some(buffer[data_start..data_end].to_vec())),
        total,
    )))
}

fn parse_array(buffer: &[u8]) -> Result<Option<(RedisValue, usize)>, ProtocolError> {
    let Some(line_end) = find_crlf(buffer, 1) else {
        return Ok(None);
    };
    let count = parse_i64(&buffer[1..line_end])?;
    if count == -1 {
        return Ok(Some((RedisValue::Array(None), line_end + 2)));
    }
    let count = usize::try_from(count).map_err(|_| ProtocolError::InvalidNumber)?;
    let mut values = Vec::with_capacity(count);
    let mut consumed = line_end + 2;
    for _ in 0..count {
        let Some((value, value_size)) = parse_value(&buffer[consumed..])? else {
            return Ok(None);
        };
        consumed = consumed
            .checked_add(value_size)
            .ok_or(ProtocolError::InvalidNumber)?;
        values.push(value);
    }
    Ok(Some((RedisValue::Array(Some(values)), consumed)))
}

fn find_crlf(buffer: &[u8], start: usize) -> Option<usize> {
    buffer
        .get(start..)?
        .windows(2)
        .position(|window| window == b"\r\n")
        .map(|position| position + start)
}

fn parse_i64(bytes: &[u8]) -> Result<i64, ProtocolError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ProtocolError::InvalidNumber)?;
    text.parse().map_err(|_| ProtocolError::InvalidNumber)
}
