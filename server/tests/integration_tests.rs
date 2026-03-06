use jukectl_server::mpd_conn::mock_mpd::MockMpd;
use jukectl_server::mpd_conn::traits::{MpdClient, Song};

#[tokio::test]
async fn test_mock_mpd_listall() {
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
    mock.add_playlist("p1", songs);
    
    let all = MpdClient::listall(&mut mock).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].file, "song1.mp3");
}

#[tokio::test]
async fn test_mock_mpd_pl_push() {
    let mut mock = MockMpd::new();
    MpdClient::pl_push(&mut mock, "p1", "new_song.mp3").unwrap();
    
    let playlist = MpdClient::playlist(&mut mock, "p1").unwrap();
    assert_eq!(playlist.len(), 1);
    assert_eq!(playlist[0].file, "new_song.mp3");
}
