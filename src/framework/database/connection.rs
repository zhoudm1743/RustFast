use std::future::Future;
use std::pin::Pin;
use crate::framework::database::model::{Row, Param};
use crate::framework::error::Result;

/// 数据库连接抽象 Trait
///
/// 每种数据库驱动（SQLite、PostgreSQL、MySQL）都实现此 Trait
pub trait DbConnection: Send + Sync {
    /// 执行查询，返回多行结果
    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Row>>> + Send + 'a>>;

    /// 执行查询，返回单行结果（没有则返回 None）
    fn query_one<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Row>>> + Send + 'a>>;

    /// 执行修改语句（INSERT/UPDATE/DELETE），返回影响行数
    fn execute<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<u64>> + Send + 'a>>;

    /// 执行 INSERT 并返回最后插入的行 ID
    fn insert<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>>;

    /// 开启事务
    fn begin_transaction<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// 提交事务
    fn commit_transaction<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// 回滚事务
    fn rollback_transaction<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// 健康检查
    fn ping<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// 关闭连接
    fn close<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}
