// mpd_pool.rs - Improved connection pool with proper testing and efficiency
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
    // Track actual connection count to enforce hard limit
    connection_count: Arc<Mutex<usize>>,
}

impl MpdConnectionPool {
    /// Create a new connection pool and initialize it with connections
    pub async fn new(
        host: &str,
        port: u16,
        max_connections: usize,
    ) -> Result<Self, mpd::error::Error> {
        let pool = Self {
            connections: Arc::new(Mutex::new(VecDeque::new())),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            host: host.to_string(),
            port,
            max_connections,
            connection_count: Arc::new(Mutex::new(0)),
        };

        // Initialize with a reasonable number of connections (25% of max, at least 1)
        let initial_size = std::cmp::max(1, max_connections / 4);
        pool.warm_pool(initial_size).await?;

        info!(
            "[+] Connection pool initialized: {} connections ready, {} max",
            initial_size, max_connections
        );
        Ok(pool)
    }

    /// Warm up the pool with initial connections
    async fn warm_pool(&self, count: usize) -> Result<(), mpd::error::Error> {
        let mut connections = self.connections.lock().await;
        let mut conn_count = self.connection_count.lock().await;

        for i in 0..count {
            match MpdConn::new_with_host(&self.host, self.port) {
                Ok(conn) => {
                    connections.push_back(conn);
                    *conn_count += 1;
                    debug!("[+] Warmed connection {}/{}", i + 1, count);
                }
                Err(e) => {
                    warn!("[!] Failed to warm connection {}/{}: {}", i + 1, count, e);
                    // Don't fail completely if we can't warm all connections
                    if i == 0 {
                        // But fail if we can't create even the first one
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<PooledMpdConnection, mpd::error::Error> {
        debug!("[.] Connection requested from pool");

        // Acquire semaphore permit (limits concurrent usage)
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        let mut connections = self.connections.lock().await;

        let mpd_conn = if let Some(mut mpd_conn) = connections.pop_front() {
            drop(connections); // Release lock before health check

            // Validate connection health
            if self.validate_connection(&mut mpd_conn) {
                debug!("[+] Reusing healthy connection");
                mpd_conn
            } else {
                debug!("[!] Stale connection detected, creating new one");
                self.create_new_connection().await?
            }
        } else {
            drop(connections); // Release lock before creating connection
            debug!("[+] No pooled connections, creating new one");
            self.create_new_connection().await?
        };

        Ok(PooledMpdConnection {
            mpd_conn: Some(mpd_conn),
            pool: self.connections.clone(),
            connection_count: self.connection_count.clone(),
            _permit: permit,
        })
    }

    /// Create a new connection, respecting max_connections limit
    async fn create_new_connection(&self) -> Result<MpdConn, mpd::error::Error> {
        let mut conn_count = self.connection_count.lock().await;

        if *conn_count >= self.max_connections {
            warn!(
                "[!] Max connections ({}) reached, waiting for available connection",
                self.max_connections
            );
            drop(conn_count);
            // In practice, the semaphore should prevent this, but just in case
            return Err(mpd::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Connection pool exhausted",
            )));
        }

        let conn = MpdConn::new_with_host(&self.host, self.port)?;
        *conn_count += 1;
        debug!("[+] Created new connection (total: {})", *conn_count);

        Ok(conn)
    }

    /// Validate that a connection is still healthy
    fn validate_connection(&self, conn: &mut MpdConn) -> bool {
        match conn.reconnect() {
            Ok(_) => true,
            Err(e) => {
                debug!("[!] Connection validation failed: {}", e);
                false
            }
        }
    }

    /// Return a connection to the pool (called by Drop)
    async fn return_connection(&self, mut conn: MpdConn) {
        // Only return if healthy
        if self.validate_connection(&mut conn) {
            let mut connections = self.connections.lock().await;
            connections.push_back(conn);
            debug!(
                "[+] Connection returned to pool ({} available)",
                connections.len()
            );
        } else {
            // Connection is dead, decrement count
            let mut conn_count = self.connection_count.lock().await;
            *conn_count = conn_count.saturating_sub(1);
            debug!("[!] Dead connection discarded (total: {})", *conn_count);
        }
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let available = self.connections.lock().await.len();
        let total = *self.connection_count.lock().await;
        let in_use = total - available;

        PoolStats {
            available,
            in_use,
            total,
            max: self.max_connections,
        }
    }

    /// Get a simple (available, max) tuple for backward compatibility
    pub async fn stats_simple(&self) -> (usize, usize) {
        let stats = self.stats().await;
        (stats.available, stats.max)
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub available: usize,
    pub in_use: usize,
    pub total: usize,
    pub max: usize,
}

pub struct PooledMpdConnection {
    mpd_conn: Option<MpdConn>,
    pool: Arc<Mutex<VecDeque<MpdConn>>>,
    connection_count: Arc<Mutex<usize>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledMpdConnection {
    pub fn mpd_conn(&mut self) -> &mut MpdConn {
        self.mpd_conn.as_mut().unwrap()
    }
}

impl Drop for PooledMpdConnection {
    fn drop(&mut self) {
        if let Some(mpd_conn) = self.mpd_conn.take() {
            let pool = self.pool.clone();
            let connection_count = self.connection_count.clone();

            // Spawn task to return connection
            tokio::spawn(async move {
                // Recreate the pool struct just for the return_connection method
                // This is a bit awkward but avoids storing the whole pool
                let temp_pool = MpdConnectionPool {
                    connections: pool.clone(),
                    semaphore: Arc::new(Semaphore::new(1)), // Dummy, not used
                    host: String::new(),
                    port: 0,
                    max_connections: 0,
                    connection_count: connection_count.clone(),
                };
                temp_pool.return_connection(mpd_conn).await;
            });
        }
        // Semaphore permit is automatically released when _permit is dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running MPD instance
    // Set RUN_INTEGRATION_TESTS=1 to enable

    fn should_run_tests() -> bool {
        std::env::var("RUN_INTEGRATION_TESTS").unwrap_or_default() == "1"
    }

    #[tokio::test]
    async fn test_pool_creation() {
        if !should_run_tests() {
            println!("⏭️  Skipping pool tests (set RUN_INTEGRATION_TESTS=1)");
            return;
        }

        let pool = MpdConnectionPool::new("localhost", 6600, 5)
            .await
            .expect("Failed to create pool");

        let stats = pool.stats().await;
        assert!(stats.available >= 1);
        assert_eq!(stats.max, 5);
    }

    #[tokio::test]
    async fn test_connection_checkout_and_return() {
        if !should_run_tests() {
            return;
        }

        let pool = MpdConnectionPool::new("localhost", 6600, 5)
            .await
            .expect("Failed to create pool");

        let initial_stats = pool.stats().await;
        let initial_available = initial_stats.available;

        {
            let mut conn = pool
                .get_connection()
                .await
                .expect("Failed to get connection");
            conn.mpd_conn().mpd.ping().expect("Ping failed");

            let stats_during = pool.stats().await;
            assert_eq!(stats_during.available, initial_available - 1);
            assert_eq!(stats_during.in_use, 1);
        }

        // Give the Drop task time to return the connection
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let final_stats = pool.stats().await;
        assert_eq!(final_stats.available, initial_available);
    }

    #[tokio::test]
    async fn test_concurrent_connections() {
        if !should_run_tests() {
            return;
        }

        let pool = Arc::new(
            MpdConnectionPool::new("localhost", 6600, 3)
                .await
                .expect("Failed to create pool"),
        );

        let mut handles = vec![];

        for i in 0..5 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let mut conn = pool_clone
                    .get_connection()
                    .await
                    .expect("Failed to get connection");
                conn.mpd_conn().mpd.ping().expect("Ping failed");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                i
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task failed");
        }

        // All connections should be returned
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        let stats = pool.stats().await;
        assert_eq!(stats.in_use, 0);
    }

    #[tokio::test]
    async fn test_connection_validation() {
        if !should_run_tests() {
            return;
        }

        let pool = MpdConnectionPool::new("localhost", 6600, 3)
            .await
            .expect("Failed to create pool");

        let mut conn = pool
            .get_connection()
            .await
            .expect("Failed to get connection");

        // Connection should be valid
        assert!(pool.validate_connection(conn.mpd_conn()));
    }
}
