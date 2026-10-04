use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha1::{Digest, Sha1};

use super::protocol::enforce_package_size;
use super::{Protocol, ProtocolError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WebSocketOpcode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebSocketFrame {
    pub fin: bool,
    pub opcode: WebSocketOpcode,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WebSocket;

impl Protocol for WebSocket {
    type Outbound = WebSocketFrame;
    type Inbound = WebSocketFrame;

    fn input(buffer: &[u8], maximum: usize) -> Result<Option<usize>, ProtocolError> {
        frame_length(buffer, maximum)
    }

    fn encode(frame: &WebSocketFrame) -> Result<Vec<u8>, ProtocolError> {
        encode_frame(frame, None)
    }

    fn decode(packet: &[u8]) -> Result<WebSocketFrame, ProtocolError> {
        decode_frame(packet)
    }
}

pub fn websocket_accept_key(client_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(client_key.trim().as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    STANDARD.encode(hasher.finalize())
}

pub(crate) fn frame_length(buffer: &[u8], maximum: usize) -> Result<Option<usize>, ProtocolError> {
    let Some(header) = buffer.get(..2) else {
        return Ok(None);
    };
    if header[0] & 0x70 != 0 {
        return Err(ProtocolError::InvalidData(
            "WebSocket RSV bits are not supported",
        ));
    }
    let short_length = usize::from(header[1] & 0x7f);
    let (payload_length, header_length): (usize, usize) = match short_length {
        126 => {
            let Some(length) = buffer.get(2..4) else {
                return Ok(None);
            };
            (
                usize::from(u16::from_be_bytes(length.try_into().unwrap())),
                4,
            )
        }
        127 => {
            let Some(length) = buffer.get(2..10) else {
                return Ok(None);
            };
            let value = u64::from_be_bytes(length.try_into().unwrap());
            if value & (1 << 63) != 0 {
                return Err(ProtocolError::InvalidNumber);
            }
            (
                usize::try_from(value).map_err(|_| ProtocolError::InvalidNumber)?,
                10,
            )
        }
        value => (value, 2),
    };
    let mask_length = if header[1] & 0x80 != 0 { 4 } else { 0 };
    let total = header_length
        .checked_add(mask_length)
        .and_then(|v| v.checked_add(payload_length))
        .ok_or(ProtocolError::InvalidNumber)?;
    enforce_package_size(total, maximum)?;
    let opcode = parse_opcode(header[0] & 0x0f)?;
    if matches!(
        opcode,
        WebSocketOpcode::Close | WebSocketOpcode::Ping | WebSocketOpcode::Pong
    ) && (header[0] & 0x80 == 0 || payload_length > 125)
    {
        return Err(ProtocolError::InvalidData(
            "invalid WebSocket control frame",
        ));
    }
    Ok(Some(total))
}

pub(crate) fn encode_frame(
    frame: &WebSocketFrame,
    mask: Option<[u8; 4]>,
) -> Result<Vec<u8>, ProtocolError> {
    if matches!(
        frame.opcode,
        WebSocketOpcode::Close | WebSocketOpcode::Ping | WebSocketOpcode::Pong
    ) && (!frame.fin || frame.payload.len() > 125)
    {
        return Err(ProtocolError::InvalidData(
            "invalid WebSocket control frame",
        ));
    }
    let mut output = Vec::with_capacity(frame.payload.len() + 14);
    output.push((if frame.fin { 0x80 } else { 0 }) | frame.opcode as u8);
    let mask_bit = if mask.is_some() { 0x80 } else { 0 };
    match frame.payload.len() {
        0..=125 => output.push(mask_bit | frame.payload.len() as u8),
        126..=65_535 => {
            output.push(mask_bit | 126);
            output.extend_from_slice(&(frame.payload.len() as u16).to_be_bytes());
        }
        length => {
            output.push(mask_bit | 127);
            output.extend_from_slice(&(length as u64).to_be_bytes());
        }
    }
    if let Some(mask) = mask {
        output.extend_from_slice(&mask);
        output.extend(
            frame
                .payload
                .iter()
                .enumerate()
                .map(|(i, byte)| byte ^ mask[i % 4]),
        );
    } else {
        output.extend_from_slice(&frame.payload);
    }
    Ok(output)
}

pub(crate) fn decode_frame(packet: &[u8]) -> Result<WebSocketFrame, ProtocolError> {
    let total = frame_length(packet, usize::MAX)?.ok_or(ProtocolError::UnexpectedEnd)?;
    if packet.len() < total {
        return Err(ProtocolError::UnexpectedEnd);
    }
    let first = packet[0];
    let masked = packet[1] & 0x80 != 0;
    let short = usize::from(packet[1] & 0x7f);
    let (length, mut offset) = match short {
        126 => (
            usize::from(u16::from_be_bytes(packet[2..4].try_into().unwrap())),
            4,
        ),
        127 => (
            usize::try_from(u64::from_be_bytes(packet[2..10].try_into().unwrap()))
                .map_err(|_| ProtocolError::InvalidNumber)?,
            10,
        ),
        value => (value, 2),
    };
    let mask = if masked {
        let value: [u8; 4] = packet[offset..offset + 4].try_into().unwrap();
        offset += 4;
        Some(value)
    } else {
        None
    };
    let mut payload = packet[offset..offset + length].to_vec();
    if let Some(mask) = mask {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    Ok(WebSocketFrame {
        fin: first & 0x80 != 0,
        opcode: parse_opcode(first & 0x0f)?,
        payload,
    })
}

fn parse_opcode(value: u8) -> Result<WebSocketOpcode, ProtocolError> {
    match value {
        0 => Ok(WebSocketOpcode::Continuation),
        1 => Ok(WebSocketOpcode::Text),
        2 => Ok(WebSocketOpcode::Binary),
        8 => Ok(WebSocketOpcode::Close),
        9 => Ok(WebSocketOpcode::Ping),
        10 => Ok(WebSocketOpcode::Pong),
        _ => Err(ProtocolError::InvalidData("unknown WebSocket opcode")),
    }
}
