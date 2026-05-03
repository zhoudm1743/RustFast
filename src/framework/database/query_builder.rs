use std::sync::Arc;
use serde_json::Value;
use crate::framework::database::connection::DbConnection;
use crate::framework::database::model::{Condition, Model, Order, Param, Row};
use crate::framework::error::{FastError, Result};

// ── WHERE 逻辑 ────────────────────────────────────────────────────────────────

/// AND / OR 连接符
#[derive(Debug, Clone)]
pub enum Connector {
    And,
    Or,
}

/// WHERE 子句（可嵌套）
#[derive(Debug, Clone)]
pub enum WhereInner {
    Simple(Condition),
    Group(Vec<WhereClause>),
}

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub connector: Connector,
    pub inner: WhereInner,
}

// ── WHERE 分组构造器 ──────────────────────────────────────────────────────────

/// 用于构造 WHERE/HAVING 嵌套分组的辅助构造器
#[derive(Debug, Clone, Default)]
pub struct WhereGroupBuilder {
    pub(crate) clauses: Vec<WhereClause>,
}

impl WhereGroupBuilder {
    pub fn new() -> Self {
        Self { clauses: Vec::new() }
    }

    fn add(mut self, connector: Connector, cond: Condition) -> Self {
        self.clauses.push(WhereClause { connector, inner: WhereInner::Simple(cond) });
        self
    }

