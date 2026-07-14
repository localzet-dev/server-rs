use std::fs;
use std::time::Duration;

use server_rs::protocols::http::session::{FileSessionHandler, SessionHandler};

#[test]
fn file_session_handler_supports_full_lifecycle() {
    let directory = std::env::temp_dir().join(format!(
        "server-rs-session-test-{}-{}",
        std::process::id(),
        unique_id()
    ));
    let handler = FileSessionHandler::new(&directory).unwrap();

    handler.write("safe_session_1", b"payload").unwrap();
    assert_eq!(
        handler
            .read("safe_session_1", Duration::from_secs(60))
            .unwrap(),
        Some(b"payload".to_vec())
    );
    assert!(handler.update_timestamp("safe_session_1").unwrap());
    assert!(handler.destroy("safe_session_1").unwrap());
    assert_eq!(
        handler
            .read("safe_session_1", Duration::from_secs(60))
            .unwrap(),
        None
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn file_session_handler_rejects_path_traversal() {
    let handler = FileSessionHandler::in_temporary_directory().unwrap();
    assert!(handler.write("../outside", b"unsafe").is_err());
}

fn unique_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
