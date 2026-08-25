# zcode-update Rust CLI 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `zcode-preview-update.sh` 转换为功能、参数、输出文案完全对齐的 Rust 二进制 `zcode-update`，支持三平台本地版本探测与 GitHub Actions 构建。

**Architecture:** 单 binary+lib 双目标 crate，模块分为 `cli`（clap 参数与来源解析）、`manifest`（下载+YAML 解析）、`local`（三平台本地版本探测）、`output`（展示渲染）。`lib.rs::run()` 编排全流程，便于集成测试注入输出缓冲区。

**Tech Stack:** clap 4（derive）、ureq 3（rustls 默认 TLS）、serde_yaml_ng 0.10、plist 1、jiff 0.2、anyhow 1；Windows 平台附加 windows-sys；dev-dependency tempfile 3。

## Global Constraints

设计规格来源：`docs/superpowers/specs/2026-08-25-zcode-update-rust-cli-design.md`。所有任务隐含遵守：

- **二进制名**：`zcode-update`（crate 名 `zcode-update`，lib 名自动 `zcode_update`）
- **Rust edition**: 2021
- **退出码**：成功（含"已最新"分支）= 0；一切用户可见错误 = 2，stderr 以 `错误：` 前缀输出中文文案
- **固定文案（逐字使用，勿改动标点/emoji）**：
  - `✅ 当前已是最新版本：{本地版本}，暂无更新。`
  - `🎉 发现新版本！` / `本地版本：{}`
  - `ℹ️ 未检测到本地 ZCode 应用，以下为最新版本信息` / `提示：如已安装在其它位置，请使用 --app 指定路径`
  - `ZCode 更新信息` / `=============` / `最新版本：{}` / `名称：{}` / `日期：{}` / `来源：{}`
  - `下载链接：` / `更新日志：` / `（Manifest 未提供更新日志）`
  - 错误：`不支持的架构：{}`、`不支持的更新通道：{}`、`不支持的目标：{}`、`--url 仅支持 HTTP/HTTPS 地址`、`无法读取文件：{}`、`Manifest 中缺少 version`、`Manifest 中缺少下载链接`
- **默认 endpoint**：`https://zcode.z.ai/api/v1/releases/electron/manifest?platform={darwin|windows}-{aarch64|x86_64}&channel={preview=3|stable=1}`
- **HTTP**：全局超时 30s、连接超时 10s、跟随重定向（默认 max 10）、Header `Accept: application/x-yaml,text/yaml,text/plain,*/*`、`User-Agent: zcode-preview-update.sh`（保持服务端统计口径不变）；非 2xx 视为错误
- **版本比较**：双方剥离首个前缀字符 `v` 或 `V` 后字符串精确相等；不做 semver
- **已知偏差（可接受）**：clap 自身的使用帮助/错误详情保留英文框架，外层统一加中文前缀 `错误：` 并保证 exit 2；`--help` 全中文且 exit 0

---

### Task 1: 项目脚手架

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `src/lib.rs`, `src/main.rs`, `src/cli.rs`, `src/manifest.rs`, `src/local.rs`, `src/output.rs`

**Interfaces:**
- Produces: 可编译的空模块骨架，后续任务逐个填充。

- [ ] **Step 1: 初始化 cargo 项目并写入依赖**

在工作目录执行（不要 `cargo init --name` 之外生成示例代码后保留；删除生成的 main 示例内容）：

```bash
cargo init --name zcode-update
cargo add anyhow@1 clap@4 --features clap/derive
cargo add serde@1 --features derive
cargo add serde_yaml_ng@0.10 ureq@3 jiff@0.2 plist@1
cargo add windows-sys --target 'cfg(target_os = "windows")' --features Win32_Foundation --features Win32_Storage_FileSystem
cargo add tempfile@3 --dev
```

`Cargo.toml` 最终应包含（核对 feature 无遗漏，`edition = "2021"`）：

```toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
jiff = "0.2"
plist = "1"
serde = { version = "1", features = ["derive"] }
serde_yaml_ng = "0.10"
ureq = "3"

[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_Foundation", "Win32_Storage_FileSystem"] }

[dev-dependencies]
tempfile = "3"

[profile.release]
strip = true
lto = true
codegen-units = 1
```

注意：核对 `[package]` 中 `edition = "2021"`（若 `cargo init` 生成了其他版本，改为 2021）。

- [ ] **Step 2: 写入 .gitignore**

```gitignore
/target
Cargo.lock.bak
.DS_Store
```

注：这是最终发布的二进制项目，`Cargo.lock` 应入库（不忽略）。

- [ ] **Step 3: 创建模块骨架**

`src/cli.rs`：

```rust
//! 命令行参数定义与清单来源解析。
```

`src/manifest.rs`：

```rust
//! 清单下载与 YAML 解析。
```

`src/local.rs`：

```rust
//! 本地已安装 ZCode 版本探测（按运行平台分派）。
```

`src/output.rs`：

```rust
//! 结果展示与文案渲染。
```

`src/lib.rs`：

```rust
//! zcode-update 库入口：编排 参数 → 来源 → 下载 → 解析 → 探测 → 输出。

pub mod cli;
pub mod local;
pub mod manifest;
pub mod output;
```

`src/main.rs`：

```rust
//! zcode-update 二进制入口（Task 7 填充实际逻辑）。

fn main() {}
```

- [ ] **Step 4: 验证构建与空测试通过**

Run: `cargo build && cargo test`
Expected: 编译成功，`running 0 tests` 通过

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/
git commit -m "chore: 初始化 zcode-update Rust 项目骨架"
```

---

### Task 2: CLI 参数与来源解析（cli.rs）

**Files:**
- Modify: `src/cli.rs`
- Test: `src/cli.rs`（文件内 `#[cfg(test)] mod tests`）

**Interfaces:**
- Produces:
  - `pub struct Cli { pub target: Option<String>, pub arch: Option<String>, pub channel: Option<String>, pub file: Option<PathBuf>, pub url: Option<String>, pub app: Option<PathBuf> }`（clap Parser + Debug + Default）
  - `pub enum Source { LocalFile(PathBuf), Remote(String) }`（derive Debug, Clone, PartialEq, Eq）
  - `pub fn normalize_arch(input: &str) -> anyhow::Result<&'static str>`
  - `pub fn resolve_source(cli: &Cli) -> anyhow::Result<Source>`

- [ ] **Step 1: 写失败测试（归一化 + 来源解析全部规则）**

