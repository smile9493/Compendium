#!/bin/bash
# ─── PDF Module MCP — 一键安装 / 升级 / 卸载 ───
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/smile9493/Compendium/main/install.sh | sudo bash
#   curl -fsSL https://raw.githubusercontent.com/smile9493/Compendium/main/install.sh | sudo bash -s -- --uninstall
#   curl -fsSL https://raw.githubusercontent.com/smile9493/Compendium/main/install.sh | sudo bash -s -- --version v0.1.4
set -euo pipefail

# ══════════════════════════════════════════════════════════════
# Constants
# ══════════════════════════════════════════════════════════════
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

INSTALL_DIR="/opt/pdf-module"
REPO_OWNER="smile9493"
REPO_NAME="Compendium"
API_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
# Must match .github/actions/setup-pdfium/action.yml and CI env
PDFIUM_CHROMIUM_VERSION="7825"

# ══════════════════════════════════════════════════════════════
# Helpers
# ══════════════════════════════════════════════════════════════
log_info()  { echo -e "${GREEN}[✓]${NC} $*"; }
log_step()  { echo -e "${YELLOW}[·]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
log_error() { echo -e "${RED}[✗]${NC} $*" >&2; }
die()       { log_error "$*"; exit 1; }

cleanup() {
    local rc=$?
    if [[ $rc -ne 0 ]]; then
        echo ""
        log_warn "安装中断。运行以下命令清理:"
        echo "  rm -rf ${INSTALL_DIR}.new"
    fi
}
trap cleanup EXIT

# ══════════════════════════════════════════════════════════════
# Print fancy banner
# ══════════════════════════════════════════════════════════════
print_banner() {
    echo -e "${CYAN}"
    cat << 'EOF'
██████╗  ██████╗ ██╗     ██╗     ██╗███╗   ██╗ ██████╗
██╔══██╗██╔═══██╗██║     ██║     ██║████╗  ██║██╔════╝
██████╔╝██║   ██║██║     ██║     ██║██╔██╗ ██║██║  ███╗
██╔═══╝ ██║   ██║██║     ██║     ██║██║╚██╗██║██║   ██║
██║     ╚██████╔╝███████╗███████╗██║██║ ╚████║╚██████╔╝
╚═╝      ╚═════╝ ╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝
EOF
    echo -e "${NC}"
    echo -e "${GREEN}PDF Module MCP — 一键安装 / 升级${NC}"
    echo ""
}

# ══════════════════════════════════════════════════════════════
# Uninstall
# ══════════════════════════════════════════════════════════════
uninstall() {
    echo -e "${YELLOW}[!] 正在卸载 PDF Module MCP...${NC}"
    echo ""

    # Stop and disable systemd service
    if systemctl is-active pdf-mcp &>/dev/null; then
        systemctl stop pdf-mcp
        log_info "已停止 pdf-mcp 服务"
    fi
    if systemctl is-enabled pdf-mcp &>/dev/null; then
        systemctl disable pdf-mcp
        log_info "已禁用 pdf-mcp 服务"
    fi

    # Remove systemd unit
    rm -f /etc/systemd/system/pdf-mcp.service
    systemctl daemon-reload 2>/dev/null || true

    # Remove symlinks
    for bin in pdf-mcp pdf-mcp-cli pdf-cli; do
        rm -f "/usr/local/bin/${bin}"
    done

    # Remove installation directory
    if [[ -d "$INSTALL_DIR" ]]; then
        echo ""
        echo -e "${YELLOW}  将删除 $INSTALL_DIR 目录${NC}"
        echo -e "${YELLOW}  配置文件备份保留在: ${INSTALL_DIR}.bak 如果存在${NC}"
        echo -n "  确认删除? [y/N] "
        read -r confirm
        if [[ "$confirm" =~ ^[Yy]$ ]]; then
            rm -rf "$INSTALL_DIR"
            log_info "已删除 $INSTALL_DIR"
        else
            log_info "已跳过删除 $INSTALL_DIR"
        fi
    fi

    echo ""
    echo -e "${GREEN}卸载完成${NC}"
    exit 0
}

# ══════════════════════════════════════════════════════════════
# Pre-flight checks
# ══════════════════════════════════════════════════════════════
check_prerequisites() {
    local missing=()

    # Root check — skip for --uninstall flag (already handled)
    if [[ $EUID -ne 0 ]]; then
        die "此脚本需要 root 权限。请使用: sudo bash $0"
    fi

    for cmd in curl tar grep sed; do
        if ! command -v "$cmd" &>/dev/null; then
            missing+=("$cmd")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_warn "缺少依赖: ${missing[*]}"
        if command -v apt-get &>/dev/null; then
            apt-get update -qq && apt-get install -y -qq "${missing[@]}"
        elif command -v yum &>/dev/null; then
            yum install -y "${missing[@]}"
        elif command -v dnf &>/dev/null; then
            dnf install -y "${missing[@]}"
        elif command -v apk &>/dev/null; then
            apk add --no-cache "${missing[@]}"
        else
            die "请手动安装: ${missing[*]}"
        fi
    fi
}

# ══════════════════════════════════════════════════════════════
# Architecture detection
# ══════════════════════════════════════════════════════════════
detect_arch() {
    ARCH=$(uname -m)
    OS=$(uname -s)

    case "$OS" in
        Linux)
            case "$ARCH" in
                x86_64)
                    BINARY_NAME="pdf-mcp-linux-x64.tar.gz"
                    PDFIUM_ARCH="linux-x64"
                    PDFIUM_LIB="libpdfium.so"
                    ;;
                aarch64|arm64)
                    BINARY_NAME="pdf-mcp-linux-arm64.tar.gz"
                    PDFIUM_ARCH="linux-arm64"
                    PDFIUM_LIB="libpdfium.so"
                    ;;
                *) die "不支持的架构: $ARCH (仅支持 x86_64 / arm64)" ;;
            esac
            ;;
        Darwin)
            case "$ARCH" in
                x86_64)
                    BINARY_NAME="pdf-mcp-macos-x64.tar.gz"
                    PDFIUM_ARCH="mac-x64"
                    PDFIUM_LIB="libpdfium.dylib"
                    ;;
                arm64)
                    BINARY_NAME="pdf-mcp-macos-arm64.tar.gz"
                    PDFIUM_ARCH="mac-arm64"
                    PDFIUM_LIB="libpdfium.dylib"
                    ;;
                *) die "不支持的架构: $ARCH" ;;
            esac
            # macOS: install sha256sum if missing
            if ! command -v sha256sum &>/dev/null; then
                if command -v brew &>/dev/null; then
                    brew install coreutils
                fi
            fi
            ;;
        *) die "不支持的操作系统: $OS" ;;
    esac

    log_info "系统: $OS $ARCH"
    log_info "二进制: $BINARY_NAME"
}

