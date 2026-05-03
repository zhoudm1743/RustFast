use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::framework::error::{FastError, Result};

type HmacSha256 = Hmac<Sha256>;

/// JWT Header（固定 HS256）
static HEADER_B64: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";

/// JWT Claims
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    /// 主题（用户 ID 等）
    pub sub: String,
    /// 签发时间（Unix 时间戳）
    pub iat: u64,
    /// 过期时间（Unix 时间戳）
    pub exp: u64,
    /// 自定义字段
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Claims {
    pub fn new(sub: &str, ttl_secs: u64) -> Self {
        let now = current_timestamp();
        Self {
            sub: sub.to_string(),
            iat: now,
            exp: now + ttl_secs,
            extra: HashMap::new(),
        }
    }

    pub fn with_field(mut self, key: &str, val: impl serde::Serialize) -> Self {
        self.extra.insert(
            key.to_string(),
            serde_json::to_value(val).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    pub fn is_expired(&self) -> bool {
        current_timestamp() > self.exp
    }
}

/// JWT 服务（自研 HS256，无第三方 JWT 库）
pub struct JwtService {
    secret: Vec<u8>,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
        }
    }

    /// 签发 JWT Token
    pub fn sign(&self, claims: &Claims) -> Result<String> {
        let payload_json = serde_json::to_string(claims)
            .map_err(|e| FastError::Internal(format!("JWT serialize error: {}", e)))?;

        let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());

        let signing_input = format!("{}.{}", HEADER_B64, payload_b64);

        let signature = self.hmac_sha256(signing_input.as_bytes())?;
        let sig_b64 = URL_SAFE_NO_PAD.encode(&signature);

        Ok(format!("{}.{}", signing_input, sig_b64))
    }

    /// 验证并解析 JWT Token
    pub fn verify(&self, token: &str) -> Result<Claims> {
        let parts: Vec<&str> = token.splitn(3, '.').collect();
        if parts.len() != 3 {
            return Err(FastError::Unauthorized("Invalid token format".into()));
        }

        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let expected_sig = self.hmac_sha256(signing_input.as_bytes())?;
        let expected_sig_b64 = URL_SAFE_NO_PAD.encode(&expected_sig);

        // 恒定时间比较，防止时序攻击
        if !constant_time_eq(parts[2].as_bytes(), expected_sig_b64.as_bytes()) {
            return Err(FastError::Unauthorized("Invalid token signature".into()));
        }

        // 解码 payload
        let payload_bytes = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| FastError::Unauthorized("Invalid token payload encoding".into()))?;

        let claims: Claims = serde_json::from_slice(&payload_bytes)
            .map_err(|_| FastError::Unauthorized("Invalid token payload".into()))?;

        if claims.is_expired() {
            return Err(FastError::Unauthorized("Token expired".into()));
        }

        Ok(claims)
    }

    fn hmac_sha256(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| FastError::Internal("Invalid JWT secret key".into()))?;
        mac.update(data);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 恒定时间字节比较（防止时序攻击）
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
