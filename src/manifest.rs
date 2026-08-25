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
        let raw: RawManifest = serde_yaml_ng::from_str(text).context("解析 Manifest YAML 失败")?;

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

    let mut response = agent
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
        assert_eq!(
            m.urls,
            vec![
                "https://example.com/zcode.dmg".to_string(),
                "https://example.com/zcode.exe".to_string(),
            ]
        );
        assert!(m
            .release_notes
            .as_deref()
            .unwrap()
            .contains("## What's New"));
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
        // 注：需附带 path 以通过下载链接校验；本测试仅关注引号去除
        let yaml = "version: \"9.9.9\"\npath: https://example.com/pkg.zip";
        assert_eq!(Manifest::from_yaml(yaml).unwrap().version, "9.9.9");
    }
}
