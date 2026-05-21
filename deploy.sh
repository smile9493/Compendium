#!/bin/bash
# ─── PDF Module MCP — 自动部署脚本（无人值守） ───
# 与 install.sh 共享逻辑，但没有交互式提示，适合自动化部署。
set -euo pipefail

# ══════════════════════════════════════════════════════════════
# Constants
# ══════════════════════════════════════════════════════════════
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

INSTALL_DIR="/opt/pdf-module"
REPO_OWNER="smile9493"
REPO_NAME="Compendium"
API_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
PDFIUM_CHROMIUM_VERSION="7825"

log_info()  { echo -e "${GREEN}[✓]${NC} $*"; }
log_step()  { echo -e "${YELLOW}[·]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
log_error() { echo -e "${RED}[✗]${NC} $*" >&2; }
die()       { log_error "$*"; exit 1; }

# ══════════════════════════════════════════════════════════════
# Pre-flight
# ══════════════════════════════════════════════════════════════
check_root() {
    if [[ $EUID -ne 0 ]]; then die "此脚本需要 root 权限。请使用: sudo $0"; fi
}

detect_arch() {
    ARCH=$(uname -m); OS=$(uname -s)
    case "$OS" in
        Linux)
            case "$ARCH" in
                x86_64)     BINARY_NAME="pdf-mcp-linux-x64.tar.gz"; PDFIUM_ARCH="linux-x64"; PDFIUM_LIB="libpdfium.so" ;;
                aarch64|arm64) BINARY_NAME="pdf-mcp-linux-arm64.tar.gz"; PDFIUM_ARCH="linux-arm64"; PDFIUM_LIB="libpdfium.so" ;;
                *) die "不支持的架构: $ARCH" ;;
            esac ;;
        Darwin)
            case "$ARCH" in
                x86_64) BINARY_NAME="pdf-mcp-macos-x64.tar.gz"; PDFIUM_ARCH="mac-x64"; PDFIUM_LIB="libpdfium.dylib" ;;
                arm64)  BINARY_NAME="pdf-mcp-macos-arm64.tar.gz"; PDFIUM_ARCH="mac-arm64"; PDFIUM_LIB="libpdfium.dylib" ;;
                *) die "不支持的架构: $ARCH" ;;
            esac ;;
        *) die "不支持的操作系统: $OS" ;;
    esac
    log_info "系统: $OS $ARCH"
}

get_latest_version() {
    log_step "获取最新版本..."
    VERSION=$(curl -sSfL "$API_URL" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)
    if [[ -z "$VERSION" ]]; then
        log_warn "无法获取最新版本，使用默认 v0.1.4"
        VERSION="v0.1.4"
    fi
    log_info "版本: $VERSION"
}

# ══════════════════════════════════════════════════════════════
# Download
# ══════════════════════════════════════════════════════════════
download_with_progress() {
    local url="$1" out="$2" label="$3"
    echo -e "${CYAN}  ↓ $label${NC}"
    curl -#fSL -o "$out" "$url" 2>&1 || die "下载失败: $url"
    echo ""
}

download_binaries() {
    log_step "下载预编译二进制..."
    local url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${BINARY_NAME}"
    mkdir -p "${INSTALL_DIR}.new"
    download_with_progress "$url" "${INSTALL_DIR}.new/pdf-mcp.tar.gz" "$BINARY_NAME"
    log_info "二进制下载完成"
}

download_pdfium() {
    log_step "下载 PDFium 库..."
    local url="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${PDFIUM_CHROMIUM_VERSION}/pdfium-${PDFIUM_ARCH}.tgz"
    local out_dir="${INSTALL_DIR}.new/lib"
    mkdir -p "$out_dir"
    download_with_progress "$url" "${out_dir}/pdfium.tgz" "pdfium-${PDFIUM_ARCH}.tgz"
    tar -xf "${out_dir}/pdfium.tgz" -C "$out_dir"
    rm "${out_dir}/pdfium.tgz"
    if [[ -f "${out_dir}/lib/${PDFIUM_LIB}" ]]; then
        mv "${out_dir}/lib/${PDFIUM_LIB}" "${out_dir}/${PDFIUM_LIB}"
        rm -rf "${out_dir}/lib"
    fi
    chmod +x "${out_dir}/${PDFIUM_LIB}" 2>/dev/null || true
    log_info "PDFium 库下载完成"
}

