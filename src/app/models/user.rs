use crate::framework::database::model::Model;
use uuid::Uuid;

/// 示例用户模型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Model)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    /// 由应用侧生成，INSERT 时不写入（保留数据库/默认值语义）
    #[model(skip_insert)]
    pub created_at: String,
}

impl User {
    pub fn new(name: &str, email: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            email: email.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
