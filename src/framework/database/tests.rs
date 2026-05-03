//! ORM 单元测试
//!
//! 使用 SQLite 内存数据库（`:memory:`）运行所有测试，无需外部依赖。
//! 执行方式：`cargo test --features sqlite -- database`

#![cfg(test)]
#![cfg(feature = "sqlite")]

use serde_json::json;

use crate::framework::database::manager::DbManager;
use crate::framework::database::model::{Model, Param, Row};

// ── 测试用模型 ─────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
struct User {
    id: i64,
    name: String,
    email: String,
    age: i64,
    score: f64,
    status: String,
    bio: Option<String>,
}

impl Model for User {
    fn table_name() -> &'static str { "users" }
    fn primary_key() -> &'static str { "id" }

    fn from_row(row: &Row) -> crate::framework::error::Result<Self> {
        Ok(User {
            id: row.get::<i64>("id").unwrap_or(0),
            name: row.get::<String>("name").unwrap_or_default(),
            email: row.get::<String>("email").unwrap_or_default(),
            age: row.get::<i64>("age").unwrap_or(0),
            score: row.get::<f64>("score").unwrap_or(0.0),
            status: row.get::<String>("status").unwrap_or_default(),
            bio: row.get::<Option<String>>("bio").unwrap_or(None),
        })
    }

    fn to_values(&self) -> Vec<(&'static str, serde_json::Value)> {
        vec![
            ("id", json!(self.id)),
            ("name", json!(self.name)),
            ("email", json!(self.email)),
            ("age", json!(self.age)),
            ("score", json!(self.score)),
            ("status", json!(self.status)),
            ("bio", json!(self.bio)),
        ]
    }
}

#[derive(Debug)]
struct Order {
    id: i64,
    user_id: i64,
    product: String,
    amount: f64,
    qty: i64,
}

impl Model for Order {
    fn table_name() -> &'static str { "orders" }

    fn from_row(row: &Row) -> crate::framework::error::Result<Self> {
        Ok(Order {
            id: row.get::<i64>("id").unwrap_or(0),
            user_id: row.get::<i64>("user_id").unwrap_or(0),
            product: row.get::<String>("product").unwrap_or_default(),
            amount: row.get::<f64>("amount").unwrap_or(0.0),
            qty: row.get::<i64>("qty").unwrap_or(0),
        })
    }

    fn to_values(&self) -> Vec<(&'static str, serde_json::Value)> {
        vec![
            ("id", json!(self.id)),
            ("user_id", json!(self.user_id)),
            ("product", json!(self.product)),
            ("amount", json!(self.amount)),
            ("qty", json!(self.qty)),
        ]
    }
}

// ── 测试辅助：创建数据库并填充测试数据 ───────────────────────────────────────

async fn setup_db() -> DbManager {
    let db = DbManager::sqlite(":memory:").unwrap();

    db.raw_execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL,
            age INTEGER NOT NULL DEFAULT 0,
            score REAL NOT NULL DEFAULT 0.0,
            status TEXT NOT NULL DEFAULT 'active',
            bio TEXT
        )",
        vec![],
    ).await.unwrap();

    db.raw_execute(
        "CREATE TABLE orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            product TEXT NOT NULL,
            amount REAL NOT NULL,
            qty INTEGER NOT NULL DEFAULT 1
        )",
        vec![],
    ).await.unwrap();

    db
}

async fn seed_users(db: &DbManager) {
    let users: Vec<Vec<(&str, serde_json::Value)>> = vec![
        vec![("name", json!("Alice")), ("email", json!("alice@test.com")), ("age", json!(25)), ("score", json!(90.5)), ("status", json!("active")), ("bio", json!(null))],
        vec![("name", json!("Bob")), ("email", json!("bob@test.com")), ("age", json!(30)), ("score", json!(75.0)), ("status", json!("active")), ("bio", json!("Developer"))],
        vec![("name", json!("Carol")), ("email", json!("carol@test.com")), ("age", json!(22)), ("score", json!(88.0)), ("status", json!("inactive")), ("bio", json!(null))],
        vec![("name", json!("Dave")), ("email", json!("dave@test.com")), ("age", json!(35)), ("score", json!(60.0)), ("status", json!("active")), ("bio", json!("Manager"))],
        vec![("name", json!("Eve")), ("email", json!("eve@test.com")), ("age", json!(28)), ("score", json!(95.0)), ("status", json!("inactive")), ("bio", json!("Designer"))],
    ];

    for u in users {
        db.table("users").insert(u).await.unwrap();
    }
}

