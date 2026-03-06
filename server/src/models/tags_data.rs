use serde::{Deserialize, Serialize};
use crate::mpd_conn::traits::{Playlist, Song};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagValue {
    pub name: String,
    pub count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagsResponse {
    pub artists: Vec<TagValue>,
    pub albums: Vec<TagValue>,
    pub genres: Vec<TagValue>,
    pub playlists: Vec<TagValue>,
}

impl TagsResponse {
    pub fn new() -> Self {
        TagsResponse {
            artists: Vec::new(),
            albums: Vec::new(),
            genres: Vec::new(),
            playlists: Vec::new(),
        }
    }

    pub fn to_api_response(songs: Vec<Song>, playlists: Vec<Playlist>) -> Self {
        let mut response = TagsResponse::new();
        let mut artists_map = std::collections::HashMap::new();
        let mut albums_map = std::collections::HashMap::new();

        for song in songs {
            if let Some(artist) = song.artist {
                *artists_map.entry(artist).or_insert(0) += 1;
            }
            if let Some(album) = song.album {
                *albums_map.entry(album).or_insert(0) += 1;
            }
        }

        response.artists = artists_map
            .into_iter()
            .map(|(name, count)| TagValue { name, count })
            .collect();
        response.albums = albums_map
            .into_iter()
            .map(|(name, count)| TagValue { name, count })
            .collect();

        response.playlists = playlists
            .into_iter()
            .map(|p| TagValue {
                name: p.name,
                count: 0,
            })
            .collect();

        response.artists.sort_by(|a, b| a.name.cmp(&b.name));
        response.albums.sort_by(|a, b| a.name.cmp(&b.name));
        response.playlists.sort_by(|a, b| a.name.cmp(&b.name));

        response
    }
}
