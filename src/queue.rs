#[derive(Serialize, Deserialize)]
struct TagsData {
    any: Vec<String>,
    not: Vec<String>,
}

struct Queue {
    songs: Vec<mpd::Song>,
}

impl Queue {
    fn new() -> Self {
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
