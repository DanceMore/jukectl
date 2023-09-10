#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Queue {
    songs: Vec<mpd::Song>,
}

impl Queue {
    pub fn new() -> Self {
        Queue { songs: Vec::new() }
    }

    fn push(&mut self, song: mpd::Song) {
        self.songs.push(song);
    }

    fn pop(&mut self) -> Option<mpd::Song> {
        self.songs.pop()
    }

    // Other methods for managing the queue
}
