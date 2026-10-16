use rustfast::bootstrap;
use rustfast::framework::http::{HttpServer, Router};
use rustfast::framework::http::middleware::{logger_middleware, cors_middleware};
use rustfast::framework::facades::Config;
use rustfast::routes;

#[tokio::main]
async fn main() {
    // ── 1. 引导应用（注册所有 Provider，初始化服务）────────────────────────────
    let _app = bootstrap::boot();

    // ── 1.5 控制台命令（migrate / migrate:rollback / migrate:status）───────────
    let args: Vec<String> = std::env::args().collect();
    match bootstrap::commands::run_console(&args).await {
        Ok(true) => return,
        Ok(false) => {}
        Err(e) => {
            eprintln!("Command failed: {}", e);
            std::process::exit(1);
        }
    }

    tracing::info!(
        "Starting {} v{}",
        Config::get_or("app.name", "RustFast".to_string()),
        "0.1.0"
    );

    // ── 2. 注册路由 ────────────────────────────────────────────────────────────
    let mut router = Router::new();
    routes::register_all(&mut router);

    // ── 3. 启动 HTTP 服务器 ────────────────────────────────────────────────────
    let host = Config::get_or("app.host", "0.0.0.0".to_string());
    let port = Config::get_or("app.port", 8080u16);
    let addr = format!("{}:{}", host, port);

    let mut server = HttpServer::new(router);

    // TLS（可选）：配置 server.tls.cert / server.tls.key 后启用 HTTPS
    #[cfg(feature = "tls")]
    if let (Some(cert), Some(key)) = (
        Config::get::<String>("server.tls.cert"),
        Config::get::<String>("server.tls.key"),
    ) {
        server.enable_tls(&cert, &key).expect("Failed to enable TLS");
        tracing::info!("TLS enabled");
    }
    #[cfg(not(feature = "tls"))]
    if Config::get::<String>("server.tls.cert").is_some() {
        tracing::warn!("server.tls is configured but the binary was built without the 'tls' feature; starting plain HTTP");
    }

    // 添加全局中间件
    server.use_middleware(logger_middleware());
    server.use_middleware(cors_middleware());

    // ── 4. 优雅关闭信号处理 ────────────────────────────────────────────────────
    let app_ref = _app.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
        tracing::info!("Shutdown signal received, stopping server...");
        app_ref.shutdown();
        std::process::exit(0);
    });

    // ── 5. 启动服务器（阻塞） ──────────────────────────────────────────────────
    if let Err(e) = server.serve(&addr).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
