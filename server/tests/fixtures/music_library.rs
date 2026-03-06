use mpd::Song;

pub fn realish_library() -> Vec<Song> {
    vec![
        // ============================================
        // NORMAL ALBUMS - Well-organized
        // ============================================

        // Pink Floyd - Dark Side of the Moon (canonical rock album)
        mk_song(
            "music/pink_floyd/dark_side/01_speak_to_me.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Speak to Me",
            1,
        ),
        mk_song(
            "music/pink_floyd/dark_side/02_breathe.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Breathe",
            2,
        ),
        mk_song(
            "music/pink_floyd/dark_side/03_on_the_run.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "On the Run",
            3,
        ),
        mk_song(
            "music/pink_floyd/dark_side/04_time.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Time",
            4,
        ),
        mk_song(
            "music/pink_floyd/dark_side/05_the_great_gig_in_the_sky.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "The Great Gig in the Sky",
            5,
        ),
        mk_song(
            "music/pink_floyd/dark_side/06_money.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Money",
            6,
        ),
        mk_song(
            "music/pink_floyd/dark_side/07_us_and_them.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Us and Them",
            7,
        ),
        mk_song(
            "music/pink_floyd/dark_side/08_any_colour_you_like.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Any Colour You Like",
            8,
        ),
        mk_song(
            "music/pink_floyd/dark_side/09_brain_damage.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Brain Damage",
            9,
        ),
        mk_song(
            "music/pink_floyd/dark_side/10_eclipse.mp3",
            "Pink Floyd",
            "Dark Side of the Moon",
            "Eclipse",
            10,
        ),
        // Led Zeppelin IV (track 2 is missing from folder!)
        mk_song(
            "music/led_zeppelin/iv/black_dog.mp3",
            "Led Zeppelin",
            "Led Zeppelin IV",
            "Black Dog",
            1,
        ),
        mk_song(
            "music/led_zeppelin/iv/rock_and_roll.mp3",
            "Led Zeppelin",
            "Led Zeppelin IV",
            "Rock and Roll",
            3,
        ), // Track 2 is missing!
        mk_song(
            "music/led_zeppelin/iv/stairway_to_heaven.mp3",
            "Led Zeppelin",
            "Led Zeppelin IV",
            "Stairway to Heaven",
            5,
        ),
        // ============================================
        // COMPILATIONS - The messiest category
        // ============================================

        // NOW That's What I Call Music! series
        mk_song(
            "music/compilations/now01/01_adele_hello.mp3",
            "Adele",
            "Now That's What I Call Music! 63",
            "Hello",
            1,
        ),
        mk_song(
            "music/compilations/now01/02_ed_sheeran_shape.mp3",
            "Ed Sheeran",
            "Now That's What I Call Music! 63",
            "Shape of You",
            2,
        ),
        // Another NOW album
        mk_song(
            "music/compilations/now02/01_drake_hotline_bling.mp3",
            "Drake",
            "Now That's What I Call Music! 64",
            "Hotline Bling",
            1,
        ),
        // Soundtracks with multiple artists
        mk_song(
            "music/soundtracks/guardians/01_hookman.mp3",
            "Blue Swede",
            "Guardians of the Galaxy: Awesome Mix Vol. 1",
            "Hooked on a Feeling",
            1,
        ),
        mk_song(
            "music/soundtracks/guardians/02_go_all_the_way.mp3",
            "The Raspberries",
            "Guardians of the Galaxy: Awesome Mix Vol. 1",
            "Go All the Way",
            2,
        ),
        // ============================================
        // SAME ALBUM NAME, DIFFERENT ARTISTS - CRITICAL
        // ============================================
        mk_song(
            "music/the_band/greatest_hits/01_up_on_cripple_creek.mp3",
            "The Band",
            "Greatest Hits",
            "Up on Cripple Creek",
            1,
        ),
        mk_song(
            "music/the_band/greatest_hits/02_the_weight.mp3",
            "The Band",
            "Greatest Hits",
            "The Weight",
            2,
        ),
        mk_song(
            "music/different_band/greatest_hits/01_another_song.mp3",
            "Different Band",
            "Greatest Hits",
            "Another Song",
            1,
        ),
        mk_song(
            "music/different_band/greatest_hits/02_yet_another.mp3",
            "Different Band",
            "Greatest Hits",
            "Yet Another",
            2,
        ),
        // ============================================
        // REMASTERS / REISSUES
        // ============================================
        mk_song(
            "music/miles_davis/kind_of_blue/original/01_so_what.mp3",
            "Miles Davis",
            "Kind of Blue (Original)",
            "So What",
            1,
        ),
        mk_song(
            "music/miles_davis/kind_of_blue/remastered/01_so_what.mp3",
            "Miles Davis",
            "Kind of Blue (Remastered)",
            "So What",
            1,
        ),
        // ============================================
        // LIVE ALBUMS
        // ============================================
        mk_song(
            "music/led_zeppelin/live_rah/01_whole_lotta_love.mp3",
            "Led Zeppelin",
            "Live at Royal Albert Hall",
            "Whole Lotta Love",
            1,
        ),
        mk_song(
            "music/led_zeppelin/live_rah/02_immigrant_song.mp3",
            "Led Zeppelin",
            "Live at Royal Albert Hall",
            "Immigrant Song",
            2,
        ),
        // ============================================
        // COLLABORATIONS / FEATURING
        // ============================================
        mk_song(
            "music/collabs/01_rockabye.mp3",
            "Clean Bandit feat. Sean Paul",
            "Symphony",
            "Rockabye",
            1,
        ),
        mk_song(
            "music/collabs/02_symphony.mp3",
            "Clean Bandit feat. Zara Larsson",
            "Symphony",
            "Symphony",
            2,
        ),
        // ============================================
        // CLASSICAL
        // ============================================
        mk_song(
            "music/beethoven/symphony_9/choral/01_Allegro.mp3",
            "Ludwig van Beethoven",
            "Symphony No. 9",
            "I. Allegro ma non troppo",
            1,
        ),
        mk_song(
            "music/beethoven/symphony_9/choral/02_Molto_allegro.mp3",
            "Ludwig van Beethoven",
            "Symphony No. 9",
            "II. Molto allegro",
            2,
        ),
        // Different conductor = different album
        mk_song(
            "music/furtwangler/beethoven_9/01_Allegro.mp3",
            "Furtwangler",
            "Beethoven: Symphony No. 9",
            "I. Allegro",
            1,
        ),
        // ============================================
        // MISSING / INCOMPLETE METADATA
        // The reality of old CD rips
        // ============================================

        // No artist
        mk_song(
            "music/unknown/01_unknown_track.mp3",
            "",
            "Unknown Album",
            "Unknown Track 1",
            1,
        ),
        mk_song(
            "music/unknown/02_unknown_track.mp3",
            "",
            "Unknown Album",
            "Unknown Track 2",
            2,
        ),
        // No track number
        mk_song(
            "music/old_cd_rips/track_name.mp3",
            "Old Artist",
            "Old Album",
            "Track Without Number",
            None,
        ),
        // Track number as string
        mk_song(
            "music/inconsistent/track_1.mp3",
            "Artist",
            "Album",
            "Track One",
            1,
        ),
        mk_song(
            "music/inconsistent/track_2.mp3",
            "Artist",
            "Album",
            "Track Two",
            2,
        ),
    ]
}

