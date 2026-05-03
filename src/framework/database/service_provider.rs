use std::sync::Arc;
use crate::framework::foundation::{Container, ServiceProvider};
use crate::framework::database::manager::DbManager;

pub struct DatabaseServiceProvider;

impl ServiceProvider for DatabaseServiceProvider {
    fn register(&self, container: &Container) {
        container.singleton::<DbManager, _>(|c| {
            use crate::framework::config::Config;

            let config = c.make::<Config>();
            let path = config
                .as_ref()
                .and_then(|cfg| cfg.get::<String>("database.path"))
                .unwrap_or_else(|| "database.db".to_string());

            let driver = config
                .as_ref()
                .and_then(|cfg| cfg.get::<String>("database.driver"))
                .unwrap_or_else(|| "sqlite".to_string());

            match driver.as_str() {
                #[cfg(feature = "sqlite")]
                "sqlite" => {
                    Arc::new(
                        DbManager::sqlite(&path)
                            .expect("Failed to create SQLite connection"),
                    )
                }
                other => panic!("Unsupported database driver: {}. Enable the corresponding feature.", other),
            }
        });
    }

    fn boot(&self, container: &Container) {
        if let Some(db) = container.make::<DbManager>() {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async move {
                match db.ping().await {
                    Ok(_) => tracing::info!("Database connection OK"),
                    Err(e) => tracing::error!("Database connection failed: {}", e),
                }
            });
        }
    }
}