    pub fn where_eq(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::Eq(col.to_string(), val.into()))
    }
    pub fn or_where_eq(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::Or, Condition::Eq(col.to_string(), val.into()))
    }
    pub fn where_ne(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::Ne(col.to_string(), val.into()))
    }
    pub fn or_where_ne(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::Or, Condition::Ne(col.to_string(), val.into()))
    }
    pub fn where_gt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::Gt(col.to_string(), val.into()))
    }
    pub fn or_where_gt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::Or, Condition::Gt(col.to_string(), val.into()))
    }
    pub fn where_gte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::Gte(col.to_string(), val.into()))
    }
    pub fn or_where_gte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::Or, Condition::Gte(col.to_string(), val.into()))
    }
    pub fn where_lt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::Lt(col.to_string(), val.into()))
    }
    pub fn or_where_lt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::Or, Condition::Lt(col.to_string(), val.into()))
    }
    pub fn where_lte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::Lte(col.to_string(), val.into()))
    }
    pub fn or_where_lte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add(Connector::Or, Condition::Lte(col.to_string(), val.into()))
    }
    pub fn where_like(self, col: &str, pattern: &str) -> Self {
        self.add(Connector::And, Condition::Like(col.to_string(), pattern.to_string()))
    }
    pub fn or_where_like(self, col: &str, pattern: &str) -> Self {
        self.add(Connector::Or, Condition::Like(col.to_string(), pattern.to_string()))
    }
    pub fn where_not_like(self, col: &str, pattern: &str) -> Self {
        self.add(Connector::And, Condition::NotLike(col.to_string(), pattern.to_string()))
    }
    pub fn or_where_not_like(self, col: &str, pattern: &str) -> Self {
        self.add(Connector::Or, Condition::NotLike(col.to_string(), pattern.to_string()))
    }
    pub fn where_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add(Connector::And, Condition::In(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn or_where_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add(Connector::Or, Condition::In(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn where_not_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add(Connector::And, Condition::NotIn(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn or_where_not_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add(Connector::Or, Condition::NotIn(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn where_null(self, col: &str) -> Self {
        self.add(Connector::And, Condition::IsNull(col.to_string()))
    }
    pub fn or_where_null(self, col: &str) -> Self {
        self.add(Connector::Or, Condition::IsNull(col.to_string()))
    }
    pub fn where_not_null(self, col: &str) -> Self {
        self.add(Connector::And, Condition::IsNotNull(col.to_string()))
    }
    pub fn or_where_not_null(self, col: &str) -> Self {
        self.add(Connector::Or, Condition::IsNotNull(col.to_string()))
    }
    pub fn where_between(self, col: &str, lo: impl Into<Param>, hi: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::Between(col.to_string(), lo.into(), hi.into()))
    }
    pub fn or_where_between(self, col: &str, lo: impl Into<Param>, hi: impl Into<Param>) -> Self {
        self.add(Connector::Or, Condition::Between(col.to_string(), lo.into(), hi.into()))
    }
    pub fn where_not_between(self, col: &str, lo: impl Into<Param>, hi: impl Into<Param>) -> Self {
        self.add(Connector::And, Condition::NotBetween(col.to_string(), lo.into(), hi.into()))
    }
    pub fn where_raw(mut self, sql: &str, params: Vec<Param>) -> Self {
        self.clauses.push(WhereClause {
            connector: Connector::And,
            inner: WhereInner::Simple(Condition::Raw(sql.to_string(), params)),
        });
        self
    }
    pub fn or_where_raw(mut self, sql: &str, params: Vec<Param>) -> Self {
        self.clauses.push(WhereClause {
            connector: Connector::Or,
            inner: WhereInner::Simple(Condition::Raw(sql.to_string(), params)),
        });
        self
    }

    /// 嵌套 AND 分组
    pub fn where_group<F: FnOnce(WhereGroupBuilder) -> WhereGroupBuilder>(mut self, f: F) -> Self {
        let inner = f(WhereGroupBuilder::new());
        self.clauses.push(WhereClause {
            connector: Connector::And,
            inner: WhereInner::Group(inner.clauses),
        });
        self
    }

    /// 嵌套 OR 分组
    pub fn or_where_group<F: FnOnce(WhereGroupBuilder) -> WhereGroupBuilder>(mut self, f: F) -> Self {
        let inner = f(WhereGroupBuilder::new());
        self.clauses.push(WhereClause {
            connector: Connector::Or,
            inner: WhereInner::Group(inner.clauses),
        });
        self
    }
}

// ── SQL 构造辅助函数 ───────────────────────────────────────────────────────────

/// 将 Condition 编译为 SQL 片段（SQLite `?` 占位符风格）
fn condition_to_sql(cond: &Condition, params: &mut Vec<Param>) -> String {
    match cond {
        Condition::Eq(col, val) => { params.push(val.clone()); format!("{} = ?", col) }
        Condition::Ne(col, val) => { params.push(val.clone()); format!("{} != ?", col) }
        Condition::Gt(col, val) => { params.push(val.clone()); format!("{} > ?", col) }
        Condition::Gte(col, val) => { params.push(val.clone()); format!("{} >= ?", col) }
        Condition::Lt(col, val) => { params.push(val.clone()); format!("{} < ?", col) }
        Condition::Lte(col, val) => { params.push(val.clone()); format!("{} <= ?", col) }
        Condition::Like(col, pattern) => {
            params.push(Param::Text(pattern.clone()));
            format!("{} LIKE ?", col)
        }
        Condition::NotLike(col, pattern) => {
            params.push(Param::Text(pattern.clone()));
            format!("{} NOT LIKE ?", col)
        }
        Condition::In(col, vals) => {
            let ph = vals.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            params.extend(vals.iter().cloned());
            format!("{} IN ({})", col, ph)
        }
        Condition::NotIn(col, vals) => {
            let ph = vals.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            params.extend(vals.iter().cloned());
            format!("{} NOT IN ({})", col, ph)
        }
        Condition::IsNull(col) => format!("{} IS NULL", col),
        Condition::IsNotNull(col) => format!("{} IS NOT NULL", col),
        Condition::Between(col, lo, hi) => {
            params.push(lo.clone());
            params.push(hi.clone());
            format!("{} BETWEEN ? AND ?", col)
        }
        Condition::NotBetween(col, lo, hi) => {
            params.push(lo.clone());
            params.push(hi.clone());
            format!("{} NOT BETWEEN ? AND ?", col)
        }
        Condition::Raw(sql, raw_params) => {
            params.extend(raw_params.iter().cloned());
            sql.clone()
        }
    }
}

/// 将 WhereClause 列表编译为 SQL 字符串（不含 WHERE/HAVING 关键字）
fn clauses_to_sql(clauses: &[WhereClause], params: &mut Vec<Param>) -> String {
    if clauses.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for (i, clause) in clauses.iter().enumerate() {
        let fragment = match &clause.inner {
            WhereInner::Simple(cond) => condition_to_sql(cond, params),
            WhereInner::Group(inner) => {
                let inner_sql = clauses_to_sql(inner, params);
                format!("({})", inner_sql)
            }
        };
        if i == 0 {
            parts.push(fragment);
        } else {
            let conn_str = match clause.connector {
                Connector::And => "AND",
                Connector::Or => "OR",
            };
            parts.push(format!("{} {}", conn_str, fragment));
        }
    }
    parts.join(" ")
}

fn build_where_sql(clauses: &[WhereClause], params: &mut Vec<Param>) -> String {
    if clauses.is_empty() { return String::new(); }
    format!(" WHERE {}", clauses_to_sql(clauses, params))
}

fn build_having_sql(clauses: &[WhereClause], params: &mut Vec<Param>) -> String {
    if clauses.is_empty() { return String::new(); }
    format!(" HAVING {}", clauses_to_sql(clauses, params))
}

// ── 查询构造器 ────────────────────────────────────────────────────────────────

/// 链式 SQL 查询构造器（GORM 风格）
#[derive(Clone)]
pub struct QueryBuilder {
    pub(crate) conn: Arc<dyn DbConnection>,
    pub(crate) table: String,
    pub(crate) selects: Vec<String>,
    pub(crate) where_clauses: Vec<WhereClause>,
    pub(crate) joins: Vec<String>,
    pub(crate) order_by: Vec<(String, Order)>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
    pub(crate) group_by: Vec<String>,
    pub(crate) having_clauses: Vec<WhereClause>,
}

impl QueryBuilder {
    pub fn new(conn: Arc<dyn DbConnection>, table: &str) -> Self {
        Self {
            conn,
            table: table.to_string(),
            selects: vec!["*".to_string()],
            where_clauses: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            group_by: Vec::new(),
            having_clauses: Vec::new(),
        }
    }

    // ── SELECT ────────────────────────────────────────────────────────────────

    pub fn select(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.selects = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    // ── WHERE ─────────────────────────────────────────────────────────────────

    fn add_where(mut self, connector: Connector, cond: Condition) -> Self {
        self.where_clauses.push(WhereClause { connector, inner: WhereInner::Simple(cond) });
        self
    }

    pub fn where_eq(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::Eq(col.to_string(), val.into()))
    }
    pub fn or_where_eq(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::Eq(col.to_string(), val.into()))
    }
    pub fn where_ne(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::Ne(col.to_string(), val.into()))
    }
    pub fn or_where_ne(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::Ne(col.to_string(), val.into()))
    }
    pub fn where_gt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::Gt(col.to_string(), val.into()))
    }
    pub fn or_where_gt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::Gt(col.to_string(), val.into()))
    }
    pub fn where_gte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::Gte(col.to_string(), val.into()))
    }
    pub fn or_where_gte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::Gte(col.to_string(), val.into()))
    }
    pub fn where_lt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::Lt(col.to_string(), val.into()))
    }
    pub fn or_where_lt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::Lt(col.to_string(), val.into()))
    }
    pub fn where_lte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::Lte(col.to_string(), val.into()))
    }
    pub fn or_where_lte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::Lte(col.to_string(), val.into()))
    }
    pub fn where_like(self, col: &str, pattern: &str) -> Self {
        self.add_where(Connector::And, Condition::Like(col.to_string(), pattern.to_string()))
    }
    pub fn or_where_like(self, col: &str, pattern: &str) -> Self {
        self.add_where(Connector::Or, Condition::Like(col.to_string(), pattern.to_string()))
    }
    pub fn where_not_like(self, col: &str, pattern: &str) -> Self {
        self.add_where(Connector::And, Condition::NotLike(col.to_string(), pattern.to_string()))
    }
    pub fn or_where_not_like(self, col: &str, pattern: &str) -> Self {
        self.add_where(Connector::Or, Condition::NotLike(col.to_string(), pattern.to_string()))
    }
    pub fn where_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add_where(Connector::And, Condition::In(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn or_where_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add_where(Connector::Or, Condition::In(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn where_not_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add_where(Connector::And, Condition::NotIn(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn or_where_not_in(self, col: &str, vals: impl IntoIterator<Item = impl Into<Param>>) -> Self {
        self.add_where(Connector::Or, Condition::NotIn(col.to_string(), vals.into_iter().map(|v| v.into()).collect()))
    }
    pub fn where_null(self, col: &str) -> Self {
        self.add_where(Connector::And, Condition::IsNull(col.to_string()))
    }
    pub fn or_where_null(self, col: &str) -> Self {
        self.add_where(Connector::Or, Condition::IsNull(col.to_string()))
    }
    pub fn where_not_null(self, col: &str) -> Self {
        self.add_where(Connector::And, Condition::IsNotNull(col.to_string()))
    }
    pub fn or_where_not_null(self, col: &str) -> Self {
        self.add_where(Connector::Or, Condition::IsNotNull(col.to_string()))
    }
    pub fn where_between(self, col: &str, lo: impl Into<Param>, hi: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::Between(col.to_string(), lo.into(), hi.into()))
    }
    pub fn or_where_between(self, col: &str, lo: impl Into<Param>, hi: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::Between(col.to_string(), lo.into(), hi.into()))
    }
    pub fn where_not_between(self, col: &str, lo: impl Into<Param>, hi: impl Into<Param>) -> Self {
        self.add_where(Connector::And, Condition::NotBetween(col.to_string(), lo.into(), hi.into()))
    }
    pub fn or_where_not_between(self, col: &str, lo: impl Into<Param>, hi: impl Into<Param>) -> Self {
        self.add_where(Connector::Or, Condition::NotBetween(col.to_string(), lo.into(), hi.into()))
    }
    pub fn where_raw(mut self, sql: &str, params: Vec<Param>) -> Self {
        self.where_clauses.push(WhereClause {
            connector: Connector::And,
            inner: WhereInner::Simple(Condition::Raw(sql.to_string(), params)),
        });
        self
    }
    pub fn or_where_raw(mut self, sql: &str, params: Vec<Param>) -> Self {
        self.where_clauses.push(WhereClause {
            connector: Connector::Or,
            inner: WhereInner::Simple(Condition::Raw(sql.to_string(), params)),
        });
        self
    }

    /// AND 嵌套分组：WHERE ... AND (col1 = ? OR col2 = ?)
    pub fn where_group<F: FnOnce(WhereGroupBuilder) -> WhereGroupBuilder>(mut self, f: F) -> Self {
        let grp = f(WhereGroupBuilder::new());
        self.where_clauses.push(WhereClause {
            connector: Connector::And,
            inner: WhereInner::Group(grp.clauses),
        });
        self
    }

    /// OR 嵌套分组：WHERE ... OR (col1 = ? AND col2 = ?)
    pub fn or_where_group<F: FnOnce(WhereGroupBuilder) -> WhereGroupBuilder>(mut self, f: F) -> Self {
        let grp = f(WhereGroupBuilder::new());
        self.where_clauses.push(WhereClause {
            connector: Connector::Or,
            inner: WhereInner::Group(grp.clauses),
        });
        self
    }

    // ── HAVING ────────────────────────────────────────────────────────────────

    fn add_having(mut self, connector: Connector, cond: Condition) -> Self {
        self.having_clauses.push(WhereClause { connector, inner: WhereInner::Simple(cond) });
        self
    }

    pub fn having_eq(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_having(Connector::And, Condition::Eq(col.to_string(), val.into()))
    }
    pub fn having_gt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_having(Connector::And, Condition::Gt(col.to_string(), val.into()))
    }
    pub fn having_gte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_having(Connector::And, Condition::Gte(col.to_string(), val.into()))
    }
    pub fn having_lt(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_having(Connector::And, Condition::Lt(col.to_string(), val.into()))
    }
    pub fn having_lte(self, col: &str, val: impl Into<Param>) -> Self {
        self.add_having(Connector::And, Condition::Lte(col.to_string(), val.into()))
    }
    pub fn having_raw(mut self, sql: &str, params: Vec<Param>) -> Self {
        self.having_clauses.push(WhereClause {
            connector: Connector::And,
            inner: WhereInner::Simple(Condition::Raw(sql.to_string(), params)),
        });
        self
    }

    // ── JOIN ──────────────────────────────────────────────────────────────────

    pub fn join(mut self, join_clause: &str) -> Self {
        self.joins.push(join_clause.to_string());
        self
    }

    pub fn left_join(self, table: &str, on: &str) -> Self {
        self.join(&format!("LEFT JOIN {} ON {}", table, on))
    }

    pub fn inner_join(self, table: &str, on: &str) -> Self {
        self.join(&format!("INNER JOIN {} ON {}", table, on))
    }

    pub fn right_join(self, table: &str, on: &str) -> Self {
        self.join(&format!("RIGHT JOIN {} ON {}", table, on))
    }

    // ── ORDER / LIMIT / OFFSET / GROUP BY ─────────────────────────────────────

    pub fn order_by(mut self, col: &str, order: Order) -> Self {
        self.order_by.push((col.to_string(), order));
        self
    }

    pub fn order_asc(self, col: &str) -> Self { self.order_by(col, Order::Asc) }
    pub fn order_desc(self, col: &str) -> Self { self.order_by(col, Order::Desc) }

    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: u64) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn group_by(mut self, col: &str) -> Self {
        self.group_by.push(col.to_string());
        self
    }

    // ── SCOPE ─────────────────────────────────────────────────────────────────

    /// 应用可复用的查询作用域（类似 GORM Scopes）
    ///
    /// ```ignore
    /// fn active(qb: QueryBuilder) -> QueryBuilder {
    ///     qb.where_eq("status", "active")
    /// }
    /// db.table("users").scope(active).get::<User>().await
    /// ```
    pub fn scope<F: FnOnce(Self) -> Self>(self, f: F) -> Self {
        f(self)
    }

    // ── 查询终结方法 ──────────────────────────────────────────────────────────

    /// 返回多行结果并映射为模型列表
    pub async fn get<M: Model>(self) -> Result<Vec<M>> {
        let conn = self.conn.clone();
        let (sql, params) = self.build_select_sql();
        let rows = conn.query(&sql, &params).await?;
        rows.iter().map(|r| M::from_row(r)).collect()
    }

    /// 返回原始 Row 列表
    pub async fn get_raw(self) -> Result<Vec<Row>> {
        let conn = self.conn.clone();
        let (sql, params) = self.build_select_sql();
        conn.query(&sql, &params).await
    }

    /// 返回第一条记录
    pub async fn first<M: Model>(self) -> Result<Option<M>> {
        let this = self.limit(1);
        let conn = this.conn.clone();
        let (sql, params) = this.build_select_sql();
        let row = conn.query_one(&sql, &params).await?;
        row.map(|r| M::from_row(&r)).transpose()
    }

    /// 返回第一条记录，不存在时返回 NotFound 错误
    pub async fn first_or_fail<M: Model>(self) -> Result<M> {
        self.first::<M>().await?.ok_or_else(|| FastError::NotFound("Record not found".into()))
    }

    /// 统计行数 COUNT(*)
    pub async fn count(self) -> Result<i64> {
        let conn = self.conn.clone();
        let table = self.table.clone();
        let joins = self.joins.clone();
        let mut params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut params);
        let join_clause = if joins.is_empty() { String::new() } else { format!(" {}", joins.join(" ")) };
        let sql = format!("SELECT COUNT(*) as _count FROM {}{}{}", table, join_clause, where_sql);
        let row = conn.query_one(&sql, &params).await?;
        row.map(|r| r.get::<i64>("_count")).unwrap_or(Ok(0))
    }

    /// 检查是否存在满足条件的记录
    pub async fn exists(self) -> Result<bool> {
        Ok(self.count().await? > 0)
    }

    /// 聚合：SUM，对列求和
    pub async fn sum(self, col: &str) -> Result<f64> {
        self.aggregate_f64(&format!("COALESCE(SUM({}), 0)", col), "_agg").await
    }

    /// 聚合：AVG，对列求平均
    pub async fn avg(self, col: &str) -> Result<Option<f64>> {
        let val = self.aggregate_value(&format!("AVG({})", col), "_agg").await?;
        Ok(val.and_then(|v| v.as_f64()))
    }

    /// 聚合：MAX，返回最大值
    pub async fn max_value(self, col: &str) -> Result<Option<Value>> {
        self.aggregate_value(&format!("MAX({})", col), "_agg").await
    }

    /// 聚合：MIN，返回最小值
    pub async fn min_value(self, col: &str) -> Result<Option<Value>> {
        self.aggregate_value(&format!("MIN({})", col), "_agg").await
    }

    /// 提取单列所有值（PLUCK）
    pub async fn pluck(self, col: &str) -> Result<Vec<Value>> {
        let this = self.select([col]);
        let rows = this.get_raw().await?;
        Ok(rows.into_iter().filter_map(|r| r.get_raw(col).cloned()).collect())
    }

    /// 分页查询：返回 (数据列表, 总数)
    pub async fn paginate<M: Model>(self, page: u64, size: u64) -> Result<(Vec<M>, i64)> {
        let count_qb = QueryBuilder {
            conn: self.conn.clone(),
            table: self.table.clone(),
            selects: vec!["COUNT(*) as _count".to_string()],
            where_clauses: self.where_clauses.clone(),
            joins: self.joins.clone(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            group_by: Vec::new(),
            having_clauses: Vec::new(),
        };
        let total = count_qb.count().await?;

        let data_qb = QueryBuilder {
            conn: self.conn,
            table: self.table,
            selects: self.selects,
            where_clauses: self.where_clauses,
            joins: self.joins,
            order_by: self.order_by,
            limit: Some(size),
            offset: Some((page - 1) * size),
            group_by: self.group_by,
            having_clauses: self.having_clauses,
        };
        let data = data_qb.get::<M>().await?;
        Ok((data, total))
    }

    // ── 写操作终结方法 ────────────────────────────────────────────────────────

    /// INSERT 单行，返回插入的行 ID
    pub async fn insert(self, values: Vec<(&str, Value)>) -> Result<i64> {
        let cols: Vec<&str> = values.iter().map(|(k, _)| *k).collect();
        let params: Vec<Param> = values.iter().map(|(_, v)| json_to_param(v)).collect();
        let ph = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!("INSERT INTO {} ({}) VALUES ({})", self.table, cols.join(", "), ph);
        self.conn.insert(&sql, &params).await
    }

    /// INSERT 多行（批量插入），返回影响行数
    pub async fn insert_many(self, rows: Vec<Vec<(&str, Value)>>) -> Result<u64> {
        if rows.is_empty() { return Ok(0); }
        let cols: Vec<&str> = rows[0].iter().map(|(k, _)| *k).collect();
        let row_ph = format!("({})", cols.iter().map(|_| "?").collect::<Vec<_>>().join(", "));
        let all_ph_parts: Vec<String> = rows.iter().map(|_| row_ph.clone()).collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES {}",
            self.table,
            cols.join(", "),
            all_ph_parts.join(", ")
        );
        let mut params = Vec::new();
        for row in &rows {
            for (_, v) in row { params.push(json_to_param(v)); }
        }
        self.conn.execute(&sql, &params).await
    }

    /// UPSERT（INSERT ... ON CONFLICT DO UPDATE SET ...）
    ///
    /// `conflict_cols` 指定冲突列（通常是主键或唯一键）。
    pub async fn upsert(self, values: Vec<(&str, Value)>, conflict_cols: &[&str]) -> Result<i64> {
        let cols: Vec<&str> = values.iter().map(|(k, _)| *k).collect();
        let params: Vec<Param> = values.iter().map(|(_, v)| json_to_param(v)).collect();
        let ph = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

        let update_set: Vec<String> = cols.iter()
            .filter(|c| !conflict_cols.contains(c))
            .map(|c| format!("{0} = excluded.{0}", c))
            .collect();

        let sql = if update_set.is_empty() {
            format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO NOTHING",
                self.table, cols.join(", "), ph, conflict_cols.join(", ")
            )
        } else {
            format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
                self.table, cols.join(", "), ph, conflict_cols.join(", "), update_set.join(", ")
            )
        };
        self.conn.insert(&sql, &params).await
    }

    /// UPDATE，返回影响行数
    pub async fn update(self, values: Vec<(&str, Value)>) -> Result<u64> {
        if values.is_empty() { return Ok(0); }
        let set_clauses: Vec<String> = values.iter().map(|(col, _)| format!("{} = ?", col)).collect();
        let mut params: Vec<Param> = values.iter().map(|(_, v)| json_to_param(v)).collect();
        let mut where_params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut where_params);
        params.extend(where_params);
        let sql = format!("UPDATE {} SET {}{}", self.table, set_clauses.join(", "), where_sql);
        self.conn.execute(&sql, &params).await
    }

    /// 对指定列做原子递增，返回影响行数
    pub async fn increment(self, col: &str, amount: i64) -> Result<u64> {
        let mut where_params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut where_params);
        let mut params = vec![Param::Int(amount)];
        params.extend(where_params);
        let sql = format!("UPDATE {} SET {col} = {col} + ?{}", self.table, where_sql);
        self.conn.execute(&sql, &params).await
    }

    /// 对指定列做原子递减，返回影响行数
    pub async fn decrement(self, col: &str, amount: i64) -> Result<u64> {
        let mut where_params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut where_params);
        let mut params = vec![Param::Int(amount)];
        params.extend(where_params);
        let sql = format!("UPDATE {} SET {col} = {col} - ?{}", self.table, where_sql);
        self.conn.execute(&sql, &params).await
    }

    /// DELETE，返回影响行数
    pub async fn delete(self) -> Result<u64> {
        let mut params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut params);
        let sql = format!("DELETE FROM {}{}", self.table, where_sql);
        self.conn.execute(&sql, &params).await
    }

    // ── 内部 SQL 构造 ──────────────────────────────────────────────────────────

    fn build_select_sql(&self) -> (String, Vec<Param>) {
        let select_clause = self.selects.join(", ");
        let join_clause = if self.joins.is_empty() {
            String::new()
        } else {
            format!(" {}", self.joins.join(" "))
        };

        let mut params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut params);

        let group_clause = if self.group_by.is_empty() {
            String::new()
        } else {
            format!(" GROUP BY {}", self.group_by.join(", "))
        };

        let having_sql = build_having_sql(&self.having_clauses, &mut params);

        let order_clause = if self.order_by.is_empty() {
            String::new()
        } else {
            let parts: Vec<String> = self.order_by.iter()
                .map(|(col, ord)| format!("{} {}", col, ord.as_str()))
                .collect();
            format!(" ORDER BY {}", parts.join(", "))
        };

        let limit_clause = self.limit.map(|n| format!(" LIMIT {}", n)).unwrap_or_default();
        let offset_clause = self.offset.map(|n| format!(" OFFSET {}", n)).unwrap_or_default();

        let sql = format!(
            "SELECT {} FROM {}{}{}{}{}{}{}{}",
            select_clause, self.table, join_clause, where_sql,
            group_clause, having_sql, order_clause, limit_clause, offset_clause,
        );

        (sql, params)
    }

    async fn aggregate_f64(&self, expr: &str, alias: &str) -> Result<f64> {
        let mut params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut params);
        let join_clause = if self.joins.is_empty() {
            String::new()
        } else {
            format!(" {}", self.joins.join(" "))
        };
        let sql = format!("SELECT {} as {} FROM {}{}{}", expr, alias, self.table, join_clause, where_sql);
        let row = self.conn.query_one(&sql, &params).await?;
        row.map(|r| r.get::<f64>(alias)).unwrap_or(Ok(0.0))
    }

    async fn aggregate_value(&self, expr: &str, alias: &str) -> Result<Option<Value>> {
        let mut params = Vec::new();
        let where_sql = build_where_sql(&self.where_clauses, &mut params);
        let join_clause = if self.joins.is_empty() {
            String::new()
        } else {
            format!(" {}", self.joins.join(" "))
        };
        let sql = format!("SELECT {} as {} FROM {}{}{}", expr, alias, self.table, join_clause, where_sql);
        let row = self.conn.query_one(&sql, &params).await?;
        Ok(row.and_then(|r| r.get_raw(alias).cloned()))
    }
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 将 serde_json::Value 转换为 Param
pub fn json_to_param(val: &Value) -> Param {
    match val {
        Value::Null => Param::Null,
        Value::Bool(b) => Param::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() { Param::Int(i) }
            else if let Some(f) = n.as_f64() { Param::Float(f) }
            else { Param::Text(n.to_string()) }
        }
        Value::String(s) => Param::Text(s.clone()),
        other => Param::Text(other.to_string()),
    }
}
