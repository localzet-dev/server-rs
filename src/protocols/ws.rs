use super::websocket::{decode_frame, encode_frame, frame_length};
use super::{Protocol, ProtocolError, WebSocketFrame};

#[derive(Clone, Copy, Debug, Default)]
pub struct Ws;

impl Protocol for Ws {
    type Outbound = WebSocketFrame;
    type Inbound = WebSocketFrame;
    fn input(buffer: &[u8], maximum: usize) -> Result<Option<usize>, ProtocolError> {
        frame_length(buffer, maximum)
    }
    fn encode(frame: &WebSocketFrame) -> Result<Vec<u8>, ProtocolError> {
        let mut mask = [0_u8; 4];
        getrandom::fill(&mut mask)
            .map_err(|_| ProtocolError::InvalidData("OS random source failed"))?;
        encode_frame(frame, Some(mask))
    }
    fn decode(packet: &[u8]) -> Result<WebSocketFrame, ProtocolError> {
        decode_frame(packet)
    }
}
