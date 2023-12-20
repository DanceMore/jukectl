 ### startup
 ```mermaid
sequenceDiagram
  participant client
  participant Jukebox
  participant mpd

  Jukebox->>mpd: give me Playlist("jukebox")
  Jukebox->>mpd: give me Playlist("explicit")
  Jukebox->>Jukebox: set Queue
  Note right of Jukebox: Set(jukebox - explicit)
  Jukebox->>Jukebox: enter Main Loop
 ```

### main loop
```mermaid
sequenceDiagram
  participant client
  participant Jukebox
  participant mpd

  critical Every 3 seconds
    Jukebox->>mpd: what is the NowPlaying playlist?
    mpd->>Jukebox: PlaylistResponse[]
    option is Playlist length < 2?
      Jukebox->>mpd: enqueue Song
  end
 ```
