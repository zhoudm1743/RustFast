use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use crate::framework::database::connection::DbConnection;
use crate::framework::error::{FastError, Result};

/// 连接池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub min_connections: usize,
    pub max_connections: usize,
    pub connection_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            connection_timeout_secs: 30,
        }
    }
}

/// 连接池
///
/// 基于 Tokio Semaphore + VecDeque 的简单连接池
/// - Semaphore 控制最大并发连接数
/// - VecDeque 存储可用连接
pub struct ConnectionPool {
    connections: Arc<Mutex<VecDeque<Box<dyn DbConnection>>>>,
    semaphore: Arc<Semaphore>,
    config: PoolConfig,
    factory: Arc<dyn Fn() -> Box<dyn DbConnection> + Send + Sync>,
}

impl ConnectionPool {
    pub fn new<F>(config: PoolConfig, factory: F) -> Self
    where
        F: Fn() -> Box<dyn DbConnection> + Send + Sync + 'static,
    {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        let connections = Arc::new(Mutex::new(VecDeque::new()));

        // 预创建最小连接数
        {
            let mut pool = connections.lock().unwrap();
            for _ in 0..config.min_connections {
                pool.push_back(factory());
            }
        }

        Self {
            connections,
            semaphore,
            config,
            factory: Arc::new(factory),
        }
    }

    /// 获取连接（阻塞直到有可用连接）
    pub async fn acquire(&self) -> Result<PooledConnection> {
        let permit = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.connection_timeout_secs),
            self.semaphore.clone().acquire_owned(),
        )
        .await
        .map_err(|_| FastError::Database("Connection pool timeout".into()))?
        .map_err(|_| FastError::Database("Connection pool closed".into()))?;

        // 从池中取出连接，或创建新连接
        let conn = {
            let mut pool = self.connections.lock().unwrap();
            pool.pop_front()
        };

        let conn = conn.unwrap_or_else(|| (self.factory)());

        Ok(PooledConnection {
            conn: Some(conn),
            pool: self.connections.clone(),
            _permit: permit,
        })
    }
}

/// 连接守卫：超出作用域时自动归还连接
pub struct PooledConnection {
    conn: Option<Box<dyn DbConnection>>,
    pool: Arc<Mutex<VecDeque<Box<dyn DbConnection>>>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl std::ops::Deref for PooledConnection {
    type Target = dyn DbConnection;
    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().unwrap().as_ref()
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let mut pool = self.pool.lock().unwrap();
            pool.push_back(conn);
        }
    }
}
