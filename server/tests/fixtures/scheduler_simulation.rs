use mpd::Song;

#[derive(Debug, Clone)]
pub struct SchedulerSnapshot {
    pub tick: u64,
    pub internal_queue_len: usize,
    pub mpd_queue_len: usize,
    pub current_tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    Tick(u64),
    TagsChanged { from: Vec<String>, to: Vec<String> },
    QueueDrained { count: usize },
    Refill { song_count: usize },
    CacheInvalidated,
}

#[derive(Debug, Clone)]
pub struct SchedulerTimeline {
    pub snapshots: Vec<SchedulerSnapshot>,
    pub events: Vec<SchedulerEvent>,
}

#[derive(Debug, Clone)]
pub struct TestScenario {
    pub initial_songs: Vec<Song>,
    pub initial_tags: Vec<String>,
    pub timeline: SchedulerTimeline,
}

pub struct ScenarioBuilder {
    initial_songs: Vec<Song>,
    initial_tags: Vec<String>,
    timeline_events: Vec<SchedulerEvent>,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        ScenarioBuilder {
            initial_songs: Vec::new(),
            initial_tags: vec!["jukebox".to_string()],
            timeline_events: Vec::new(),
        }
    }

    pub fn with_library(mut self, songs: Vec<Song>) -> Self {
        self.initial_songs = songs;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.initial_tags = tags;
        self
    }

    pub fn tag_change_at(mut self, tick: u64, new_tags: Vec<String>) -> Self {
        self.timeline_events.push(SchedulerEvent::TagsChanged {
            from: self.initial_tags.clone(),
            to: new_tags,
        });
        self
    }

    pub fn drain_at(mut self, tick: u64, count: usize) -> Self {
        self.timeline_events
            .push(SchedulerEvent::QueueDrained { count });
        self
    }

    pub fn build(self) -> TestScenario {
        TestScenario {
            initial_songs: self.initial_songs,
            initial_tags: self.initial_tags,
            timeline: SchedulerTimeline {
                snapshots: Vec::new(),
                events: self.timeline_events,
            },
        }
    }
}

pub fn scenario_refill_on_empty() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["rock".to_string()])
        .with_library(sample_rock_library())
        .build()
}

pub fn scenario_tag_hot_swap() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["rock".to_string()])
        .with_library(sample_rock_library())
        .tag_change_at(5, vec!["jazz".to_string()])
        .build()
}

pub fn scenario_rapid_drain() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["music".to_string()])
        .with_library(sample_rock_library())
        .drain_at(3, 5)
        .drain_at(4, 3)
        .build()
}

pub fn scenario_empty_library() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["empty_tag".to_string()])
        .with_library(Vec::new())
        .build()
}

pub fn scenario_cache_invalidation() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["rock".to_string()])
        .with_library(sample_rock_library())
        .tag_change_at(3, vec!["pop".to_string()])
        .tag_change_at(6, vec!["rock".to_string()])
        .build()
}

fn sample_rock_library() -> Vec<Song> {
    vec![
        mk_song("rock/artist1/album1/track1.mp3", "Artist1", "Album1", 1),
        mk_song("rock/artist1/album1/track2.mp3", "Artist1", "Album1", 2),
        mk_song("rock/artist2/album2/track1.mp3", "Artist2", "Album2", 1),
        mk_song("rock/artist2/album2/track2.mp3", "Artist2", "Album2", 2),
        mk_song("rock/artist3/album3/track1.mp3", "Artist3", "Album3", 1),
        mk_song("rock/artist3/album3/track2.mp3", "Artist3", "Album3", 2),
        mk_song("rock/artist3/album3/track3.mp3", "Artist3", "Album3", 3),
        mk_song("rock/artist4/album4/track1.mp3", "Artist4", "Album4", 1),
        mk_song("rock/artist4/album4/track2.mp3", "Artist4", "Album4", 2),
        mk_song("rock/artist4/album4/track3.mp3", "Artist4", "Album4", 3),
    ]
}

fn mk_song(path: &str, artist: &str, album: &str, track: u32) -> Song {
    let mut song = Song::default();
    song.file = path.to_string();
    song.tags = vec![
        ("Artist".to_string(), artist.to_string()),
        ("Album".to_string(), album.to_string()),
        ("Track".to_string(), track.to_string()),
    ];
    song
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_refill_has_songs() {
        let scenario = scenario_refill_on_empty();
        assert!(!scenario.initial_songs.is_empty());
    }

    #[test]
    fn test_scenario_empty_library_has_no_songs() {
        let scenario = scenario_empty_library();
        assert!(scenario.initial_songs.is_empty());
    }

    #[test]
    fn test_scenario_tag_hot_swap_has_tag_change() {
        let scenario = scenario_tag_hot_swap();
        let has_tag_change = scenario
            .timeline
            .events
            .iter()
            .any(|e| matches!(e, SchedulerEvent::TagsChanged { .. }));
        assert!(has_tag_change);
    }
}