extract_binaries() {
    log_step "解压二进制..."
    tar -xf "${INSTALL_DIR}.new/pdf-mcp.tar.gz" -C "${INSTALL_DIR}.new/"
    rm "${INSTALL_DIR}.new/pdf-mcp.tar.gz"
    for bin in pdf-mcp pdf-mcp-cli pdf-cli; do
        local p="${INSTALL_DIR}.new/${bin}"; [[ -f "$p" ]] && chmod +x "$p"
    done
    log_info "解压完成"
}

# ══════════════════════════════════════════════════════════════
# Setup
# ══════════════════════════════════════════════════════════════
setup_dirs() {
    log_step "创建数据目录..."
    mkdir -p "${INSTALL_DIR}.new/logs" "${INSTALL_DIR}.new/wiki/raw"
    mkdir -p "${INSTALL_DIR}.new/wiki/wiki" "${INSTALL_DIR}.new/wiki/scheme"
    mkdir -p "${INSTALL_DIR}.new/data"
}

setup_env() {
    log_step "配置环境..."
    local new_env="${INSTALL_DIR}.new/.env.local"

    if [[ -f "${INSTALL_DIR}/.env.local" ]]; then
        cp "${INSTALL_DIR}/.env.local" "$new_env"
        log_info "保留已有配置"
    else
        cat > "$new_env" << ENVEOF
PDFIUM_LIB_PATH=${INSTALL_DIR}/lib/${PDFIUM_LIB}
VLM_API_KEY=
VLM_MODEL=glm-4v-flash
VLM_ENDPOINT=https://open.bigmodel.cn/api/paas/v4/chat/completions
DASHBOARD_PORT=8000
STORAGE_TYPE=local
STORAGE_LOCAL_DIR=${INSTALL_DIR}/data
RUST_LOG=info
ENVEOF
        log_info "已创建配置文件"
    fi
}

finalize() {
    log_step "完成安装..."
    if [[ -d "$INSTALL_DIR" ]]; then
        local bak="${INSTALL_DIR}.bak.$(date +%Y%m%d%H%M%S)"
        mv "$INSTALL_DIR" "$bak"
        log_info "旧版本备份到 $bak"
    fi
    mv "${INSTALL_DIR}.new" "$INSTALL_DIR"
}

setup_symlinks() {
    log_step "创建命令快捷方式..."
    for bin in pdf-mcp pdf-mcp-cli pdf-cli; do
        local src="${INSTALL_DIR}/${bin}"
        if [[ -f "$src" ]]; then
            ln -sf "$src" "/usr/local/bin/${bin}"
            log_info "  /usr/local/bin/${bin}"
        fi
    done

    # Convenience dashboard launcher
    cat > "/usr/local/bin/pdf-mcp-dashboard" << 'LAUNCHER'
#!/bin/bash
INSTALL_DIR="/opt/pdf-module"
ENV_FILE="${INSTALL_DIR}/.env.local"
[[ -f "$ENV_FILE" ]] && { set -a; source "$ENV_FILE"; set +a; }
export LD_LIBRARY_PATH="${INSTALL_DIR}/lib:${LD_LIBRARY_PATH:-}"
exec "${INSTALL_DIR}/pdf-mcp" dashboard "$@"
LAUNCHER
    chmod +x "/usr/local/bin/pdf-mcp-dashboard"
}

setup_service() {
    log_step "创建 systemd 服务..."
    if ! command -v systemctl &>/dev/null; then
        log_warn "systemctl 未找到，跳过服务创建"; return
    fi

    systemctl stop pdf-mcp 2>/dev/null || true
    cat > /etc/systemd/system/pdf-mcp.service << UNIT
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
    systemctl start pdf-mcp
    log_info "服务已启动"
}

# ══════════════════════════════════════════════════════════════
# Main
# ══════════════════════════════════════════════════════════════
print_success() {
    echo ""
    echo -e "${GREEN}══════════════════════════════════════════${NC}"
    echo -e "${GREEN}  部署完成! PDF Module MCP ${VERSION}${NC}"
    echo -e "${GREEN}══════════════════════════════════════════${NC}"
    echo ""
    echo -e "  配置文件: ${INSTALL_DIR}/.env.local"
    echo -e "  管理命令: pdf-mcp-cli config"
    echo -e "  Web 界面: http://localhost:8000"
    echo ""
    echo -e "  查看日志: journalctl -u pdf-mcp -f"
    echo ""
}

main() {
    check_root
    detect_arch
    get_latest_version
    download_binaries
    download_pdfium
    extract_binaries
    setup_dirs
    setup_env
    finalize
    setup_symlinks
    setup_service
    print_success
}

main "$@"
