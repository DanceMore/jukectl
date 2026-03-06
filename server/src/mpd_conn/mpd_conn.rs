use anyhow::Result;
use std::env;

use crate::mpd_conn::mock_mpd::MockMpd;
use crate::mpd_conn::traits::{MpdClient, Playlist, Query, Song};
use crate::mpd_conn::raw_client::RawMpdClient;
use log::{debug, info};

pub enum MpdBackend {
    Real(RawMpdClient),
    Mock(MockMpd),
}

impl MpdClient for MpdBackend {
    fn ping(&mut self) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.ping(),
            MpdBackend::Mock(m) => m.ping(),
        }
    }

    fn playlist(&mut self, name: &str) -> Result<Vec<Song>> {
        match self {
            MpdBackend::Real(c) => c.get_playlist_songs(name),
            MpdBackend::Mock(m) => m.playlist(name),
        }
    }

    fn playlists(&mut self) -> Result<Vec<Playlist>> {
        match self {
            MpdBackend::Real(c) => c.list_playlists(),
            MpdBackend::Mock(m) => m.playlists(),
        }
    }

    fn queue(&mut self) -> Result<Vec<Song>> {
        match self {
            MpdBackend::Real(c) => c.get_queue(),
            MpdBackend::Mock(m) => m.queue(),
        }
    }

    fn search(&mut self, query: &Query, _window: Option<(u32, u32)>) -> Result<Vec<Song>> {
        match self {
            MpdBackend::Real(c) => c.search(query),
            MpdBackend::Mock(m) => m.search(query, _window),
        }
    }

    fn consume(&mut self, state: bool) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.set_consume(state),
            MpdBackend::Mock(m) => m.consume(state),
        }
    }

    fn push(&mut self, file: &str) -> Result<u32> {
        match self {
            MpdBackend::Real(c) => {
                c.queue_add(file)?;
                Ok(0)
            }
            MpdBackend::Mock(m) => m.push(file),
        }
    }

    fn delete(&mut self, pos: u32) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.queue_delete(pos),
            MpdBackend::Mock(m) => m.delete(pos),
        }
    }

    fn play(&mut self) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.play(),
            MpdBackend::Mock(m) => m.play(),
        }
    }

    fn pl_push(&mut self, playlist: &str, file: &str) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.playlist_add(playlist, file),
            MpdBackend::Mock(m) => m.pl_push(playlist, file),
        }
    }

    fn pl_delete(&mut self, playlist: &str, pos: u32) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.playlist_delete(playlist, pos),
            MpdBackend::Mock(m) => m.pl_delete(playlist, pos),
        }
    }

    fn pl_remove(&mut self, playlist: &str) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.playlist_clear(playlist),
            MpdBackend::Mock(m) => m.pl_remove(playlist),
        }
    }

    fn listall(&mut self) -> Result<Vec<Song>> {
        match self {
            MpdBackend::Real(c) => c.list_all_songs(),
            MpdBackend::Mock(m) => m.listall(),
        }
    }
}

pub struct MpdConn {
    pub mpd: MpdBackend,
    address: String,
    port: u16,
    is_dev_mode: bool,
}

impl MpdConn {
    pub fn new() -> Result<Self> {
        let is_dev_mode = env::var("JUKECTL_DEV_MODE").unwrap_or_default() == "1";

        if is_dev_mode {
            info!("[!] JUKECTL_DEV_MODE is enabled, using MockMpd");
            return Ok(MpdConn {
                mpd: MpdBackend::Mock(MockMpd::new()),
                address: "mock".to_string(),
                port: 0,
                is_dev_mode: true,
            });
        }

        debug!("[!] connecting to mpd...");
        let host = env::var("MPD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port: u16 = env::var("MPD_PORT")
            .unwrap_or_else(|_| "6600".to_string())
            .parse()
            .unwrap_or(6600);

        let mpd = RawMpdClient::connect(&host, port)?;
        mpd.set_consume(true)?;

        Ok(MpdConn {
            mpd: MpdBackend::Real(mpd),
            address: host,
            port,
            is_dev_mode: false,
        })
    }

    pub fn new_with_host(host: &str, port: u16) -> Result<Self> {
        let is_dev_mode = env::var("JUKECTL_DEV_MODE").unwrap_or_default() == "1";

        if is_dev_mode {
            return Ok(MpdConn {
                mpd: MpdBackend::Mock(MockMpd::new()),
                address: "mock".to_string(),
                port: 0,
                is_dev_mode: true,
            });
        }

        debug!("[!] connecting to mpd at {}:{}...", host, port);
        let mpd = RawMpdClient::connect(host, port)?;
        mpd.set_consume(true)?;

        Ok(MpdConn {
            mpd: MpdBackend::Real(mpd),
            address: host.to_string(),
            port,
            is_dev_mode: false,
        })
    }

    pub fn get_host_info(&self) -> (String, u16) {
        (self.address.clone(), self.port)
    }

    pub fn reconnect(&mut self) -> Result<()> {
        if self.is_dev_mode {
            return Ok(());
        }

        if !self.is_connected() {
            debug!("[!] Reconnecting to mpd...");
            let mpd = RawMpdClient::connect(&self.address, self.port)?;
            mpd.set_consume(true)?;
            self.mpd = MpdBackend::Real(mpd);
        }
        Ok(())
    }

    fn is_connected(&mut self) -> bool {
        if self.is_dev_mode {
            return true;
        }
        self.ping().is_ok()
    }

    pub fn ping(&mut self) -> Result<()> {
        match &mut self.mpd {
            MpdBackend::Real(c) => c.ping(),
            MpdBackend::Mock(m) => m.ping(),
        }
    }
}
