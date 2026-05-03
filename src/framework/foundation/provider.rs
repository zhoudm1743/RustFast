use std::sync::Arc;
use crate::framework::foundation::container::Container;

/// ServiceProvider 生命周期 Trait
///
/// 所有服务提供者必须实现此 Trait，框架按如下顺序调用：
/// 1. 所有 Provider 的 `register()` — 只注册工厂，不解析依赖
/// 2. 所有 Provider 的 `boot()`     — 可安全使用容器中的其他服务
/// 3. 应用关闭时逆序调用 `shutdown()`
pub trait ServiceProvider: Send + Sync {
    /// 注册阶段：向容器绑定服务工厂或实例
    /// 此阶段禁止调用 `container.make()`，因为依赖服务可能尚未注册
    fn register(&self, container: &Container);

    /// 启动阶段：可安全依赖其他已注册服务
    fn boot(&self, container: &Container) {}

    /// 关闭阶段（可选）：释放资源，逆序执行
    fn shutdown(&self, container: &Container) {}
}
