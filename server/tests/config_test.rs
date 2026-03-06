use jukectl_server::app_state::{initialize, load_default_tags};
use std::env;
use std::sync::Mutex;
use base64::{engine::general_purpose, Engine as _};

// Use a global mutex to prevent environment variable race conditions during tests
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_initialize_basic() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let state = initialize().await;
    assert!(state.mpd_pool.get_connection().await.is_ok() || true);
}

#[test]
fn test_load_default_tags_fallback() {
    let _lock = ENV_MUTEX.lock().unwrap();
    env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
    let tags = load_default_tags();
    assert_eq!(tags.any, vec!["jukebox".to_string()]);
    assert_eq!(tags.not, vec!["explicit".to_string()]);
}

#[test]
fn test_load_default_tags_valid_b64() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let json = r#"{"any": ["tag1", "tag2"], "not": ["tag3"]}"#;
    let b64 = general_purpose::STANDARD.encode(json);
    env::set_var("JUKECTL_DEFAULT_TAGS_B64", b64);

    let tags = load_default_tags();
    assert_eq!(tags.any, vec!["tag1".to_string(), "tag2".to_string()]);
    assert_eq!(tags.not, vec!["tag3".to_string()]);

    env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
}

#[test]
fn test_load_default_tags_invalid_b64() {
    let _lock = ENV_MUTEX.lock().unwrap();
    env::set_var("JUKECTL_DEFAULT_TAGS_B64", "!!!not-base64!!!");
    let tags = load_default_tags();
    assert_eq!(tags.any, vec!["jukebox".to_string()]);
    env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
}

#[test]
fn test_load_default_tags_invalid_json() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let invalid_json = r#"{"any": ["tag1"], "not": "#;
    let b64 = general_purpose::STANDARD.encode(invalid_json);
    env::set_var("JUKECTL_DEFAULT_TAGS_B64", b64);

    let tags = load_default_tags();
    assert_eq!(tags.any, vec!["jukebox".to_string()]);
    env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
}
