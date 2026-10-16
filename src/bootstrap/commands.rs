//! 控制台命令
//!
//! 在 HTTP 服务启动前处理命令行参数，命中命令则执行并返回 true（调用方应退出进程）。
//!
//! 支持的命令：
//! - `cargo run -- migrate`                 执行所有未应用的迁移
//! - `cargo run -- migrate:rollback [步数]`  回滚最近 N 个迁移（默认 1）
//! - `cargo run -- migrate:status`           查看迁移状态

use crate::app::migrations;
use crate::framework::database::{DbManager, Migrator};
use crate::framework::error::Result;
use crate::framework::foundation::application::app;

/// 处理控制台命令，返回 true 表示命令已执行（进程应退出）
pub async fn run_console(args: &[String]) -> Result<bool> {
    let command = match args.get(1).map(|s| s.as_str()) {
        Some(cmd) => cmd,
        None => return Ok(false),
    };

    if !matches!(command, "migrate" | "migrate:rollback" | "migrate:status") {
        return Ok(false);
    }

    let db = app()
        .make::<DbManager>()
        .ok_or_else(|| crate::framework::error::FastError::Database(
            "DbManager not registered".into(),
        ))?;

    let mut migrator = Migrator::new((*db).clone());
    migrator.add_all(migrations::all());

    match command {
        "migrate" => {
            let applied = migrator.migrate().await?;
            if applied.is_empty() {
                println!("Nothing to migrate.");
            } else {
                println!("Migrated {} migration(s):", applied.len());
                for v in applied {
                    println!("  ✓ {}", v);
                }
            }
        }
        "migrate:rollback" => {
            let steps: usize = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            let rolled = migrator.rollback(steps).await?;
            if rolled.is_empty() {
                println!("Nothing to rollback.");
            } else {
                println!("Rolled back {} migration(s):", rolled.len());
                for v in rolled {
                    println!("  ✓ {}", v);
                }
            }
        }
        "migrate:status" => {
            let status = migrator.status().await?;
            println!("{:<20} {:<30} {}", "VERSION", "NAME", "STATUS");
            for (version, name, applied) in status {
                println!(
                    "{:<20} {:<30} {}",
                    version,
                    name,
                    if applied { "✓ applied" } else { "✗ pending" }
                );
            }
        }
        _ => unreachable!(),
    }

    Ok(true)
}