async fn seed_orders(db: &DbManager) {
    let orders: Vec<Vec<(&str, serde_json::Value)>> = vec![
        vec![("user_id", json!(1)), ("product", json!("Apple")), ("amount", json!(10.0)), ("qty", json!(2))],
        vec![("user_id", json!(1)), ("product", json!("Banana")), ("amount", json!(5.0)), ("qty", json!(5))],
        vec![("user_id", json!(2)), ("product", json!("Cherry")), ("amount", json!(20.0)), ("qty", json!(1))],
        vec![("user_id", json!(2)), ("product", json!("Apple")), ("amount", json!(10.0)), ("qty", json!(3))],
        vec![("user_id", json!(3)), ("product", json!("Durian")), ("amount", json!(50.0)), ("qty", json!(1))],
    ];

    for o in orders {
        db.table("orders").insert(o).await.unwrap();
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// 基础 CRUD 测试
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_insert_and_select_all() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").get::<User>().await.unwrap();
    assert_eq!(users.len(), 5);
}

#[tokio::test]
async fn test_insert_returns_id() {
    let db = setup_db().await;
    let id = db.table("users")
        .insert(vec![
            ("name", json!("Test")),
            ("email", json!("t@t.com")),
            ("age", json!(20)),
            ("score", json!(0.0)),
            ("status", json!("active")),
        ])
        .await
        .unwrap();
    assert_eq!(id, 1);
}

#[tokio::test]
async fn test_update_by_condition() {
    let db = setup_db().await;
    seed_users(&db).await;

    let affected = db.table("users")
        .where_eq("name", "Alice")
        .update(vec![("age", json!(26))])
        .await
        .unwrap();
    assert_eq!(affected, 1);

    let alice: User = db.table("users").where_eq("name", "Alice").first_or_fail::<User>().await.unwrap();
    assert_eq!(alice.age, 26);
}

#[tokio::test]
async fn test_delete_by_condition() {
    let db = setup_db().await;
    seed_users(&db).await;

    let affected = db.table("users").where_eq("name", "Carol").delete().await.unwrap();
    assert_eq!(affected, 1);

    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 4);
}

#[tokio::test]
async fn test_first_returns_one() {
    let db = setup_db().await;
    seed_users(&db).await;

    let user = db.table("users").order_asc("id").first::<User>().await.unwrap();
    assert!(user.is_some());
    assert_eq!(user.unwrap().name, "Alice");
}

#[tokio::test]
async fn test_first_or_fail_not_found() {
    let db = setup_db().await;
    seed_users(&db).await;

    let result = db.table("users").where_eq("name", "Nonexistent").first_or_fail::<User>().await;
    assert!(result.is_err());
}

// ═════════════════════════════════════════════════════════════════════════════
// WHERE 条件测试
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_where_eq() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_eq("status", "inactive").get::<User>().await.unwrap();
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_where_ne() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_ne("status", "active").get::<User>().await.unwrap();
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_where_gt_lt() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_gt("age", 25i64).get::<User>().await.unwrap();
    assert_eq!(users.len(), 3);

    let users2: Vec<User> = db.table("users").where_lt("age", 25i64).get::<User>().await.unwrap();
    assert_eq!(users2.len(), 1);
}

#[tokio::test]
async fn test_where_gte_lte() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_gte("age", 28i64).where_lte("age", 35i64).get::<User>().await.unwrap();
    assert_eq!(users.len(), 3);
}

#[tokio::test]
async fn test_where_like_and_not_like() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_like("email", "%test.com").get::<User>().await.unwrap();
    assert_eq!(users.len(), 5);

    let users2: Vec<User> = db.table("users").where_not_like("name", "A%").get::<User>().await.unwrap();
    assert_eq!(users2.len(), 4);
}

#[tokio::test]
async fn test_where_in_and_not_in() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_in("name", ["Alice", "Bob"]).get::<User>().await.unwrap();
    assert_eq!(users.len(), 2);

    let users2: Vec<User> = db.table("users").where_not_in("name", ["Alice", "Bob"]).get::<User>().await.unwrap();
    assert_eq!(users2.len(), 3);
}

