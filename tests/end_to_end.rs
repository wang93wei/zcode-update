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
