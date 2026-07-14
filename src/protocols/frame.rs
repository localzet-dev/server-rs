use super::protocol::enforce_package_size;
use super::{Protocol, ProtocolError};

const HEADER_SIZE: usize = size_of::<u32>();

#[derive(Clone, Copy, Debug, Default)]
pub struct Frame;

impl Protocol for Frame {
    type Outbound = [u8];
    type Inbound = Vec<u8>;

    fn input(buffer: &[u8], max_package_size: usize) -> Result<Option<usize>, ProtocolError> {
        let Some(header) = buffer.get(..HEADER_SIZE) else {
            return Ok(None);
        };
        let package_size = u32::from_be_bytes(header.try_into().expect("header size is checked"));
        let package_size = usize::try_from(package_size)
            .map_err(|_| ProtocolError::InvalidData("frame size does not fit usize"))?;

        if package_size < HEADER_SIZE {
            return Err(ProtocolError::InvalidData(
                "frame size is smaller than its header",
            ));
        }
        enforce_package_size(package_size, max_package_size)?;
        Ok(Some(package_size))
    }

    fn encode(data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let package_size = data
            .len()
            .checked_add(HEADER_SIZE)
            .ok_or(ProtocolError::InvalidData("frame size overflow"))?;
        let package_size = u32::try_from(package_size)
            .map_err(|_| ProtocolError::InvalidData("frame exceeds u32 length"))?;

        let mut encoded = Vec::with_capacity(package_size as usize);
        encoded.extend_from_slice(&package_size.to_be_bytes());
        encoded.extend_from_slice(data);
        Ok(encoded)
    }

    fn decode(packet: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        let payload = packet
            .get(HEADER_SIZE..)
            .ok_or(ProtocolError::UnexpectedEnd)?;
        Ok(payload.to_vec())
    }
}
