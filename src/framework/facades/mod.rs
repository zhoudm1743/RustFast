//! Facades — 静态门面，提供对框架服务的零样板代码访问
//!
//! 用法示例：
//! ```ignore
//! use rustfast::framework::facades::{Config, Db, Cache};
//!
//! let name: String = Config::get("app.name").unwrap_or_default();
//! let users = Db::table("users").where_eq("active", 1).get::<User>().await?;
//! Cache::set("key", "value", None);
//! ```

use std::sync::Arc;
use crate::framework::foundation::application::app;

// ── Config Facade ─────────────────────────────────────────────────────────────

pub struct Config;

impl Config {
    fn service() -> Arc<crate::framework::config::Config> {
        app()
            .make::<crate::framework::config::Config>()
            .expect("Config service not registered. Did you add ConfigServiceProvider?")
    }

    pub fn get<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
        Self::service().get::<T>(key)
    }

    pub fn get_or<T: serde::de::DeserializeOwned>(key: &str, default: T) -> T {
        Self::service().get_or(key, default)
    }

    pub fn has(key: &str) -> bool {
        Self::service().has(key)
    }
}

// ── Db Facade ─────────────────────────────────────────────────────────────────

pub struct Db;

impl Db {
    fn service() -> Arc<crate::framework::database::DbManager> {
        app()
            .make::<crate::framework::database::DbManager>()
            .expect("DbManager not registered. Did you add DatabaseServiceProvider?")
    }

    /// 开始针对指定表的查询
    pub fn table(table: &str) -> crate::framework::database::QueryBuilder {
        Self::service().table(table)
    }
}

// ── Cache Facade ──────────────────────────────────────────────────────────────

pub struct Cache;

impl Cache {
    fn service() -> Arc<crate::framework::cache::MemoryStore> {
        app()
            .make::<crate::framework::cache::MemoryStore>()
            .expect("Cache not registered. Did you add CacheServiceProvider?")
    }

    pub fn get(key: &str) -> Option<String> {
        Self::service().get(key)
    }

    pub fn set(key: &str, value: &str, ttl: Option<std::time::Duration>) {
        Self::service().set(key, value, ttl);
    }

    pub fn forget(key: &str) -> bool {
        Self::service().forget(key)
    }

    pub fn has(key: &str) -> bool {
        Self::service().has(key)
    }

    pub fn increment(key: &str, step: i64) -> i64 {
        Self::service().increment(key, step)
    }

    pub fn decrement(key: &str, step: i64) -> i64 {
        Self::service().decrement(key, step)
    }
}

// ── Route Facade ──────────────────────────────────────────────────────────────

pub struct Route;

impl Route {
    fn router() -> &'static std::sync::Arc<std::sync::RwLock<crate::framework::http::Router>> {
        crate::framework::http::global_router()
    }

    pub fn get<F, Fut>(path: &'static str, handler: F)
    where
        F: Fn(crate::framework::http::Request) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::framework::error::Result<crate::framework::http::Response>>
            + Send
            + 'static,
    {
        Self::router().write().unwrap().get(path, handler);
    }

    pub fn post<F, Fut>(path: &'static str, handler: F)
    where
        F: Fn(crate::framework::http::Request) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::framework::error::Result<crate::framework::http::Response>>
            + Send
            + 'static,
    {
        Self::router().write().unwrap().post(path, handler);
    }

    pub fn put<F, Fut>(path: &'static str, handler: F)
    where
        F: Fn(crate::framework::http::Request) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::framework::error::Result<crate::framework::http::Response>>
            + Send
            + 'static,
    {
        Self::router().write().unwrap().put(path, handler);
    }

    pub fn patch<F, Fut>(path: &'static str, handler: F)
    where
        F: Fn(crate::framework::http::Request) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::framework::error::Result<crate::framework::http::Response>>
            + Send
            + 'static,
    {
        Self::router().write().unwrap().patch(path, handler);
    }

    pub fn delete<F, Fut>(path: &'static str, handler: F)
    where
        F: Fn(crate::framework::http::Request) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = crate::framework::error::Result<crate::framework::http::Response>>
            + Send
            + 'static,
    {
        Self::router().write().unwrap().delete(path, handler);
    }
}
