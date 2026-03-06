use jukectl_server::app_state::load_default_tags;
use std::env;
use base64::{engine::general_purpose, Engine as _};

#[test]
fn test_load_default_tags_fallback() {
    env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
    let tags = load_default_tags();
    assert_eq!(tags.any, vec!["jukebox".to_string()]);
    assert_eq!(tags.not, vec!["explicit".to_string()]);
}

#[test]
fn test_load_default_tags_valid_b64() {
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
    env::set_var("JUKECTL_DEFAULT_TAGS_B64", "!!!not-base64!!!");
    let tags = load_default_tags();
    // Should fallback
    assert_eq!(tags.any, vec!["jukebox".to_string()]);

    env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
}

#[test]
fn test_load_default_tags_invalid_json() {
    let invalid_json = r#"{"any": ["tag1"], "not": "#; // missing value
    let b64 = general_purpose::STANDARD.encode(invalid_json);
    env::set_var("JUKECTL_DEFAULT_TAGS_B64", b64);

    let tags = load_default_tags();
    // Should fallback
    assert_eq!(tags.any, vec!["jukebox".to_string()]);

    env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
}
