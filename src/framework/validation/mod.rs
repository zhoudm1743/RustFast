use std::collections::HashMap;
use std::sync::Arc;
use crate::framework::error::{FastError, Result};

/// 验证错误（字段 → 错误列表）
pub type ValidationErrors = HashMap<String, Vec<String>>;

/// 验证规则枚举
pub enum Rule {
    Required,
    MinLen(usize),
    MaxLen(usize),
    Min(f64),
    Max(f64),
    Email,
    Regex(String),
    In(Vec<String>),
    Numeric,
    Alpha,
    AlphaNumeric,
    Custom(String, Arc<dyn Fn(&str) -> bool + Send + Sync>),
}

/// 字段验证器
pub struct FieldValidator {
    field: String,
    value: Option<String>,
    rules: Vec<Rule>,
}

impl FieldValidator {
    pub fn new(field: &str, value: Option<&str>) -> Self {
        Self {
            field: field.to_string(),
            value: value.map(|s| s.to_string()),
            rules: Vec::new(),
        }
    }

    pub fn required(mut self) -> Self {
        self.rules.push(Rule::Required);
        self
    }

    pub fn min_len(mut self, n: usize) -> Self {
        self.rules.push(Rule::MinLen(n));
        self
    }

    pub fn max_len(mut self, n: usize) -> Self {
        self.rules.push(Rule::MaxLen(n));
        self
    }

    pub fn min(mut self, n: f64) -> Self {
        self.rules.push(Rule::Min(n));
        self
    }

    pub fn max(mut self, n: f64) -> Self {
        self.rules.push(Rule::Max(n));
        self
    }

    pub fn email(mut self) -> Self {
        self.rules.push(Rule::Email);
        self
    }

    pub fn numeric(mut self) -> Self {
        self.rules.push(Rule::Numeric);
        self
    }

    pub fn alpha(mut self) -> Self {
        self.rules.push(Rule::Alpha);
        self
    }

    pub fn alpha_numeric(mut self) -> Self {
        self.rules.push(Rule::AlphaNumeric);
        self
    }

    pub fn in_list(mut self, list: Vec<&str>) -> Self {
        self.rules.push(Rule::In(list.iter().map(|s| s.to_string()).collect()));
        self
    }

    pub fn custom(mut self, message: &str, f: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        self.rules.push(Rule::Custom(message.to_string(), Arc::new(f)));
        self
    }

    /// 验证并收集错误
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let val = self.value.as_deref().unwrap_or("");

        for rule in &self.rules {
            match rule {
                Rule::Required => {
                    if val.trim().is_empty() {
                        errors.push(format!("The {} field is required.", self.field));
                    }
                }
                Rule::MinLen(n) => {
                    if val.len() < *n {
                        errors.push(format!(
                            "The {} field must be at least {} characters.",
                            self.field, n
                        ));
                    }
                }
                Rule::MaxLen(n) => {
                    if val.len() > *n {
                        errors.push(format!(
                            "The {} field may not be greater than {} characters.",
                            self.field, n
                        ));
                    }
                }
                Rule::Min(n) => {
                    if let Ok(v) = val.parse::<f64>() {
                        if v < *n {
                            errors.push(format!(
                                "The {} field must be at least {}.",
                                self.field, n
                            ));
                        }
                    }
                }
                Rule::Max(n) => {
                    if let Ok(v) = val.parse::<f64>() {
                        if v > *n {
                            errors.push(format!(
                                "The {} field may not be greater than {}.",
                                self.field, n
                            ));
                        }
                    }
                }
                Rule::Email => {
                    if !is_valid_email(val) {
                        errors.push(format!("The {} field must be a valid email address.", self.field));
                    }
                }
                Rule::Numeric => {
                    if val.parse::<f64>().is_err() {
                        errors.push(format!("The {} field must be numeric.", self.field));
                    }
                }
                Rule::Alpha => {
                    if !val.chars().all(|c| c.is_alphabetic()) {
                        errors.push(format!("The {} field may only contain letters.", self.field));
                    }
                }
                Rule::AlphaNumeric => {
                    if !val.chars().all(|c| c.is_alphanumeric()) {
                        errors.push(format!(
                            "The {} field may only contain letters and numbers.",
                            self.field
                        ));
                    }
                }
                Rule::In(list) => {
                    if !list.iter().any(|s| s == val) {
                        errors.push(format!(
                            "The {} field must be one of: {}.",
                            self.field,
                            list.join(", ")
                        ));
                    }
                }
                Rule::Regex(pattern) => {
                    // 简单正则检查（无第三方 regex 库，仅支持基本模式）
                    // 如需完整正则支持，可添加 `regex` crate
                    errors.push(format!("Regex validation not supported without regex crate."));
                }
                Rule::Custom(msg, f) => {
                    if !f(val) {
                        errors.push(msg.clone());
                    }
                }
            }
        }

        errors
    }
}

/// 验证器构造器
pub struct Validator {
    fields: Vec<FieldValidator>,
}

impl Validator {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn field(mut self, validator: FieldValidator) -> Self {
        self.fields.push(validator);
        self
    }

    /// 执行验证，返回所有错误
    pub fn validate(self) -> ValidationErrors {
        let mut errors = HashMap::new();
        for fv in self.fields {
            let errs = fv.validate();
            if !errs.is_empty() {
                errors.insert(fv.field.clone(), errs);
            }
        }
        errors
    }

    /// 执行验证，有错误时返回 Err
    pub fn check(self) -> Result<()> {
        let errors = self.validate();
        if errors.is_empty() {
            Ok(())
        } else {
            let msg = errors
                .iter()
                .flat_map(|(_, errs)| errs.iter().map(|e| e.as_str()))
                .collect::<Vec<_>>()
                .join("; ");
            Err(FastError::Validation(msg))
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

/// 简单 Email 验证（无第三方库）
fn is_valid_email(s: &str) -> bool {
    let parts: Vec<&str> = s.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
}
