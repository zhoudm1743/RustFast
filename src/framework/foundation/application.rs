use std::fmt;
use std::sync::{Arc, OnceLock};
use crate::framework::foundation::container::Container;
use crate::framework::foundation::provider::ServiceProvider;

/// RustFast 应用主体
///
/// 管理服务容器和 Provider 生命周期
pub struct Application {
    pub(crate) container: Arc<Container>,
    providers: Vec<Box<dyn ServiceProvider>>,
    base_path: String,
}

impl Application {
    /// 创建新应用实例
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            container: Arc::new(Container::new()),
            providers: Vec::new(),
            base_path: base_path.into(),
        }
    }

    /// 设置服务提供者列表
    pub fn set_providers(&mut self, providers: Vec<Box<dyn ServiceProvider>>) {
        self.providers = providers;
    }

    /// 注册单个 Provider
    pub fn register_provider(&mut self, provider: Box<dyn ServiceProvider>) {
        self.providers.push(provider);
    }

    /// 引导应用：依次执行所有 Provider 的 register 和 boot
    pub fn boot(&self) {
        // Phase 1: 所有 register
        for provider in &self.providers {
            provider.register(&self.container);
        }
        // Phase 2: 所有 boot
        for provider in &self.providers {
            provider.boot(&self.container);
        }
    }

    /// 优雅关闭：逆序执行所有 Provider 的 shutdown
    pub fn shutdown(&self) {
        for provider in self.providers.iter().rev() {
            provider.shutdown(&self.container);
        }
    }

    /// 获取服务容器引用
    pub fn container(&self) -> &Container {
        &self.container
    }

    /// 解析服务（委托给容器）
    pub fn make<T: std::any::Any + Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.container.make::<T>()
    }

    /// 获取应用根路径
    pub fn base_path(&self) -> &str {
        &self.base_path
    }
}

impl fmt::Debug for Application {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Application")
            .field("base_path", &self.base_path)
            .finish()
    }
}

impl Application {

    /// 获取配置文件路径
    pub fn config_path(&self) -> String {
        format!("{}/config/config.yaml", self.base_path)
    }
}

// ── 全局应用实例 ──────────────────────────────────────────────────────────────

static GLOBAL_APP: OnceLock<Arc<Application>> = OnceLock::new();

/// 设置全局应用实例（由 bootstrap::Boot 调用，仅调用一次）
pub fn set_app(app: Arc<Application>) {
    GLOBAL_APP
        .set(app)
        .expect("Global application already initialized");
}

/// 获取全局应用实例引用
pub fn app() -> &'static Arc<Application> {
    GLOBAL_APP.get().expect("Application not initialized. Call bootstrap::boot() first")
}
