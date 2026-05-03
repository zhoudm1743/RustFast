use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::framework::http::request::{Method, Request};
use crate::framework::error::{FastError, Result};

const MAX_HEADER_SIZE: usize = 8192;   // 8 KB
const MAX_BODY_SIZE: usize = 32 * 1024 * 1024; // 32 MB

/// 从 TcpStream 解析完整 HTTP/1.1 请求
pub async fn parse_request(stream: &mut TcpStream, remote_addr: String) -> Result<Request> {
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 4096];

    // 读取请求头（找到 \r\n\r\n）
    let header_end = loop {
        let n = stream
            .read(&mut tmp)
            .await
            .map_err(|e| FastError::Http(format!("Read error: {}", e)))?;

        if n == 0 {
            return Err(FastError::Http("Connection closed before request complete".into()));
        }

        buf.extend_from_slice(&tmp[..n]);

        if buf.len() > MAX_HEADER_SIZE {
            return Err(FastError::Http("Request header too large".into()));
        }

        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
    };

    // 解析请求行 + 头
    let header_bytes = &buf[..header_end];
    let header_str = std::str::from_utf8(header_bytes)
        .map_err(|_| FastError::Http("Invalid UTF-8 in headers".into()))?;

    let mut lines = header_str.split("\r\n");

    // 解析请求行
    let request_line = lines
        .next()
        .ok_or_else(|| FastError::Http("Empty request".into()))?;
    let (method, path, query) = parse_request_line(request_line)?;

    // 解析请求头
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some(colon) = line.find(':') {
            let key = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    // 读取 body
    let body_start = header_end + 4; // 跳过 \r\n\r\n
    let mut body = buf[body_start..].to_vec();

    let content_length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if content_length > MAX_BODY_SIZE {
        return Err(FastError::Http("Request body too large".into()));
    }

    while body.len() < content_length {
        let n = stream
            .read(&mut tmp)
            .await
            .map_err(|e| FastError::Http(format!("Body read error: {}", e)))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    Ok(Request {
        method,
        path,
        query,
        headers,
        body,
        params: HashMap::new(),
        remote_addr,
    })
}

/// 解析请求行：`GET /path?query HTTP/1.1`
fn parse_request_line(line: &str) -> Result<(Method, String, HashMap<String, String>)> {
    let mut parts = line.splitn(3, ' ');
    let method_str = parts
        .next()
        .ok_or_else(|| FastError::Http("Missing method".into()))?;
    let raw_path = parts
        .next()
        .ok_or_else(|| FastError::Http("Missing path".into()))?;

    let method = Method::from_str(method_str);

    let (path, query_str) = if let Some(pos) = raw_path.find('?') {
        (&raw_path[..pos], Some(&raw_path[pos + 1..]))
    } else {
        (raw_path, None)
    };

    // URL 解码路径
    let decoded_path = url_decode(path);

    // 解析查询字符串
    let query = query_str
        .map(parse_query_string)
        .unwrap_or_default();

    Ok((method, decoded_path, query))
}

/// 解析查询字符串 `key=value&key2=value2`
fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in query.split('&') {
        if let Some(eq) = pair.find('=') {
            let key = url_decode(&pair[..eq]);
            let value = url_decode(&pair[eq + 1..]);
            map.insert(key, value);
        } else if !pair.is_empty() {
            map.insert(url_decode(pair), String::new());
        }
    }
    map
}

/// 简单 URL 解码（%XX → 字符，+ → 空格）
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            result.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte as char);
                    i += 3;
                    continue;
                }
            }
            result.push('%');
            i += 1;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

/// 查找 \r\n\r\n 的结束位置
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}
