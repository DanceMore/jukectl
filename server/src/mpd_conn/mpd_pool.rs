// mpd_pool.rs - Fixed to work with your MpdConn wrapper
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

use crate::mpd_conn::mpd_conn::MpdConn;

use log::{debug, error, info, trace, warn};

pub struct MpdConnectionPool {
    connections: Arc<Mutex<VecDeque<MpdConn>>>,
    semaphore: Arc<Semaphore>,
    host: String,
    port: u16,
    max_connections: usize,
}

impl MpdConnectionPool {
    pub fn new(host: &str, port: u16, max_connections: usize) -> Self {
        Self {
            connections: Arc::new(Mutex::new(VecDeque::new())),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            host: host.to_string(),
            port,
            max_connections,
        }
    }

    // Separate async method to initialize the pool
    pub async fn initialize(&self) -> Result<(), mpd::error::Error> {
        let initial_size = std::cmp::min(2, self.max_connections);
        let mut connections = self.connections.lock().await;

        for _ in 0..initial_size {
            if let Ok(conn) = MpdConn::new_with_host(&self.host, self.port) {
                connections.push_back(conn);
            }
        }

        info!(
            "[+] Connection pool initialized with {} connections",
            connections.len()
        );
        Ok(())
    }

    pub async fn get_connection(&self) -> Result<PooledMpdConnection, mpd::error::Error> {
        debug!("[.] connection pool was asked for a connection...");
        // Acquire semaphore permit (limits concurrent connections)
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        let mut connections = self.connections.lock().await;

        let mpd_conn = if let Some(mut mpd_conn) = connections.pop_front() {
            // Test if connection is still alive by trying to reconnect
            if mpd_conn.reconnect().is_ok() {
                debug!("[.] re-using an existing connection!");
                mpd_conn
            } else {
                // Connection is dead, create a new one
                drop(connections); // Release lock before creating connection
                debug!("[+] creating new connection!");
                MpdConn::new_with_host(&self.host, self.port)?
            }
        } else {
            // No pooled connections available, create new one
            drop(connections); // Release lock before creating connection
            debug!("[+] creating new connection!");
            MpdConn::new_with_host(&self.host, self.port)?
        };

        Ok(PooledMpdConnection {
            mpd_conn: Some(mpd_conn),
            pool: self.connections.clone(),
            _permit: permit,
        })
    }

    pub async fn stats(&self) -> (usize, usize) {
        let available = self.connections.lock().await.len();
        (available, self.max_connections)
    }
}

pub struct PooledMpdConnection {
    mpd_conn: Option<MpdConn>,
    pool: Arc<Mutex<VecDeque<MpdConn>>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledMpdConnection {
    pub fn mpd_conn(&mut self) -> &mut MpdConn {
        self.mpd_conn.as_mut().unwrap()
    }
}

impl Drop for PooledMpdConnection {
    fn drop(&mut self) {
        if let Some(mut mpd_conn) = self.mpd_conn.take() {
            // Return connection to pool if it's still healthy
            if mpd_conn.reconnect().is_ok() {
                let pool = self.pool.clone();
                tokio::spawn(async move {
                    let mut connections = pool.lock().await;
                    connections.push_back(mpd_conn);
                });
            }
        }
        // Semaphore permit is automatically released when _permit is dropped
    }
}