追加到 `src/cli.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_arch_maps_aliases() {
        assert_eq!(normalize_arch("arm64").unwrap(), "aarch64");
        assert_eq!(normalize_arch("aarch64").unwrap(), "aarch64");
        assert_eq!(normalize_arch("x64").unwrap(), "x86_64");
        assert_eq!(normalize_arch("x86_64").unwrap(), "x86_64");
        assert_eq!(normalize_arch("amd64").unwrap(), "x86_64");
    }

    #[test]
    fn normalize_arch_rejects_unknown() {
        let err = normalize_arch("mips").unwrap_err().to_string();
        assert!(err.contains("不支持的架构"), "got: {err}");
    }

    #[test]
    fn file_takes_priority() {
        let cli = Cli {
            file: Some(PathBuf::from("/tmp/m.yml")),
            url: Some("https://example.com/m".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_source(&cli).unwrap(),
            Source::LocalFile(PathBuf::from("/tmp/m.yml"))
        );
    }

    #[test]
    fn url_must_be_http_or_https() {
        let cli = Cli { url: Some("ftp://example.com".into()), ..Default::default() };
        let err = resolve_source(&cli).unwrap_err().to_string();
        assert!(err.contains("--url 仅支持 HTTP/HTTPS 地址"), "got: {err}");

        let cli = Cli { url: Some("http://example.com/m".into()), ..Default::default() };
        assert_eq!(
            resolve_source(&cli).unwrap(),
            Source::Remote("http://example.com/m".into())
        );
    }

    #[test]
    fn default_target_mac_uses_channel_preview_and_local_arch() {
        let cli = Cli::default();
        match resolve_source(&cli).unwrap() {
            Source::Remote(url) => {
                let arch = normalize_arch(std::env::consts::ARCH).unwrap();
                assert_eq!(
                    url,
                    format!("https://zcode.z.ai/api/v1/releases/electron/manifest?platform=darwin-{arch}&channel=3")
                );
            }
            other => panic!("期望 Remote，得到 {other:?}"),
        }
    }

    #[test]
    fn windows_defaults_x86_64_and_stable_is_1() {
        let cli = Cli {
            target: Some("win".into()),
            channel: Some("stable".into()),
            ..Default::default()
        };
        match resolve_source(&cli).unwrap() {
            Source::Remote(url) => assert_eq!(
                url,
                "https://zcode.z.ai/api/v1/releases/electron/manifest?platform=windows-x86_64&channel=1"
            ),
            other => panic!("期望 Remote，得到 {other:?}"),
        }
    }

    #[test]
    fn explicit_arch_overrides_default() {
        let cli = Cli {
            target: Some("windows".into()),
            arch: Some("arm64".into()),
            ..Default::default()
        };
        match resolve_source(&cli).unwrap() {
            Source::Remote(url) => assert!(url.contains("platform=windows-aarch64"), "got: {url}"),
            other => panic!("期望 Remote，得到 {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_target_and_channel() {
        let cli = Cli { target: Some("linux".into()), ..Default::default() };
        assert!(resolve_source(&cli).unwrap_err().to_string().contains("不支持的目标"));

        let cli = Cli { channel: Some("beta".into()), ..Default::default() };
        assert!(resolve_source(&cli).unwrap_err().to_string().contains("不支持的更新通道"));
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test cli:: -v`
Expected: FAIL（`Cli`/`Source`/函数未定义，编译错误）

- [ ] **Step 3: 实现 cli.rs**

```rust
//! 命令行参数定义与清单来源解析。

use anyhow::{bail, Result};
use clap::Parser;
use std::path::PathBuf;

/// 默认更新清单地址（channel: preview=3, stable=1）。
const DEFAULT_ENDPOINT: &str = "https://zcode.z.ai/api/v1/releases/electron/manifest";

/// 查询并解析 ZCode Electron 更新清单。
#[derive(Parser, Debug, Default)]
#[command(name = "zcode-update", version, about = "查询并解析 ZCode Electron 更新清单")]
pub struct Cli {
    /// 查询目标：mac（默认）或 windows
    #[arg(long)]
    pub target: Option<String>,

    /// 目标架构：arm64、aarch64、x64、x86_64
    #[arg(long)]
    pub arch: Option<String>,

    /// 更新通道：preview（默认）或 stable
    #[arg(long)]
    pub channel: Option<String>,

    /// 解析本地 YAML Manifest，不请求网络（与 --url 互斥）
    #[arg(long, conflicts_with = "url")]
    pub file: Option<PathBuf>,

    /// 解析指定的 HTTP/HTTPS Manifest（与 --file 互斥）
    #[arg(long)]
    pub url: Option<String>,

    /// 本地应用路径（macOS 为 .app 目录；Windows 为 exe；Linux 影响安装判定）
    #[arg(long)]
    pub app: Option<PathBuf>,
}

/// 清单来源：本地文件或远程 URL。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    LocalFile(PathBuf),
    Remote(String),
}

/// 架构别名归一化：统一为 aarch64 / x86_64。
pub fn normalize_arch(input: &str) -> Result<&'static str> {
    match input {
        "arm64" | "aarch64" => Ok("aarch64"),
        "x64" | "x86_64" | "amd64" => Ok("x86_64"),
        other => bail!("不支持的架构：{other}"),
    }
}

/// channel 名称 → 服务端 channel id。
fn channel_id(channel: &str) -> Result<&'static str> {
    match channel {
        "preview" => Ok("3"),
        "stable" => Ok("1"),
        other => bail!("不支持的更新通道：{other}"),
    }
}

/// 依据 CLI 参数决定清单来源（优先级：--file > --url > 默认 endpoint）。
pub fn resolve_source(cli: &Cli) -> Result<Source> {
    // 1. 本地文件最高优先级（--file/--url 互斥由 clap 保证，此处兜底防御）
    if let Some(file) = &cli.file {
        return Ok(Source::LocalFile(file.clone()));
    }

    // 2. 显式远程 URL
    if let Some(url) = &cli.url {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            bail!("--url 仅支持 HTTP/HTTPS 地址");
        }
        return Ok(Source::Remote(url.clone()));
    }

    // 3. 默认 endpoint：先确定 target（无效立即报错），再决定 arch 缺省值
    let platform = match cli.target.as_deref().unwrap_or("mac") {
        "mac" | "macos" | "darwin" => {
            let arch = match &cli.arch {
                Some(a) => normalize_arch(a)?,
                // macOS 缺省取当前运行架构
                None => normalize_arch(std::env::consts::ARCH)?,
            };
            format!("darwin-{arch}")
        }
        "windows" | "win" => {
            let arch = match &cli.arch {
                Some(a) => normalize_arch(a)?,
                None => "x86_64",
            };
            format!("windows-{arch}")
        }
        other => bail!("不支持的目标：{other}"),
    };

    let id = channel_id(cli.channel.as_deref().unwrap_or("preview"))?;
    Ok(Source::Remote(format!("{DEFAULT_ENDPOINT}?platform={platform}&channel={id}")))
}

#[cfg(test)]
mod tests {
    // Step 1 的测试代码粘贴于此
}
```

- [ ] **Step 4: 运行确认全部通过**

