use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;

use crate::framework::http::middleware::{MiddlewareChain, MiddlewareFn, HandlerFn};
use crate::framework::http::parser::parse_request;
use crate::framework::http::request::Method;
use crate::framework::http::response::Response;
use crate::framework::http::router::Router;
use crate::framework::error::{FastError, Result};

/// HTTP 服务器
pub struct HttpServer {
    router: Arc<Router>,
    global_middlewares: Vec<MiddlewareFn>,
}

impl HttpServer {
    pub fn new(router: Router) -> Self {
        Self {
            router: Arc::new(router),
            global_middlewares: Vec::new(),
        }
    }

    /// 添加全局中间件
    pub fn use_middleware(&mut self, mw: MiddlewareFn) {
        self.global_middlewares.push(mw);
    }

    /// 启动服务器（阻塞，直到关闭信号）
    pub async fn serve(self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| FastError::Http(format!("Failed to bind {}: {}", addr, e)))?;

        tracing::info!("RustFast HTTP server listening on http://{}", addr);

        let router = self.router.clone();
        let middlewares = Arc::new(self.global_middlewares.clone());

        loop {
            let (stream, peer_addr) = listener.accept().await.map_err(|e| {
                FastError::Http(format!("Accept error: {}", e))
            })?;

            let router = router.clone();
            let middlewares = middlewares.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, peer_addr, router, middlewares).await {
                    tracing::debug!("Connection error: {}", e);
                }
            });
        }
    }
}

/// 处理单个 TCP 连接
async fn handle_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    router: Arc<Router>,
    middlewares: Arc<Vec<MiddlewareFn>>,
) -> Result<()> {
    let remote_addr = peer_addr.to_string();

    let mut request = match parse_request(&mut stream, remote_addr).await {
        Ok(req) => req,
        Err(e) => {
            let resp = Response::fail(400, 400, &format!("Bad Request: {}", e));
            stream.write_all(&resp.to_bytes()).await?;
            return Ok(());
        }
    };

    // OPTIONS 预检请求直接响应
    if request.method == Method::Options {
        let mut resp = Response::new(204);
        resp.set_header("access-control-allow-origin", "*");
        resp.set_header("access-control-allow-methods", "GET,POST,PUT,PATCH,DELETE,OPTIONS");
        resp.set_header("access-control-allow-headers", "content-type,authorization");
        stream.write_all(&resp.to_bytes()).await?;
        return Ok(());
    }

    let response = match router.find(&request.method, &request.path) {
        Some(route_match) => {
            // 填充路径参数
            request.params = route_match.params;

            let handler: HandlerFn = route_match.handler;
            let chain = MiddlewareChain::new(middlewares.as_ref().clone());
            match chain.execute(request, handler).await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!("Handler error: {}", e);
                    match e {
                        FastError::NotFound(msg) => Response::fail(404, 404, &msg),
                        FastError::Unauthorized(msg) => Response::fail(401, 401, &msg),
                        FastError::Validation(msg) => Response::fail(422, 422, &msg),
                        _ => Response::fail(500, 500, "Internal Server Error"),
                    }
                }
            }
        }
        None => {
            if router.has_path(&request.path) {
                Response::fail(405, 405, "Method Not Allowed")
            } else {
                Response::fail(404, 404, "Not Found")
            }
        }
    };

    stream.write_all(&response.to_bytes()).await?;
    stream.flush().await?;

    Ok(())
}
