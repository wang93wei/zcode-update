//! zcode-update 库入口：编排 参数 → 来源 → 下载 → 解析 → 探测 → 输出。

pub mod cli;
pub mod local;
pub mod manifest;
pub mod output;

use cli::{resolve_source, Cli};
use std::io::Write;

/// 执行完整流程，返回进程退出码：成功 0，业务错误 2。
pub fn run(cli: Cli, out: &mut dyn Write) -> i32 {
    match try_run(&cli, out) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("错误：{e:#}");
            2
        }
    }
}

fn try_run(cli: &Cli, out: &mut dyn Write) -> anyhow::Result<i32> {
    // 1. 决定清单来源（--file / --url / 默认 endpoint）
    let source = resolve_source(cli)?;

    // 2. 加载并解析清单
    let (manifest, source_label) = manifest::load(&source)?;

    // 3. 探测本地安装状态与版本
    let app = local::detect(cli.app.as_deref());

    // 4. 比较（剥 v/V 前缀后精确相等）；已装但本地版本不可读时不视为最新
    let remote_cmp = output::strip_v(&manifest.version);
    let up_to_date = app.installed
        && app
            .version
            .as_deref()
            .map(|lv| output::strip_v(lv) == remote_cmp)
            .unwrap_or(false);

    // 5. 输出
    if up_to_date {
        output::render_up_to_date(app.version.as_deref().unwrap_or_default(), out);
    } else {
        output::render(&manifest, &source_label, &app, out);
    }
    Ok(0)
}
