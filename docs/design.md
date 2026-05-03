# RustFast 框架设计文档

> RustFast — 参照 GoFast 架构思想，基于纯 Rust 实现的轻量、高性能后端框架。
> **核心原则：HTTP 服务器与 ORM 均为自研，不依赖 Axum/Actix/Diesel/SQLx 等高层框架。**

---

## 一、技术选型与依赖说明

### 允许使用的底层依赖

| 依赖 | 用途 | 说明 |
|------|------|------|
| `tokio` | 异步运行时 | Rust 异步生态基础设施 |
| `serde` / `serde_json` | 序列化/反序列化 | JSON 处理必需 |
| `serde_yaml` | YAML 配置解析 | 配置文件读取 |
| `uuid` | UUID 生成 | 主键生成 |
| `chrono` | 日期时间 | 时间处理 |
| `tracing` / `tracing-subscriber` | 结构化日志 | 低层日志基础 |
| `tokio-postgres` | PostgreSQL 驱动 | 数据库通信协议 |
| `mysql_async` | MySQL 驱动 | 数据库通信协议 |
| `rusqlite` | SQLite 驱动 | 嵌入式数据库 |
| `bcrypt` / `sha2` | 密码/签名 | JWT/安全 |
| `base64` | Base64 编码 | JWT 实现 |

### 禁止使用的高层框架
- ❌ Axum、Actix-web、Warp、Rocket（HTTP 框架）
- ❌ Diesel、SQLx、Sea-ORM（ORM 框架）
- ❌ Tower（中间件框架）

---

## 二、项目目录结构

```
RustFast/
├── Cargo.toml                    # Workspace 根配置
├── src/
│   ├── main.rs                   # 入口点
│   ├── lib.rs                    # 库根
│   ├── app/                      # 业务代码（用户编写）
│   │   ├── http/
│   │   │   ├── controllers/      # 控制器
│   │   │   ├── middleware/       # 中间件
│   │   │   └── requests/         # 请求结构体
│   │   ├── models/               # 数据模型
│   │   ├── providers/            # 自定义 ServiceProvider
│   │   └── console/
│   │       └── commands/         # 自定义命令
│   ├── bootstrap/
│   │   ├── app.rs                # 应用引导 & Provider 列表
│   │   └── commands.rs           # 命令注册
│   ├── config/
│   │   └── config.yaml           # 配置文件
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── api.rs                # API 路由
│   │   └── admin.rs              # 后台路由
│   └── framework/                # 框架核心
│       ├── mod.rs
│       ├── foundation/           # IoC 容器 & Application
│       │   ├── mod.rs
│       │   ├── container.rs      # 服务容器
│       │   ├── application.rs    # Application 主体
│       │   └── provider.rs       # ServiceProvider trait
│       ├── contracts/            # Trait 接口定义
│       │   ├── mod.rs
│       │   ├── http.rs           # HTTP 接口
│       │   ├── orm.rs            # ORM 接口
│       │   ├── config.rs         # 配置接口
│       │   ├── log.rs            # 日志接口
│       │   ├── cache.rs          # 缓存接口
│       │   └── validation.rs     # 验证接口
│       ├── facades/              # 静态门面
│       │   ├── mod.rs
│       │   ├── config.rs
│       │   ├── db.rs
│       │   ├── log.rs
│       │   ├── route.rs
│       │   └── cache.rs
│       ├── config/               # 配置服务
│       │   ├── mod.rs
│       │   ├── config.rs
│       │   └── service_provider.rs
│       ├── log/                  # 日志服务
│       │   ├── mod.rs
│       │   └── service_provider.rs
│       ├── http/                 # 自研 HTTP 框架
│       │   ├── mod.rs
│       │   ├── server.rs         # TCP 监听 & 连接管理
│       │   ├── parser.rs         # HTTP/1.1 协议解析
│       │   ├── request.rs        # Request 类型
│       │   ├── response.rs       # Response 类型
│       │   ├── router.rs         # Trie 路由树
│       │   ├── context.rs        # 请求上下文
│       │   ├── middleware.rs     # 中间件链
│       │   └── service_provider.rs
│       ├── database/             # 自研 ORM
│       │   ├── mod.rs
│       │   ├── connection.rs     # 连接抽象
│       │   ├── pool.rs           # 连接池
│       │   ├── query_builder.rs  # SQL 查询构造器
│       │   ├── model.rs          # Model trait
│       │   ├── migration.rs      # 迁移系统
│       │   ├── service_provider.rs
│       │   └── drivers/
│       │       ├── mod.rs
│       │       ├── postgres.rs
│       │       ├── mysql.rs
│       │       └── sqlite.rs
│       ├── cache/                # 缓存服务
│       │   ├── mod.rs
│       │   ├── memory_store.rs
│       │   └── service_provider.rs
│       ├── validation/           # 验证服务
│       │   ├── mod.rs
│       │   └── service_provider.rs
│       ├── jwt/                  # JWT（自研，无第三方）
│       │   ├── mod.rs
│       │   └── service_provider.rs
│       └── utils/                # 工具函数
│           ├── mod.rs
│           └── string_util.rs
```

