use jukectl_mpdclient_sys::*;
use std::ffi::{CStr, CString};
use std::ptr;
use anyhow::{anyhow, Result};
use crate::mpd_conn::traits::{Song, Playlist, Query, FilterTerm};

pub struct RawMpdClient {
    conn: *mut mpd_connection,
}

unsafe impl Send for RawMpdClient {}

impl RawMpdClient {
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        let host_c = CString::new(host)?;
        let conn = unsafe { mpd_connection_new(host_c.as_ptr(), port as u32, 30000) };
        
        if conn.is_null() {
            return Err(anyhow!("Out of memory creating MPD connection"));
        }

        let client = RawMpdClient { conn };
        client.check_error()?;
        
        Ok(client)
    }

    fn check_error(&self) -> Result<()> {
        unsafe {
            let error = mpd_connection_get_error(self.conn);
            if error != mpd_error_MPD_ERROR_SUCCESS {
                let msg = mpd_connection_get_error_message(self.conn);
                let msg_str = if msg.is_null() {
                    "Unknown error"
                } else {
                    CStr::from_ptr(msg).to_str().unwrap_or("Invalid UTF-8 error message")
                };
                return Err(anyhow!("MPD Error ({}): {}", error, msg_str));
            }
        }
        Ok(())
    }

    pub fn ping(&self) -> Result<()> {
        let ping_c = CString::new("ping")?;
        unsafe {
            if !mpd_send_command(self.conn, ping_c.as_ptr(), ptr::null::<::std::os::raw::c_char>()) {
                self.check_error()?;
            }
            if !mpd_response_finish(self.conn) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn set_consume(&self, state: bool) -> Result<()> {
        unsafe {
            if !mpd_run_consume(self.conn, state) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn play(&self) -> Result<()> {
        unsafe {
            if !mpd_run_play(self.conn) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn queue_add(&self, file: &str) -> Result<()> {
        let file_c = CString::new(file)?;
        unsafe {
            if !mpd_run_add(self.conn, file_c.as_ptr()) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn queue_delete(&self, pos: u32) -> Result<()> {
        unsafe {
            if !mpd_run_delete(self.conn, pos) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn get_queue(&self) -> Result<Vec<Song>> {
        let mut songs = Vec::new();
        unsafe {
            if !mpd_send_list_queue_meta(self.conn) {
                self.check_error()?;
            }

            while let Some(song) = self.recv_song()? {
                songs.push(song);
            }

            if !mpd_response_finish(self.conn) {
                self.check_error()?;
            }
        }
        Ok(songs)
    }

    pub fn list_all_songs(&self) -> Result<Vec<Song>> {
        let mut songs = Vec::new();
        unsafe {
            if !mpd_send_list_all_meta(self.conn, ptr::null()) {
                self.check_error()?;
            }

            while let Some(song) = self.recv_song()? {
                songs.push(song);
            }

            if !mpd_response_finish(self.conn) {
                self.check_error()?;
            }
        }
        Ok(songs)
    }

    pub fn list_playlists(&self) -> Result<Vec<Playlist>> {
        let mut playlists = Vec::new();
        unsafe {
            if !mpd_send_list_playlists(self.conn) {
                self.check_error()?;
            }

            loop {
                let pl = mpd_recv_playlist(self.conn);
                if pl.is_null() {
                    break;
                }
                let name = mpd_playlist_get_path(pl);
                if !name.is_null() {
                    playlists.push(Playlist {
                        name: CStr::from_ptr(name).to_string_lossy().into_owned(),
                    });
                }
                mpd_playlist_free(pl);
            }

            if !mpd_response_finish(self.conn) {
                self.check_error()?;
            }
        }
        Ok(playlists)
    }

    pub fn get_playlist_songs(&self, name: &str) -> Result<Vec<Song>> {
        let name_c = CString::new(name)?;
        let mut songs = Vec::new();
        unsafe {
            if !mpd_send_list_playlist_meta(self.conn, name_c.as_ptr()) {
                self.check_error()?;
            }

            while let Some(song) = self.recv_song()? {
                songs.push(song);
            }

            if !mpd_response_finish(self.conn) {
                self.check_error()?;
            }
        }
        Ok(songs)
    }

    pub fn playlist_add(&self, playlist: &str, file: &str) -> Result<()> {
        let pl_c = CString::new(playlist)?;
        let file_c = CString::new(file)?;
        unsafe {
            if !mpd_run_playlist_add(self.conn, pl_c.as_ptr(), file_c.as_ptr()) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn playlist_delete(&self, playlist: &str, pos: u32) -> Result<()> {
        let pl_c = CString::new(playlist)?;
        unsafe {
            if !mpd_run_playlist_delete(self.conn, pl_c.as_ptr(), pos) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn playlist_clear(&self, playlist: &str) -> Result<()> {
        let pl_c = CString::new(playlist)?;
        unsafe {
            if !mpd_run_playlist_clear(self.conn, pl_c.as_ptr()) {
                self.check_error()?;
            }
        }
        Ok(())
    }

    pub fn search(&self, query: &Query) -> Result<Vec<Song>> {
        unsafe {
            if !mpd_search_db_songs(self.conn, true) {
                self.check_error()?;
            }

            for term in &query.terms {
                match term {
                    FilterTerm::Any(val) => {
                        let val_c = CString::new(val.as_str())?;
                        if !mpd_search_add_any_tag_constraint(self.conn, mpd_operator_MPD_OPERATOR_DEFAULT, val_c.as_ptr()) {
                            return Err(anyhow!("Failed to add search constraint"));
                        }
                    }
                    FilterTerm::Tag(tag, val) => {
                        let tag_type = self.map_tag_name(tag);
                        let val_c = CString::new(val.as_str())?;
                        if !mpd_search_add_tag_constraint(self.conn, mpd_operator_MPD_OPERATOR_DEFAULT, tag_type, val_c.as_ptr()) {
                            return Err(anyhow!("Failed to add tag constraint"));
                        }
                    }
                }
            }

            if !mpd_search_commit(self.conn) {
                self.check_error()?;
            }

            let mut songs = Vec::new();
            while let Some(song) = self.recv_song()? {
                songs.push(song);
            }

            if !mpd_response_finish(self.conn) {
                self.check_error()?;
            }

            Ok(songs)
        }
    }

    fn recv_song(&self) -> Result<Option<Song>> {
        unsafe {
            let song_ptr = mpd_recv_song(self.conn);
            if song_ptr.is_null() {
                return Ok(None);
            }

            let file = CStr::from_ptr(mpd_song_get_uri(song_ptr)).to_string_lossy().into_owned();
            
            let title = self.get_tag(song_ptr, mpd_tag_type_MPD_TAG_TITLE);
            let artist = self.get_tag(song_ptr, mpd_tag_type_MPD_TAG_ARTIST);
            let album = self.get_tag(song_ptr, mpd_tag_type_MPD_TAG_ALBUM);
            let duration = mpd_song_get_duration(song_ptr);
            let pos = mpd_song_get_pos(song_ptr);
            let id = mpd_song_get_id(song_ptr);

            mpd_song_free(song_ptr);

            Ok(Some(Song {
                file,
                title,
                artist,
                album,
                duration: if duration > 0 { Some(duration) } else { None },
                pos: if pos != u32::MAX { Some(pos) } else { None },
                id: if id != u32::MAX { Some(id) } else { None },
            }))
        }
    }

    fn get_tag(&self, song: *const mpd_song, tag: mpd_tag_type) -> Option<String> {
        unsafe {
            let val = mpd_song_get_tag(song, tag, 0);
            if val.is_null() {
                None
            } else {
                Some(CStr::from_ptr(val).to_string_lossy().into_owned())
            }
        }
    }

    fn map_tag_name(&self, name: &str) -> mpd_tag_type {
        match name.to_lowercase().as_str() {
            "artist" => mpd_tag_type_MPD_TAG_ARTIST,
            "album" => mpd_tag_type_MPD_TAG_ALBUM,
            "title" => mpd_tag_type_MPD_TAG_TITLE,
            "genre" => mpd_tag_type_MPD_TAG_GENRE,
            _ => mpd_tag_type_MPD_TAG_UNKNOWN,
        }
    }
}

impl Drop for RawMpdClient {
    fn drop(&mut self) {
        unsafe {
            mpd_connection_free(self.conn);
        }
    }
}