#[tokio::test]
async fn test_where_null_and_not_null() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_null("bio").get::<User>().await.unwrap();
    assert_eq!(users.len(), 2); // Alice, Carol

    let users2: Vec<User> = db.table("users").where_not_null("bio").get::<User>().await.unwrap();
    assert_eq!(users2.len(), 3); // Bob, Dave, Eve
}

#[tokio::test]
async fn test_where_between_and_not_between() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").where_between("age", 25i64, 30i64).get::<User>().await.unwrap();
    assert_eq!(users.len(), 3); // Alice(25), Bob(30), Eve(28)

    let users2: Vec<User> = db.table("users").where_not_between("age", 25i64, 30i64).get::<User>().await.unwrap();
    assert_eq!(users2.len(), 2); // Carol(22), Dave(35)
}

#[tokio::test]
async fn test_where_raw() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users")
        .where_raw("age > ? AND score > ?", vec![Param::Int(20), Param::Float(80.0)])
        .get::<User>()
        .await
        .unwrap();
    assert_eq!(users.len(), 3); // Alice(90.5), Carol(88.0), Eve(95.0)
}

// ═════════════════════════════════════════════════════════════════════════════
// OR 条件 & 嵌套分组测试
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_or_where() {
    let db = setup_db().await;
    seed_users(&db).await;

    // WHERE name = 'Alice' OR name = 'Bob'
    let users: Vec<User> = db.table("users")
        .where_eq("name", "Alice")
        .or_where_eq("name", "Bob")
        .get::<User>()
        .await
        .unwrap();
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_where_group_and() {
    let db = setup_db().await;
    seed_users(&db).await;

    // WHERE status = 'active' AND (age >= 28 OR score >= 90)
    let users: Vec<User> = db.table("users")
        .where_eq("status", "active")
        .where_group(|g| {
            g.where_gte("age", 28i64)
             .or_where_gte("score", Param::Float(90.0))
        })
        .get::<User>()
        .await
        .unwrap();
    // Alice(active, age=25, score=90.5) ✓ (score>=90), Bob(active,age=30) ✓ (age>=28), Dave(active,age=35) ✓
    assert_eq!(users.len(), 3);
}

#[tokio::test]
async fn test_or_where_group() {
    let db = setup_db().await;
    seed_users(&db).await;

    // WHERE age < 23 OR (status = 'inactive' AND score > 85)
    let users: Vec<User> = db.table("users")
        .where_lt("age", 23i64)
        .or_where_group(|g| {
            g.where_eq("status", "inactive")
             .where_gt("score", Param::Float(85.0))
        })
        .get::<User>()
        .await
        .unwrap();
    // Carol(age=22) ✓, Eve(inactive, score=95) ✓
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_nested_groups() {
    let db = setup_db().await;
    seed_users(&db).await;

    // WHERE (status='active' AND age >= 30) OR (status='inactive' AND score >= 88)
    let users: Vec<User> = db.table("users")
        .where_group(|g| {
            g.where_eq("status", "active").where_gte("age", 30i64)
        })
        .or_where_group(|g| {
            g.where_eq("status", "inactive").where_gte("score", Param::Float(88.0))
        })
        .get::<User>()
        .await
        .unwrap();
    // Bob(active,30) ✓, Dave(active,35) ✓, Carol(inactive,88) ✓, Eve(inactive,95) ✓
    assert_eq!(users.len(), 4);
}

// ═════════════════════════════════════════════════════════════════════════════
// ORDER BY / LIMIT / OFFSET
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_order_by_asc() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").order_asc("age").get::<User>().await.unwrap();
    let ages: Vec<i64> = users.iter().map(|u| u.age).collect();
    assert_eq!(ages, vec![22, 25, 28, 30, 35]);
}

#[tokio::test]
async fn test_order_by_desc() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").order_desc("score").get::<User>().await.unwrap();
    let first = &users[0];
    assert_eq!(first.name, "Eve");
}

#[tokio::test]
async fn test_limit_and_offset() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users").order_asc("id").limit(2).offset(1).get::<User>().await.unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Bob");
    assert_eq!(users[1].name, "Carol");
}

