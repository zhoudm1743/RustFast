use std::sync::Arc;
use crate::framework::database::connection::DbConnection;
use crate::framework::database::query_builder::QueryBuilder;
use crate::framework::error::Result;

#[cfg(feature = "sqlite")]
use crate::framework::database::drivers::sqlite::{SqliteConnection, SqliteConfig};

/// 数据库管理器
pub struct DbManager {
    connection: Arc<dyn DbConnection>,
}

impl DbManager {
    /// 创建 SQLite 连接
    #[cfg(feature = "sqlite")]
    pub fn sqlite(path: &str) -> Result<Self> {
        let config = SqliteConfig { path: path.to_string() };
        let conn = SqliteConnection::open(&config)?;
        Ok(Self { connection: Arc::new(conn) })
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
