//! 数据库迁移系统
//!
//! 迁移以代码形式定义（实现 `Migration` trait），按 `version` 升序执行，
//! 执行记录存储在 `_fast_migrations` 表中。
//!
//! 注意：`up()` / `down()` 中的多条 SQL 语句以 `;` 分隔，
//! 不支持语句体内含分号的场景（如存储过程）。

use std::sync::Arc;

use crate::framework::database::manager::DbManager;
use crate::framework::database::model::Param;
use crate::framework::error::{FastError, Result};

/// 迁移记录表名
const MIGRATIONS_TABLE: &str = "_fast_migrations";

/// 数据库迁移
pub trait Migration: Send + Sync {
    /// 版本号（执行顺序的排序依据），建议格式 "20240101_000001"
    fn version(&self) -> &'static str;

    /// 迁移名称（仅用于展示）
    fn name(&self) -> &'static str;

    /// 升级 SQL（可包含多条语句，以 `;` 分隔）
    fn up(&self) -> &'static str;

    /// 回滚 SQL
    fn down(&self) -> &'static str;
}

/// 迁移执行器
pub struct Migrator {
    db: DbManager,
    migrations: Vec<Arc<dyn Migration>>,
}

impl Migrator {
    pub fn new(db: DbManager) -> Self {
        Self {
            db,
            migrations: Vec::new(),
        }
    }

    /// 注册一个迁移
    pub fn add(&mut self, migration: impl Migration + 'static) -> &mut Self {
        self.migrations.push(Arc::new(migration));
        self
    }

    /// 注册一批迁移
    pub fn add_all(&mut self, migrations: Vec<Arc<dyn Migration>>) -> &mut Self {
        self.migrations.extend(migrations);
        self
    }

    /// 执行所有未应用的迁移，返回本次应用的版本号列表
    pub async fn migrate(&self) -> Result<Vec<String>> {
        self.ensure_table().await?;
        let applied = self.applied_versions().await?;

        let mut pending: Vec<&Arc<dyn Migration>> = self
            .migrations
            .iter()
            .filter(|m| !applied.contains(&m.version().to_string()))
            .collect();
        pending.sort_by_key(|m| m.version());

        let mut done = Vec::new();
        for m in pending {
            self.db.begin().await?;
            let result = async {
                for stmt in split_statements(m.up()) {
                    self.db.raw_execute(&stmt, vec![]).await?;
                }
                self.record(m.version(), m.name()).await
            }
            .await;

            match result {
                Ok(()) => {
                    self.db.commit().await?;
                    tracing::info!("Migrated: {} ({})", m.version(), m.name());
                    done.push(m.version().to_string());
                }
                Err(e) => {
                    let _ = self.db.rollback().await;
                    return Err(FastError::Database(format!(
                        "Migration {} failed: {}",
                        m.version(),
                        e
                    )));
                }
            }
        }
        Ok(done)
    }

    /// 回滚最近 `steps` 个已应用的迁移，返回回滚的版本号列表
    pub async fn rollback(&self, steps: usize) -> Result<Vec<String>> {
        self.ensure_table().await?;
        let applied = self.applied_versions().await?;

        let mut done = Vec::new();
        // 已应用的按版本号倒序回滚
        let mut applied_sorted = applied;
        applied_sorted.sort_by(|a, b| b.cmp(a));

        for version in applied_sorted.into_iter().take(steps) {
            let m = self
                .migrations
                .iter()
                .find(|m| m.version() == version)
                .ok_or_else(|| {
                    FastError::Database(format!(
                        "Migration {} is applied but not registered in code",
                        version
                    ))
                })?;

            self.db.begin().await?;
            let result = async {
                for stmt in split_statements(m.down()) {
                    self.db.raw_execute(&stmt, vec![]).await?;
                }
                self.db
                    .raw_execute(
                        &format!("DELETE FROM {} WHERE version = ?", MIGRATIONS_TABLE),
                        vec![Param::Text(version.clone())],
                    )
                    .await
            }
            .await;

            match result {
                Ok(_) => {
                    self.db.commit().await?;
                    tracing::info!("Rolled back: {} ({})", m.version(), m.name());
                    done.push(version);
                }
                Err(e) => {
                    let _ = self.db.rollback().await;
                    return Err(FastError::Database(format!(
                        "Rollback of {} failed: {}",
                        version, e
                    )));
                }
            }
        }
        Ok(done)
    }

    /// 迁移状态：返回 (版本号, 名称, 是否已应用)
    pub async fn status(&self) -> Result<Vec<(String, String, bool)>> {
        self.ensure_table().await?;
        let applied = self.applied_versions().await?;

        let mut all: Vec<&Arc<dyn Migration>> = self.migrations.iter().collect();
        all.sort_by_key(|m| m.version());

        Ok(all
            .into_iter()
            .map(|m| {
                (
                    m.version().to_string(),
                    m.name().to_string(),
                    applied.contains(&m.version().to_string()),
                )
            })
            .collect())
    }

