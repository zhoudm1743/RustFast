use std::collections::HashMap;
use serde::Serialize;

/// HTTP 响应
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain; charset=utf-8".to_string());
        Self {
            status,
            headers,
            body: Vec::new(),
        }
    }

    /// 设置响应头
    pub fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_lowercase(), value.to_string());
    }

    /// 发送 JSON 响应
    pub fn json<T: Serialize>(status: u16, data: &T) -> Self {
        let body = serde_json::to_vec(data).unwrap_or_default();
        let mut resp = Self::new(status);
        resp.set_header("content-type", "application/json; charset=utf-8");
        resp.set_header("content-length", &body.len().to_string());
        resp.body = body;
        resp
    }

    /// 发送纯文本响应
    pub fn text(status: u16, text: &str) -> Self {
        let body = text.as_bytes().to_vec();
        let mut resp = Self::new(status);
        resp.set_header("content-type", "text/plain; charset=utf-8");
        resp.set_header("content-length", &body.len().to_string());
        resp.body = body;
        resp
    }

    /// 标准成功响应 { code: 0, message: "ok", data: T }
    pub fn success<T: Serialize>(data: T, message: &str) -> Self {
        let payload = serde_json::json!({
            "code": 0,
            "message": message,
            "data": data
        });
        Self::json(200, &payload)
    }

    /// 标准失败响应
    pub fn fail(status: u16, code: i32, message: &str) -> Self {
        let payload = serde_json::json!({
            "code": code,
            "message": message,
            "data": null
        });
        Self::json(status, &payload)
    }

    /// 分页响应
    pub fn paginate<T: Serialize>(list: T, total: i64, page: i64, size: i64, message: &str) -> Self {
        let payload = serde_json::json!({
            "code": 0,
            "message": message,
            "data": {
                "list": list,
                "total": total,
                "page": page,
                "size": size,
                "pages": (total + size - 1) / size
            }
        });
        Self::json(200, &payload)
    }

    /// 序列化为 HTTP/1.1 字节流（用于写回 TCP 连接）
    pub fn to_bytes(&self) -> Vec<u8> {
        let status_text = status_text(self.status);
        let mut out = format!("HTTP/1.1 {} {}\r\n", self.status, status_text);

        // 写入 headers
        for (key, value) in &self.headers {
            out.push_str(&format!("{}: {}\r\n", key, value));
        }

        // Content-Length（如果 headers 中没有）
        if !self.headers.contains_key("content-length") {
            out.push_str(&format!("content-length: {}\r\n", self.body.len()));
        }

        out.push_str("connection: close\r\n");
        out.push_str("\r\n");

        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}
