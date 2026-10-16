//! 应用数据库迁移
//!
//! 每个迁移实现 `Migration` trait，在 `all()` 中注册。
//! 版本号建议格式 "20240101_000001"，执行时按版本号升序应用。

use std::sync::Arc;

use crate::framework::database::Migration;

/// 创建 users 表
pub struct CreateUsersTable;

impl Migration for CreateUsersTable {
    fn version(&self) -> &'static str {
        "20240101_000001"
    }

    fn name(&self) -> &'static str {
        "create_users_table"
    }

    fn up(&self) -> &'static str {
        "CREATE TABLE IF NOT EXISTS users (
            id VARCHAR(36) PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL,
            created_at VARCHAR(32) NOT NULL DEFAULT ''
        );"
    }

    fn down(&self) -> &'static str {
        "DROP TABLE IF EXISTS users;"
    }
}

/// 所有已注册的迁移
pub fn all() -> Vec<Arc<dyn Migration>> {
    vec![Arc::new(CreateUsersTable)]
}
