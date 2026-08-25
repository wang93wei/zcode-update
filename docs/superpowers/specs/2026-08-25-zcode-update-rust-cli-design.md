# zcode-update：Bash 脚本转 Rust CLI 设计文档

日期：2026-08-25
状态：已确认（用户批准）

## 1. 背景与目标

将仓库中的 `zcode-preview-update.sh`（查询并解析 ZCode Electron 更新清单）转换为 Rust 二进制可执行文件。

**目标：**
- 功能、CLI 参数、输出文案（含 emoji）、退出码与 `.sh` 保持一致
- 本地已安装版本检测扩展到 macOS / Windows / Linux 三平台
- 提供 GitHub Actions 跨平台构建流水线

**非目标（YAGNI）：**
- 不做自动下载/安装更新（与原脚本一致，仅展示信息）
- 不做 semver 比较（保持字符串精确比较）
- 不做多 crate workspace、不做包管理器分发（cargo install/Homebrew）
- 原 `.sh` 保留共存，不删除

## 2. 已确认的关键决策

| 决策点 | 结论 |
|---|---|
| 依赖策略 | 标准生态库 |
| 本地检测范围 | macOS + Windows + Linux |
| Win/Linux 版本来源 | 按 Electron 约定自动探测 |
| 交付范围 | 项目本体 + GitHub Actions 三平台矩阵构建 |
| 实现方案 | 方案 A：单 binary crate 分层模块 |

## 3. 技术选型

| 关注点 | 选型 | 理由 |
|---|---|---|
| CLI 参数 | clap（derive API） | 与原脚本选项一一对应，自带 help |
| HTTP | ureq 3.x（rustls） | 轻量同步、免 OpenSSL 系统依赖，利于三平台 CI |
| YAML | serde_yaml_ng | 原 serde_yaml 已归档，此为活跃维护 fork |
| macOS 版本 | plist crate | 替代 PlistBuddy 读 Info.plist |
| Windows 版本 | windows-sys（VERSIONINFO API） | 读 exe 文件版本资源 |
| 时间格式化 | jiff | ISO8601 解析 → 本地时区格式化 |
| 错误处理 | anyhow | 统一错误链，中文用户可见文案 |

## 4. 架构

单 binary crate `zcode-update`：

```
src/
├── main.rs      # 编排：参数 → 来源选择 → 下载/读取 → 解析 → 探测 → 比较 → 输出
├── cli.rs       # clap derive 定义
├── manifest.rs  # Manifest 结构体 + 下载(ureq) + YAML 解析
├── local.rs     # 三平台本地版本探测（cfg(target_os) 分支）
└── output.rs    # 展示逻辑（文案/emoji/下载链接分类）
```

数据流：

```
CLI 参数 ──► 来源选择 ──► Manifest 文本 ──► 解析为结构体 ──┐
                                                        ├─► 版本比较 ──► 输出
运行平台探测本地版本 ────────────────────────────────────┘
```

### 来源选择优先级
1. `--file <path>`：读本地文件，不请求网络
2. `--url <url>`：请求指定 HTTP/HTTPS 地址
3. 默认：`https://zcode.z.ai/api/v1/releases/electron/manifest?platform=<PLATFORM>&channel=<CHANNEL_ID>`
   - PLATFORM：`darwin-{arch}` 或 `windows-{arch}`
   - CHANNEL_ID：preview=3，stable=1
   - mac 时 arch 缺省取本机架构；windows 缺省 x86_64

## 5. CLI 接口（与 .sh 对齐）

```
--target <mac|windows>   查询目标，默认 mac；windows 默认 arch=x86_64
--arch <arm64|x64|...>   归一化：arm64/aarch64→aarch64；x64/x86_64/amd64→x86_64
--channel <preview|stable>  默认 preview
--file <path>            本地 manifest，与 --url 互斥
--url <url>              仅支持 http:// https://，与 --file 互斥
--app <path>             本地应用路径覆盖默认值
-h, --help               帮助
```

错误行为：未知参数、缺参数值、互斥冲突、不支持的 target/channel/arch → 中文错误信息 + exit code 2（对齐 `die()`）。

注：`--target` 仅影响远程清单的 platform 参数；本地探测始终按运行平台执行。Linux 运行时可正常探测本地版本，但远程清单仍只有 mac/windows 两类 platform（与原脚本一致）。

## 6. Manifest 解析（manifest.rs）

```rust
struct Manifest {
    version: Option<String>,
    release_name: Option<String>,
    release_date: Option<String>,
    files: Vec<FileEntry>,   // FileEntry { url: String }
    path: Option<String>,    // 顶层回退字段
    release_notes: Option<String>,
}
```

