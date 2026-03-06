use mpd::{error::Result, Client, Query};
use std::env;
use std::net::ToSocketAddrs;
use std::sync::OnceLock;

use crate::mpd_conn::mock_mpd::MockMpd;
use crate::mpd_conn::traits::MpdClient;
use log::{debug, warn};

pub enum MpdBackend {
    Real(Client),
    Mock(MockMpd),
}

impl MpdClient for MpdBackend {
    fn ping(&mut self) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.ping(),
            MpdBackend::Mock(m) => m.ping(),
        }
    }

    fn playlist(&mut self, name: &str) -> Result<Vec<mpd::Song>> {
        match self {
            MpdBackend::Real(c) => c.playlist(name),
            MpdBackend::Mock(m) => m.playlist(name),
        }
    }

    fn playlists(&mut self) -> Result<Vec<mpd::Playlist>> {
        match self {
            MpdBackend::Real(c) => c.playlists(),
            MpdBackend::Mock(m) => m.playlists(),
        }
    }

    fn queue(&mut self) -> Result<Vec<mpd::Song>> {
        match self {
            MpdBackend::Real(c) => c.queue(),
            MpdBackend::Mock(m) => m.queue(),
        }
    }

    fn search(&mut self, query: &Query, window: Option<(u32, u32)>) -> Result<Vec<mpd::Song>> {
        match self {
            MpdBackend::Real(c) => c.search(query, window),
            MpdBackend::Mock(m) => m.search(query, window),
        }
    }

    fn consume(&mut self, state: bool) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.consume(state),
            MpdBackend::Mock(m) => m.consume(state),
        }
    }

    fn push(&mut self, song: mpd::Song) -> Result<mpd::Id> {
        match self {
            MpdBackend::Real(c) => c.push(song),
            MpdBackend::Mock(m) => m.push(song),
        }
    }

    fn delete(&mut self, pos: u32) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.delete(pos),
            MpdBackend::Mock(m) => m.delete(pos),
        }
    }

    fn play(&mut self) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.play(),
            MpdBackend::Mock(m) => m.play(),
        }
    }

    fn pl_push(&mut self, playlist: &str, song: mpd::Song) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.pl_push(playlist, song),
            MpdBackend::Mock(m) => m.pl_push(playlist, song),
        }
    }

    fn pl_delete(&mut self, playlist: &str, pos: u32) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.pl_delete(playlist, pos),
            MpdBackend::Mock(m) => m.pl_delete(playlist, pos),
        }
    }

    fn pl_remove(&mut self, playlist: &str) -> Result<()> {
        match self {
            MpdBackend::Real(c) => c.pl_remove(playlist),
            MpdBackend::Mock(m) => m.pl_remove(playlist),
        }
    }

    fn listall(&mut self) -> Result<Vec<mpd::Song>> {
        match self {
            MpdBackend::Real(c) => c.listall(),
            MpdBackend::Mock(m) => m.listall(),
        }
    }
}

pub struct MpdConn {
    pub mpd: MpdBackend,
    address: Vec<std::net::SocketAddr>,
    // Store original host/port for parallel task creation
    host: String,
    port: u16,
    is_dev_mode: bool,
}

static SHARED_MOCK: OnceLock<MockMpd> = OnceLock::new();

impl MpdConn {
    pub fn new() -> Result<Self> {
        let is_dev_mode = env::var("JUKECTL_DEV_MODE").unwrap_or_default() == "1";

        if is_dev_mode {
            debug!("[!] JUKECTL_DEV_MODE is enabled, using MockMpd");
            let mock = SHARED_MOCK.get_or_init(MockMpd::new).clone();
            return Ok(MpdConn {
                mpd: MpdBackend::Mock(mock),
                address: Vec::new(),
                host: "mock".to_string(),
                port: 0,
                is_dev_mode: true,
            });
        }

        debug!("[!] connecting to mpd...");
        let (mpd, address, host, port) = MpdConn::connect_mpd()?;
        Ok(MpdConn {
            mpd: MpdBackend::Real(mpd),
            address,
            host,
            port,
            is_dev_mode: false,
        })
    }

    // New method for creating connections with specific host/port
    pub fn new_with_host(host: &str, port: u16) -> Result<Self> {
        let is_dev_mode = env::var("JUKECTL_DEV_MODE").unwrap_or_default() == "1";

        if is_dev_mode {
            let mock = SHARED_MOCK.get_or_init(MockMpd::new).clone();
            return Ok(MpdConn {
                mpd: MpdBackend::Mock(mock),
                address: Vec::new(),
                host: "mock".to_string(),
                port: 0,
                is_dev_mode: true,
            });
        }

        debug!("[!] connecting to mpd at {}:{}...", host, port);

        // Resolve the host and port
        let address = (host, port)
            .to_socket_addrs()
            .map_err(|e| mpd::error::Error::Io(e))?
            .collect::<Vec<std::net::SocketAddr>>();

        // Create an MPD client and connect using the resolved address
        let mut mpd = Client::connect(address[0])?;

        // Set consume to true as part of Jukectl
        mpd.consume(true)?;

        Ok(MpdConn {
            mpd: MpdBackend::Real(mpd),
            address,
            host: host.to_string(),
            port,
            is_dev_mode: false,
        })
    }

    // Method to expose connection info for parallel tasks
    pub fn get_host_info(&self) -> (String, u16) {
        (self.host.clone(), self.port)
    }

    fn connect_mpd() -> Result<(Client, Vec<std::net::SocketAddr>, String, u16)> {
        // Get environment variables for MPD configuration
        let host = env::var("MPD_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = env::var("MPD_PORT")
            .unwrap_or_else(|_| "6600".to_string())
            .parse()
            .expect("Failed to parse MPD_PORT as u16");

        // Resolve the host and port once
        let address = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| mpd::error::Error::Io(e))?
            .collect::<Vec<std::net::SocketAddr>>();

        // Create an MPD client and connect using the resolved address
        let mut mpd = Client::connect(address[0])?;

        // Set consume to true as part of Jukectl
        mpd.consume(true)?;

        Ok((mpd, address, host, port))
    }

    pub fn reconnect(&mut self) -> Result<()> {
        if self.is_dev_mode {
            return Ok(());
        }

        debug!("[!] Checking connection...");
        if self.is_connected()? {
            debug!("[+] Connection is alive.");
            return Ok(());
        }

        debug!("[!] Reconnecting to mpd...");
        let mut new_mpd = Client::connect(self.address[0])?;
        new_mpd.consume(true)?;
        self.mpd = MpdBackend::Real(new_mpd);
        Ok(())
    }

    fn is_connected(&mut self) -> Result<bool> {
        if self.is_dev_mode {
            return Ok(true);
        }

        match self.mpd.ping() {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("[!] Connection check failed: {}", e);
                Ok(false)
            }
        }
    }

    // Expose search method for album-aware functionality
    pub fn search(&mut self, query: &Query, window: Option<(u32, u32)>) -> Result<Vec<mpd::Song>> {
        self.mpd.search(query, window)
    }
}
