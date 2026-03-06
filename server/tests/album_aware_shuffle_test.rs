use jukectl_server::models::song_queue::{DequeueMode, SongQueue};
use jukectl_server::mpd_conn::mock_mpd::MockMpd;
use jukectl_server::mpd_conn::traits::Song;
use jukectl_server::mpd_conn::mpd_pool::MpdPool;

#[tokio::test]
async fn test_album_aware_shuffle_basic() {
    let mut queue = SongQueue::new();
    queue.set_album_aware(true);
    
    let song1 = Song {
        file: "artist1/album1/01.mp3".to_string(),
        title: Some("Song 1".to_string()),
        artist: Some("Artist 1".to_string()),
        album: Some("Album 1".to_string()),
        duration: Some(180),
        pos: Some(0),
        id: Some(1),
    };
    
    queue.add(song1);
    assert_eq!(queue.len(), 1);
}
