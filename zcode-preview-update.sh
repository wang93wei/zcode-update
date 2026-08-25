#!/bin/bash

# 查询并解析 ZCode Electron 更新清单。
# 仅依赖 macOS 自带的 Bash、awk、sed 和 curl，不会自动下载安装包。

set -euo pipefail

readonly DEFAULT_ENDPOINT="https://zcode.z.ai/api/v1/releases/electron/manifest"

TARGET="mac"
ARCH=""
CHANNEL="preview"
MANIFEST_FILE=""
MANIFEST_URL=""
TEMP_FILE=""
LOCAL_APP="/Applications/ZCode.app"

usage() {
  cat <<'EOF'
用法：
  ./zcode-preview-update.sh
  ./zcode-preview-update.sh --target mac|windows [--arch arm64|x64]
  ./zcode-preview-update.sh --channel preview|stable
  ./zcode-preview-update.sh --file /path/to/manifest.yml
  ./zcode-preview-update.sh --url https://example.com/manifest

选项：
  --target   查询目标，默认 mac；windows 默认使用 x64
  --arch     目标架构：arm64、aarch64、x64、x86_64
  --channel  更新通道：preview（默认）或 stable
  --file     解析本地 ZCode YAML Manifest，不请求网络
  --url      解析指定的 HTTP/HTTPS Manifest
  --app      本地应用路径，默认 /Applications/ZCode.app
  -h, --help 显示帮助

示例：
  ./zcode-preview-update.sh
  ./zcode-preview-update.sh --target windows
  ./zcode-preview-update.sh --file ~/Downloads/zcode-manifest.yml
EOF
}

die() {
  printf '错误：%s\n' "$*" >&2
  exit 2
}

cleanup() {
  if [ -n "$TEMP_FILE" ] && [ -f "$TEMP_FILE" ]; then
    rm -f "$TEMP_FILE"
  fi
}

trap cleanup EXIT

require_value() {
  [ "$#" -ge 2 ] || die "$1 缺少参数"
}

normalize_arch() {
  case "$1" in
    arm64|aarch64)
      printf '%s\n' "aarch64"
      ;;
    x64|x86_64|amd64)
      printf '%s\n' "x86_64"
      ;;
    *)
      die "不支持的架构：$1"
      ;;
  esac
}

detect_mac_arch() {
  normalize_arch "$(uname -m)"
}

yaml_value() {
  local key="$1"
  local file="$2"

  awk -v wanted_key="$key" '
    function unquote(value, first, last) {
      first = substr(value, 1, 1)
      last = substr(value, length(value), 1)
      if ((first == "\"" && last == "\"") || (first == "\047" && last == "\047")) {
        return substr(value, 2, length(value) - 2)
      }
      return value
    }

    index($0, wanted_key ":") == 1 {
      value = substr($0, length(wanted_key) + 2)
      sub(/^[[:space:]]*/, "", value)
      print unquote(value)
      exit
    }
  ' "$file"
}

download_urls() {
  local file="$1"

  awk '
    function clean(value, first, last) {
      sub(/^[[:space:]]*/, "", value)
      first = substr(value, 1, 1)
      last = substr(value, length(value), 1)
      if ((first == "\"" && last == "\"") || (first == "\047" && last == "\047")) {
        value = substr(value, 2, length(value) - 2)
      }
      return value
    }

    /^files:[[:space:]]*$/ {
      in_files = 1
      next
    }

    in_files && /^[^[:space:]]/ {
      in_files = 0
    }

    in_files && /^[[:space:]]*-[[:space:]]+url:[[:space:]]*/ {
      value = $0
      sub(/^[[:space:]]*-[[:space:]]+url:[[:space:]]*/, "", value)
      value = clean(value)
      if (value != "" && !seen[value]++) {
        print value
        found = 1
      }
    }

    /^path:[[:space:]]*/ {
      fallback = $0
      sub(/^path:[[:space:]]*/, "", fallback)
      fallback = clean(fallback)
    }

    END {
      if (!found && fallback != "") {
        print fallback
      }
    }
  ' "$file"
}

