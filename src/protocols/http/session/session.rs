use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use super::SessionHandler;

pub struct Session {
    id: String,
    data: BTreeMap<String, String>,
    handler: Arc<dyn SessionHandler>,
    lifetime: Duration,
    dirty: bool,
}

impl Session {
    pub fn open(
        id: impl Into<String>,
        handler: Arc<dyn SessionHandler>,
        lifetime: Duration,
    ) -> io::Result<Self> {
        let id = id.into();
        let data = handler
            .read(&id, lifetime)?
            .map(|bytes| decode(&bytes))
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            id,
            data,
            handler,
            lifetime,
            dirty: false,
        })
    }

    pub fn create(handler: Arc<dyn SessionHandler>, lifetime: Duration) -> io::Result<Self> {
        let mut random = [0_u8; 24];
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        Self::open(hex(&random), handler, lifetime)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.data.get(name).map(String::as_str)
    }

    pub fn pull(&mut self, name: &str) -> Option<String> {
        let value = self.data.remove(name);
        self.dirty |= value.is_some();
        value
    }

    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.data.insert(name.into(), value.into());
        self.dirty = true;
    }

    pub fn delete(&mut self, name: &str) -> bool {
        let removed = self.data.remove(name).is_some();
        self.dirty |= removed;
        removed
    }

    pub fn has(&self, name: &str) -> bool {
        self.data.contains_key(name)
    }

    pub fn all(&self) -> &BTreeMap<String, String> {
        &self.data
    }

    pub fn flush(&mut self) {
        self.dirty |= !self.data.is_empty();
        self.data.clear();
    }

    pub fn save(&mut self) -> io::Result<()> {
        if self.dirty {
            self.handler.write(&self.id, &encode(&self.data))?;
            self.dirty = false;
        } else {
            self.handler.update_timestamp(&self.id)?;
        }
        Ok(())
    }

    pub fn refresh(&self) -> io::Result<bool> {
        self.handler.update_timestamp(&self.id)
    }

    pub fn regenerate_id(&mut self, delete_old: bool) -> io::Result<String> {
        let old_id = self.id.clone();
        let mut random = [0_u8; 24];
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        self.id = hex(&random);
        self.dirty = true;
        if delete_old {
            self.handler.destroy(&old_id)?;
        }
        Ok(self.id.clone())
    }

    pub fn gc(&self) -> io::Result<usize> {
        self.handler.gc(self.lifetime)
    }
}

fn encode(values: &BTreeMap<String, String>) -> Vec<u8> {
    values
        .iter()
        .map(|(key, value)| format!("{}={}", escape(key.as_bytes()), escape(value.as_bytes())))
        .collect::<Vec<_>>()
        .join("&")
        .into_bytes()
}

fn decode(bytes: &[u8]) -> io::Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for pair in bytes
        .split(|byte| *byte == b'&')
        .filter(|pair| !pair.is_empty())
    {
        let position = pair
            .iter()
            .position(|byte| *byte == b'=')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid session data"))?;
        values.insert(
            unescape(&pair[..position])?,
            unescape(&pair[position + 1..])?,
        );
    }
    Ok(values)
}

fn escape(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unescape(bytes: &[u8]) -> io::Result<String> {
    if !bytes.len().is_multiple_of(2) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid hex"));
    }
    let decoded: Result<Vec<_>, _> = bytes
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| ())?;
            u8::from_str_radix(text, 16).map_err(|_| ())
        })
        .collect();
    String::from_utf8(
        decoded.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid hex"))?,
    )
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "session value is not UTF-8"))
}

fn hex(bytes: &[u8]) -> String {
    escape(bytes)
}
