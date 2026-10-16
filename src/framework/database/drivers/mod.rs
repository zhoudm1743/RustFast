/// 数据库驱动模块
///
/// 基于 sqlx `Any` 驱动的统一实现，通过连接 URL 区分
/// SQLite / PostgreSQL / MySQL，无需每个数据库单独一套驱动代码。
pub mod sqlx_conn;

pub use sqlx_conn::{DbKind, SqlxConnection};
