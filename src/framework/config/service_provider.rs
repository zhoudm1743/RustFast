use std::sync::Arc;
use crate::framework::foundation::{Container, ServiceProvider};
use crate::framework::foundation::application::app;
use super::config::Config;

pub struct ConfigServiceProvider;

impl ServiceProvider for ConfigServiceProvider {
    fn register(&self, container: &Container) {
        let base_path = {
            // 通过全局应用实例获取根路径
            // 此时 app() 已设置，因为 Application::new() 先于 boot()
            let application = app();
            application.base_path().to_string()
        };

        container.singleton::<Config, _>(move |_| {
            let config_path = format!("{}/config/config.yaml", base_path);
            let config = Config::load(&config_path)
                .unwrap_or_else(|e| {
                    tracing::warn!("Config load failed: {}. Using empty config.", e);
                    Config::from_str("{}").unwrap()
                });
            Arc::new(config)
        });
    }
}