format_release_date() {
  local value="$1"
  local normalized

  # 仅处理 ISO 8601 格式（如 2026-07-31T13:02:56.736Z），其他格式原样输出
  case "$value" in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9]*)
      ;;
    *)
      printf '%s\n' "$value"
      return 0
      ;;
  esac

  # date 需要不带冒号的时区偏移；先去掉毫秒，再统一 UTC/偏移格式。
  normalized="$(printf '%s\n' "$value" | sed -E \
    -e 's/\.[0-9]+(Z|[+-][0-9]{2}:[0-9]{2})$/\1/' \
    -e 's/Z$/+0000/' \
    -e 's/([+-][0-9]{2}):([0-9]{2})$/\1\2/')"

  # BSD date 会按当前系统时区输出；解析失败时保留原值，避免显示错误时间。
  if ! date -j -f '%Y-%m-%dT%H:%M:%S%z' "$normalized" \
    '+%Y-%m-%d %H:%M:%S' 2>/dev/null; then
    printf '%s\n' "$value"
  fi
}

release_notes() {
  local file="$1"

  awk '
    function unquote(value, first, last) {
      first = substr(value, 1, 1)
      last = substr(value, length(value), 1)
      if ((first == "\"" && last == "\"") || (first == "\047" && last == "\047")) {
        return substr(value, 2, length(value) - 2)
      }
      return value
    }

    !in_notes && /^releaseNotes:[[:space:]]*/ {
      value = $0
      sub(/^releaseNotes:[[:space:]]*/, "", value)
      if (value ~ /^[|>][-+]?[[:space:]]*$/) {
        in_notes = 1
        next
      }
      if (value != "") {
        print unquote(value)
      }
      exit
    }

    in_notes {
      if ($0 ~ /^[^[:space:]]/) {
        exit
      }

      line = $0
      if (!indent_set && line ~ /[^[:space:]]/) {
        match(line, /[^[:space:]]/)
        indent = RSTART - 1
        indent_set = 1
      }

      if (line == "") {
        print ""
      } else if (indent_set) {
        print substr(line, indent + 1)
      }
    }
  ' "$file"
}

read_local_version() {
  local plist="$1/Contents/Info.plist"

  [ -r "$plist" ] || return 0
  /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$plist" 2>/dev/null \
    || defaults read "$plist" CFBundleShortVersionString 2>/dev/null || true
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --target)
      require_value "$@"
      TARGET="$2"
      shift 2
      ;;
    --arch)
      require_value "$@"
      ARCH="$2"
      shift 2
      ;;
    --channel)
      require_value "$@"
      CHANNEL="$2"
      shift 2
      ;;
    --file)
      require_value "$@"
      MANIFEST_FILE="$2"
      shift 2
      ;;
    --url)
      require_value "$@"
      MANIFEST_URL="$2"
      shift 2
      ;;
    --app)
      require_value "$@"
      LOCAL_APP="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "未知参数：$1（使用 --help 查看帮助）"
      ;;
  esac
done

if [ -n "$MANIFEST_FILE" ] && [ -n "$MANIFEST_URL" ]; then
  die "--file 与 --url 不能同时使用"
fi

if [ -n "$MANIFEST_FILE" ]; then
  [ -r "$MANIFEST_FILE" ] || die "无法读取文件：$MANIFEST_FILE"
  INPUT_FILE="$MANIFEST_FILE"
  SOURCE_LABEL="$MANIFEST_FILE"