Run: `cargo test cli:: -v`
Expected: 全部 PASS（8 个测试）

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: CLI 参数定义与清单来源解析"
```

---

### Task 3: 输出纯函数（output.rs 第一部分）

**Files:**
- Modify: `src/output.rs`
- Test: `src/output.rs`（文件内 tests）

**Interfaces:**
- Produces:
  - `pub fn strip_v(version: &str) -> &str`
  - `pub fn classify_url(url: &str) -> &'static str`（DMG/ZIP/EXE/FILE）
  - `pub fn format_release_date_in(raw: &str, tz: &jiff::tz::TimeZone) -> Option<String>`（供测试注入时区）
  - `pub fn format_release_date(raw: &str) -> String`（系统时区，解析失败原样返回）

- [ ] **Step 1: 写失败测试**

追加到 `src/output.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use jiff::tz::TimeZone;

    #[test]
    fn strip_v_removes_single_v_or_v_capital_prefix() {
        assert_eq!(strip_v("v1.2.3"), "1.2.3");
        assert_eq!(strip_v("V1.2.3"), "1.2.3");
        assert_eq!(strip_v("1.2.3"), "1.2.3");
        assert_eq!(strip_v(""), "");
        // 与 shell 行为一致：最多剥一层 v，再剥一层 V
        assert_eq!(strip_v("vv1.2"), "v1.2");
    }

    #[test]
    fn classify_urls_by_extension_anywhere_in_url() {
        assert_eq!(classify_url("https://x/a.dmg"), "DMG");
        assert_eq!(classify_url("https://x/a.zip?sig=1"), "ZIP");
        assert_eq!(classify_url("https://x/ZCode.exe"), "EXE");
        assert_eq!(classify_url("https://x/zcode-1.2.3.tar.gz"), "FILE");
    }

    #[test]
    fn iso8601_formats_in_fixed_utc() {
        let utc = TimeZone::UTC;
        assert_eq!(
            format_release_date_in("2026-07-31T13:02:56.736Z", &utc).unwrap(),
            "2026-07-31 13:02:56"
        );
        assert_eq!(
            format_release_date_in("2026-07-31T15:02:56+02:00", &utc).unwrap(),
            "2026-07-31 13:02:56"
        );
    }

    #[test]
    fn non_iso_input_passes_through() {
        let utc = TimeZone::UTC;
        assert_eq!(format_release_date_in("2026-07-31", &utc), None);
        assert_eq!(format_release_date_in("", &utc), None);
    }

    #[test]
    fn system_fallback_keeps_raw_on_error() {
        assert_eq!(format_release_date("不是日期"), "不是日期");
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test output:: -v`
Expected: FAIL（函数未定义）

- [ ] **Step 3: 实现**

`src/output.rs` 全量替换为：

```rust
//! 结果展示与文案渲染。

use jiff::Timestamp;

/// 与 shell 一致：最多剥一个 `v` 前缀，随后最多再剥一个 `V` 前缀。
pub fn strip_v(version: &str) -> &str {
    let s = version.strip_prefix('v').unwrap_or(version);
    s.strip_prefix('V').unwrap_or(s)
}

/// 按链接中的扩展名分类展示标签（子串匹配，与 shell case 模式一致）。
pub fn classify_url(url: &str) -> &'static str {
    if url.contains(".dmg") {
        "DMG"
    } else if url.contains(".zip") {
        "ZIP"
    } else if url.contains(".exe") {
        "EXE"
    } else {
        "FILE"
    }
}

/// ISO 8601 时间戳 → 指定时区的 `%Y-%m-%d %H:%M:%S`；
/// 无法解析为时间戳（含纯日期、空串等）返回 None。
pub fn format_release_date_in(raw: &str, tz: &jiff::tz::TimeZone) -> Option<String> {
    let ts: Timestamp = raw.parse().ok()?;
    let zoned = ts.to_zoned(tz.to_owned());
    Some(zoned.strftime("%Y-%m-%d %H:%M:%S").to_string())
}

/// 使用系统本地时区格式化；解析失败时原样返回输入。
pub fn format_release_date(raw: &str) -> String {
    format_release_date_in(raw, &jiff::tz::TimeZone::system())
        .unwrap_or_else(|| raw.to_string())
}

#[cfg(test)]
mod tests {
    // Step 1 的测试代码粘贴于此
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test output:: -v`
Expected: 全部 PASS（5 个测试）

- [ ] **Step 5: Commit**

```bash
git add src/output.rs
git commit -m "feat: 版本剥离/链接分类/日期格式化纯函数"
```

---

### Task 4: 清单解析与下载（manifest.rs）

**Files:**
- Modify: `src/manifest.rs`
- Test: `src/manifest.rs`（文件内 tests）

**Interfaces:**
- Consumes: `crate::cli::Source`
- Produces:
  - `pub struct Manifest { pub version: String, pub release_name: Option<String>, pub release_date: Option<String>, pub urls: Vec<String>, pub release_notes: Option<String> }`
  - `impl Manifest { pub fn from_yaml(text: &str) -> anyhow::Result<Manifest> }`
  - `pub fn load(source: &Source) -> anyhow::Result<(Manifest, String)>`（返回 (清单, 来源标签)）
  - `pub(crate) fn fetch(url: &str) -> anyhow::Result<String>`（ureq 实现）

- [ ] **Step 1: 写失败测试**

追加到 `src/manifest.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
version: 1.2.3
releaseName: "ZCode Preview"
releaseDate: 2026-07-31T13:02:56.736Z
releaseNotes: |
  ## What's New

  - 修复若干问题
files:
  - url: https://example.com/zcode.dmg
  - url: https://example.com/zcode.exe
  - url: https://example.com/zcode.dmg
"#;

    #[test]
    fn parses_fields_and_dedups_urls_in_order() {
        let m = Manifest::from_yaml(SAMPLE).unwrap();
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.release_name.as_deref(), Some("ZCode Preview"));
        assert_eq!(m.release_date.as_deref(), Some("2026-07-31T13:02:56.736Z"));
        assert_eq!(m.urls, vec![
            "https://example.com/zcode.dmg".to_string(),
            "https://example.com/zcode.exe".to_string(),
        ]);
        assert!(m.release_notes.as_deref().unwrap().contains("## What's New"));
    }

    #[test]
    fn falls_back_to_top_level_path_when_no_files() {
        let yaml = "version: 2.0.0\npath: https://example.com/pkg.zip";
        let m = Manifest::from_yaml(yaml).unwrap();
        assert_eq!(m.urls, vec!["https://example.com/pkg.zip".to_string()]);
    }

    #[test]
    fn missing_version_is_rejected_in_chinese() {
        let yaml = "files:\n  - url: https://example.com/a";
        let err = Manifest::from_yaml(yaml).unwrap_err().to_string();
        assert!(err.contains("Manifest 中缺少 version"), "got: {err}");
    }

    #[test]
    fn missing_urls_and_path_is_rejected() {
        let yaml = "version: 1.0.0";
        let err = Manifest::from_yaml(yaml).unwrap_err().to_string();
        assert!(err.contains("Manifest 中缺少下载链接"), "got: {err}");
    }

    #[test]
    fn quoted_values_are_unquoted_by_yaml_spec() {
        let yaml = "version: \"9.9.9\"";
        assert_eq!(Manifest::from_yaml(yaml).unwrap().version, "9.9.9");
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test manifest:: -v`
Expected: FAIL（类型/函数未定义）

