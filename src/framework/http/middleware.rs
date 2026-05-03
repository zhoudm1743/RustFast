use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::framework::http::request::Request;
use crate::framework::http::response::Response;
use crate::framework::error::Result;

/// 中间件函数签名
pub type MiddlewareFn =
    Arc<dyn Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>
        + Send
        + Sync>;

/// Handler 函数签名（用于中间件链末端）
pub type HandlerFn =
    Arc<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>
        + Send
        + Sync>;

/// 责任链传递器
///
/// 每个中间件调用 `next.run(req)` 将请求传递给下一个中间件或最终 handler
#[derive(Clone)]
pub struct Next {
    middlewares: Arc<Vec<MiddlewareFn>>,
    index: usize,
    handler: HandlerFn,
}

impl Next {
    pub fn new(middlewares: Arc<Vec<MiddlewareFn>>, handler: HandlerFn) -> Self {
        Self {
            middlewares,
            index: 0,
            handler,
        }
    }

    /// 传递请求到下一个中间件（或最终 handler）
    pub fn run(mut self, req: Request) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> {
        if self.index < self.middlewares.len() {
            let middleware = self.middlewares[self.index].clone();
            self.index += 1;
            Box::pin(async move { middleware(req, self).await })
        } else {
            let handler = self.handler.clone();
            Box::pin(async move { handler(req).await })
        }
    }
}

/// 中间件链执行器
pub struct MiddlewareChain {
    middlewares: Arc<Vec<MiddlewareFn>>,
}

impl MiddlewareChain {
    pub fn new(middlewares: Vec<MiddlewareFn>) -> Self {
        Self {
            middlewares: Arc::new(middlewares),
        }
    }

    pub fn execute(
        &self,
        req: Request,
        handler: HandlerFn,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> {
        let next = Next::new(self.middlewares.clone(), handler);
        next.run(req)
    }
}

// ── 内置中间件 ────────────────────────────────────────────────────────────────

/// 请求日志中间件
pub fn logger_middleware() -> MiddlewareFn {
    Arc::new(|req, next| {
        Box::pin(async move {
            let method = req.method.as_str().to_string();
            let path = req.path.clone();
            let start = std::time::Instant::now();

            let response = next.run(req).await;

            let elapsed = start.elapsed().as_millis();
            let status = response
                .as_ref()
                .map(|r| r.status)
                .unwrap_or(500);

            tracing::info!(
                method = %method,
                path = %path,
                status = status,
                elapsed_ms = elapsed,
                "request"
            );

            response
        })
    })
}

/// CORS 中间件（允许所有来源，开发用）
pub fn cors_middleware() -> MiddlewareFn {
    Arc::new(|req, next| {
        Box::pin(async move {
            let mut response = next.run(req).await?;
            response.set_header("access-control-allow-origin", "*");
            response.set_header("access-control-allow-methods", "GET,POST,PUT,PATCH,DELETE,OPTIONS");
            response.set_header("access-control-allow-headers", "content-type,authorization");
            Ok(response)
        })
    })
}
