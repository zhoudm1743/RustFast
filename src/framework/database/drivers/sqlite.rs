#![cfg(feature = "sqlite")]

use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;
use std::sync::Mutex;

use rusqlite::{Connection, params_from_iter, types::ValueRef};
use serde_json::Value;

use crate::framework::database::connection::DbConnection;
use crate::framework::database::model::{Param, Row};
use crate::framework::error::{FastError, Result};

/// SQLite 连接配置
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// 数据库文件路径，`:memory:` 为内存数据库
    pub path: String,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: ":memory:".to_string(),
        }
    }
}

/// SQLite 数据库连接
///
/// 使用 `Mutex<Connection>` 包装以满足 `Send + Sync`（rusqlite Connection 非 Send）
pub struct SqliteConnection {
    conn: Mutex<Connection>,
}

impl SqliteConnection {
    pub fn open(config: &SqliteConfig) -> Result<Self> {
        let conn = Connection::open(&config.path)
            .map_err(|e| FastError::Database(format!("SQLite open error: {}", e)))?;

        // 开启 WAL 模式提升并发性能
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| FastError::Database(format!("SQLite pragma error: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl DbConnection for SqliteConnection {
    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Row>>> + Send + 'a>> {
        Box::pin(async move {
            let conn = self.conn.lock().unwrap();
            let sqlite_params = params_to_sqlite(params);
            let mut stmt = conn
                .prepare(sql)
                .map_err(|e| FastError::Database(format!("SQLite prepare error: {}", e)))?;

            let column_names: Vec<String> = stmt
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();

            let rows = stmt
                .query_map(params_from_iter(sqlite_params.iter().map(|p| p as &dyn rusqlite::ToSql)), |row| {
                    let mut map = HashMap::new();
                    for (i, col_name) in column_names.iter().enumerate() {
                        let val = match row.get_ref(i)? {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(n) => Value::Number(n.into()),
                            ValueRef::Real(f) => {
                                Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into()))
                            }
                            ValueRef::Text(s) => {
                                Value::String(String::from_utf8_lossy(s).into_owned())
                            }
                            ValueRef::Blob(b) => {
                                Value::String(base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    b,
                                ))
                            }
                        };
                        map.insert(col_name.clone(), val);
                    }
                    Ok(Row::new(map))
                })
                .map_err(|e| FastError::Database(format!("SQLite query error: {}", e)))?
                .collect::<std::result::Result<Vec<Row>, _>>()
                .map_err(|e| FastError::Database(format!("SQLite row error: {}", e)))?;

            Ok(rows)
        })
    }

    fn query_one<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Row>>> + Send + 'a>> {
        Box::pin(async move {
            let mut rows = self.query(sql, params).await?;
            Ok(rows.into_iter().next())
        })
    }

    fn execute<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<u64>> + Send + 'a>> {
        Box::pin(async move {
            let conn = self.conn.lock().unwrap();
            let sqlite_params = params_to_sqlite(params);
            let affected = conn
                .execute(sql, params_from_iter(sqlite_params.iter().map(|p| p as &dyn rusqlite::ToSql)))
                .map_err(|e| FastError::Database(format!("SQLite execute error: {}", e)))?;
            Ok(affected as u64)
        })
    }

    fn insert<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
        Box::pin(async move {
            let conn = self.conn.lock().unwrap();
            let sqlite_params = params_to_sqlite(params);
            conn.execute(sql, params_from_iter(sqlite_params.iter().map(|p| p as &dyn rusqlite::ToSql)))
                .map_err(|e| FastError::Database(format!("SQLite insert error: {}", e)))?;
            Ok(conn.last_insert_rowid())
        })
    }

    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let conn = self.conn.lock().unwrap();
            conn.execute_batch("SELECT 1")
                .map_err(|e| FastError::Database(format!("SQLite ping error: {}", e)))?;
            Ok(())
        })
    }

    fn begin_transaction<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let conn = self.conn.lock().unwrap();
            conn.execute_batch("BEGIN")
                .map_err(|e| FastError::Database(format!("SQLite begin error: {}", e)))?;
            Ok(())
        })
    }

    fn commit_transaction<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let conn = self.conn.lock().unwrap();
            conn.execute_batch("COMMIT")
                .map_err(|e| FastError::Database(format!("SQLite commit error: {}", e)))?;
            Ok(())
        })
    }

    fn rollback_transaction<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let conn = self.conn.lock().unwrap();
            conn.execute_batch("ROLLBACK")
                .map_err(|e| FastError::Database(format!("SQLite rollback error: {}", e)))?;
            Ok(())
        })
    }

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }
}

// ── 参数转换 ──────────────────────────────────────────────────────────────────

enum SqliteValue {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl rusqlite::ToSql for SqliteValue {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        use rusqlite::types::{ToSqlOutput, Value};
        Ok(match self {
            SqliteValue::Null => ToSqlOutput::Owned(Value::Null),
            SqliteValue::Int(i) => ToSqlOutput::Owned(Value::Integer(*i)),
            SqliteValue::Float(f) => ToSqlOutput::Owned(Value::Real(*f)),
            SqliteValue::Text(s) => ToSqlOutput::Owned(Value::Text(s.clone())),
            SqliteValue::Blob(b) => ToSqlOutput::Owned(Value::Blob(b.clone())),
        })
    }
}

fn params_to_sqlite(params: &[Param]) -> Vec<SqliteValue> {
    params
        .iter()
        .map(|p| match p {
            Param::Null => SqliteValue::Null,
            Param::Bool(b) => SqliteValue::Int(if *b { 1 } else { 0 }),
            Param::Int(i) => SqliteValue::Int(*i),
            Param::Float(f) => SqliteValue::Float(*f),
            Param::Text(s) => SqliteValue::Text(s.clone()),
            Param::Bytes(b) => SqliteValue::Blob(b.clone()),
        })
        .collect()
}