- [ ] **Step 3: 实现**

`src/manifest.rs` 全量替换为：

```rust
//! 清单下载与 YAML 解析。

use crate::cli::Source;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

/// files 数组元素，仅关心 url。
#[derive(Debug, Deserialize)]
struct FileEntry {
    url: String,
}

/// 服务端原始清单结构，字段均可缺失。
#[derive(Debug, Deserialize)]
struct RawManifest {
    #[serde(rename = "releaseName")]
    release_name: Option<String>,
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
    version: Option<String>,
    files: Option<Vec<FileEntry>>,
    /// files 为空时的顶层回退下载地址。
    path: Option<String>,
    #[serde(rename = "releaseNotes")]
    release_notes: Option<String>,
}

/// 解析后的有效清单。
#[derive(Debug)]
pub struct Manifest {
    pub version: String,
    pub release_name: Option<String>,
    pub release_date: Option<String>,
    /// 去重保序后的下载链接；files 缺失时回退顶层 path。
    pub urls: Vec<String>,
    pub release_notes: Option<String>,
}

impl Manifest {
    /// 从 YAML 文本解析；version 或下载链接缺失时报错（文案对齐 shell 的 die）。
    pub fn from_yaml(text: &str) -> Result<Manifest> {
        let raw: RawManifest =
            serde_yaml_ng::from_str(text).context("解析 Manifest YAML 失败")?;

        let version = raw
            .version
            .filter(|v| !v.trim().is_empty())
            .context("Manifest 中缺少 version")?;

        // files[].url 去重保序
        let mut urls: Vec<String> = Vec::new();
        for entry in raw.files.into_iter().flatten() {
            if !entry.url.is_empty() && !urls.contains(&entry.url) {
                urls.push(entry.url);
            }
        }
        // 回退：顶层 path 作为唯一下载链接（对齐 awk END 逻辑）
        if urls.is_empty() {
            if let Some(path) = raw.path.filter(|p| !p.trim().is_empty()) {
                urls.push(path);
            }
        }
        if urls.is_empty() {
            bail!("Manifest 中缺少下载链接");
        }

        Ok(Manifest {
            version,
            release_name: raw.release_name,
            release_date: raw.release_date,
            urls,
            release_notes: raw.release_notes,
        })
    }
}

/// 加载清单：读文件或发起请求；返回 (清单, 用于展示的来源标签)。
pub fn load(source: &Source) -> Result<(Manifest, String)> {
    let text = match source {
        Source::LocalFile(path) => std::fs::read_to_string(path)
            .with_context(|| format!("无法读取文件：{}", path.display()))?,
        Source::Remote(url) => fetch(url)?,
    };
    let label = match source {
        Source::LocalFile(path) => path.display().to_string(),
        Source::Remote(url) => url.clone(),
    };
    Ok((Manifest::from_yaml(&text)?, label))
}

/// 下载清单文本。超时/头部与原 curl 参数对齐；UA 保持不变以便服务端统计口径一致。
pub(crate) fn fetch(url: &str) -> Result<String> {
    use std::time::Duration;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .timeout_connect(Some(Duration::from_secs(10)))
        .user_agent("zcode-preview-update.sh")
        .build()
        .into();

    let response = agent
        .get(url)
        .header("Accept", "application/x-yaml,text/yaml,text/plain,*/*")
        .call()
        .map_err(|e| anyhow!("下载 Manifest 失败：{e}"))?;

    response
        .body_mut()
        .read_to_string()
        .context("读取 Manifest 响应失败")
}

#[cfg(test)]
mod tests {
    // Step 1 的测试代码粘贴于此
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test manifest:: -v`
Expected: 全部 PASS（5 个测试）

- [ ] **Step 5: Commit**

```bash
git add src/manifest.rs
git commit -m "feat: 清单 YAML 解析与 ureq 下载封装"
```

---

### Task 5: 三平台本地版本探测（local.rs）

**Files:**
- Modify: `src/local.rs`
- Test: `src/local.rs`（文件内 tests；跨平台纯函数测试 + macOS 专属 plist 测试）

**Interfaces:**
- Consumes: `crate::manifest::fetch` 无关；仅标准库 + plist/windows-sys
- Produces:
  - `pub struct LocalApp { pub installed: bool, pub version: Option<String> }`（derive Debug, PartialEq, Eq）
  - `pub fn detect(override_path: Option<&std::path::Path>) -> LocalApp`
  - 内部可测纯函数（不限 pub）：`fn read_plist_version(path: &Path) -> Option<String>`、`fn version_from_squirrel_dir_name(name: &str) -> Option<String>`、`fn squirrel_pick_latest(dir: &Path) -> Option<String>`（cfg windows）、`fn version_from_dpkg_output(text: &str) -> Option<String>`、`fn version_from_rpm_output(text: &str) -> Option<String>`、`fn linux_dirs_installed() -> bool`

- [ ] **Step 1: 写失败测试**

追加到 `src/local.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Squirrel 目录名解析（Windows 逻辑，但纯函数全平台可测） ----------

    #[test]
    fn squirrel_dir_name_yields_version() {
        assert_eq!(version_from_squirrel_dir_name("app-1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(version_from_squirrel_dir_name("app-10.0.1"), Some("10.0.1".to_string()));
        assert_eq!(version_from_squirrel_dir_name("app-beta"), None);
        assert_eq!(version_from_squirrel_dir_name("packages"), None);
    }

    #[test]
    fn squirrel_numeric_ordering_prefers_highest() {
        let dir = std::env::temp_dir().join(format!("zc-sq-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for name in ["app-1.10.0", "app-1.9.0", "app-1.2.3"] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
        }
        assert_eq!(squirrel_pick_latest(&dir), Some("1.10.0".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------- dpkg / rpm 输出解析（Linux 逻辑，纯函数全平台可测） ----------

    #[test]
    fn dpkg_status_output_parses_version_line() {
        let text = "Package: zcode\nStatus: install ok installed\nVersion: 1.2.3\n";
        assert_eq!(version_from_dpkg_output(text), Some("1.2.3".to_string()));
        assert_eq!(version_from_dpkg_output("no version here"), None);
    }

    #[test]
    fn rpm_query_output_parses_version_field() {
        assert_eq!(version_from_rpm_output("zcode-1.2.3-1.x86_64\n"), Some("1.2.3".to_string()));
        assert_eq!(version_from_rpm_output("package zcode is not installed\n"), None);
    }

    // ---------- macOS Info.plist 解析（fixture 文件，全平台可跑） ----------

    #[test]
    fn plist_fixture_version_is_read() {
        let mut tmp = tempfile::Builder::new().suffix(".plist").tempfile().unwrap();
        use std::io::Write as _;
        write!(
            tmp.as_file_mut(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\
             <plist version=\"1.0\"><dict>\
             <key>CFBundleShortVersionString</key><string>3.2.1</string>\
             </dict></plist>"
        )
        .unwrap();
        assert_eq!(
            read_plist_version(tmp.path()),
            Some("3.2.1".to_string())
        );
        assert_eq!(read_plist_version(Path::new("/nonexistent/info.plist")), None);
    }

    #[test]
    fn detect_missing_override_reports_not_installed() {
        // 传入必然不存在的路径：任何平台上都应报告未安装
        let app = detect(Some(Path::new("/nonexistent-zcode-path/ZCode")));
        assert_eq!(app, LocalApp { installed: false, version: None });
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test local:: -v`
Expected: FAIL（函数未定义）