// ═════════════════════════════════════════════════════════════════════════════
// SELECT 列过滤
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_select_columns() {
    let db = setup_db().await;
    seed_users(&db).await;

    let rows = db.table("users")
        .select(["name", "age"])
        .order_asc("id")
        .get_raw()
        .await
        .unwrap();
    assert_eq!(rows.len(), 5);
    // name 和 age 存在，email 不存在
    assert!(rows[0].get_raw("name").is_some());
    assert!(rows[0].get_raw("age").is_some());
    assert!(rows[0].get_raw("email").is_none());
}

// ═════════════════════════════════════════════════════════════════════════════
// JOIN 查询
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_inner_join() {
    let db = setup_db().await;
    seed_users(&db).await;
    seed_orders(&db).await;

    let rows = db.table("users")
        .select(["users.name", "orders.product", "orders.amount"])
        .inner_join("orders", "users.id = orders.user_id")
        .where_eq("users.name", "Alice")
        .order_asc("orders.id")
        .get_raw()
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String>("product").unwrap(), "Apple");
    assert_eq!(rows[1].get::<String>("product").unwrap(), "Banana");
}

#[tokio::test]
async fn test_left_join() {
    let db = setup_db().await;
    seed_users(&db).await;
    seed_orders(&db).await;

    // Dave 和 Eve 没有订单，LEFT JOIN 应该也能查到他们
    let rows = db.table("users")
        .select(["users.name", "orders.product"])
        .left_join("orders", "users.id = orders.user_id")
        .order_asc("users.id")
        .get_raw()
        .await
        .unwrap();

    // Alice(2) + Bob(2) + Carol(1) + Dave(1, NULL product) + Eve(1, NULL product)
    assert_eq!(rows.len(), 7);
}

// ═════════════════════════════════════════════════════════════════════════════
// GROUP BY + HAVING
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_group_by_count() {
    let db = setup_db().await;
    seed_users(&db).await;

    let rows = db.table("users")
        .select(["status", "COUNT(*) as cnt"])
        .group_by("status")
        .order_asc("status")
        .get_raw()
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    let active = rows.iter().find(|r| r.get::<String>("status").unwrap() == "active").unwrap();
    assert_eq!(active.get::<i64>("cnt").unwrap(), 3);
}

#[tokio::test]
async fn test_having_gte() {
    let db = setup_db().await;
    seed_orders(&db).await;

    // 按 user_id 分组，只保留订单总金额 >= 20 的
    let rows = db.table("orders")
        .select(["user_id", "SUM(amount) as total"])
        .group_by("user_id")
        .having_gte("total", Param::Float(20.0))
        .order_asc("user_id")
        .get_raw()
        .await
        .unwrap();

    // user_id=1: 10+5=15 (排除), user_id=2: 20+10=30 (保留), user_id=3: 50 (保留)
    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn test_having_raw() {
    let db = setup_db().await;
    seed_orders(&db).await;

    let rows = db.table("orders")
        .select(["user_id", "COUNT(*) as order_count"])
        .group_by("user_id")
        .having_raw("order_count > ?", vec![Param::Int(1)])
        .get_raw()
        .await
        .unwrap();

    // user_id=1 有 2 单，user_id=2 有 2 单
    assert_eq!(rows.len(), 2);
}

// ═════════════════════════════════════════════════════════════════════════════
// 聚合函数
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_count() {
    let db = setup_db().await;
    seed_users(&db).await;

    let n = db.table("users").count().await.unwrap();
    assert_eq!(n, 5);

    let n2 = db.table("users").where_eq("status", "active").count().await.unwrap();
    assert_eq!(n2, 3);
}

#[tokio::test]
async fn test_sum() {
    let db = setup_db().await;
    seed_orders(&db).await;

    let total = db.table("orders").sum("amount").await.unwrap();
    // 10 + 5 + 20 + 10 + 50 = 95
    assert!((total - 95.0).abs() < 0.001);
}

#[tokio::test]
async fn test_avg() {
    let db = setup_db().await;
    seed_users(&db).await;

    let avg = db.table("users").avg("age").await.unwrap();
    // (25+30+22+35+28)/5 = 28.0
    assert!(avg.is_some());
    assert!((avg.unwrap() - 28.0).abs() < 0.001);
}

#[tokio::test]
async fn test_max_min() {
    let db = setup_db().await;
    seed_users(&db).await;

    let max = db.table("users").max_value("age").await.unwrap();
    assert_eq!(max.unwrap().as_i64().unwrap(), 35);

    let min = db.table("users").min_value("age").await.unwrap();
    assert_eq!(min.unwrap().as_i64().unwrap(), 22);
}

#[tokio::test]
async fn test_exists() {
    let db = setup_db().await;
    seed_users(&db).await;

    assert!(db.table("users").where_eq("name", "Alice").exists().await.unwrap());
    assert!(!db.table("users").where_eq("name", "Nobody").exists().await.unwrap());
}

// ═════════════════════════════════════════════════════════════════════════════
// PLUCK
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_pluck() {
    let db = setup_db().await;
    seed_users(&db).await;

    let names = db.table("users").order_asc("id").pluck("name").await.unwrap();
    let name_strs: Vec<&str> = names.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(name_strs, vec!["Alice", "Bob", "Carol", "Dave", "Eve"]);
}

// ═════════════════════════════════════════════════════════════════════════════
// 分页
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_paginate() {
    let db = setup_db().await;
    seed_users(&db).await;

    let (users, total) = db.table("users").order_asc("id").paginate::<User>(1, 2).await.unwrap();
    assert_eq!(total, 5);
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");

    let (users2, _) = db.table("users").order_asc("id").paginate::<User>(3, 2).await.unwrap();
    assert_eq!(users2.len(), 1); // 最后一页只有1条
    assert_eq!(users2[0].name, "Eve");
}

// ═════════════════════════════════════════════════════════════════════════════
// 批量插入
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_insert_many() {
    let db = setup_db().await;

    let rows = vec![
        vec![("name", json!("U1")), ("email", json!("u1@t.com")), ("age", json!(20)), ("score", json!(50.0)), ("status", json!("active"))],
        vec![("name", json!("U2")), ("email", json!("u2@t.com")), ("age", json!(21)), ("score", json!(60.0)), ("status", json!("active"))],
        vec![("name", json!("U3")), ("email", json!("u3@t.com")), ("age", json!(22)), ("score", json!(70.0)), ("status", json!("inactive"))],
    ];

    let affected = db.table("users").insert_many(rows).await.unwrap();
    assert_eq!(affected, 3);

    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 3);
}

