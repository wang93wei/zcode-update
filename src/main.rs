//! zcode-update 二进制入口。

use clap::Parser as _;

fn main() {
    // 参数解析错误：统一中文前缀 + exit 2；--help/--version 正常输出后 exit 0
    let cli = match zcode_update::cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            if e.use_stderr() {
                eprintln!("错误：{e}");
                std::process::exit(2);
            }
            let _ = e.print();
            std::process::exit(0);
        }
    };

    std::process::exit(zcode_update::run(cli, &mut std::io::stdout()));
}
