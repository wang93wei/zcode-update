//! 结果展示与文案渲染。

use crate::local::LocalApp;
use crate::manifest::Manifest;
use jiff::Timestamp;
use std::io::Write;

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
    format_release_date_in(raw, &jiff::tz::TimeZone::system()).unwrap_or_else(|| raw.to_string())
}

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
        // 对齐 shell：命令替换会剥掉尾随换行，printf 只补一个
        Some(notes) => {
            let _ = writeln!(out, "{}", notes.trim_end_matches('\n'));
        }
        None => {
            let _ = writeln!(out, "（Manifest 未提供更新日志）");
        }
    }
}

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

    // ---------- 渲染 golden 测试 ----------

    fn sample_manifest() -> crate::manifest::Manifest {
        crate::manifest::Manifest {
            version: "1.2.3".into(),
            release_name: Some("ZCode Preview".into()),
            release_date: Some("2026-07-31T13:02:56.736Z".into()),
            urls: vec![
                "https://example.com/a.dmg".into(),
                "https://example.com/b.exe".into(),
            ],
            release_notes: Some("- 修复若干问题".into()),
        }
    }

    #[test]
    fn up_to_date_line_matches_shell_copy() {
        let mut buf = Vec::new();
        render_up_to_date("1.2.3", &mut buf);
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "✅ 当前已是最新版本：1.2.3，暂无更新。\n"
        );
    }

    #[test]
    fn full_render_installed_branch_matches_shell_layout() {
        let m = sample_manifest();
        let local = crate::local::LocalApp {
            installed: true,
            version: Some("1.2.2".into()),
        };
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
        let local = crate::local::LocalApp {
            installed: false,
            version: None,
        };
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
        let local = crate::local::LocalApp {
            installed: true,
            version: Some("0.0.1".into()),
        };
        let mut buf = Vec::new();
        render(&m, "src", &local, &mut buf);
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.ends_with("\n更新日志：\n（Manifest 未提供更新日志）\n"),
            "got:\n{text}"
        );
    }

    #[test]
    fn block_scalar_notes_trailing_newlines_are_normalized() {
        let mut m = sample_manifest();
        m.release_notes = Some("- 修复若干问题\n\n".into());
        let local = crate::local::LocalApp {
            installed: true,
            version: Some("0.0.1".into()),
        };
        let mut buf = Vec::new();
        render(&m, "src", &local, &mut buf);
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.ends_with("\n更新日志：\n- 修复若干问题\n"),
            "got:\n{text}"
        );
    }
}
