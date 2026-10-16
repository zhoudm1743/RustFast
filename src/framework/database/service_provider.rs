use std::sync::Arc;
use crate::framework::foundation::{Container, ServiceProvider};
use crate::framework::database::manager::DbManager;

pub struct DatabaseServiceProvider;

impl ServiceProvider for DatabaseServiceProvider {
    fn register(&self, container: &Container) {
        container.singleton::<DbManager, _>(|c| {
            use crate::framework::config::Config;

            let config = c.make::<Config>();
            let driver = config
                .as_ref()
                .and_then(|cfg| cfg.get::<String>("database.driver"))
                .unwrap_or_else(|| "sqlite".to_string());

            let manager = match driver.as_str() {
                "sqlite" => {
                    let path = config
                        .as_ref()
                        .and_then(|cfg| cfg.get::<String>("database.path"))
                        .unwrap_or_else(|| "database.db".to_string());
                    DbManager::sqlite(&path)
                }
                // postgres / mysql 等通过完整连接 URL 配置
                _ => {
                    let url = config
                        .as_ref()
                        .and_then(|cfg| cfg.get::<String>("database.url"))
                        .unwrap_or_else(|| panic!(
                            "database.url is required when database.driver = '{}'",
                            driver
                        ));
                    DbManager::connect(&url)
                }
            };

            Arc::new(manager.unwrap_or_else(|e| {
                panic!("Failed to create database connection (driver '{}'): {}", driver, e)
            }))
        });
    }

    fn boot(&self, container: &Container) {
        if let Some(db) = container.make::<DbManager>() {
            // 异步执行连通性检查（不能在此处 block_on，boot 运行在 tokio 上下文中）
            tokio::spawn(async move {
                match db.ping().await {
                    Ok(_) => tracing::info!("Database connection OK"),
                    Err(e) => tracing::error!("Database connection failed: {}", e),
                }
                // WAL 模式对文件库是持久设置，连接建立后执行一次即可
                if db.kind() == crate::framework::database::DbKind::Sqlite {
                    let _ = db.raw_execute("PRAGMA journal_mode=WAL;", vec![]).await;
                }
            });
        }
    }
}
