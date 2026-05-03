pub mod service_provider;
pub use service_provider::LogServiceProvider;

// 重导出 tracing 宏，方便框架用户使用
pub use tracing::{debug, error, info, trace, warn};
