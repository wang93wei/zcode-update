# zcode-update

查询并解析 ZCode Electron 更新清单的命令行工具（`zcode-preview-update.sh` 的 Rust 重写版），
不会自动下载或安装更新包。

## 特性

- 与 shell 版完全一致的查询能力：目标平台 / 架构 / 更新通道 / 本地文件 / 自定义 URL
- 自动检测本地已安装版本（macOS / Windows / Linux），版本一致时静默提示"已最新"
- 单一静态二进制，无外部运行时依赖（TLS 内置 rustls）

## 构建与安装

```bash
cargo build --release
# 产物：target/release/zcode-update
```

## 用法

```text
用法：
  zcode-update
  zcode-update --target mac|windows|linux [--arch arm64|x64]
  zcode-update --channel preview|stable
  zcode-update --file /path/to/manifest.yml
  zcode-update --url https://example.com/manifest

选项：
  --target   查询目标，默认 mac；windows 默认使用 x64，linux 默认取本机架构
  --arch     目标架构：arm64、aarch64、x64、x86_64
  --channel  更新通道：preview（默认）或 stable
  --file     解析本地 ZCode YAML Manifest，不请求网络（与 --url 互斥）
  --url      解析指定的 HTTP/HTTPS Manifest（与 --file 互斥）
  --app      本地应用路径（见下方各平台说明）
  -h, --help 显示帮助
```

示例：

```bash
zcode-update                       # 查询本机架构的 mac preview 最新版
zcode-update --target windows      # 查询 windows x64
zcode-update --file ./manifest.yml # 离线解析本地清单
```

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

## 协议

[GPL-3.0-or-later](LICENSE) © wang93wei
