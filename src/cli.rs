//! 命令行参数定义与清单来源解析。

use anyhow::{bail, Result};
use clap::Parser;
use std::path::PathBuf;

/// 默认更新清单地址（channel: preview=3, stable=1）。
const DEFAULT_ENDPOINT: &str = "https://zcode.z.ai/api/v1/releases/electron/manifest";

/// 查询并解析 ZCode Electron 更新清单。
#[derive(Parser, Debug, Default)]
#[command(
    name = "zcode-update",
    version,
    about = "查询并解析 ZCode Electron 更新清单"
)]
pub struct Cli {
    /// 查询目标：mac（默认）、windows 或 linux
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
        "linux" => {
            let arch = match &cli.arch {
                Some(a) => normalize_arch(a)?,
                // Linux 缺省取当前运行架构
                None => normalize_arch(std::env::consts::ARCH)?,
            };
            format!("linux-{arch}")
        }
        other => bail!("不支持的目标：{other}"),
    };

    let id = channel_id(cli.channel.as_deref().unwrap_or("preview"))?;
    Ok(Source::Remote(format!(
        "{DEFAULT_ENDPOINT}?platform={platform}&channel={id}"
    )))
}

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
        let cli = Cli {
            url: Some("ftp://example.com".into()),
            ..Default::default()
        };
        let err = resolve_source(&cli).unwrap_err().to_string();
        assert!(err.contains("--url 仅支持 HTTP/HTTPS 地址"), "got: {err}");

        let cli = Cli {
            url: Some("http://example.com/m".into()),
            ..Default::default()
        };
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
        let cli = Cli {
            target: Some("freebsd".into()),
            ..Default::default()
        };
        assert!(resolve_source(&cli)
            .unwrap_err()
            .to_string()
            .contains("不支持的目标"));

        let cli = Cli {
            channel: Some("beta".into()),
            ..Default::default()
        };
        assert!(resolve_source(&cli)
            .unwrap_err()
            .to_string()
            .contains("不支持的更新通道"));
    }

    #[test]
    fn linux_defaults_to_native_arch() {
        let cli = Cli {
            target: Some("linux".into()),
            ..Default::default()
        };
        match resolve_source(&cli).unwrap() {
            Source::Remote(url) => {
                let arch = normalize_arch(std::env::consts::ARCH).unwrap();
                assert_eq!(
                    url,
                    format!("https://zcode.z.ai/api/v1/releases/electron/manifest?platform=linux-{arch}&channel=3")
                );
            }
            other => panic!("期望 Remote，得到 {other:?}"),
        }
    }

    #[test]
    fn linux_explicit_arm64_maps_to_aarch64() {
        let cli = Cli {
            target: Some("linux".into()),
            arch: Some("arm64".into()),
            ..Default::default()
        };
        match resolve_source(&cli).unwrap() {
            Source::Remote(url) => {
                assert!(url.contains("platform=linux-aarch64"), "got: {url}");
            }
            other => panic!("期望 Remote，得到 {other:?}"),
        }
    }
}