---

## 三、核心架构设计

### 3.1 IoC 服务容器（Container）

```rust
// 核心 API
container.bind::<dyn Service>(factory_fn);      // 每次创建新实例
container.singleton::<dyn Service>(factory_fn); // 全局单例，惰性初始化
container.instance::<dyn Service>(instance);    // 直接注册已有实例
container.make::<dyn Service>() -> Arc<dyn Service>; // 解析服务
```

**实现要点：**
- 使用 `TypeId` 作为 Key，`Arc<dyn Any + Send + Sync>` 存储实例
- Singleton 使用 `OnceLock` 或 `Mutex<Option<>>` 实现惰性初始化
- Factory 函数签名：`Fn(&Container) -> Arc<dyn Any + Send + Sync>`

### 3.2 ServiceProvider 生命周期

```rust
pub trait ServiceProvider: Send + Sync {
    fn register(&self, app: &mut Application);  // 注册阶段：只绑定工厂
    fn boot(&self, app: &Application);           // 启动阶段：可依赖其他服务
    fn shutdown(&self, app: &Application) {}     // 关闭钩子（可选）
}
```

**执行顺序：**
1. 依次调用所有 Provider 的 `register()`
2. 依次调用所有 Provider 的 `boot()`
3. 启动 HTTP Server
4. 收到 SIGTERM/SIGINT 后依次调用所有 Provider 的 `shutdown()`（逆序）

### 3.3 Facade 门面

```rust
// 用法示例
Config::get::<String>("app.name");
Log::info("Server started");
Db::table("users").where_eq("id", 1).first::<User>().await;
Route::get("/api/users", user_controller::index);
```

**实现方式：**
- 全局 `static APP: OnceLock<Arc<Application>>` 存储应用实例
- Facade 结构体通过全局应用实例解析对应服务

---

## 四、自研 HTTP 框架设计

### 4.1 整体架构

```
TCP连接
  └─► HTTP/1.1 解析器 (parser.rs)
        └─► Request 对象
              └─► 路由匹配 (router.rs) - Trie 树
                    └─► 中间件链 (middleware.rs) - 责任链模式
                          └─► Handler (用户控制器)
                                └─► Response 对象
                                      └─► HTTP/1.1 序列化 → TCP 写回
```

### 4.2 HTTP 解析器

自研 HTTP/1.1 协议解析，支持：
- 请求行解析（Method、Path、Query、HTTP版本）
- 请求头解析（大小写不敏感）
- Body 读取（Content-Length 和 chunked）
- Multipart 文件上传解析

### 4.3 路由树（Trie）

```
/ ─── api ─── users ─── :id  (GET /api/users/:id)
         │         └─── ""   (GET /api/users, POST /api/users)
         └── admin ─── ...

路由节点类型：
- Static: 精确匹配（/users）
- Param:  参数捕获（:id）
- Wildcard: 通配符（*）
```

### 4.4 中间件链

```rust
pub type MiddlewareFn = Arc<dyn Fn(Context, Next) -> BoxFuture<Result<Response>> + Send + Sync>;

pub struct Next {
    middlewares: Arc<Vec<MiddlewareFn>>,
    index: usize,
    handler: Arc<HandlerFn>,
}

impl Next {
    pub async fn run(self, ctx: Context) -> Result<Response> {
        // 依次执行中间件，最后调用 handler
    }
}
```

### 4.5 Context（请求上下文）

```rust
pub struct Context {
    pub request: Request,
    state: HashMap<String, Box<dyn Any + Send + Sync>>,  // 中间件间共享状态
    pub(crate) response_writer: ResponseWriter,
}

impl Context {
    pub fn param(&self, key: &str) -> Option<&str>;
    pub fn query(&self, key: &str) -> Option<&str>;
    pub fn header(&self, key: &str) -> Option<&str>;
    pub async fn body_json<T: DeserializeOwned>(&self) -> Result<T>;
    pub fn set<T: Any + Send + Sync>(&mut self, key: &str, val: T);
    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<&T>;
    pub fn json(&mut self, status: u16, data: impl Serialize) -> Result<()>;
    pub fn string(&mut self, status: u16, s: &str) -> Result<()>;
}
```

---

## 五、自研 ORM 设计

### 5.1 整体架构

```
QueryBuilder（SQL 构造器）
  ├── select / where / join / order / limit / offset
  ├── 生成参数化 SQL
  └── execute → Connection → Driver（postgres/mysql/sqlite）
                                └── 结果集 Row 映射 → Model
```