规则（对齐 awk 行为）：
- `files[].url` 去重保序
- `files` 为空/缺失时，回退到顶层 `path` 字段作为唯一下载链接
- `version` 或最终下载链接缺失 → 报错 exit 2
- 引号剥离、块标量（`releaseNotes: |` / `>`）由 YAML 规范原生处理
- `releaseNotes` 为 inline 字符串或缺失均可

## 7. 三平台本地版本探测（local.rs）

返回 `LocalApp { installed: bool, version: Option<String> }`。

| 平台 | 默认路径 | 版本来源 |
|---|---|---|
| macOS | `/Applications/ZCode.app` | `Contents/Info.plist` 的 `CFBundleShortVersionString`（plist crate）；不可读则视为未提供版本 |
| Windows | `%LOCALAPPDATA%\Programs\ZCode\ZCode.exe`；兜底 `%LOCALAPPDATA%\zcode\app-*\ZCode.exe`（Squirrel 目录名含版本） | exe 的 VERSIONINFO 资源（ProductVersion，兜底 FileVersion），windows-sys 实现；Squirrel 目录名可直接提取版本 |
| Linux | `/opt/ZCode*`、`/usr/lib/zcode*`、`/opt/zcode*` 依次探测目录存在性 | 优先 dpkg/rpm 包管理器查询（`dpkg -s zcode` / `rpm -q zcode`）；查不到则仅视为已安装但无版本 |

- `--app` 覆盖默认路径：macOS 视作 .app 目录；Windows/Linux 若指向 exe 则读其 VERSIONINFO
- 任一步骤失败均不 panic，降级为"未安装"或"无版本"

## 8. 版本比较与输出（output.rs）

- 比较逻辑：双方去掉前导 `v`/`V` 后字符串精确相等（与 .sh 一致）
- **已安装且版本相等** → 输出 `✅ 当前已是最新版本：<本地版本>，暂无更新。` exit 0
- **已安装且不等** → `🎉 发现新版本！` + 本地/最新版本等完整信息
- **未安装** → `ℹ️ 未检测到本地 ZCode 应用…` + 提示 `--app` + 完整最新版信息
- 完整信息包含：最新版本、名称（可选）、日期（可选，格式化后）、来源标签、下载链接分类列表、更新日志
- 下载链接分类：`.dmg`→DMG、`.zip`→ZIP、`.exe`→EXE、其他→FILE，左对齐 4 字符宽度
- 更新日志为空时输出 `（Manifest 未提供更新日志）`
- 日期格式化：ISO8601（如 `2026-07-31T13:02:56.736Z`）→ 本地时区 `%Y-%m-%d %H:%M:%S`（jiff）；非 ISO8601 或解析失败原样保留

## 9. 网络（manifest.rs 内薄封装）

对齐原 curl 参数：
- connect timeout 10s、总超时 30s、跟随重定向
- Header：`Accept: application/x-yaml,text/yaml,text/plain,*/*`、`User-Agent: zcode-preview-update.sh`（保持 UA 一致以便服务端统计口径不变）

HTTP 非 2xx → 中文错误 + exit 2。

## 10. 错误处理

- anyhow 统一错误链；面向用户的顶层错误为中文文案，stderr 输出，exit 2
- 所有 IO/解析失败路径均有明确中文错误信息，不出现裸 panic

## 11. 测试策略（TDD）

单元测试：
- arch 归一化、target/channel 校验映射
- YAML 解析：files 去重保序、path 回退、引号剥离、块标量 releaseNotes、缺字段容错
- 版本比较：v/V 前缀、相等/不等、空本地版本
- 链接分类：dmg/zip/exe/其他
- 日期格式化：带毫秒 Z、时区偏移、非法输入原样返回

集成测试：
- fixture manifest 文件通过 `--file` 走端到端，断言完整 stdout（三种分支：最新/有新版/未安装）
- clap 参数校验：`--file`+`--url` 互斥报错

HTTP 下载层做薄封装（传入 url 返回 String），不强造 mock；CI 中不跑真实网络测试。

## 12. CI（GitHub Actions）

`.github/workflows/release.yml`：
- matrix：`macos-latest`(aarch64) / `ubuntu-latest`(x86_64) / `windows-latest`(x86_64)
- 步骤：checkout → dtolnay/rust-toolchain@stable → Swatinem/rust-cache → `cargo test --release` → `cargo build --release` → 打包（tar.gz / zip）→ actions/upload-artifact 上传产物

## 13. 交付物清单

- [ ] `Cargo.toml`（edition 2021，release profile 优化体积）
- [ ] `src/{main,cli,manifest,local,output}.rs`
- [ ] 单元测试 + 集成测试（`tests/`）
- [ ] `README.md`（用法、构建方式、三平台探测说明）
- [ ] `.github/workflows/release.yml`
- [ ] `.gitignore`（target/ 等）
- 原 `zcode-preview-update.sh` 保留不动
