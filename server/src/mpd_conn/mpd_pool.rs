use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

use crate::mpd_conn::mpd_conn::MpdConn;

pub struct MpdPool {
    connections: Arc<Mutex<Vec<MpdConn>>>,
    semaphore: Arc<Semaphore>,
    host: String,
    port: u16,
}

pub struct PooledMpdConnection {
    conn: Option<MpdConn>,
    pool: Arc<Mutex<Vec<MpdConn>>>,
}

impl PooledMpdConnection {
    pub fn mpd_conn(&mut self) -> &mut MpdConn {
        self.conn.as_mut().unwrap()
    }
}

impl Drop for PooledMpdConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                let mut pool_lock = pool.lock().await;
                pool_lock.push(conn);
            });
        }
    }
}

impl MpdPool {
    pub fn new(host: String, port: u16, max_connections: usize) -> Result<Self> {
        Ok(MpdPool {
            connections: Arc::new(Mutex::new(Vec::with_capacity(max_connections))),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            host,
            port,
        })
    }

    pub async fn warm_pool(&self, count: usize) -> Result<()> {
        let mut conns = Vec::with_capacity(count);
        for _ in 0..count {
            if let Ok(conn) = self.create_new_connection().await {
                conns.push(conn);
            }
        }
        let mut pool_lock = self.connections.lock().await;
        pool_lock.extend(conns);
        Ok(())
    }

    pub async fn get_connection(&self) -> Result<PooledMpdConnection> {
        let _permit = self.semaphore.acquire().await.map_err(|e| anyhow!("Pool permit error: {}", e))?;
        
        let mut pool_lock = self.connections.lock().await;
        if let Some(mut conn) = pool_lock.pop() {
            if let Err(_) = conn.reconnect() {
                drop(pool_lock);
                let new_conn = self.create_new_connection().await?;
                return Ok(PooledMpdConnection {
                    conn: Some(new_conn),
                    pool: self.connections.clone(),
                });
            }
            return Ok(PooledMpdConnection {
                conn: Some(conn),
                pool: self.connections.clone(),
            });
        }

        drop(pool_lock);
        let new_conn = self.create_new_connection().await?;
        Ok(PooledMpdConnection {
            conn: Some(new_conn),
            pool: self.connections.clone(),
        })
    }

    async fn create_new_connection(&self) -> Result<MpdConn> {
        let host = self.host.clone();
        let port = self.port;
        tokio::task::spawn_blocking(move || MpdConn::new_with_host(&host, port))
            .await
            .map_err(|e| anyhow!("Blocking task join error: {}", e))?
    }
}
