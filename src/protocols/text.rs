use super::protocol::enforce_package_size;
use super::{Protocol, ProtocolError};

#[derive(Clone, Copy, Debug, Default)]
pub struct Text;

impl Protocol for Text {
    type Outbound = [u8];
    type Inbound = Vec<u8>;

    fn input(buffer: &[u8], max_package_size: usize) -> Result<Option<usize>, ProtocolError> {
        if let Some(position) = buffer.iter().position(|byte| *byte == b'\n') {
            let package_size = position + 1;
            enforce_package_size(package_size, max_package_size)?;
            return Ok(Some(package_size));
        }

        if buffer.len() >= max_package_size {
            return Err(ProtocolError::PackageTooLarge {
                actual: buffer.len(),
                maximum: max_package_size,
            });
        }
        Ok(None)
    }

    fn encode(data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let mut encoded = Vec::with_capacity(data.len() + 1);
        encoded.extend_from_slice(data);
        encoded.push(b'\n');
        Ok(encoded)
    }

    fn decode(packet: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let end = packet
            .iter()
            .rposition(|byte| !matches!(*byte, b'\r' | b'\n'))
            .map_or(0, |position| position + 1);
        Ok(packet[..end].to_vec())
    }
}
