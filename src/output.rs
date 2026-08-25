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
