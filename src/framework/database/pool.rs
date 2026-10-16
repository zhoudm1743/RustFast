use sqlx::any::AnyPoolOptions;

/// 连接池配置
///
/// 连接池由 sqlx `Pool` 实现（健康检查、超时、回收均由 sqlx 处理），
/// 此配置仅映射为 `AnyPoolOptions`。
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
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

impl PoolConfig {
    pub(crate) fn to_pool_options(&self) -> AnyPoolOptions {
        AnyPoolOptions::new()
            .min_connections(self.min_connections)
            .max_connections(self.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(self.connection_timeout_secs))
    }
}