// ═════════════════════════════════════════════════════════════════════════════
// UPSERT
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_upsert_insert() {
    let db = setup_db().await;
    seed_users(&db).await;

    // 插入一个 email 不存在的行（视为普通 INSERT）
    db.table("users")
        .upsert(
            vec![
                ("name", json!("Frank")),
                ("email", json!("frank@test.com")),
                ("age", json!(40)),
                ("score", json!(55.0)),
                ("status", json!("active")),
            ],
            &["email"],
        )
        .await
        .unwrap();

    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 6);
}

#[tokio::test]
async fn test_upsert_update() {
    let db = setup_db().await;
    seed_users(&db).await;

    // Alice 的 email 已存在，更新 age 和 score
    db.table("users")
        .upsert(
            vec![
                ("name", json!("Alice")),
                ("email", json!("alice@test.com")),
                ("age", json!(99)),
                ("score", json!(100.0)),
                ("status", json!("active")),
            ],
            &["email"],
        )
        .await
        .unwrap();

    let alice: User = db.table("users").where_eq("email", "alice@test.com").first_or_fail::<User>().await.unwrap();
    assert_eq!(alice.age, 99);
    assert!((alice.score - 100.0).abs() < 0.001);
    // 总数不变
    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 5);
}

// ═════════════════════════════════════════════════════════════════════════════
// INCREMENT / DECREMENT
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_increment_and_decrement() {
    let db = setup_db().await;
    seed_users(&db).await;

    db.table("users").where_eq("name", "Alice").increment("age", 5).await.unwrap();
    let alice: User = db.table("users").where_eq("name", "Alice").first_or_fail::<User>().await.unwrap();
    assert_eq!(alice.age, 30);

    db.table("users").where_eq("name", "Alice").decrement("age", 3).await.unwrap();
    let alice2: User = db.table("users").where_eq("name", "Alice").first_or_fail::<User>().await.unwrap();
    assert_eq!(alice2.age, 27);
}

// ═════════════════════════════════════════════════════════════════════════════
// 事务：提交
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_transaction_commit() {
    let db = setup_db().await;

    db.begin().await.unwrap();
    db.table("users").insert(vec![
        ("name", json!("TxUser")), ("email", json!("tx@t.com")),
        ("age", json!(33)), ("score", json!(77.0)), ("status", json!("active")),
    ]).await.unwrap();
    db.commit().await.unwrap();

    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 1);
}

