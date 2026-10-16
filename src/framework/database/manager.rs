use std::sync::Arc;
use crate::framework::database::connection::DbConnection;
use crate::framework::database::drivers::SqlxConnection;
use crate::framework::database::pool::PoolConfig;
use crate::framework::database::query_builder::QueryBuilder;
use crate::framework::error::Result;

/// 数据库管理器
#[derive(Clone)]
pub struct DbManager {
    connection: Arc<dyn DbConnection>,
    kind: crate::framework::database::drivers::DbKind,
}

impl DbManager {
    /// 创建 SQLite 连接
    ///
    /// `path` 为数据库文件路径，`:memory:` 为内存数据库
    /// （内存库强制单连接池，否则多个连接会各自得到独立的内存库）
    pub fn sqlite(path: &str) -> Result<Self> {
        let (url, pool_config) = if path == ":memory:" {
            (
                "sqlite::memory:".to_string(),
                PoolConfig {
                    min_connections: 1,
                    max_connections: 1,
                    ..Default::default()
                },
            )
        } else {
            // mode=rwc：文件不存在时自动创建（Any URL 仅支持 mode/cache/immutable/vfs 参数）
            (format!("sqlite://{}?mode=rwc", path), PoolConfig::default())
        };
        Self::connect_with_config(&url, &pool_config)
    }

    /// 通过连接 URL 创建连接（sqlite:/postgres:/mysql: 协议）
    pub fn connect(url: &str) -> Result<Self> {
        Self::connect_with_config(url, &PoolConfig::default())
    }

    /// 通过连接 URL + 自定义连接池配置创建连接
    pub fn connect_with_config(url: &str, pool_config: &PoolConfig) -> Result<Self> {
        let conn = SqlxConnection::connect_lazy(url, pool_config)?;
        let kind = conn.kind();
        Ok(Self {
            connection: Arc::new(conn),
            kind,
        })
    }

    /// 数据库类型
    pub fn kind(&self) -> crate::framework::database::drivers::DbKind {
        self.kind
    }

    /// 开始一个查询（返回 QueryBuilder）
    pub fn table(&self, table: &str) -> QueryBuilder {
        QueryBuilder::new(self.connection.clone(), table)
    }

    /// 执行原始 SQL 查询（返回多行）
    pub async fn raw_query(
        &self,
        sql: &str,
        params: Vec<crate::framework::database::model::Param>,
    ) -> Result<Vec<crate::framework::database::model::Row>> {
        self.connection.query(sql, &params).await
    }

    /// 执行原始 SQL 修改语句（返回影响行数）
    pub async fn raw_execute(
        &self,
        sql: &str,
        params: Vec<crate::framework::database::model::Param>,
    ) -> Result<u64> {
        self.connection.execute(sql, &params).await
    }

    /// 健康检查
    pub async fn ping(&self) -> Result<()> {
        self.connection.ping().await
    }

    /// 开启事务
    pub async fn begin(&self) -> Result<()> {
        self.connection.begin_transaction().await
    }

    /// 提交事务
    pub async fn commit(&self) -> Result<()> {
        self.connection.commit_transaction().await
    }

    /// 回滚事务
    pub async fn rollback(&self) -> Result<()> {
        self.connection.rollback_transaction().await
    }

    /// 在事务中执行一个异步闭包，成功则提交，失败则回滚
    pub async fn transaction<'a, F, Fut, T>(&'a self, f: F) -> Result<T>
    where
        F: FnOnce(&'a DbManager) -> Fut,
        Fut: std::future::Future<Output = Result<T>> + 'a,
    {
        self.begin().await?;
        match f(self).await {
            Ok(val) => {
                self.commit().await?;
                Ok(val)
            }
            Err(e) => {
                let _ = self.rollback().await;
                Err(e)
            }
        }
    }
}