# ══════════════════════════════════════════════════════════════
# Version resolution
# ══════════════════════════════════════════════════════════════
resolve_version() {
    if [[ -n "${SPECIFIC_VERSION:-}" ]]; then
        VERSION="$SPECIFIC_VERSION"
        log_info "指定版本: $VERSION"
    else
        log_step "获取最新版本..."
        VERSION=$(curl -sSfL "$API_URL" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)
        if [[ -z "$VERSION" ]]; then
            log_warn "无法获取最新版本，使用默认 v0.1.4"
            VERSION="v0.1.4"
        fi
        log_info "最新版本: $VERSION"
    fi
}

# ══════════════════════════════════════════════════════════════
# Download helpers
# ══════════════════════════════════════════════════════════════
download_with_progress() {
    local url="$1"
    local out="$2"
    local label="$3"

    echo -e "${CYAN}  ↓ $label${NC}"
    curl -#fSL -o "$out" "$url" 2>&1 || die "下载失败: $url"
    echo ""
}

verify_sha256() {
    local file="$1"
    local expected="$2"
    if [[ -z "$expected" ]]; then
        log_warn "跳过校验（无 SHA256）"
        return 0
    fi
    local actual
    actual=$(sha256sum "$file" | cut -d' ' -f1)
    if [[ "$actual" != "$expected" ]]; then
        rm -f "$file"
        die "SHA256 不匹配: 期望 $expected, 实际 $actual"
    fi
    log_info "校验通过"
}

# ══════════════════════════════════════════════════════════════
# Download & install release binaries
# ══════════════════════════════════════════════════════════════
download_binaries() {
    log_step "下载预编译二进制..."

    local download_url
    download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${BINARY_NAME}"

    mkdir -p "${INSTALL_DIR}.new"
    download_with_progress "$download_url" "${INSTALL_DIR}.new/pdf-mcp.tar.gz" "$BINARY_NAME"

    log_info "二进制下载完成"
}

download_pdfium() {
    log_step "下载 PDFium 库..."

    local pdfium_url="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${PDFIUM_CHROMIUM_VERSION}/pdfium-${PDFIUM_ARCH}.tgz"
    local out_dir="${INSTALL_DIR}.new/lib"
    mkdir -p "$out_dir"

    download_with_progress "$pdfium_url" "${out_dir}/pdfium.tgz" "pdfium-${PDFIUM_ARCH}.tgz"

    tar -xf "${out_dir}/pdfium.tgz" -C "$out_dir"
    rm "${out_dir}/pdfium.tgz"

    # The tarball extracts into out_dir/lib/... so flatten it
    if [[ -f "${out_dir}/lib/${PDFIUM_LIB}" ]]; then
        mv "${out_dir}/lib/${PDFIUM_LIB}" "${out_dir}/${PDFIUM_LIB}"
        rm -rf "${out_dir}/lib"
    fi

    chmod +x "${out_dir}/${PDFIUM_LIB}" 2>/dev/null || true
    log_info "PDFium 库下载完成"
}

