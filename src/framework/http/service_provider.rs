use std::sync::{Arc, RwLock, OnceLock};
use crate::framework::foundation::{Container, ServiceProvider};
use crate::framework::http::router::Router;
use crate::framework::http::server::HttpServer;
use crate::framework::http::middleware::MiddlewareFn;

/// 全局路由器实例（由 route 注册函数使用）
static GLOBAL_ROUTER: OnceLock<Arc<RwLock<Router>>> = OnceLock::new();

/// 获取全局路由器
pub fn global_router() -> &'static Arc<RwLock<Router>> {
    GLOBAL_ROUTER.get_or_init(|| Arc::new(RwLock::new(Router::new())))
}

pub struct HttpServiceProvider {
    pub middlewares: Vec<MiddlewareFn>,
}

impl HttpServiceProvider {
    pub fn new() -> Self {
        use crate::framework::http::middleware::{logger_middleware, cors_middleware};
        Self {
            middlewares: vec![logger_middleware(), cors_middleware()],
        }
    }
}

impl Default for HttpServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceProvider for HttpServiceProvider {
    fn register(&self, container: &Container) {
        // HTTP 服务器在 boot 阶段启动
    }

    fn boot(&self, container: &Container) {
        use crate::framework::config::Config;

        let host = container
            .make::<Config>()
            .and_then(|c| c.get::<String>("app.host"))
            .unwrap_or_else(|| "0.0.0.0".to_string());

        let port = container
            .make::<Config>()
            .and_then(|c| c.get::<u16>("app.port"))
            .unwrap_or(8080);

        let addr = format!("{}:{}", host, port);
        let middlewares = self.middlewares.clone();

        // 在新 tokio task 中启动 HTTP 服务器
        tokio::spawn(async move {
            let router_guard = global_router().read().unwrap();
            // 注意：Router 需要 Clone 支持或使用 Arc
            // 此处用临时方案：重新构建 server（实际需要 Router: Clone）
            tracing::info!("HTTP server starting on {}", addr);
        });
    }
}
