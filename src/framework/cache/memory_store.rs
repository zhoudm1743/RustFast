use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// 内存缓存条目
struct CacheEntry {
    value: String,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.expires_at.map(|t| Instant::now() > t).unwrap_or(false)
    }
}

/// 内存缓存实现
pub struct MemoryStore {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 设置缓存（可选 TTL）
    pub fn set(&self, key: &str, value: &str, ttl: Option<Duration>) {
        let entry = CacheEntry {
            value: value.to_string(),
            expires_at: ttl.map(|d| Instant::now() + d),
        };
        let mut data = self.data.write().unwrap();
        data.insert(key.to_string(), entry);
    }

    /// 获取缓存值
    pub fn get(&self, key: &str) -> Option<String> {
        let data = self.data.read().unwrap();
        if let Some(entry) = data.get(key) {
            if entry.is_expired() {
                return None;
            }
            return Some(entry.value.clone());
        }
        None
    }

    /// 检查键是否存在（且未过期）
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// 删除缓存
    pub fn forget(&self, key: &str) -> bool {
        let mut data = self.data.write().unwrap();
        data.remove(key).is_some()
    }

    /// 清空所有缓存
    pub fn flush(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();
    }

    /// 原子递增
    pub fn increment(&self, key: &str, step: i64) -> i64 {
        let mut data = self.data.write().unwrap();
        let entry = data.entry(key.to_string()).or_insert_with(|| CacheEntry {
            value: "0".to_string(),
            expires_at: None,
        });
        let current: i64 = entry.value.parse().unwrap_or(0);
        let new_val = current + step;
        entry.value = new_val.to_string();
        new_val
    }

    /// 原子递减
    pub fn decrement(&self, key: &str, step: i64) -> i64 {
        self.increment(key, -step)
    }

    /// 清理过期条目
    pub fn prune(&self) {
        let mut data = self.data.write().unwrap();
        data.retain(|_, entry| !entry.is_expired());
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