### 5.2 Model Trait

```rust
pub trait Model: Sized + Send + Sync {
    type PrimaryKey: Serialize + for<'de> Deserialize<'de> + Send + Sync;

    fn table_name() -> &'static str;
    fn primary_key() -> &'static str { "id" }
    fn from_row(row: &Row) -> Result<Self>;
    fn to_values(&self) -> Vec<(&'static str, Value)>;
}

// 使用派生宏（或手动实现）
#[derive(Model)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}
```

### 5.3 QueryBuilder API

```rust
// 查询
let users = Db::table("users")
    .select(["id", "name", "email"])
    .where_eq("active", 1)
    .where_gt("age", 18)
    .order_by("created_at", Order::Desc)
    .limit(10)
    .offset(20)
    .get::<User>()
    .await?;

// 单条
let user = Db::table("users")
    .where_eq("id", user_id)
    .first::<User>()
    .await?;

// 插入
let id = Db::table("users")
    .insert(user.to_values())
    .await?;

// 更新
Db::table("users")
    .where_eq("id", user_id)
    .update([("name", "Alice"), ("updated_at", now)])
    .await?;

// 删除
Db::table("users")
    .where_eq("id", user_id)
    .delete()
    .await?;

// 原始 SQL
let rows = Db::raw("SELECT * FROM users WHERE id = $1", &[&user_id])
    .fetch_all::<User>()
    .await?;
```

### 5.4 连接池

- 自研简单连接池（基于 `tokio::sync::Semaphore` + `Mutex<VecDeque<Connection>>`）
- 支持最小/最大连接数、超时、健康检查
- 支持 PostgreSQL、MySQL、SQLite 三种驱动

### 5.5 迁移系统

```rust
pub trait Migration: Send + Sync {
    fn version(&self) -> &'static str;   // "20240101_000001"
    fn up(&self) -> &'static str;        // SQL DDL
    fn down(&self) -> &'static str;      // 回滚 SQL
}
```

---

## 六、其他服务设计

### 6.1 配置服务（Config）

- 读取 `config/config.yaml`
- 支持环境变量覆盖（`APP_NAME` 覆盖 `app.name`）
- 点号路径访问：`Config::get::<String>("database.connections.default.host")`
- 类型安全的反序列化

### 6.2 日志服务（Log）

- 基于 `tracing` 构建
- 支持级别：trace / debug / info / warn / error
- 结构化字段支持
- 文件轮转

### 6.3 缓存服务（Cache）

- 支持 Memory / Redis Store
- API：`get / set / forget / has / increment / decrement`
- 标签分组、过期时间

### 6.4 验证服务（Validation）

- 自研字段验证（required、min、max、email、regex 等）
- 基于 Rust proc-macro 或手动 impl 声明验证规则
- 返回字段级错误信息

### 6.5 JWT 服务（自研，无第三方）

- 手动实现 HS256 签名（HMAC-SHA256）
- 支持 Claims 自定义
- 无需任何 JWT 库

---

## 七、实现计划（分阶段）

### Phase 1：核心骨架
1. ✅ Cargo.toml 项目初始化
2. IoC 容器（Container + Application）
3. ServiceProvider trait
4. 配置服务（Config）
5. 日志服务（Log）

### Phase 2：自研 HTTP 框架
1. TCP 服务器（TcpListener）
2. HTTP/1.1 协议解析器
3. Request / Response 类型
4. Trie 路由树
5. 中间件链
6. Context / Response Builder
7. HTTP ServiceProvider

### Phase 3：自研 ORM
1. 连接抽象与连接池
2. QueryBuilder（SELECT/INSERT/UPDATE/DELETE）
3. Model trait
4. PostgreSQL 驱动适配
5. MySQL 驱动适配
6. SQLite 驱动适配
7. 迁移系统
8. Database ServiceProvider

### Phase 4：辅助服务
1. 缓存服务（Memory Store）
2. 验证服务
3. JWT 服务（自研 HS256）
4. Facades 门面层

### Phase 5：应用层示例
1. 示例控制器
2. 示例模型
3. 示例路由注册
4. 完整的 main.rs

---

## 八、关键设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 异步模型 | `tokio` multi-thread | Rust 异步生态事实标准 |
| HTTP 并发 | 每连接一个 tokio task | 简单高效，支持大量并发 |
| 类型擦除 | `Arc<dyn Any + Send + Sync>` | IoC 容器需要运行时类型擦除 |
| 路由算法 | Trie（前缀树）| O(path_length) 匹配，支持参数路由 |
| 连接池 | Semaphore + VecDeque | 无需第三方，纯 tokio 实现 |
| ORM 返回值 | `Result<T>` | Rust 错误处理惯用法 |
| 配置格式 | YAML | 与 GoFast 保持一致 |
| 主键类型 | UUID v4 | 分布式友好 |