- [ ] **Step 3: 实现**

`src/local.rs` 全量替换为：

```rust
//! 本地已安装 ZCode 版本探测（按运行平台分派）。

use std::path::{Path, PathBuf};

/// 本地应用状态：是否安装、可读到的版本号。
#[derive(Debug, PartialEq, Eq)]
pub struct LocalApp {
    pub installed: bool,
    pub version: Option<String>,
}

/// 探测入口。`override_path` 来自 --app：
/// - macOS：视作 .app 目录
/// - Windows：视作 exe 文件（读取其版本资源）
/// - Linux：仅影响“是否安装”的判定
pub fn detect(override_path: Option<&Path>) -> LocalApp {
    #[cfg(target_os = "macos")]
    {
        detect_macos(override_path.unwrap_or_else(|| Path::new("/Applications/ZCode.app")))
    }
    #[cfg(target_os = "windows")]
    {
        detect_windows(override_path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        detect_linux(override_path)
    }
}

// ===================== macOS =====================

#[cfg(target_os = "macos")]
fn detect_macos(app_dir: &Path) -> LocalApp {
    if !app_dir.is_dir() {
        return LocalApp { installed: false, version: None };
    }
    let info_plist = app_dir.join("Contents").join("Info.plist");
    LocalApp { installed: true, version: read_plist_version(&info_plist) }
}

/// 读取 Info.plist 的 CFBundleShortVersionString（XML/Binary 均支持）。
/// 不限平台，方便用 fixture 测试。
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn read_plist_version(plist_path: &Path) -> Option<String> {
    use std::io::Cursor;
    let bytes = std::fs::read(plist_path).ok()?;
    let value = plist::Value::from_reader(Cursor::new(bytes)).ok()?;
    let dict = value.as_dictionary()?;
    dict.get("CFBundleShortVersionString")?
        .as_string()
        .map(str::to_string)
}

// ===================== Windows =====================

#[cfg(target_os = "windows")]
fn detect_windows(override_path: Option<&Path>) -> LocalApp {
    // 显式指定路径：直接读该 exe 的版本资源
    if let Some(exe) = override_path {
        return LocalApp {
            installed: exe.is_file(),
            version: read_exe_version(exe).ok(),
        };
    }

    // electron-builder NSIS 默认 per-user 安装路径
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let base = PathBuf::from(local);
        let nsis_exe = base.join("Programs").join("ZCode").join("ZCode.exe");
        if nsis_exe.is_file() {
            return LocalApp { installed: true, version: read_exe_version(&nsis_exe).ok() };
        }
        // Squirrel 安装形态：%LOCALAPPDATA%\zcode\app-<version>\ZCode.exe
        let squirrel_base = base.join("zcode");
        if squirrel_base.is_dir() {
            if let Some(version) = squirrel_pick_latest(&squirrel_base) {
                return LocalApp { installed: true, version: Some(version) };
            }
        }
    }
    LocalApp { installed: false, version: None }
}

/// 从 Squirrel 目录名 `app-<version>` 提取版本号；非匹配目录返回 None。
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn version_from_squirrel_dir_name(name: &str) -> Option<String> {
    let rest = name.strip_prefix("app-")?;
    let first = rest.chars().next()?;
    if first.is_ascii_digit() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') && !rest.is_empty() {
        Some(rest.to_string())
    } else {
        None
    }
}

/// 扫描 Squirrel 父目录，返回数值序最大的 app-<version> 目录版本号。
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn squirrel_pick_latest(base: &Path) -> Option<String> {
    let mut best: Option<((u64, u64, u64, u64), String)> = None;
    for entry in std::fs::read_dir(base).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(ver) = version_from_squirrel_dir_name(&name) {
            let key = version_sort_key(&ver);
            if best.as_ref().map(|(k, _)| key > *k).unwrap_or(true) {
                best = Some((key, ver));
            }
        }
    }
    best.map(|(_, v)| v)
}

/// "1.2.3" → (1,2,3,0)，用于数值比较排序。
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn version_sort_key(v: &str) -> (u64, u64, u64, u64) {
    let mut parts = [0u64; 4];
    for (i, seg) in v.split('.').take(4).enumerate() {
        parts[i] = seg.parse().unwrap_or(0);
    }
    (parts[0], parts[1], parts[2], parts[3])
}

/// 读取 Windows PE 的 VERSIONINFO 资源：优先 ProductVersion 字符串，
/// 兜底 VS_FIXEDFILEINFO 数值（裁剪尾随多余的 .0，避免与 semver 远程版本不一致）。
#[cfg(target_os = "windows")]
fn read_exe_version(exe: &Path) -> Result<String, anyhow::Error> {
    use anyhow::bail;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let wide = to_wide(&exe.to_string_lossy());
    // SAFETY：全程使用 Win32 版本 API 的合法用法；指针仅在调用期间有效。
    unsafe {
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), std::ptr::null_mut());
        if size == 0 {
            bail!("GetFileVersionInfoSizeW 失败（可能无版本资源）：{}", exe.display());
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, data.as_mut_ptr() as *mut _) == 0 {
            bail!("GetFileVersionInfoW 失败");
        }

        // 主路径：StringFileInfo\<lang>\<codepage>\ProductVersion
        let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let trans_key = to_wide("\\VarFileInfo\\Translation");
        if VerQueryValueW(data.as_ptr() as *const _, trans_key.as_ptr(), &mut ptr, &mut len) != 0
            && !ptr.is_null()
            && len >= 4
        {
            let words = std::slice::from_raw_parts(ptr as *const u16, (len as usize) / 2);
            if words.len() >= 2 {
                let pv_key = to_wide(&format!(
                    "\\StringFileInfo\\{:04x}{:04x}\\ProductVersion",
                    words[0], words[1]
                ));
                let mut sptr: *mut std::ffi::c_void = std::ptr::null_mut();
                let mut slen = 0u32;
                if VerQueryValueW(data.as_ptr() as *const _, pv_key.as_ptr(), &mut sptr, &mut slen) != 0
                    && !sptr.is_null()
                    && slen > 0
                {
                    let wstr = std::slice::from_raw_parts(sptr as *const u16, slen as usize);
                    let end = wstr.iter().position(|&c| c == 0).unwrap_or(wstr.len());
                    let s = String::from_utf16_lossy(&wstr[..end]);
                    if !s.trim().is_empty() {
                        return Ok(s.trim().to_string());
                    }
                }
            }
        }

        // 兜底：根块 VS_FIXEDFILEINFO 数值版本
        let root = to_wide("\\");
        if VerQueryValueW(data.as_ptr() as *const _, root.as_ptr(), &mut ptr, &mut len) == 0
            || ptr.is_null()
        {
            bail!("VerQueryValueW 查询根块失败");
        }
        let fi = &*(ptr as *const VS_FIXEDFILEINFO);
        let parts = [
            (fi.dwFileVersionMS >> 16) & 0xffff,
            fi.dwFileVersionMS & 0xffff,
            (fi.dwFileVersionLS >> 16) & 0xffff,
            fi.dwFileVersionLS & 0xffff,
        ];
        let mut comps: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
        while comps.len() > 3 && comps.last().map(String::as_str) == Some("0") {
            comps.pop();
        }
        Ok(comps.join("."))
    }
}

// ===================== Linux =====================

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn detect_linux(override_path: Option<&Path>) -> LocalApp {
    let installed = match override_path {
        Some(p) => p.exists(),
        None => linux_dirs_installed(),
    };
    // deb/rpm 包内没有稳定的纯文本版本文件，依赖包管理器查询
    let version = package_manager_version();
    LocalApp { installed, version }
}

/// 常见 Electron 安装目录存在性判定（含 /opt 下大小写前缀扫描）。
#[cfg_attr(any(target_os = "macos", target_os = "windows"), allow(dead_code))]
fn linux_dirs_installed() -> bool {
    const CANDIDATES: [&str; 3] = ["/opt/ZCode", "/opt/zcode", "/usr/lib/zcode"];
    if CANDIDATES.iter().any(|d| Path::new(d).is_dir()) {
        return true;
    }
    // /opt 下大小写不敏感前缀扫描（如 /opt/ZCode-1.2.3）
    ["ZCode", "zcode"].iter().any(|prefix| {
        std::fs::read_dir("/opt").map(|rd| {
            rd.flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with(prefix))
        }).unwrap_or(false)
    })
}

#[cfg_attr(any(target_os = "macos", target_os = "windows"), allow(dead_code))]
fn package_manager_version() -> Option<String> {
    if let Ok(out) = std::process::Command::new("dpkg").args(["-s", "zcode"]).output() {
        if out.status.success() {
            if let Some(v) = version_from_dpkg_output(&String::from_utf8_lossy(&out.stdout)) {
                return Some(v);
            }
        }
    }
    if let Ok(out) = std::process::Command::new("rpm").args(["-q", "zcode"]).output() {
        if out.status.success() {
            if let Some(v) = version_from_rpm_output(&String::from_utf8_lossy(&out.stdout)) {
                return Some(v);
            }
        }
    }
    None
}

/// 从 `dpkg -s` 输出中提取 `Version:` 行的值。
#[cfg_attr(any(target_os = "macos", target_os = "windows"), allow(dead_code))]
fn version_from_dpkg_output(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("Version: "))
        .map(str::trim_end)
        .filter(|v| !v.is_empty())
        .map(String::from)
}

/// 从 `rpm -q zcode` 输出（如 `zcode-1.2.3-1.x86_64`）提取版本段。
#[cfg_attr(any(target_os = "macos", target_os = "windows"), allow(dead_code))]
fn version_from_rpm_output(text: &str) -> Option<String> {
    let line = text.trim();
    let after_name = line.split_once('-')?.1;
    let ver = after_name.split('-').next()?;
    if ver.is_empty() || !ver.chars().next()?.is_ascii_digit() {
        return None;
    }
    Some(ver.to_string())
}
```

