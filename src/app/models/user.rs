use serde_json::Value;
use crate::framework::database::model::{Model, Row, Param};
use crate::framework::error::Result;
use uuid::Uuid;

/// 示例用户模型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

impl Model for User {
    fn table_name() -> &'static str {
        "users"
    }

    fn from_row(row: &Row) -> Result<Self> {
        Ok(User {
            id: row.get_or::<String>("id"),
            name: row.get_or::<String>("name"),
            email: row.get_or::<String>("email"),
            created_at: row.get_or::<String>("created_at"),
        })
    }

    fn to_values(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("id", Value::String(self.id.clone())),
            ("name", Value::String(self.name.clone())),
            ("email", Value::String(self.email.clone())),
        ]
    }
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
