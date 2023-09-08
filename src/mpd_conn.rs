use mpd::{Client, error::Result};
use std::env;

pub struct MpdConn {
    pub mpd: Client,
}

impl MpdConn {
    pub fn new() -> Result<Self> {
        println!("[!] connecting to mpd...");

        // Get environment variables for MPD configuration
        let host = env::var("MPD_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = env::var("MPD_PORT")
            .unwrap_or_else(|_| "6600".to_string())
            .parse()
            .expect("Failed to parse MPD_PORT as u16");
        let password = env::var("MPD_PASS").ok();

        // Create an MPD client and connect
        let mut mpd = Client::connect((host.as_str(), port))?;
        //if let Some(pass) = password {
        //    mpd.password(pass.as_str())?;
        //}
        mpd.consume(true)?;

        Ok(MpdConn { mpd })
    }
}
