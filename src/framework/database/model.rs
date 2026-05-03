use serde_json::Value;
use std::collections::HashMap;
use crate::framework::error::{FastError, Result};

/// 数据库行（从查询结果中解析的列值映射）
#[derive(Debug, Clone)]
pub struct Row {
    columns: HashMap<String, Value>,
}

impl Row {
    pub fn new(columns: HashMap<String, Value>) -> Self {
        Self { columns }
    }

    /// 获取列值（原始 JSON Value）
    pub fn get_raw(&self, col: &str) -> Option<&Value> {
        self.columns.get(col)
    }

    /// 获取列值并反序列化为目标类型
    pub fn get<T: serde::de::DeserializeOwned>(&self, col: &str) -> Result<T> {
        let val = self.columns.get(col).ok_or_else(|| {
            FastError::Database(format!("Column '{}' not found in row", col))
        })?;
        serde_json::from_value(val.clone())
            .map_err(|e| FastError::Database(format!("Failed to deserialize column '{}': {}", col, e)))
    }

    /// 获取列值，不存在时返回默认值
    pub fn get_or<T: serde::de::DeserializeOwned + Default>(&self, col: &str) -> T {
        self.get(col).unwrap_or_default()
    }

    /// 获取所有列名
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.keys().map(|s| s.as_str()).collect()
    }
}

/// Model Trait：所有数据库模型必须实现此 Trait
pub trait Model: Sized + Send + Sync + 'static {
    /// 数据库表名
    fn table_name() -> &'static str;

    /// 主键字段名（默认 "id"）
    fn primary_key() -> &'static str {
        "id"
    }

    /// 从数据库行构造模型实例
    fn from_row(row: &Row) -> Result<Self>;

    /// 将模型转换为 (列名, 值) 对（用于 INSERT/UPDATE）
    fn to_values(&self) -> Vec<(&'static str, Value)>;
}

/// SQL 参数值
#[derive(Debug, Clone)]
pub enum Param {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
}

impl From<i32> for Param {
    fn from(v: i32) -> Self { Param::Int(v as i64) }
}
impl From<i64> for Param {
    fn from(v: i64) -> Self { Param::Int(v) }
}
impl From<u32> for Param {
    fn from(v: u32) -> Self { Param::Int(v as i64) }
}
impl From<u64> for Param {
    fn from(v: u64) -> Self { Param::Int(v as i64) }
}
impl From<f32> for Param {
    fn from(v: f32) -> Self { Param::Float(v as f64) }
}
impl From<f64> for Param {
    fn from(v: f64) -> Self { Param::Float(v) }
}
impl From<String> for Param {
    fn from(v: String) -> Self { Param::Text(v) }
}
impl From<&str> for Param {
    fn from(v: &str) -> Self { Param::Text(v.to_string()) }
}
impl From<bool> for Param {
    fn from(v: bool) -> Self { Param::Bool(v) }
}
impl<T: Into<Param>> From<Option<T>> for Param {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => Param::Null,
        }
    }
}

/// WHERE 条件
#[derive(Debug, Clone)]
pub enum Condition {
    Eq(String, Param),
    Ne(String, Param),
    Gt(String, Param),
    Gte(String, Param),
    Lt(String, Param),
    Lte(String, Param),
    Like(String, String),
    NotLike(String, String),
    In(String, Vec<Param>),
    NotIn(String, Vec<Param>),
    IsNull(String),
    IsNotNull(String),
    Between(String, Param, Param),
    NotBetween(String, Param, Param),
    Raw(String, Vec<Param>),
}

/// ORDER BY 排序方向
#[derive(Debug, Clone)]
pub enum Order {
    Asc,
    Desc,
}

impl Order {
    pub fn as_str(&self) -> &'static str {
        match self {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        }
    }
}