extract_binaries() {
    log_step "解压二进制文件..."

    local archive="${INSTALL_DIR}.new/pdf-mcp.tar.gz"
    tar -xf "$archive" -C "${INSTALL_DIR}.new/"
    rm "$archive"

    for bin in pdf-mcp pdf-mcp-cli pdf-cli; do
        local path="${INSTALL_DIR}.new/${bin}"
        if [[ -f "$path" ]]; then
            chmod +x "$path"
            log_info "  $bin 就绪"
        fi
    done
}

# ══════════════════════════════════════════════════════════════
# Directory & config setup
# ══════════════════════════════════════════════════════════════
setup_directories() {
    log_step "创建数据目录..."
    mkdir -p "${INSTALL_DIR}.new/logs"
    mkdir -p "${INSTALL_DIR}.new/wiki/raw"
    mkdir -p "${INSTALL_DIR}.new/wiki/wiki"
    mkdir -p "${INSTALL_DIR}.new/wiki/scheme"
    mkdir -p "${INSTALL_DIR}.new/data"
    log_info "数据目录已创建"
}

setup_env_file() {
    log_step "配置文件..."

    local existing_env="${INSTALL_DIR}/.env.local"
    local new_env="${INSTALL_DIR}.new/.env.local"

    if [[ -f "$existing_env" ]]; then
        # Preserve existing config during upgrade
        cp "$existing_env" "$new_env"
        log_info "保留已有配置: .env.local"
    else
        cat > "$new_env" << ENVEOF
# PDF Module MCP 环境变量配置

# PDFium 库路径
PDFIUM_LIB_PATH=${INSTALL_DIR}/lib/${PDFIUM_LIB}

# VLM (Visual Language Model) 配置
VLM_API_KEY=
VLM_MODEL=glm-4v-flash
VLM_ENDPOINT=https://open.bigmodel.cn/api/paas/v4/chat/completions

# Dashboard 端口
DASHBOARD_PORT=8000

# 存储配置
STORAGE_TYPE=local
STORAGE_LOCAL_DIR=${INSTALL_DIR}/data

# 日志级别
RUST_LOG=info
ENVEOF
        log_info "已创建配置文件，请编辑 .env.local 填入 VLM_API_KEY"
    fi
}

# ══════════════════════════════════════════════════════════════
# symlinks in /usr/local/bin
# ══════════════════════════════════════════════════════════════
setup_symlinks() {
    log_step "创建命令快捷方式..."

    # Wrapper script that sources .env.local and sets LD_LIBRARY_PATH
    local wrapper="${INSTALL_DIR}/pdf-mcp"
    if [[ ! -f "$wrapper" ]]; then
        # Should not happen after extract, but just in case
        die "pdf-mcp 二进制未找到"
    fi

    for bin in pdf-mcp pdf-mcp-cli pdf-cli; do
        local src="${INSTALL_DIR}/${bin}"
        local link="/usr/local/bin/${bin}"
        if [[ -f "$src" ]]; then
            ln -sf "$src" "$link"
            log_info "  /usr/local/bin/${bin} → ${src}"
        fi
    done

    # Create a convenience launcher script that sources .env.local
    local launcher="/usr/local/bin/pdf-mcp-dashboard"
    cat > "$launcher" << 'LAUNCHER'
#!/bin/bash
# PDF Module MCP Dashboard launcher
# Sources .env.local for environment variables
INSTALL_DIR="/opt/pdf-module"
ENV_FILE="${INSTALL_DIR}/.env.local"

if [[ -f "$ENV_FILE" ]]; then
    set -a
    source "$ENV_FILE"
    set +a
fi

export LD_LIBRARY_PATH="${INSTALL_DIR}/lib:${LD_LIBRARY_PATH:-}"

exec "${INSTALL_DIR}/pdf-mcp" dashboard "$@"
LAUNCHER
    chmod +x "$launcher"
    log_info "  /usr/local/bin/pdf-mcp-dashboard 已创建"
}

