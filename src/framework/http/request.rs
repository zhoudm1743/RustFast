use std::collections::HashMap;

/// HTTP 请求方法
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    Other(String),
}

impl Method {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "PATCH" => Method::Patch,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            "TRACE" => Method::Trace,
            "CONNECT" => Method::Connect,
            other => Method::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Trace => "TRACE",
            Method::Connect => "CONNECT",
            Method::Other(s) => s.as_str(),
        }
    }
}

/// HTTP 请求
#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    /// 不含查询字符串的路径，如 `/api/users`
    pub path: String,
    /// 查询字符串参数
    pub query: HashMap<String, String>,
    /// 请求头（键已转为小写）
    pub headers: HashMap<String, String>,
    /// 原始请求体
    pub body: Vec<u8>,
    /// 路由路径参数（由路由器填充，如 `:id`）
    pub params: HashMap<String, String>,
    /// 客户端 IP
    pub remote_addr: String,
}

impl Request {
    pub fn new() -> Self {
        Self {
            method: Method::Get,
            path: "/".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            params: HashMap::new(),
            remote_addr: String::new(),
        }
    }

    /// 获取请求头（键不区分大小写）
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(&key.to_lowercase()).map(|s| s.as_str())
    }

    /// 获取查询参数
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(|s| s.as_str())
    }

    /// 获取路由参数
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// 获取 Content-Type
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// 将 body 反序列化为 JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// 将 body 作为 UTF-8 字符串返回
    pub fn body_string(&self) -> Option<String> {
        String::from_utf8(self.body.clone()).ok()
    }
}

impl Default for Request {
    fn default() -> Self {
        Self::new()
    }
}
