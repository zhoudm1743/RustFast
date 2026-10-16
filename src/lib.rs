// 允许 proc-macro 生成的代码在本 crate 内部通过 `::rustfast` 路径引用本 crate
extern crate self as rustfast;

pub mod framework;
pub mod bootstrap;
pub mod app;
pub mod routes;
