use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use super::SessionHandler;

const SESSION_FILE_PREFIX: &str = "session_";
static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct FileSessionHandler {
    save_path: PathBuf,
}

impl FileSessionHandler {
    pub fn new(save_path: impl Into<PathBuf>) -> io::Result<Self> {
        let save_path = save_path.into();
        fs::create_dir_all(&save_path)?;
        Ok(Self { save_path })
    }

    pub fn in_temporary_directory() -> io::Result<Self> {
        Self::new(std::env::temp_dir().join("localzet-server-sessions"))
    }

    pub fn save_path(&self) -> &Path {
        &self.save_path
    }

    fn session_file(&self, session_id: &str) -> io::Result<PathBuf> {
        validate_session_id(session_id)?;
        Ok(self
            .save_path
            .join(format!("{SESSION_FILE_PREFIX}{session_id}")))
    }
}

impl SessionHandler for FileSessionHandler {
    fn read(&self, session_id: &str, lifetime: Duration) -> io::Result<Option<Vec<u8>>> {
        let path = self.session_file(session_id)?;
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let expired = metadata
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age > lifetime);
        if expired {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            return Ok(None);
        }
        fs::read(path).map(Some)
    }

    fn write(&self, session_id: &str, session_data: &[u8]) -> io::Result<()> {
        let destination = self.session_file(session_id)?;
        let temp_id = TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let temporary = self.save_path.join(format!(
            ".{SESSION_FILE_PREFIX}{}.{}.tmp",
            std::process::id(),
            temp_id
        ));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        if let Err(error) = (|| {
            file.write_all(session_data)?;
            file.sync_all()?;
            drop(file);
            replace_file(&temporary, &destination)
        })() {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(())
    }

    fn update_timestamp(&self, session_id: &str) -> io::Result<bool> {
        let path = self.session_file(session_id)?;
        let file = match OpenOptions::new().write(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        file.set_modified(SystemTime::now())?;
        Ok(true)
    }

    fn destroy(&self, session_id: &str) -> io::Result<bool> {
        match fs::remove_file(self.session_file(session_id)?) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn gc(&self, max_lifetime: Duration) -> io::Result<usize> {
        let now = SystemTime::now();
        let mut removed = 0;
        for entry in fs::read_dir(&self.save_path)? {
            let entry = entry?;
            if !entry
                .file_name()
                .to_string_lossy()
                .starts_with(SESSION_FILE_PREFIX)
            {
                continue;
            }
            let metadata = entry.metadata()?;
            let expired = metadata
                .modified()
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age > max_lifetime);
            if metadata.is_file() && expired {
                fs::remove_file(entry.path())?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

fn validate_session_id(session_id: &str) -> io::Result<()> {
    if session_id.is_empty()
        || !session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "session id contains invalid characters",
        ));
    }
    Ok(())
}

fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(destination)?;
            fs::rename(source, destination)
        }
        Err(error) => Err(error),
    }
}
