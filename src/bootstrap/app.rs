use std::sync::Arc;
use crate::framework::foundation::Application;
use crate::framework::foundation::application::set_app;
use crate::framework::foundation::ServiceProvider;

// 内置 Providers
use crate::framework::config::ConfigServiceProvider;
use crate::framework::log::LogServiceProvider;
use crate::framework::cache::CacheServiceProvider;
use crate::framework::database::DatabaseServiceProvider;

/// 应用引导函数
///
/// 创建 Application，注册所有 Provider，执行 boot，
/// 并设置全局应用实例（供 Facades 使用）
pub fn boot() -> Arc<Application> {
    let mut app = Application::new(".");

    // 注册框架内置 Providers（顺序即执行顺序）
    app.set_providers(providers());

    // 先设置全局实例（Config ServiceProvider 需要读取 base_path）
    let app = Arc::new(app);
    set_app(app.clone());

    // 执行 register + boot
    app.boot();

    app
}

/// 框架内置 ServiceProvider 列表
fn providers() -> Vec<Box<dyn ServiceProvider>> {
    vec![
        Box::new(ConfigServiceProvider),       // 1. 配置
        Box::new(LogServiceProvider),           // 2. 日志
        Box::new(CacheServiceProvider),         // 3. 缓存
        Box::new(DatabaseServiceProvider),      // 4. 数据库
    ]
}
