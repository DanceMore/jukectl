use jukectl_server::mpd_conn::mock_mpd::MockMpd;
use jukectl_server::mpd_conn::traits::{MpdClient, Song};

#[tokio::test]
async fn test_mock_mpd_ping() {
    let mut mock = MockMpd::new();
    assert!(MpdClient::ping(&mut mock).is_ok());
}

#[tokio::test]
async fn test_mock_mpd_disconnect() {
    let mut mock = MockMpd::new();
    mock.simulate_disconnect();
    assert!(MpdClient::ping(&mut mock).is_err());
}

#[tokio::test]
async fn test_mock_mpd_reconnect() {
    let mut mock = MockMpd::new();
    mock.simulate_disconnect();
    assert!(MpdClient::ping(&mut mock).is_err());
    mock.simulate_reconnect();
    assert!(MpdClient::ping(&mut mock).is_ok());
}

#[tokio::test]
async fn test_mock_mpd_playlist() {
    let mut mock = MockMpd::new();
    let songs = vec![
        Song {
            file: "song1.mp3".to_string(),
            title: Some("Song 1".to_string()),
            artist: Some("Artist 1".to_string()),
            album: Some("Album 1".to_string()),
            duration: Some(180),
            pos: None,
            id: None,
        },
    ];
    mock.add_playlist("test", songs.clone());
    
    let result = MpdClient::playlist(&mut mock, "test").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].file, "song1.mp3");
}

#[tokio::test]
async fn test_mock_mpd_queue() {
    let mut mock = MockMpd::new();
    MpdClient::push(&mut mock, "song1.mp3").unwrap();
    MpdClient::push(&mut mock, "song2.mp3").unwrap();
    
    let queue = MpdClient::queue(&mut mock).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0].file, "song1.mp3");
    assert_eq!(queue[1].file, "song2.mp3");
}