    /// 创建迁移记录表（若不存在）
    async fn ensure_table(&self) -> Result<()> {
        self.db
            .raw_execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        version VARCHAR(64) PRIMARY KEY,
                        name VARCHAR(255) NOT NULL,
                        applied_at VARCHAR(32) NOT NULL
                    )",
                    MIGRATIONS_TABLE
                ),
                vec![],
            )
            .await?;
        Ok(())
    }

    async fn applied_versions(&self) -> Result<Vec<String>> {
        let rows = self
            .db
            .raw_query(
                &format!("SELECT version FROM {}", MIGRATIONS_TABLE),
                vec![],
            )
            .await?;
        Ok(rows
            .iter()
            .filter_map(|r| r.get::<String>("version").ok())
            .collect())
    }

    async fn record(&self, version: &str, name: &str) -> Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        self.db
            .raw_execute(
                &format!(
                    "INSERT INTO {} (version, name, applied_at) VALUES (?, ?, ?)",
                    MIGRATIONS_TABLE
                ),
                vec![
                    Param::Text(version.to_string()),
                    Param::Text(name.to_string()),
                    Param::Text(now),
                ],
            )
            .await?;
        Ok(())
    }
}

/// 按 `;` 切分多条 SQL 语句（忽略空片段）
fn split_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CreateUsersTable;
    impl Migration for CreateUsersTable {
        fn version(&self) -> &'static str {
            "20240101_000001"
        }
        fn name(&self) -> &'static str {
            "create_users_table"
        }
        fn up(&self) -> &'static str {
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
             CREATE INDEX idx_users_name ON users(name);"
        }
        fn down(&self) -> &'static str {
            "DROP TABLE IF EXISTS users;"
        }
    }

    struct CreatePostsTable;
    impl Migration for CreatePostsTable {
        fn version(&self) -> &'static str {
            "20240102_000001"
        }
        fn name(&self) -> &'static str {
            "create_posts_table"
        }
        fn up(&self) -> &'static str {
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL);"
        }
        fn down(&self) -> &'static str {
            "DROP TABLE IF EXISTS posts;"
        }
    }

    async fn table_exists(db: &DbManager, table: &str) -> bool {
        let rows = db
            .raw_query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
                vec![Param::Text(table.to_string())],
            )
            .await
            .unwrap();
        !rows.is_empty()
    }

    #[tokio::test]
    async fn test_migrate_and_rollback() {
        let db = DbManager::sqlite(":memory:").unwrap();
        let mut migrator = Migrator::new(db.clone());
        migrator.add(CreatePostsTable); // 故意乱序注册
        migrator.add(CreateUsersTable);

        // 首次迁移：两个都应应用，且按版本号顺序
        let applied = migrator.migrate().await.unwrap();
        assert_eq!(applied, vec!["20240101_000001", "20240102_000001"]);
        assert!(table_exists(&db, "users").await);
        assert!(table_exists(&db, "posts").await);

        // 再次迁移：无待执行项
        let applied = migrator.migrate().await.unwrap();
        assert!(applied.is_empty());

        // 状态
        let status = migrator.status().await.unwrap();
        assert_eq!(status.len(), 2);
        assert!(status.iter().all(|(_, _, applied)| *applied));

        // 回滚一步：只回滚最新的 posts
        let rolled = migrator.rollback(1).await.unwrap();
        assert_eq!(rolled, vec!["20240102_000001"]);
        assert!(table_exists(&db, "users").await);
        assert!(!table_exists(&db, "posts").await);

        // 再迁移可重新应用
        let applied = migrator.migrate().await.unwrap();
        assert_eq!(applied, vec!["20240102_000001"]);
        assert!(table_exists(&db, "posts").await);
    }

    #[tokio::test]
    async fn test_failed_migration_rolls_back() {
        struct BadMigration;
        impl Migration for BadMigration {
            fn version(&self) -> &'static str {
                "20240103_000001"
            }
            fn name(&self) -> &'static str {
                "bad_migration"
            }
            fn up(&self) -> &'static str {
                "CREATE TABLE good_table (id INTEGER); THIS IS NOT SQL;"
            }
            fn down(&self) -> &'static str {
                "DROP TABLE IF EXISTS good_table;"
            }
        }

        let db = DbManager::sqlite(":memory:").unwrap();
        let mut migrator = Migrator::new(db.clone());
        migrator.add(BadMigration);

        let err = migrator.migrate().await.unwrap_err();
        assert!(err.to_string().contains("20240103_000001"));

        // 失败的事务已回滚：表不应存在，记录不应写入
        assert!(!table_exists(&db, "good_table").await);
        let status = migrator.status().await.unwrap();
        assert!(!status[0].2);
    }
}