fn mk_song(path: &str, artist: &str, album: &str, title: &str, track: Option<u32>) -> Song {
    let mut song = Song::default();
    song.file = path.to_string();
    song.tags = vec![
        ("Artist".to_string(), artist.to_string()),
        ("Album".to_string(), album.to_string()),
        ("Title".to_string(), title.to_string()),
    ];
    if let Some(t) = track {
        song.tags.push(("Track".to_string(), t.to_string()));
    }
    song
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_has_pink_floyd() {
        let lib = realish_library();
        let pf_songs: Vec<_> = lib
            .iter()
            .filter(|s| s.tags.iter().any(|(_, v)| v == "Pink Floyd"))
            .collect();
        assert!(pf_songs.len() >= 10, "Should have Dark Side of Moon");
    }

    #[test]
    fn test_library_has_compilations() {
        let lib = realish_library();
        let now_songs: Vec<_> = lib
            .iter()
            .filter(|s| {
                s.tags
                    .iter()
                    .any(|(_, v)| v.contains("Now That's What I Call Music"))
            })
            .collect();
        assert!(now_songs.len() >= 2, "Should have NOW compilations");
    }

    #[test]
    fn test_library_has_same_album_different_artist() {
        let lib = realish_library();
        let greatest_hits: Vec<_> = lib
            .iter()
            .filter(|s| s.tags.iter().any(|(_, v)| v == "Greatest Hits"))
            .collect();
        // Should have 2 different artists
        let artists: std::collections::HashSet<_> = greatest_hits
            .iter()
            .filter_map(|s| s.tags.iter().find(|(k, _)| k == "Artist"))
            .map(|(_, v)| v.clone())
            .collect();
        assert!(
            artists.len() >= 2,
            "Should have Greatest Hits from different artists"
        );
    }
}
