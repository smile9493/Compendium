#!/usr/bin/env bash
# =============================================================================
# Quality Guard — Pre-commit / CI 本地质量守卫脚本
# =============================================================================
#
# 在提交代码前运行此脚本，确保：
#   1. 代码格式化符合 rustfmt 规范
#   2. 无 clippy 警告（deny-by-default）
#   3. 文档测试通过（cargo test --doc）
#   4. 全部测试通过（cargo test --all-targets）
#   5. 库 crate 文档可正常生成（cargo doc --no-deps --document-private-items）
#   6. [可选] Miri 检测 unsafe 代码（cargo +nightly miri test）
#
# 用法:
#   ./scripts/quality-guard.sh             # 默认：运行 fmt + clippy + test + doc
#   ./scripts/quality-guard.sh --miri      # 额外启用 Miri 检测
#   ./scripts/quality-guard.sh --quick     # 快速模式：仅 fmt + clippy
#
# =============================================================================
# 安装必要工具（首次使用前执行）:
#
#   # 基础工具（需要 stable 工具链）
#   rustup component add rustfmt clippy
#
#   # Miri（需要 nightly 工具链）
#   rustup toolchain install nightly --component miri
#
#   # 将此脚本添加为 Git pre-commit hook（可选）
#   cp scripts/quality-guard.sh .git/hooks/pre-commit
#   chmod +x .git/hooks/pre-commit
#
# =============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------
# 配置区
# ---------------------------------------------------------------------------

WORKSPACE_DIR="pdf-module-rs"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

PASSED=0
FAILED=0
SKIPPED=0

# 包含 unsafe 代码的库 crate（Miri 检查目标）
UNSAFE_LIB_CRATES=(
    "pdf-core"
    "pdf-wasm"
    "vlm-visual-gateway"
)

SKIP_MIRI=true
QUICK_MODE=false

# ---------------------------------------------------------------------------
# 参数解析
# ---------------------------------------------------------------------------
for arg in "$@"; do
    case "$arg" in
        --miri)   SKIP_MIRI=false ;;
        --quick)  QUICK_MODE=true ;;
        --help|-h)
            echo "用法: $0 [--miri] [--quick]"
            echo ""
            echo "  --miri     启用 Miri 对 unsafe 代码的检测（需要 nightly 工具链）"
            echo "  --quick    快速模式：仅运行 fmt + clippy"
            echo "  --help     显示此帮助信息"
            exit 0
            ;;
        *)
            echo -e "${RED}未知参数: $arg${NC}"
            echo "用法: $0 [--miri] [--quick] [--help]"
            exit 1
            ;;
    esac
done

# ---------------------------------------------------------------------------
# 辅助函数
# ---------------------------------------------------------------------------

stage_header() {
    echo ""
    echo -e "${BOLD}${CYAN}===============================================================================${NC}"
    echo -e "${BOLD}${CYAN}==> $1${NC}"
    echo -e "${BOLD}${CYAN}===============================================================================${NC}"
}

pass() {
    echo -e "  ${GREEN}✓ PASS${NC}  $1"
    PASSED=$((PASSED + 1))
}

fail() {
    echo -e "  ${RED}✗ FAIL${NC}  $1"
    FAILED=$((FAILED + 1))
    if [ -n "${2:-}" ]; then
        echo -e "  ${YELLOW}  → 修复命令: $2${NC}"
    fi
}

skip_msg() {
    echo -e "  ${YELLOW}○ SKIP${NC}  $1"
    SKIPPED=$((SKIPPED + 1))
}

print_summary() {
    echo ""
    echo -e "${BOLD}===============================================================================${NC}"
    echo -e "${BOLD}  Quality Guard 结果汇总${NC}"
    echo -e "${BOLD}===============================================================================${NC}"
    echo -e "  ${GREEN}通过: ${PASSED}${NC}"
    echo -e "  ${RED}失败: ${FAILED}${NC}"
    if [ "$SKIPPED" -gt 0 ]; then
        echo -e "  ${YELLOW}跳过: ${SKIPPED}${NC}"
    fi
    echo ""

    if [ "$FAILED" -gt 0 ]; then
        echo -e "${RED}${BOLD}✗ 存在失败的检查项，请在提交前修复以上问题。${NC}"
        echo ""
        exit 1
    else
        echo -e "${GREEN}${BOLD}✓ 全部检查通过，可以安全提交。${NC}"
        echo ""
        exit 0
    fi
}

# 运行 cargo 命令并处理成功/失败。失败时立刻退出。
run_cargo() {
    local desc="$1"
    local fix_cmd="$2"
    shift 2
    if "$@"; then
        pass "$desc"
    else
        fail "$desc" "$fix_cmd"
        print_summary
    fi
}

