use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::framework::http::request::{Method, Request};
use crate::framework::http::response::Response;
use crate::framework::error::Result;

/// Handler 函数类型
pub type HandlerFn =
    dyn Fn(Request) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> + Send + Sync;

/// 路由树节点
#[derive(Default)]
struct Node {
    /// 子节点（静态路径段 → 节点）
    children: HashMap<String, Node>,
    /// 动态参数子节点（`:name` → 节点）
    param_child: Option<(String, Box<Node>)>,
    /// 通配符子节点（`*`）
    wildcard_child: Option<Box<Node>>,
    /// 当前节点注册的 handlers（按 Method 分）
    handlers: HashMap<Method, Arc<HandlerFn>>,
}

/// 路由匹配结果
pub struct RouteMatch {
    pub handler: Arc<HandlerFn>,
    /// 路径参数，如 `:id` 的值
    pub params: HashMap<String, String>,
}

/// Trie 路由树
///
/// 支持：
/// - 静态路由：`/api/users`
/// - 参数路由：`/api/users/:id`
/// - 通配符：`/static/*`
pub struct Router {
    root: Node,
}

impl Router {
    pub fn new() -> Self {
        Self {
            root: Node::default(),
        }
    }

    /// 注册路由
    pub fn add<F, Fut>(&mut self, method: Method, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let handler_arc: Arc<HandlerFn> = Arc::new(move |req| Box::pin(handler(req)));
        let segments = path_segments(path);
        insert_node(&mut self.root, &segments, method, handler_arc);
    }

    /// 便捷方法
    pub fn get<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.add(Method::Get, path, handler);
    }

    pub fn post<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.add(Method::Post, path, handler);
    }

    pub fn put<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.add(Method::Put, path, handler);
    }

    pub fn patch<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.add(Method::Patch, path, handler);
    }

    pub fn delete<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.add(Method::Delete, path, handler);
    }

    /// 路由组（前缀分组）
    pub fn group<F>(&mut self, prefix: &str, f: F)
    where
        F: FnOnce(&mut RouteGroup),
    {
        let mut group = RouteGroup {
            prefix: prefix.to_string(),
            router: self,
        };
        f(&mut group);
    }

    /// 匹配路由
    pub fn find(&self, method: &Method, path: &str) -> Option<RouteMatch> {
        let segments = path_segments(path);
        let mut params = HashMap::new();
        let node = find_node(&self.root, &segments, &mut params)?;
        let handler = node.handlers.get(method).cloned()?;
        Some(RouteMatch { handler, params })
    }

    /// 检查路径是否有任何方法注册（用于 405 判断）
    pub fn has_path(&self, path: &str) -> bool {
        let segments = path_segments(path);
        let mut params = HashMap::new();
        find_node(&self.root, &segments, &mut params)
            .map(|n| !n.handlers.is_empty())
            .unwrap_or(false)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// 路由组（路径前缀）
pub struct RouteGroup<'a> {
    prefix: String,
    router: &'a mut Router,
}

impl<'a> RouteGroup<'a> {
    pub fn get<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let full = format!("{}{}", self.prefix, path);
        self.router.get(&full, handler);
    }

    pub fn post<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let full = format!("{}{}", self.prefix, path);
        self.router.post(&full, handler);
    }

    pub fn put<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let full = format!("{}{}", self.prefix, path);
        self.router.put(&full, handler);
    }

    pub fn patch<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let full = format!("{}{}", self.prefix, path);
        self.router.patch(&full, handler);
    }

    pub fn delete<F, Fut>(&mut self, path: &str, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let full = format!("{}{}", self.prefix, path);
        self.router.delete(&full, handler);
    }

    /// 嵌套路由组
    pub fn group<F>(&mut self, sub_prefix: &str, f: F)
    where
        F: FnOnce(&mut RouteGroup),
    {
        let full_prefix = format!("{}{}", self.prefix, sub_prefix);
        let mut sub = RouteGroup {
            prefix: full_prefix,
            router: self.router,
        };
        f(&mut sub);
    }
}

// ── 内部辅助函数 ──────────────────────────────────────────────────────────────

fn path_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

fn insert_node(node: &mut Node, segments: &[&str], method: Method, handler: Arc<HandlerFn>) {
    if segments.is_empty() {
        node.handlers.insert(method, handler);
        return;
    }

    let seg = segments[0];
    let rest = &segments[1..];

    if seg == "*" {
        let child = node.wildcard_child.get_or_insert_with(|| Box::new(Node::default()));
        insert_node(child, rest, method, handler);
    } else if let Some(param_name) = seg.strip_prefix(':') {
        if let Some((_, child)) = &mut node.param_child {
            insert_node(child, rest, method, handler);
        } else {
            let mut child = Box::new(Node::default());
            insert_node(&mut child, rest, method, handler);
            node.param_child = Some((param_name.to_string(), child));
        }
    } else {
        let child = node.children.entry(seg.to_string()).or_default();
        insert_node(child, rest, method, handler);
    }
}

fn find_node<'a>(
    node: &'a Node,
    segments: &[&str],
    params: &mut HashMap<String, String>,
) -> Option<&'a Node> {
    if segments.is_empty() {
        return Some(node);
    }

    let seg = segments[0];
    let rest = &segments[1..];

    // 1. 精确匹配
    if let Some(child) = node.children.get(seg) {
        if let Some(found) = find_node(child, rest, params) {
            return Some(found);
        }
    }

    // 2. 参数匹配
    if let Some((name, child)) = &node.param_child {
        let mut local_params = params.clone();
        local_params.insert(name.clone(), seg.to_string());
        if let Some(found) = find_node(child, rest, &mut local_params) {
            *params = local_params;
            return Some(found);
        }
    }

    // 3. 通配符匹配
    if let Some(child) = &node.wildcard_child {
        // 通配符消费剩余所有段
        let wildcard_val = std::iter::once(seg)
            .chain(rest.iter().copied())
            .collect::<Vec<_>>()
            .join("/");
        params.insert("*".to_string(), wildcard_val);
        return Some(child.as_ref());
    }

    None
}