else
  if [ -z "$MANIFEST_URL" ]; then
    case "$TARGET" in
      mac|macos|darwin)
        TARGET="mac"
        [ -n "$ARCH" ] || ARCH="$(detect_mac_arch)"
        PLATFORM="darwin-$(normalize_arch "$ARCH")"
        ;;
      windows|win)
        TARGET="windows"
        [ -n "$ARCH" ] || ARCH="x86_64"
        PLATFORM="windows-$(normalize_arch "$ARCH")"
        ;;
      *)
        die "不支持的目标：$TARGET"
        ;;
    esac

    case "$CHANNEL" in
      preview)
        CHANNEL_ID="3"
        ;;
      stable)
        CHANNEL_ID="1"
        ;;
      *)
        die "不支持的更新通道：$CHANNEL"
        ;;
    esac

    MANIFEST_URL="${DEFAULT_ENDPOINT}?platform=${PLATFORM}&channel=${CHANNEL_ID}"
  fi

  case "$MANIFEST_URL" in
    http://*|https://*)
      ;;
    *)
      die "--url 仅支持 HTTP/HTTPS 地址"
      ;;
  esac

  command -v curl >/dev/null 2>&1 || die "未找到 curl"
  TEMP_FILE="$(mktemp "${TMPDIR:-/tmp}/zcode-preview-update.XXXXXX")"
  curl \
    --fail \
    --silent \
    --show-error \
    --location \
    --connect-timeout 10 \
    --max-time 30 \
    --header "Accept: application/x-yaml,text/yaml,text/plain,*/*" \
    --header "User-Agent: zcode-preview-update.sh" \
    --output "$TEMP_FILE" \
    "$MANIFEST_URL"

  INPUT_FILE="$TEMP_FILE"
  SOURCE_LABEL="$MANIFEST_URL"
fi

VERSION="$(yaml_value "version" "$INPUT_FILE")"
RELEASE_NAME="$(yaml_value "releaseName" "$INPUT_FILE")"
RELEASE_DATE="$(yaml_value "releaseDate" "$INPUT_FILE")"
DOWNLOAD_URLS="$(download_urls "$INPUT_FILE")"
RELEASE_NOTES="$(release_notes "$INPUT_FILE")"

[ -n "$VERSION" ] || die "Manifest 中缺少 version"
[ -n "$DOWNLOAD_URLS" ] || die "Manifest 中缺少下载链接"

# 读取本地应用版本号，与远程版本比对（忽略可能的 v/V 前缀）
LOCAL_VERSION="$(read_local_version "$LOCAL_APP" || true)"
REMOTE_CMP="${VERSION#v}"
REMOTE_CMP="${REMOTE_CMP#V}"
LOCAL_CMP="${LOCAL_VERSION#v}"
LOCAL_CMP="${LOCAL_CMP#V}"

# 检测本地是否安装 ZCode（目录存在即视为已安装）
APP_INSTALLED=0
if [ -d "$LOCAL_APP" ]; then
  APP_INSTALLED=1
fi

# 已安装且版本一致 → 暂无更新；未安装或版本不一致 → 展示完整更新信息
if [ "$APP_INSTALLED" -eq 1 ] && [ -n "$LOCAL_CMP" ] && [ "$LOCAL_CMP" = "$REMOTE_CMP" ]; then
  printf '✅ 当前已是最新版本：%s，暂无更新。\n' "$LOCAL_VERSION"
  exit 0
fi

printf '\nZCode 更新信息\n'
printf '=============\n'
if [ "$APP_INSTALLED" -eq 1 ]; then
  printf '🎉 发现新版本！\n'
  printf '本地版本：%s\n' "$LOCAL_VERSION"
else
  # 未检测到本地应用时，默认展示远程最新版本的更新日志和下载链接
  printf 'ℹ️ 未检测到本地 ZCode 应用，以下为最新版本信息\n'
  printf '提示：如已安装在其它位置，请使用 --app 指定路径\n'
fi
printf '最新版本：%s\n' "$VERSION"
[ -z "$RELEASE_NAME" ] || printf '名称：%s\n' "$RELEASE_NAME"
[ -z "$RELEASE_DATE" ] || printf '日期：%s\n' "$(format_release_date "$RELEASE_DATE")"
printf '来源：%s\n' "$SOURCE_LABEL"

printf '\n下载链接：\n'
printf '%s\n' "$DOWNLOAD_URLS" | while IFS= read -r url; do
  case "$url" in
    *".dmg"*)
      kind="DMG"
      ;;
    *".zip"*)
      kind="ZIP"
      ;;
    *".exe"*)
      kind="EXE"
      ;;
    *)
      kind="FILE"
      ;;
  esac
  printf '  %-4s %s\n' "$kind" "$url"
done

printf '\n更新日志：\n'
if [ -n "$RELEASE_NOTES" ]; then
  printf '%s\n' "$RELEASE_NOTES"
else
  printf '（Manifest 未提供更新日志）\n'
fi
