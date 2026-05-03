use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};
use crate::framework::foundation::{Container, ServiceProvider};

pub struct LogServiceProvider;

impl ServiceProvider for LogServiceProvider {
    fn register(&self, _container: &Container) {
        // 日志系统初始化在 boot 阶段完成（需要读取配置）
    }

    fn boot(&self, container: &Container) {
        use crate::framework::config::Config;

        let level = container
            .make::<Config>()
            .and_then(|c| c.get::<String>("log.level"))
            .unwrap_or_else(|| "info".to_string());

        let filter = EnvFilter::try_new(&level)
            .unwrap_or_else(|_| EnvFilter::new("info"));

        // 初始化全局 tracing 订阅者（忽略重复初始化错误）
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .try_init();
    }
}
