use crate::framework::http::Router;
use crate::app::http::controllers::user_controller;

/// 注册 API 路由
pub fn register(router: &mut Router) {
    router.group("/api", |r| {
        // 用户 CRUD
        r.get("/users", user_controller::index);
        r.post("/users", user_controller::store);
        r.get("/users/:id", user_controller::show);
        r.delete("/users/:id", user_controller::destroy);

        // 健康检查
        r.get("/health", |_req| async move {
            Ok(crate::framework::http::Response::success(
                serde_json::json!({ "status": "ok", "version": "0.1.0" }),
                "healthy",
            ))
        });
    });
}
