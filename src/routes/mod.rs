pub mod api;

use crate::framework::http::Router;

/// 注册所有路由
pub fn register_all(router: &mut Router) {
    api::register(router);
}