- [ ] **Step 4: 运行确认通过（macOS 上额外做一次真实探测冒烟）**

Run: `cargo test local:: -v`
Expected: 全部 PASS（6 个测试）

补充冒烟（仅记录结果，不做断言——本机可能没装 ZCode）：
Run: `cargo test local::tests::detect_missing_override_reports_not_installed -- --nocapture && ls /Applications/ | grep -i zcode || echo "本机未见 ZCode"`
Expected: 测试通过；grep 结果仅供 Task 8 冒烟参考

- [ ] **Step 5: Commit**

```bash
git add src/local.rs
git commit -m "feat: 三平台本地 ZCode 版本探测"
```

---

### Task 6: 渲染输出（output.rs 第二部分）

**Files:**
- Modify: `src/output.rs`
- Test: `src/output.rs`（tests 追加）

**Interfaces:**
- Consumes: `crate::manifest::Manifest`、`crate::local::LocalApp`、本任务前的纯函数
- Produces:
  - `pub fn render_up_to_date(local_version: &str, out: &mut dyn std::io::Write)`
  - `pub fn render(manifest: &Manifest, source_label: &str, local: &LocalApp, out: &mut dyn std::io::Write)`

- [ ] **Step 1: 写失败测试（golden 文本严格对齐 shell printf 序列）**

在 `src/output.rs` 的 `mod tests` 中追加（需要 `use std::io::Write as _;` 引入作用域用于构造 Manifest）：

