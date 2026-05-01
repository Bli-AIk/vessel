//! Command-line interface for Cauld-ron.
//!
//! Cauld-ron 的命令行接口。

use crate::component_host::build_component;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Cauld-ron CLI entrypoint.
///
/// Cauld-ron CLI 入口。
#[derive(Debug, Parser)]
#[command(name = "cauld-ron")]
#[command(about = "Build RON files from WASM content modules")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Supported Cauld-ron commands.
///
/// Cauld-ron 支持的命令。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Execute a content module and write its generated RON files.
    ///
    /// 执行内容模块并写出其生成的 RON 文件。
    Build {
        /// Path to the WASM component.
        component: PathBuf,
        /// Output root directory.
        #[arg(long)]
        output: PathBuf,
    },
}

/// Parse CLI arguments and execute the selected command.
///
/// 解析命令行参数并执行所选命令。
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { component, output } => {
            let summary = build_component(&component, &output)?;
            println!(
                "cauld-ron: built {} file(s) from {} -> {}",
                summary.written_files,
                summary.component_path.display(),
                summary.output_dir.display()
            );
            Ok(())
        }
    }
}