// ═════════════════════════════════════════════════════════════════════════════
// 事务：回滚
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_transaction_rollback() {
    let db = setup_db().await;

    db.begin().await.unwrap();
    db.table("users").insert(vec![
        ("name", json!("RollbackUser")), ("email", json!("rb@t.com")),
        ("age", json!(18)), ("score", json!(40.0)), ("status", json!("active")),
    ]).await.unwrap();
    db.rollback().await.unwrap();

    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 0); // 回滚后应为 0
}

// ═════════════════════════════════════════════════════════════════════════════
// 事务：transaction() 辅助方法
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_transaction_helper_success() {
    let db = setup_db().await;

    db.transaction(|db| async move {
        db.table("users").insert(vec![
            ("name", json!("TxHelper")), ("email", json!("txh@t.com")),
            ("age", json!(44)), ("score", json!(88.0)), ("status", json!("active")),
        ]).await?;
        Ok(())
    }).await.unwrap();

    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_transaction_helper_rollback_on_error() {
    let db = setup_db().await;

    let result = db.transaction(|db| async move {
        db.table("users").insert(vec![
            ("name", json!("WillRollback")), ("email", json!("wr@t.com")),
            ("age", json!(55)), ("score", json!(50.0)), ("status", json!("active")),
        ]).await?;
        // 故意触发错误：插入重复 email
        db.table("users").insert(vec![
            ("name", json!("Dup")), ("email", json!("wr@t.com")),
            ("age", json!(56)), ("score", json!(51.0)), ("status", json!("active")),
        ]).await?;
        Ok(())
    }).await;

    assert!(result.is_err());
    let count = db.table("users").count().await.unwrap();
    assert_eq!(count, 0); // 事务已回滚
}

// ═════════════════════════════════════════════════════════════════════════════
// SCOPE
// ═════════════════════════════════════════════════════════════════════════════

fn active_scope(qb: crate::framework::database::query_builder::QueryBuilder) -> crate::framework::database::query_builder::QueryBuilder {
    qb.where_eq("status", "active")
}

fn high_score_scope(qb: crate::framework::database::query_builder::QueryBuilder) -> crate::framework::database::query_builder::QueryBuilder {
    qb.where_gte("score", Param::Float(80.0))
}

#[tokio::test]
async fn test_scope() {
    let db = setup_db().await;
    seed_users(&db).await;

    let users: Vec<User> = db.table("users")
        .scope(active_scope)
        .scope(high_score_scope)
        .get::<User>()
        .await
        .unwrap();

    // active: Alice(90.5), Bob(75.0), Dave(60.0)
    // high_score: Alice(90.5), Carol(88.0), Eve(95.0)
    // active AND high_score: Alice(90.5)
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Alice");
}

// ═════════════════════════════════════════════════════════════════════════════
// 原始 SQL
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_raw_query() {
    let db = setup_db().await;
    seed_users(&db).await;

    let rows = db.raw_query(
        "SELECT name, age FROM users WHERE age > ? ORDER BY age ASC",
        vec![Param::Int(25)],
    ).await.unwrap();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].get::<String>("name").unwrap(), "Eve");
}

#[tokio::test]
async fn test_raw_execute() {
    let db = setup_db().await;
    seed_users(&db).await;

    let affected = db.raw_execute(
        "UPDATE users SET score = score + 5.0 WHERE status = ?",
        vec![Param::Text("active".into())],
    ).await.unwrap();
    assert_eq!(affected, 3);
}

// ═════════════════════════════════════════════════════════════════════════════
// 多条件组合压测
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_complex_combined_query() {
    let db = setup_db().await;
    seed_users(&db).await;
    seed_orders(&db).await;

    // 查询下了订单、年龄在20-30之间、状态为active的用户姓名
    let rows = db.table("users")
        .select(["users.name", "users.age", "SUM(orders.amount) as total"])
        .inner_join("orders", "users.id = orders.user_id")
        .where_eq("users.status", "active")
        .where_between("users.age", 20i64, 30i64)
        .group_by("users.id")
        .having_gt("total", Param::Float(0.0))
        .order_desc("total")
        .get_raw()
        .await
        .unwrap();

    // Alice(active,25, total=15), Bob(active,30, total=30)
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String>("name").unwrap(), "Bob"); // total=30
}
