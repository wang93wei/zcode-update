//! 指定版本的 CDN 安装包探测；文件存在不代表已进入官方更新清单。

use crate::cli::{resolve_platform, Cli, Source};
use crate::{manifest, output};
use anyhow::{bail, Context, Result};
use std::io::Write;
use std::time::Duration;

fn installer_url(version: &str, platform: &str) -> Result<String> {
    let (directory, suffix) = match platform {
        "darwin-aarch64" => ("macos-arm64", "mac-arm64.dmg"),
        "darwin-x86_64" => ("macos-x64", "mac-x64.dmg"),
        "windows-aarch64" => ("windows-arm64", "win-arm64.exe"),
        "windows-x86_64" => ("windows-x64", "win-x64.exe"),
        "linux-aarch64" => ("linux-arm64", "linux-arm64.AppImage"),
        "linux-x86_64" => ("linux-x64", "linux-x64.AppImage"),
        other => bail!("不支持的探测平台：{other}"),
    };
    Ok(format!(
        "https://cdn-zcode.z.ai/zcode/electron/releases/{version}/{directory}/ZCode-{version}-{suffix}"
    ))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .timeout_connect(Some(Duration::from_secs(10)))
        .http_status_as_error(false)
        .user_agent("zcode-update")
        .build()
        .into()
}

fn exists(agent: &ureq::Agent, url: &str) -> Result<bool> {
    let response = agent
        .head(url)
        .call()
        .with_context(|| format!("探测失败；请求地址：{url}"))?;
    let status = response.status().as_u16();
    if status == 404 {
        return Ok(false);
    }
    if status != 200 {
        bail!("探测失败；请求地址：{url}；HTTP：{status}");
    }
    let headers = response.headers();
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if content_type.contains("text/html")
        || content_type.contains("application/json")
        || headers.get("content-length").and_then(|v| v.to_str().ok()) == Some("0")
    {
        bail!("探测失败：响应不是有效安装包；请求地址：{url}");
    }
    Ok(true)
}

pub(crate) fn run(cli: &Cli, version: &str, source: &Source, out: &mut dyn Write) -> Result<i32> {
    let platform = resolve_platform(
        cli.target.as_deref(),
        cli.arch.as_deref(),
        std::env::consts::OS,
        std::env::consts::ARCH,
    )?;
    let url = installer_url(version, &platform)?;
    let available = exists(&agent(), &url)?;
    let official = match manifest::load(source) {
        Ok((manifest, _)) => {
            writeln!(out, "官方推送版本：{}", manifest.version)?;
            Some(manifest)
        }
        Err(error) => {
            writeln!(out, "官方推送版本：未知（更新清单查询失败：{error:#}）")?;
            None
        }
    };
    writeln!(out, "探测版本：{version}；平台：{platform}")?;
    if !available {
        writeln!(out, "未找到该版本安装包（HTTP 404）；该结果不代表其他平台或其他格式不存在。")?;
        return Ok(0);
    }
    writeln!(out, "安装包链接可用（HTTP 200）：{url}")?;
    match official {
        Some(manifest) if output::strip_v(&manifest.version) == version => {
            writeln!(out, "该版本已进入所选通道的更新清单。")?;
            writeln!(
                out,
                "更新日志：{}",
                manifest.release_notes.as_deref().filter(|v| !v.trim().is_empty()).unwrap_or("未知")
            )?;
        }
        Some(_) => {
            writeln!(out, "该版本不是所选通道当前推送版本；更新日志：未知。")?;
        }
        None => {
            writeln!(out, "无法确认该版本是否进入更新清单；更新日志：未知。")?;
        }
    }
    writeln!(out, "仅验证链接响应，未下载安装包或验证签名。")?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;

    #[test]
    fn installer_paths_cover_all_platforms() {
        for (platform, path) in [
            ("darwin-aarch64", "macos-arm64/ZCode-3.16.1-mac-arm64.dmg"),
            ("darwin-x86_64", "macos-x64/ZCode-3.16.1-mac-x64.dmg"),
            ("windows-aarch64", "windows-arm64/ZCode-3.16.1-win-arm64.exe"),
            ("windows-x86_64", "windows-x64/ZCode-3.16.1-win-x64.exe"),
            ("linux-aarch64", "linux-arm64/ZCode-3.16.1-linux-arm64.AppImage"),
            ("linux-x86_64", "linux-x64/ZCode-3.16.1-linux-x64.AppImage"),
        ] {
            assert_eq!(installer_url("3.16.1", platform).unwrap(), format!("https://cdn-zcode.z.ai/zcode/electron/releases/3.16.1/{path}"));
        }
        assert!(installer_url("3.16.1", "unknown").is_err());
    }

    #[test]
    fn head_distinguishes_missing_errors_and_non_installers_without_reading_body() {
        for (status, content_type, length, expected) in [
            (200, "application/octet-stream", 123, Some(true)),
            (404, "text/html", 123, Some(false)),
            (403, "text/html", 123, None),
            (429, "application/json", 123, None),
            (500, "text/html", 123, None),
            (200, "text/html; charset=utf-8", 123, None),
            (200, "application/json", 123, None),
            (200, "application/octet-stream", 0, None),
            (204, "application/octet-stream", 0, None),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/installer", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                let mut request = Vec::new();
                let mut byte = [0; 1];
                while !request.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                assert!(request.starts_with(b"HEAD /installer HTTP/1.1\r\n"));
                // HEAD 不发送响应正文，即使 Content-Length 非零。
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n").unwrap();
            });
            let test_agent: ureq::Agent = ureq::Agent::config_builder()
                .proxy(None)
                .http_status_as_error(false)
                .timeout_global(Some(Duration::from_secs(10)))
                .build()
                .into();
            let result = exists(&test_agent, &url);
            server.join().unwrap();
            match expected {
                Some(expected) => assert_eq!(result.unwrap(), expected),
                None => assert!(result.is_err(), "HTTP {status}, {content_type}"),
            }
        }
    }
}
