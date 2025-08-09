# Album-Aware Mode Architecture

## Current Architecture

```mermaid
classDiagram
    class AppState {
        +mpd_conn: Arc<RwLock<MpdConn>>
        +song_queue: Arc<RwLock<SongQueue>>
        +tags_data: Arc<RwLock<TagsData>>
    }

    class TagsData {
        +any: Vec<String>
        +not: Vec<String>
        +album_aware: bool
        +album_tags: Vec<String>
        +get_allowed_songs(mpd_client: &mut MpdConn) -> HashSet<HashableSong>
    }

    class SongQueue {
        +inner: VecDeque<mpd::Song>
        +album_aware: bool
        +shuffle_and_add(songs: HashSet<HashableSong>)
    }

    AppState --> TagsData
    AppState --> SongQueue
    TagsData --> SongQueue : Uses album_aware flag

    class DiagramNote {
        +Note: album_aware is currently in both TagsData and SongQueue
        +Note: album_tags determines which playlists to use for album-aware mode
    }
```

## Proposed Architecture

```mermaid
classDiagram
    class AppState {
        +mpd_conn: Arc<RwLock<MpdConn>>
        +song_queue: Arc<RwLock<SongQueue>>
        +tags_data: Arc<RwLock<TagsData>>
        +album_aware: bool  # Moved from TagsData to AppState
    }

    class TagsData {
        +any: Vec<String>
        +not: Vec<String>
        +get_allowed_songs(mpd_client: &mut MpdConn) -> HashSet<HashableSong>
    }

    class SongQueue {
        +inner: VecDeque<mpd::Song>
        +shuffle_and_add(songs: HashSet<HashableSong>)
    }

    AppState --> TagsData
    AppState --> SongQueue

    class DiagramNote {
        +Note: album_aware moved to AppState as a separate setting
        +Note: All decision-making goes through tags in TagsData
        +Note: Simpler architecture with clearer separation of concerns
    }
