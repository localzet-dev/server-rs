use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chunk {
    buffer: Vec<u8>,
}

impl Chunk {
    pub fn new(buffer: impl Into<Vec<u8>>) -> Self {
        Self {
            buffer: buffer.into(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = format!("{:x}\r\n", self.buffer.len()).into_bytes();
        encoded.extend_from_slice(&self.buffer);
        encoded.extend_from_slice(b"\r\n");
        encoded
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let encoded = self.encode();
        formatter.write_str(&String::from_utf8_lossy(&encoded))
    }
}
