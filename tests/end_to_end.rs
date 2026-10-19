//! 端到端集成测试：fixture 清单走 --file 路径，不触网。

use clap::Parser as _;
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
    let cli = Cli {
        file: Some(file),
        ..Default::default()
    };
    // run() 的错误走 stderr，这里只断言退出码
    let mut buf = Vec::new();
    assert_eq!(run(cli, &mut buf), 2);
}

#[test]
fn unreadable_file_errors_with_exit_code_2() {
    let cli = Cli {
        file: Some(std::path::PathBuf::from(
            "/nonexistent-zcode-e2e/manifest.yml",
        )),
        ..Default::default()
    };
    let mut buf = Vec::new();
    assert_eq!(run(cli, &mut buf), 2);
}

#[test]
fn clap_conflicts_file_and_url() {
    // --file 与 --url 同时给出必须在参数层被拒绝
    let parsed = Cli::try_parse_from([
        "zcode-update",
        "--file",
        "a.yml",
        "--url",
        "https://example.com",
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

/// 通过真实 HTTP 和 CLI stderr 验证异常响应，不访问外网。
#[test]
fn remote_responses_report_context_and_preserve_success() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::process::Command;

    for (status, content_type, body, expected) in [
        (
            "200 OK",
            "application/json",
            r#"{"code":429,"message":"请求频繁"}"#,
            Some("Manifest 中缺少 version"),
        ),
        (
            "429 Too Many Requests",
            "application/json",
            r#"{"message":"请求频繁"}"#,
            Some("下载 Manifest 失败"),
        ),
        (
            "200 OK",
            "text/html",
            "<html>服务暂不可用</html>",
            Some("解析远程 Manifest 失败"),
        ),
        (
            "200 OK",
            "application/yaml",
            "",
            Some("解析远程 Manifest 失败"),
        ),
        ("200 OK", "application/yaml", FRESH_MANIFEST, None),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/manifest", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        });
        let output = Command::new(env!("CARGO_BIN_EXE_zcode-update"))
            .args(["--url", &url, "--app", "/nonexistent-zcode-e2e/App"])
            .env_remove("HTTP_PROXY")
            .env_remove("HTTPS_PROXY")
            .env_remove("ALL_PROXY")
            .env_remove("http_proxy")
            .env_remove("https_proxy")
            .env_remove("all_proxy")
            .output()
            .unwrap();
        server.join().unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        if let Some(expected) = expected {
            assert_eq!(output.status.code(), Some(2));
            for text in [expected, &url, status, content_type, "响应摘要：", body] {
                assert!(stderr.contains(text), "missing {text:?}: {stderr}");
            }
            assert!(output.stdout.is_empty());
        } else {
            assert!(output.status.success(), "{stderr}");
            assert!(stderr.is_empty());
            assert!(String::from_utf8(output.stdout)
                .unwrap()
                .contains("最新版本：99.0.0"));
        }
    }
}

#[test]
fn probe_version_validates_input_and_manifest_conflicts() {
    for value in ["3.16.1", "v3.16.1", "V3.16.1"] {
        let cli = Cli::try_parse_from(["zcode-update", "--probe-version", value]).unwrap();
        assert_eq!(cli.probe_version.as_deref(), Some("3.16.1"));
    }
    for value in ["", "3.16", "3.16.1.2", "../3.16.1", "3.16.1?x=1", "3.16.1-beta", "03.16.1"] {
        assert!(Cli::try_parse_from(["zcode-update", "--probe-version", value]).is_err());
    }
    for args in [
        vec!["zcode-update", "--probe-version", "3.16.1", "--file", "a.yml"],
        vec!["zcode-update", "--probe-version", "3.16.1", "--url", "https://example.com"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}