# ---------------------------------------------------------------------------
# 前置检查
# ---------------------------------------------------------------------------

echo -e "${BOLD}${CYAN}"
echo "   ____        _ _ _       ____                     _ "
echo "  / __ \\      | (_) |     / ___|_   _  __ _ _ __ __| |"
echo " | |  | | __ _| |_| |_   | |  _| | | |/ _  | '__/ _  |"
echo " | |  | |/ _  | | | __|  | |_| | |_| | (_| | | | (_| |"
echo " | |__| | (_| | | | |_    \\____|\\__,_|\\__,_|_|  \\__,_|"
echo "  \\___\\_\\\\__,_|_|_|\\__|"
echo -e "${NC}"
echo "  Rust 项目代码质量守卫"
echo ""

if [ ! -d "$WORKSPACE_DIR" ]; then
    echo -e "${RED}错误: 找不到工作区目录 '$WORKSPACE_DIR'${NC}"
    echo "请在项目根目录下运行此脚本。"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}错误: 未找到 cargo，请先安装 Rust 工具链。${NC}"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo -e "${CYAN}Rust 版本:${NC} $(rustc --version 2>/dev/null || echo '未知')"
echo -e "${CYAN}Cargo 版本:${NC} $(cargo --version 2>/dev/null || echo '未知')"
echo -e "${CYAN}工作区:${NC} $(pwd)/${WORKSPACE_DIR}"

# =============================================================================
# Stage 1: 代码格式化检查（cargo fmt）
# =============================================================================

stage_header "Stage 1: 检查代码格式化"

run_cargo \
    "cargo fmt --all -- --check" \
    "cargo fmt --all" \
    cargo fmt --all -- --check \
    --manifest-path "$WORKSPACE_DIR/Cargo.toml"

# =============================================================================
# Stage 2: Clippy 静态检查
# =============================================================================

stage_header "Stage 2: Clippy 静态检查"

run_cargo \
    "cargo clippy --all-targets --workspace -- -D warnings" \
    "cargo clippy --all-targets --workspace" \
    cargo clippy --all-targets --workspace -- -D warnings \
    --manifest-path "$WORKSPACE_DIR/Cargo.toml"

if [ "$QUICK_MODE" = true ]; then
    echo ""
    echo -e "${YELLOW}快速模式：跳过后续阶段。${NC}"
    print_summary
fi

# =============================================================================
# Stage 3: 文档测试（cargo test --doc）
# =============================================================================

stage_header "Stage 3: 文档测试"

run_cargo \
    "cargo test --doc --workspace" \
    "cargo test --doc --workspace -- --nocapture" \
    cargo test --doc --workspace \
    --manifest-path "$WORKSPACE_DIR/Cargo.toml"

# =============================================================================
# Stage 4: 全量测试（cargo test --all-targets）
# =============================================================================

stage_header "Stage 4: 全量测试"

run_cargo \
    "cargo test --all-targets --workspace" \
    "cargo test --all-targets --workspace -- --nocapture" \
    cargo test --all-targets --workspace \
    --manifest-path "$WORKSPACE_DIR/Cargo.toml"

# =============================================================================
# Stage 5: 库 crate 文档生成（cargo doc）
# =============================================================================

stage_header "Stage 5: 库 crate 文档生成"

run_cargo \
    "cargo doc --no-deps --document-private-items --workspace" \
    "cargo doc --no-deps --document-private-items --workspace 2>&1 | head -50" \
    cargo doc --no-deps --document-private-items --workspace \
    --manifest-path "$WORKSPACE_DIR/Cargo.toml"

# =============================================================================
# Stage 6: Miri 检测 unsafe 代码（可选，需要 nightly）
# =============================================================================

stage_header "Stage 6: Miri unsafe 代码检测"

if [ "$SKIP_MIRI" = true ]; then
    skip_msg "Miri 检测未启用（使用 --miri 参数启用）"
else
    if ! rustup run nightly cargo --version &> /dev/null; then
        fail \
            "Miri 需要 nightly 工具链，请先安装" \
            "rustup toolchain install nightly --component miri"
        skip_msg "跳过 Miri 检测"
    else
        for crate in "${UNSAFE_LIB_CRATES[@]}"; do
            echo ""
            echo -e "  ${CYAN}→ 正在检查 $crate ...${NC}"
            run_cargo \
                "cargo +nightly miri test -p $crate" \
                "cargo +nightly miri test -p $crate -- --nocapture" \
                cargo +nightly miri test -p "$crate" -- --nocapture \
                --manifest-path "$WORKSPACE_DIR/Cargo.toml"
        done
    fi
fi

# =============================================================================
# 汇总
# =============================================================================

print_summary