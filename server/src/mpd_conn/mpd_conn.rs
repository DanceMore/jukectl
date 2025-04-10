use mpd::{error::Result, Client};
use std::env;
use std::net::ToSocketAddrs;

pub struct MpdConn {
    pub mpd: Client,
    address: Vec<std::net::SocketAddr>,
}

impl MpdConn {
    pub fn new() -> Result<Self> {
        println!("[!] connecting to mpd...");
        let (mpd, address) = MpdConn::connect_mpd()?;
        Ok(MpdConn { mpd, address })
    }

    fn connect_mpd() -> Result<(Client, Vec<std::net::SocketAddr>)> {
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

        Ok((mpd, address))
    }

    pub fn reconnect(&mut self) -> Result<()> {
        println!("[!] Checking connection...");
        if self.is_connected()? {
            println!("[+] Connection is alive.");
            return Ok(());
        }

        println!("[!] Reconnecting to mpd...");
        let mut new_mpd = Client::connect(self.address[0])?;
        new_mpd.consume(true)?;
        self.mpd = new_mpd;
        Ok(())
    }

    fn is_connected(&mut self) -> Result<bool> {
        match self.mpd.ping() {
            Ok(_) => Ok(true),
            Err(e) => {
                println!("[!] Connection check failed: {}", e);
                Ok(false)
            }
        }
    }
}
