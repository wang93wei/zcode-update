//! 本地已安装 ZCode 版本探测（按运行平台分派）。

use std::path::Path;

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
        return LocalApp {
            installed: false,
            version: None,
        };
    }
    let info_plist = app_dir.join("Contents").join("Info.plist");
    LocalApp {
        installed: true,
        version: read_plist_version(&info_plist),
    }
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
    use std::path::PathBuf;
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
            return LocalApp {
                installed: true,
                version: read_exe_version(&nsis_exe).ok(),
            };
        }
        // Squirrel 安装形态：%LOCALAPPDATA%\zcode\app-<version>\ZCode.exe
        let squirrel_base = base.join("zcode");
        if squirrel_base.is_dir() {
            if let Some(version) = squirrel_pick_latest(&squirrel_base) {
                return LocalApp {
                    installed: true,
                    version: Some(version),
                };
            }
        }
    }
    LocalApp {
        installed: false,
        version: None,
    }
}

/// 从 Squirrel 目录名 `app-<version>` 提取版本号；非匹配目录返回 None。
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn version_from_squirrel_dir_name(name: &str) -> Option<String> {
    let rest = name.strip_prefix("app-")?;
    let first = rest.chars().next()?;
    if first.is_ascii_digit()
        && rest.chars().all(|c| c.is_ascii_digit() || c == '.')
        && !rest.is_empty()
    {
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
            bail!(
                "GetFileVersionInfoSizeW 失败（可能无版本资源）：{}",
                exe.display()
            );
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, data.as_mut_ptr() as *mut _) == 0 {
            bail!("GetFileVersionInfoW 失败");
        }

        // 主路径：StringFileInfo\<lang>\<codepage>\ProductVersion
        let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let trans_key = to_wide("\\VarFileInfo\\Translation");
        if VerQueryValueW(
            data.as_ptr() as *const _,
            trans_key.as_ptr(),
            &mut ptr,
            &mut len,
        ) != 0
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
                if VerQueryValueW(
                    data.as_ptr() as *const _,
                    pv_key.as_ptr(),
                    &mut sptr,
                    &mut slen,
                ) != 0
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
        // SAFETY：VerQueryValueW 成功返回时 ptr 指向缓冲区内的 VS_FIXEDFILEINFO，
        // 缓冲区按 DWORD 分配满足对齐要求；签名不符则拒绝（MSDN 建议的防御检查）。
        let fi_ptr = ptr as *const VS_FIXEDFILEINFO;
        if fi_ptr.read_unaligned().dwSignature != 0xFEEF_04BD {
            bail!("版本资源签名校验失败");
        }
        let fi = &*fi_ptr;
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
        std::fs::read_dir("/opt")
            .map(|rd| {
                rd.flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with(prefix))
            })
            .unwrap_or(false)
    })
}

#[cfg_attr(any(target_os = "macos", target_os = "windows"), allow(dead_code))]
fn package_manager_version() -> Option<String> {
    if let Ok(out) = std::process::Command::new("dpkg")
        .args(["-s", "zcode"])
        .output()
    {
        if out.status.success() {
            if let Some(v) = version_from_dpkg_output(&String::from_utf8_lossy(&out.stdout)) {
                return Some(v);
            }
        }
    }
    if let Ok(out) = std::process::Command::new("rpm")
        .args(["-q", "zcode"])
        .output()
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Squirrel 目录名解析（Windows 逻辑，但纯函数全平台可测） ----------

    #[test]
    fn squirrel_dir_name_yields_version() {
        assert_eq!(
            version_from_squirrel_dir_name("app-1.2.3"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            version_from_squirrel_dir_name("app-10.0.1"),
            Some("10.0.1".to_string())
        );
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
        assert_eq!(
            version_from_rpm_output("zcode-1.2.3-1.x86_64\n"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            version_from_rpm_output("package zcode is not installed\n"),
            None
        );
    }

    // ---------- macOS Info.plist 解析（fixture 文件，全平台可跑） ----------

    #[test]
    fn plist_fixture_version_is_read() {
        let mut tmp = tempfile::Builder::new()
            .suffix(".plist")
            .tempfile()
            .unwrap();
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
        assert_eq!(read_plist_version(tmp.path()), Some("3.2.1".to_string()));
        assert_eq!(
            read_plist_version(Path::new("/nonexistent/info.plist")),
            None
        );
    }

    #[test]
    fn detect_missing_override_reports_not_installed() {
        // 传入必然不存在的路径：任何平台上都应报告未安装
        let app = detect(Some(Path::new("/nonexistent-zcode-path/ZCode")));
        assert_eq!(
            app,
            LocalApp {
                installed: false,
                version: None
            }
        );
    }
}