```rust
    // ---------- 渲染 golden 测试 ----------

    fn sample_manifest() -> crate::manifest::Manifest {
        crate::manifest::Manifest {
            version: "1.2.3".into(),
            release_name: Some("ZCode Preview".into()),
            release_date: Some("2026-07-31T13:02:56.736Z".into()),
            urls: vec!["https://example.com/a.dmg".into(), "https://example.com/b.exe".into()],
            release_notes: Some("- 修复若干问题".into()),
        }
    }

    #[test]
    fn up_to_date_line_matches_shell_copy() {
        let mut buf = Vec::new();
        render_up_to_date("1.2.3", &mut buf);
        assert_eq!(String::from_utf8(buf).unwrap(), "✅ 当前已是最新版本：1.2.3，暂无更新。\n");
    }

    #[test]
    fn full_render_installed_branch_matches_shell_layout() {
        let m = sample_manifest();
        let local = crate::local::LocalApp { installed: true, version: Some("1.2.2".into()) };
        let mut buf = Vec::new();
        render(&m, "https://src.example", &local, &mut buf);
        let text = String::from_utf8(buf).unwrap();
        // 日期经系统时区转换，只断言结构行；日期行单独校验前缀
        assert!(text.starts_with("\nZCode 更新信息\n=============\n🎉 发现新版本！\n本地版本：1.2.2\n最新版本：1.2.3\n名称：ZCode Preview\n日期："), "got:\n{text}");
        assert!(text.contains("来源：https://src.example\n\n下载链接：\n  DMG  https://example.com/a.dmg\n  EXE  https://example.com/b.exe\n\n更新日志：\n- 修复若干问题\n"), "got:\n{text}");
    }

    #[test]
    fn full_render_not_installed_branch_matches_shell_layout() {
        let m = sample_manifest();
        let local = crate::local::LocalApp { installed: false, version: None };
        let mut buf = Vec::new();
        render(&m, "file.yml", &local, &mut buf);
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("ℹ️ 未检测到本地 ZCode 应用，以下为最新版本信息\n提示：如已安装在其它位置，请使用 --app 指定路径\n"), "got:\n{text}");
        assert!(!text.contains("本地版本："));
    }

    #[test]
    fn missing_notes_prints_placeholder() {
        let mut m = sample_manifest();
        m.release_notes = None;
        let local = crate::local::LocalApp { installed: true, version: Some("0.0.1".into()) };
        let mut buf = Vec::new();
        render(&m, "src", &local, &mut buf);
        let text = String::from_utf8(buf).unwrap();
        assert!(text.ends_with("\n更新日志：\n（Manifest 未提供更新日志）\n"), "got:\n{text}");
    }
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test output:: -v`
Expected: FAIL（`render_up_to_date`/`render` 未定义）

- [ ] **Step 3: 实现渲染函数**

在 `src/output.rs` 顶部区域追加（`use` 增加 `std::io::Write` 与 crate 引用）：

```rust
use crate::local::LocalApp;
use crate::manifest::Manifest;
use std::io::Write;
```

函数实现：

```rust
/// “已是最新”分支的单行输出（exit 0，无其他内容）。
pub fn render_up_to_date(local_version: &str, out: &mut dyn Write) {
    let _ = writeln!(out, "✅ 当前已是最新版本：{local_version}，暂无更新。");
}

/// 完整更新信息渲染；布局与 shell printf 序列逐行对齐。
pub fn render(m: &Manifest, source_label: &str, local: &LocalApp, out: &mut dyn Write) {
    let _ = writeln!(out);
    let _ = writeln!(out, "ZCode 更新信息");
    let _ = writeln!(out, "=============");

    if local.installed {
        let _ = writeln!(out, "🎉 发现新版本！");
        let _ = writeln!(out, "本地版本：{}", local.version.as_deref().unwrap_or(""));
    } else {
        let _ = writeln!(out, "ℹ️ 未检测到本地 ZCode 应用，以下为最新版本信息");
        let _ = writeln!(out, "提示：如已安装在其它位置，请使用 --app 指定路径");
    }

    let _ = writeln!(out, "最新版本：{}", m.version);
    if let Some(name) = &m.release_name {
        let _ = writeln!(out, "名称：{name}");
    }
    if let Some(date) = &m.release_date {
        let _ = writeln!(out, "日期：{}", format_release_date(date));
    }
    let _ = writeln!(out, "来源：{source_label}");

    let _ = writeln!(out);
    let _ = writeln!(out, "下载链接：");
    for url in &m.urls {
        let _ = writeln!(out, "  {:<4} {}", classify_url(url), url);
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "更新日志：");
    match &m.release_notes {
        Some(notes) => {
            let _ = writeln!(out, "{notes}");
        }
        None => {
            let _ = writeln!(out, "（Manifest 未提供更新日志）");
        }
    }
}
```

- [ ] **Step 4: 运行确认通过**

Run: `cargo test output:: -v`
Expected: 全部 PASS（新增 4 个，共 9 个）

- [ ] **Step 5: Commit**

```bash
git add src/output.rs
git commit -m "feat: 更新信息渲染（对齐 shell 输出布局）"
```

---

### Task 7: 编排与入口（lib.rs + main.rs）及集成测试

**Files:**
- Modify: `src/lib.rs`, `src/main.rs`
- Test: `tests/end_to_end.rs`

**Interfaces:**
- Consumes: 前四个任务的全部公开接口
- Produces:
  - `pub fn run(cli: Cli, out: &mut dyn Write) -> i32`（错误打印到 stderr 并返回 2）

- [ ] **Step 1: 写失败集成测试**

`tests/end_to_end.rs`：

```rust
//! 端到端集成测试：fixture 清单走 --file 路径，不触网。

use std::path::PathBuf;
use zcode_update::{cli::Cli, run};

/// 生成临时 manifest 文件，返回路径。
fn write_manifest(content: &str) -> PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.yml");
    std::fs::write(&path, content).unwrap();
    // tempdir 离开作用域会删除目录；这里泄漏目录保证文件存活至进程结束（测试进程短命，可接受）
    std::mem::forget(dir);
    path
}

const FRESH_MANIFEST: &str = "\
version: 99.0.0
releaseNotes: |
  - 新特性 A
files:
  - url: https://example.com/zcode-99.0.0.dmg
";

#[test]
fn not_installed_branch_prints_full_info_and_exits_zero() {
    let file = write_manifest(FRESH_MANIFEST);
    let cli = Cli {
        file: Some(file),
        // 必然不存在的本地路径
        app: Some(std::path::PathBuf::from("/nonexistent-zcode-e2e/App")),
        ..Default::default()
    };
    let mut buf = Vec::new();
    let code = run(cli, &mut buf);
    let text = String::from_utf8(buf).unwrap();

    assert_eq!(code, 0);
    assert!(text.contains("ℹ️ 未检测到本地 ZCode 应用"), "got:\n{text}");
    assert!(text.contains("最新版本：99.0.0"));
    assert!(text.contains("DMG  https://example.com/zcode-99.0.0.dmg"));
}

#[test]
fn missing_version_file_errors_with_exit_code_2() {
    let file = write_manifest("releaseName: broken\n");
    let cli = Cli { file: Some(file), ..Default::default() };
    // run() 的错误走 stderr，这里只断言退出码
    let mut buf = Vec::new();
    assert_eq!(run(cli, &mut buf), 2);
}

#[test]
fn unreadable_file_errors_with_exit_code_2() {
    let cli = Cli {
        file: Some(std::path::PathBuf::from("/nonexistent-zcode-e2e/manifest.yml")),
        ..Default::default()
    };
    let mut buf = Vec::new();
    assert_eq!(run(cli, &mut buf), 2);
}

#[test]
fn clap_conflicts_file_and_url() {
    // --file 与 --url 同时给出必须在参数层被拒绝
    let parsed = Cli::try_parse_from([
        "zcode-update", "--file", "a.yml", "--url", "https://example.com",
    ]);
    assert!(parsed.is_err(), "--file 与 --url 应互斥");
}

// “已最新”分支依赖本地探测读 Info.plist，仅 macOS 端到端验证
#[cfg(target_os = "macos")]
#[test]
fn up_to_date_branch_exits_zero_on_macos() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("Fake.app").join("Contents");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(
        app.join("Info.plist"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <plist version=\"1.0\"><dict>\
         <key>CFBundleShortVersionString</key><string>99.0.0</string>\
         </dict></plist>",
    )
    .unwrap();

    let file = write_manifest(FRESH_MANIFEST);
    let cli = Cli {
        file: Some(file),
        app: Some(dir.path().join("Fake.app")),
        ..Default::default()
    };
    let mut buf = Vec::new();
    let code = run(cli, &mut buf);
    assert_eq!(code, 0);
    // 退出码为准（文案已由 render 的 golden 单测覆盖）
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cargo test --test end_to_end -v`
Expected: FAIL（`run` 未定义，编译错误）