# ══════════════════════════════════════════════════════════════
# systemd service
# ══════════════════════════════════════════════════════════════
create_service() {
    log_step "创建 systemd 服务..."

    if ! command -v systemctl &>/dev/null; then
        log_warn "systemctl 未找到，跳过服务创建"
        return
    fi

    # Stop existing service if running
    systemctl stop pdf-mcp 2>/dev/null || true

    local service_file="/etc/systemd/system/pdf-mcp.service"
    cat > "$service_file" << UNIT
[Unit]
Description=PDF Module MCP Service
Documentation=https://github.com/${REPO_OWNER}/${REPO_NAME}
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=${INSTALL_DIR}
EnvironmentFile=${INSTALL_DIR}/.env.local
Environment=LD_LIBRARY_PATH=${INSTALL_DIR}/lib
ExecStart=${INSTALL_DIR}/pdf-mcp dashboard --port \${DASHBOARD_PORT:-8000}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
UNIT

    systemctl daemon-reload
    systemctl enable pdf-mcp
    log_info "systemd 服务已创建并启用"

    # Ask whether to start now
    echo -n -e "${YELLOW}  立即启动 pdf-mcp 服务? [Y/n] ${NC}"
    read -r start_now
    if [[ -z "$start_now" || "$start_now" =~ ^[Yy]$ ]]; then
        systemctl start pdf-mcp
        log_info "服务已启动"
        echo ""
        echo "  查看状态: systemctl status pdf-mcp"
        echo "  查看日志: journalctl -u pdf-mcp -f"
    fi
}

# ══════════════════════════════════════════════════════════════
# Swap in new installation atomically
# ══════════════════════════════════════════════════════════════
finalize_installation() {
    log_step "完成安装..."

    # Backup existing installation
    if [[ -d "$INSTALL_DIR" ]]; then
        local bak="${INSTALL_DIR}.bak.$(date +%Y%m%d%H%M%S)"
        log_info "备份旧版本到 $bak"
        mv "$INSTALL_DIR" "$bak"
    fi

    mv "${INSTALL_DIR}.new" "$INSTALL_DIR"
    log_info "安装目录已更新: $INSTALL_DIR"
}

# ══════════════════════════════════════════════════════════════
# Success output
# ══════════════════════════════════════════════════════════════
print_success() {
    echo ""
    echo -e "${GREEN}══════════════════════════════════════════${NC}"
    echo -e "${GREEN}  安装完成! PDF Module MCP ${VERSION}${NC}"
    echo -e "${GREEN}══════════════════════════════════════════${NC}"
    echo ""
    echo -e "  ${CYAN}管理命令:${NC}"
    echo "    pdf-mcp-cli           交互式配置"
    echo "    pdf-mcp-cli config    配置 API Key"
    echo "    pdf-mcp-cli status    查看状态"
    echo ""
    echo -e "  ${CYAN}启动服务:${NC}"
    echo "    sudo systemctl start pdf-mcp"
    echo "    journalctl -u pdf-mcp -f   查看日志"
    echo ""
    echo -e "  ${CYAN}Web 界面:${NC}"
    echo "    http://localhost:8000"
    echo ""
    echo -e "  ${CYAN}快速运行 (不安装服务):${NC}"
    echo "    pdf-mcp-dashboard"
    echo ""
    echo -e "  ${CYAN}文件位置:${NC}"
    echo "    安装目录:  $INSTALL_DIR"
    echo "    配置文件:  $INSTALL_DIR/.env.local"
    echo "    日志目录:  $INSTALL_DIR/logs"
    echo ""
    echo -e "  ${YELLOW}提示: 编辑 $INSTALL_DIR/.env.local 填入 VLM_API_KEY 后重启服务${NC}"
    echo ""
}

# ══════════════════════════════════════════════════════════════
# Usage
# ══════════════════════════════════════════════════════════════
usage() {
    cat << USAGE
用法: bash install.sh [选项]

选项:
  --version VERSION    指定安装版本 (默认: 最新 release)
  --uninstall          卸载 PDF Module MCP
  --help               显示此帮助

示例:
  curl -fsSL https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/main/install.sh | sudo bash
  curl -fsSL ... | sudo bash -s -- --version v0.1.4
  curl -fsSL ... | sudo bash -s -- --uninstall
USAGE
    exit 0
}

# ══════════════════════════════════════════════════════════════
# Main
# ══════════════════════════════════════════════════════════════
main() {
    # Parse args
    SPECIFIC_VERSION=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --uninstall) uninstall ;;
            --version) shift; SPECIFIC_VERSION="${1:-}"; shift ;;
            --help|-h) usage ;;
            *) echo -e "${RED}未知选项: $1${NC}"; usage ;;
        esac
    done

    print_banner
    check_prerequisites
    detect_arch
    resolve_version

    download_binaries
    download_pdfium
    extract_binaries
    setup_directories
    setup_env_file
    finalize_installation
    setup_symlinks
    create_service
    print_success
}

main "$@"
