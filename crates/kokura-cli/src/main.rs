//! `kokuradbg` コマンドのエントリポイント。
//!
//! 引数解析は `args`、JSONジョブの読み込みは `json_io`、ROM実行と成果物生成は
//! `run` に分け、同じ実行機能をCLIの単発実行とジョブ形式の両方から呼び出します。

mod args;
mod json_io;
mod run;

use anyhow::Result;
use args::Args;
use clap::Parser;

fn main() -> Result<()> {
    let args = Args::parse();
    run::run(args)
}