- [ ] **Step 3: 实现 lib.rs 编排与 main.rs 入口**

`src/lib.rs` 全量替换为：

```rust
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
```

`src/main.rs` 全量替换为：

```rust
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
```

- [ ] **Step 4: 运行全部测试**

Run: `cargo test -v`
Expected: 全部 PASS（单元 + 集成；macOS 上含 up_to_date 用例）

- [ ] **Step 5: 手动冒烟（与 shell 对照）**

```bash
cargo build --release
./target/release/zcode-update --help
./target/release/zcode-update --target linux ; echo "exit=$?"
./zcode-preview-update.sh --target linux ; echo "exit=$?"
```

Expected: `--help` 显示中文帮助；两个脚本的错误文案同为"不支持的目标：linux"、退出码同为 2（Rust 版多一行 `错误：usage...` 英文详情属已知偏差）。

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/main.rs tests/
git commit -m "feat: 流程编排与 CLI 入口，附端到端测试"
```

---

### Task 8: README

**Files:**
- Create: `README.md`

**Interfaces:** 无代码接口。

- [ ] **Step 1: 编写 README.md**

```markdown
# zcode-update

查询并解析 ZCode Electron 更新清单的命令行工具（`zcode-preview-update.sh` 的 Rust 重写版），
不会自动下载或安装更新包。

## 特性

- 与 shell 版完全一致的查询能力：目标平台 / 架构 / 更新通道 / 本地文件 / 自定义 URL
- 自动检测本地已安装版本（macOS / Windows / Linux），版本一致时静默提示"已最新"
- 单一静态二进制，无外部运行时依赖（TLS 内置 rustls）

## 构建与安装

​```bash
cargo build --release
# 产物：target/release/zcode-update
​```

## 用法

​```text
用法：
  zcode-update
  zcode-update --target mac|windows [--arch arm64|x64]
  zcode-update --channel preview|stable
  zcode-update --file /path/to/manifest.yml
  zcode-update --url https://example.com/manifest

选项：
  --target   查询目标，默认 mac；windows 默认使用 x64
  --arch     目标架构：arm64、aarch64、x64、x86_64
  --channel  更新通道：preview（默认）或 stable
  --file     解析本地 ZCode YAML Manifest，不请求网络（与 --url 互斥）
  --url      解析指定的 HTTP/HTTPS Manifest（与 --file 互斥）
  --app      本地应用路径（见下方各平台说明）
  -h, --help 显示帮助
​```

示例：

​```bash
zcode-update                       # 查询本机架构的 mac preview 最新版
zcode-update --target windows      # 查询 windows x64
zcode-update --file ./manifest.yml # 离线解析本地清单
​```

## 本地版本检测

| 平台 | 默认检测路径 | 版本来源 |
|---|---|---|
| macOS | `/Applications/ZCode.app` | `Contents/Info.plist` 的 `CFBundleShortVersionString` |
| Windows | `%LOCALAPPDATA%\Programs\ZCode\ZCode.exe`，兜底 `%LOCALAPPDATA%\zcode\app-*` | exe 版本资源（ProductVersion）；Squirrel 目录名 |
| Linux | `/opt/ZCode*`、`/opt/zcode*`、`/usr/lib/zcode` | `dpkg -s zcode` / `rpm -q zcode` |

`--app` 可覆盖默认路径：macOS 传 `.app` 目录，Windows 传 exe 文件，Linux 仅影响安装判定。

## CI

GitHub Actions 矩阵（macOS arm64 / Ubuntu x86_64 / Windows x86_64）自动测试、构建并上传压缩产物，
见 `.github/workflows/release.yml`。

## 退出码

- `0`：成功（包括"当前已是最新版本"）
- `2`：参数错误、网络失败、清单缺失字段等一切业务错误
```

（注：写入文件时去掉代码块前的零宽占位符 ​，此处为转义展示。）

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: 添加 README（用法与平台检测说明）"
```

---

### Task 9: GitHub Actions 跨平台构建

**Files:**
- Create: `.github/workflows/release.yml`

**Interfaces:** 无代码接口。

- [ ] **Step 1: 编写 workflow**

`.github/workflows/release.yml`：

```yaml
name: release

on:
  push:
    branches: [main]
    tags: ["v*"]
  pull_request:

permissions:
  contents: write

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest        # aarch64
            target: aarch64-apple-darwin
            archive: tar.gz
          - os: ubuntu-latest       # x86_64
            target: x86_64-unknown-linux-gnu
            archive: tar.gz
          - os: windows-latest      # x86_64
            target: x86_64-pc-windows-msvc
            archive: zip
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2

      - name: Test
        run: cargo test --release --verbose

      - name: Build
        run: cargo build --release --target ${{ matrix.target }} --verbose

      - name: Package (unix)
        if: matrix.archive == 'tar.gz'
        shell: bash
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../zcode-update-${{ matrix.target }}.tar.gz zcode-update

      - name: Package (windows)
        if: matrix.archive == 'zip'
        shell: pwsh
        run: |
          Compress-Archive -Path target/${{ matrix.target }}/release/zcode-update.exe `
            -DestinationPath zcode-update-${{ matrix.target }}.zip

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: zcode-update-${{ matrix.target }}
          path: zcode-update-${{ matrix.target }}.*

      - name: Attach to release (tag only)
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: zcode-update-${{ matrix.target }}.*
```

- [ ] **Step 2: 校验 YAML 语法**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML OK')"`
Expected: 输出 `YAML OK`（若本机无 PyYAML 则改为目视检查并记录）

- [ ] **Step 3: 全量回归**

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
若 fmt/clippy 报错：修复后再跑到全绿（预期常见项：多余 return、 needless borrow 等）。

Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: 三平台矩阵构建并上传压缩产物"
```

---

## 任务依赖关系

```
Task 1 ─► Task 2 ─► Task 4 ─┐
        ├─► Task 3 ─► Task 6 ├─► Task 7 ─► Task 8 ─► Task 9
        └─► Task 5 ──────────┘
```

（Task 2/3/5 相互独立，可并行实施。）
