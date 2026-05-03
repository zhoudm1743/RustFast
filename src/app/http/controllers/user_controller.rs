use crate::framework::http::{Request, Response};
use crate::framework::error::Result;
use crate::framework::facades::{Db, Cache};
use crate::framework::database::model::Model;
use crate::app::models::User;

/// GET /api/users — 获取用户列表
pub async fn index(req: Request) -> Result<Response> {
    // 先查缓存
    if let Some(cached) = Cache::get("users:all") {
        let data: serde_json::Value = serde_json::from_str(&cached)?;
        return Ok(Response::success(data, "ok"));
    }

    let page: u64 = req.query("page").and_then(|v| v.parse().ok()).unwrap_or(1);
    let size: u64 = req.query("size").and_then(|v| v.parse().ok()).unwrap_or(10);

    let (users, total) = Db::table("users")
        .order_desc("created_at")
        .paginate::<User>(page, size)
        .await?;

    Ok(Response::paginate(users, total, page as i64, size as i64, "ok"))
}

/// GET /api/users/:id — 获取单个用户
pub async fn show(req: Request) -> Result<Response> {
    let id = req.param("id").unwrap_or("");

    let user = Db::table("users")
        .where_eq("id", id)
        .first_or_fail::<User>()
        .await?;

    Ok(Response::success(user, "ok"))
}

/// POST /api/users — 创建用户
pub async fn store(req: Request) -> Result<Response> {
    #[derive(serde::Deserialize)]
    struct CreateUserRequest {
        name: String,
        email: String,
    }

    let body: CreateUserRequest = req.json()?;

    // 验证
    use crate::framework::validation::{Validator, FieldValidator};
    Validator::new()
        .field(FieldValidator::new("name", Some(&body.name)).required().min_len(2).max_len(50))
        .field(FieldValidator::new("email", Some(&body.email)).required().email())
        .check()?;

    let user = User::new(&body.name, &body.email);
    let id = Db::table("users")
        .insert(user.to_values())
        .await?;

    // 清除缓存
    Cache::forget("users:all");

    Ok(Response::success(
        serde_json::json!({ "id": id }),
        "User created successfully",
    ))
}

/// DELETE /api/users/:id — 删除用户
pub async fn destroy(req: Request) -> Result<Response> {
    let id = req.param("id").unwrap_or("");

    Db::table("users")
        .where_eq("id", id)
        .delete()
        .await?;

    Cache::forget("users:all");

    Ok(Response::success(serde_json::Value::Null, "User deleted"))
}
