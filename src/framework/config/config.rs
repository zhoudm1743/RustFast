use std::collections::HashMap;
use std::env;
use std::fs;
use serde::de::DeserializeOwned;
use serde_json::Value;
use crate::framework::error::FastError;

/// 配置服务
///
/// 支持：
/// - 读取 YAML 配置文件
/// - 环境变量覆盖（APP_NAME → app.name）
/// - 点号路径访问：`config.get::<String>("database.host")`
pub struct Config {
    data: Value,
}

impl Config {
    /// 从 YAML 文件加载配置
    pub fn load(path: &str) -> Result<Self, FastError> {
        let content = fs::read_to_string(path)
            .map_err(|e| FastError::Config(format!("Failed to read config file '{}': {}", path, e)))?;

        let mut data: Value = serde_yaml::from_str(&content)
            .map_err(|e| FastError::Config(format!("Failed to parse YAML: {}", e)))?;

        // 应用环境变量覆盖
        Self::apply_env_overrides(&mut data);

        Ok(Self { data })
    }

    /// 从字符串加载（用于测试）
    pub fn from_str(yaml: &str) -> Result<Self, FastError> {
        let data: Value = serde_yaml::from_str(yaml)
            .map_err(|e| FastError::Config(format!("Failed to parse YAML: {}", e)))?;
        Ok(Self { data })
    }

    /// 按点号路径获取配置值并反序列化为目标类型
    ///
    /// ```ignore
    /// let host: String = config.get("database.host").unwrap_or("localhost".into());
    /// let port: u16 = config.get("database.port").unwrap_or(5432);
    /// ```
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let value = self.get_raw(key)?;
        serde_json::from_value(value.clone()).ok()
    }

    /// 获取原始 JSON Value
    pub fn get_raw(&self, key: &str) -> Option<&Value> {
        let mut current = &self.data;
        for part in key.split('.') {
            current = current.get(part)?;
        }
        Some(current)
    }

    /// 获取配置值，如果不存在则返回默认值
    pub fn get_or<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// 检查配置键是否存在
    pub fn has(&self, key: &str) -> bool {
        self.get_raw(key).is_some()
    }

    /// 设置配置值（运行时覆盖）
    pub fn set(&mut self, key: &str, value: Value) {
        let parts: Vec<&str> = key.split('.').collect();
        Self::set_nested(&mut self.data, &parts, value);
    }

    fn set_nested(current: &mut Value, parts: &[&str], value: Value) {
        if parts.is_empty() {
            return;
        }
        if parts.len() == 1 {
            if let Value::Object(map) = current {
                map.insert(parts[0].to_string(), value);
            }
            return;
        }
        if let Value::Object(map) = current {
            let entry = map
                .entry(parts[0].to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            Self::set_nested(entry, &parts[1..], value);
        }
    }

    /// 将环境变量映射到配置键
    /// 规则：APP_DB_HOST → app.db.host（下划线分隔 → 点号路径，全转小写）
    fn apply_env_overrides(data: &mut Value) {
        for (key, val) in env::vars() {
            // 只处理全大写且包含下划线的键（避免误覆盖）
            if key.chars().all(|c| c.is_uppercase() || c == '_') && key.contains('_') {
                let config_key = key.to_lowercase().replace('_', ".");
                let json_val = Value::String(val);
                let parts: Vec<&str> = config_key.split('.').collect();
                if parts.len() > 1 {
                    Self::set_nested(data, &parts, json_val);
                }
            }
        }
    }
}
