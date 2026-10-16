//! 基于 sqlx `Any` 驱动的统一数据库连接实现
//!
//! 一个实现同时支撑 SQLite / PostgreSQL / MySQL：
//! - 连接池、健康检查、超时由 sqlx `Pool` 提供
//! - 框架内部 SQL 统一使用 `?` 占位符，执行前按数据库类型改写（Postgres → `$n`）
//! - 事务通过 `Transaction<'static, Any>` 持有，期间所有查询路由到事务连接

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use sqlx::any::{Any, AnyArguments, AnyRow};
use sqlx::query::Query;
use sqlx::{AnyPool, Column, Row as _, Transaction, TypeInfo, ValueRef};
use tokio::sync::Mutex;

use crate::framework::database::connection::DbConnection;
use crate::framework::database::model::{Param, Row};
use crate::framework::database::pool::PoolConfig;
use crate::framework::error::{FastError, Result};

/// 数据库类型（由连接 URL scheme 推断）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    Sqlite,
    Postgres,
    Mysql,
}

impl DbKind {
    pub fn from_url(url: &str) -> Result<Self> {
        if url.starts_with("sqlite:") {
            Ok(Self::Sqlite)
        } else if url.starts_with("postgres:") || url.starts_with("postgresql:") {
            Ok(Self::Postgres)
        } else if url.starts_with("mysql:") {
            Ok(Self::Mysql)
        } else {
            Err(FastError::Database(format!(
                "Unsupported database URL scheme: {}",
                url
            )))
        }
    }

    /// Postgres 使用 `$n` 占位符，SQLite / MySQL 使用 `?`
    fn uses_dollar_placeholders(&self) -> bool {
        matches!(self, DbKind::Postgres)
    }
}

/// 统一的 sqlx 数据库连接
pub struct SqlxConnection {
    pool: AnyPool,
    kind: DbKind,
    /// 当前事务（同一时间最多一个；事务期间所有语句都路由到该连接）
    tx: Arc<Mutex<Option<Transaction<'static, Any>>>>,
}

impl SqlxConnection {
    /// 创建连接（惰性连接池：首次执行 SQL 时才真正建立连接）
    pub fn connect_lazy(url: &str, config: &PoolConfig) -> Result<Self> {
        install_drivers_once();
        let kind = DbKind::from_url(url)?;
        let pool = config
            .to_pool_options()
            .connect_lazy(url)
            .map_err(|e| FastError::Database(format!("Invalid database URL: {}", e)))?;
        Ok(Self {
            pool,
            kind,
            tx: Arc::new(Mutex::new(None)),
        })
    }

    pub fn kind(&self) -> DbKind {
        self.kind
    }

    /// 将框架内部统一的 `?` 占位符改写为目标数据库风格。
    /// 注意：字符串字面量内的 `?` 不会被改写，但仅做了简单的单引号配对处理。
    fn rewrite_placeholders(&self, sql: &str) -> String {
        if !self.kind.uses_dollar_placeholders() {
            return sql.to_string();
        }
        let mut out = String::with_capacity(sql.len() + 8);
        let mut n = 0usize;
        let mut in_str = false;
        for ch in sql.chars() {
            match ch {
                '\'' => {
                    in_str = !in_str;
                    out.push(ch);
                }
                '?' if !in_str => {
                    n += 1;
                    out.push('$');
                    out.push_str(&n.to_string());
                }
                _ => out.push(ch),
            }
        }
        out
    }

    fn bind_params<'q>(
        mut q: Query<'q, Any, AnyArguments<'q>>,
        params: &'q [Param],
    ) -> Query<'q, Any, AnyArguments<'q>> {
        for p in params {
            q = match p {
                Param::Null => q.bind(Option::<String>::None),
                Param::Bool(b) => q.bind(*b),
                Param::Int(i) => q.bind(*i),
                Param::Float(f) => q.bind(*f),
                Param::Text(s) => q.bind(s.clone()),
                Param::Bytes(b) => q.bind(b.clone()),
            };
        }
        q
    }
}

fn db_err(e: sqlx::Error) -> FastError {
    FastError::Database(e.to_string())
}

fn install_drivers_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        sqlx::any::install_default_drivers();
    });
}

impl DbConnection for SqlxConnection {
    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Row>>> + Send + 'a>> {
        Box::pin(async move {
            let sql = self.rewrite_placeholders(sql);
            let mut guard = self.tx.lock().await;
            let rows = if let Some(tx) = guard.as_mut() {
                Self::bind_params(sqlx::query(&sql), params)
                    .fetch_all(&mut **tx)
                    .await
            } else {
                Self::bind_params(sqlx::query(&sql), params)
                    .fetch_all(&self.pool)
                    .await
            }
            .map_err(db_err)?;
            rows.iter().map(any_row_to_row).collect()
        })
    }

    fn query_one<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Row>>> + Send + 'a>> {
        Box::pin(async move {
            let sql = self.rewrite_placeholders(sql);
            let mut guard = self.tx.lock().await;
            let row = if let Some(tx) = guard.as_mut() {
                Self::bind_params(sqlx::query(&sql), params)
                    .fetch_optional(&mut **tx)
                    .await
            } else {
                Self::bind_params(sqlx::query(&sql), params)
                    .fetch_optional(&self.pool)
                    .await
            }
            .map_err(db_err)?;
            row.map(|r| any_row_to_row(&r)).transpose()
        })
    }

    fn execute<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<u64>> + Send + 'a>> {
        Box::pin(async move {
            let sql = self.rewrite_placeholders(sql);
            let mut guard = self.tx.lock().await;
            let res = if let Some(tx) = guard.as_mut() {
                Self::bind_params(sqlx::query(&sql), params)
                    .execute(&mut **tx)
                    .await
            } else {
                Self::bind_params(sqlx::query(&sql), params)
                    .execute(&self.pool)
                    .await
            }
            .map_err(db_err)?;
            Ok(res.rows_affected())
        })
    }

    fn insert<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Param],
    ) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
        Box::pin(async move {
            let sql = self.rewrite_placeholders(sql);
            let mut guard = self.tx.lock().await;

            match self.kind {
                // Postgres 无 last_insert_id 概念，约定表含 `id` 列，用 RETURNING 取回
                DbKind::Postgres => {
                    let sql = format!("{} RETURNING id", sql);
                    let row = if let Some(tx) = guard.as_mut() {
                        Self::bind_params(sqlx::query(&sql), params)
                            .fetch_optional(&mut **tx)
                            .await
                    } else {
                        Self::bind_params(sqlx::query(&sql), params)
                            .fetch_optional(&self.pool)
                            .await
                    }
                    .map_err(db_err)?;
                    let row = row.ok_or_else(|| {
                        FastError::Database("INSERT ... RETURNING id returned no row".into())
                    })?;
                    row.try_get::<i64, _>(0).map_err(db_err)
                }
                DbKind::Sqlite | DbKind::Mysql => {
                    let id_sql = if self.kind == DbKind::Sqlite {
                        "SELECT last_insert_rowid()"
                    } else {
                        "SELECT LAST_INSERT_ID()"
                    };
                    // last_insert_id 是连接级状态，INSERT 与取值必须在同一连接上执行
                    if let Some(tx) = guard.as_mut() {
                        Self::bind_params(sqlx::query(&sql), params)
                            .execute(&mut **tx)
                            .await
                            .map_err(db_err)?;
                        sqlx::query_scalar::<_, i64>(id_sql)
                            .fetch_one(&mut **tx)
                            .await
                            .map_err(db_err)
                    } else {
                        let mut conn = self.pool.acquire().await.map_err(db_err)?;
                        Self::bind_params(sqlx::query(&sql), params)
                            .execute(&mut *conn)
                            .await
                            .map_err(db_err)?;
                        sqlx::query_scalar::<_, i64>(id_sql)
                            .fetch_one(&mut *conn)
                            .await
                            .map_err(db_err)
                    }
                }
            }
        })
    }

    fn begin_transaction<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut guard = self.tx.lock().await;
            if guard.is_some() {
                return Err(FastError::Database(
                    "Nested transactions are not supported".into(),
                ));
            }
            let tx = self.pool.begin().await.map_err(db_err)?;
            *guard = Some(tx);
            Ok(())
        })
    }

    fn commit_transaction<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let tx = self.tx.lock().await.take().ok_or_else(|| {
                FastError::Database("No active transaction to commit".into())
            })?;
            tx.commit().await.map_err(db_err)
        })
    }

    fn rollback_transaction<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let tx = self.tx.lock().await.take().ok_or_else(|| {
                FastError::Database("No active transaction to rollback".into())
            })?;
            tx.rollback().await.map_err(db_err)
        })
    }

    fn ping<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            sqlx::query("SELECT 1")
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
            Ok(())
        })
    }

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            self.pool.close().await;
            Ok(())
        })
    }
}

// ── 行值解码 ──────────────────────────────────────────────────────────────────

fn any_row_to_row(row: &AnyRow) -> Result<Row> {
    let mut map = HashMap::new();
    for col in row.columns() {
        let idx = col.ordinal();
        let raw = row.try_get_raw(idx).map_err(db_err)?;
        let value = if raw.is_null() {
            Value::Null
        } else {
            decode_value(row, idx, col.type_info().name())?
        };
        map.insert(col.name().to_string(), value);
    }
    Ok(Row::new(map))
}

/// 按声明类型名解码列值，失败时按备选类型依次降级
fn decode_value(row: &AnyRow, idx: usize, type_name: &str) -> Result<Value> {
    let ty = type_name.to_uppercase();

    if ty.contains("BOOL") {
        if let Ok(v) = row.try_get::<bool, _>(idx) {
            return Ok(Value::Bool(v));
        }
    }
    if ty.contains("INT") || ty == "SERIAL" {
        if let Ok(v) = row.try_get::<i64, _>(idx) {
            return Ok(Value::Number(v.into()));
        }
    }
    if ["REAL", "FLOA", "DOUB", "NUM", "DEC"].iter().any(|p| ty.contains(p)) {
        if let Ok(v) = row.try_get::<f64, _>(idx) {
            return Ok(f64_to_json(v));
        }
    }
    if ["BLOB", "BYTEA", "BINARY"].iter().any(|p| ty.contains(p)) {
        if let Ok(v) = row.try_get::<Vec<u8>, _>(idx) {
            use base64::Engine;
            return Ok(Value::String(
                base64::engine::general_purpose::STANDARD.encode(v),
            ));
        }
    }

    // 兜底链：String（TEXT/VARCHAR/JSON/UUID/DATETIME 文本…）→ i64 → f64 → bool
    // 注意：sqlx Any 驱动不支持 chrono 类型解码；时间列在 SQLite/MySQL 中通常
    // 以文本存储可直接解为 String，Postgres 原生时间类型建议在 SQL 中 CAST 为 TEXT。
    if let Ok(v) = row.try_get::<String, _>(idx) {
        return Ok(Value::String(v));
    }
    if let Ok(v) = row.try_get::<i64, _>(idx) {
        return Ok(Value::Number(v.into()));
    }
    if let Ok(v) = row.try_get::<f64, _>(idx) {
        return Ok(f64_to_json(v));
    }
    if let Ok(v) = row.try_get::<bool, _>(idx) {
        return Ok(Value::Bool(v));
    }

    Err(FastError::Database(format!(
        "Cannot decode column of type '{}'",
        type_name
    )))
}

fn f64_to_json(v: f64) -> Value {
    serde_json::Number::from_f64(v)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}
