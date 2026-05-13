#!/usr/bin/env bash
# =============================================================================
# ADR Generator — 基于模板创建架构决策记录
# =============================================================================
#
# 用法:
#   scripts/adr.sh new "使用 PostgreSQL 作为主数据库"
#   scripts/adr.sh list
#   scripts/adr.sh init   (创建 docs/adr/ 目录并复制模板)
#
# 生成的文件命名规则: docs/adr/NNNN-lowercase-title.md
# =============================================================================

set -euo pipefail

ADR_DIR="docs/adr"
TEMPLATE="${ADR_DIR}/0000-template.md"

# ─── 确保 ADR 目录和模板存在 ──────────────────────────────

init() {
    mkdir -p "${ADR_DIR}"

    if [[ ! -f "${TEMPLATE}" ]]; then
        cat > "${TEMPLATE}" << 'TMPL'
# {NUMBER}. {TITLE}

**Status**: {STATUS}
**Date**: {DATE}
**Deciders**: {DECIDERS}

## Context

{CONTEXT}

## Decision

{DECISION}

## Consequences

{CONSEQUENCES}

---

*Template: [ADR-0000](docs/adr/0000-template.md)*
TMPL
        echo "[adr] 已创建模板: ${TEMPLATE}"
    else
        echo "[adr] 模板已存在: ${TEMPLATE}"
    fi
}

# ─── 列出已有 ADR ──────────────────────────────────────────

list() {
    if [[ ! -d "${ADR_DIR}" ]]; then
        echo "[adr] ADR 目录不存在。请先运行: adr.sh init"
        exit 1
    fi

    local count=0
    for f in "${ADR_DIR}"/*.md; do
        if [[ -f "$f" && "$(basename "$f")" != "0000-template.md" ]]; then
            local basename
            basename=$(basename "$f")
            local title
            title=$(head -1 "$f" | sed 's/^# //')
            echo "  ${basename}  →  ${title}"
            count=$((count + 1))
        fi
    done

    if [[ $count -eq 0 ]]; then
        echo "[adr] 暂无 ADR 记录。使用 'adr.sh new \"标题\"' 创建第一个。"
    fi
}

# ─── 创建新 ADR ────────────────────────────────────────────

new() {
    local title="$1"
    if [[ -z "${title}" ]]; then
        echo "用法: adr.sh new \"决策标题\""
        exit 1
    fi

    # 确保目录和模板存在
    if [[ ! -f "${TEMPLATE}" ]]; then
        init
    fi

    # 计算新的编号
    local max_num=0
    if compgen -G "${ADR_DIR}/[0-9][0-9][0-9][0-9]-*.md" > /dev/null 2>&1; then
        for f in "${ADR_DIR}"/[0-9][0-9][0-9][0-9]-*.md; do
            local num
            num=$(basename "$f" | sed -n 's/^\([0-9]\{4\}\)-.*/\1/p')
            if [[ -n "${num}" && "${num}" -gt "${max_num}" ]]; then
                max_num="${num}"
            fi
        done
    fi
    local next_num
    next_num=$(printf "%04d" $((max_num + 1)))

    # 生成文件名 slug (仅保留 ASCII 字母数字和连字符)
    local slug
    slug=$(echo "${title}" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9-')
    # 去除前后连字符，若结果为空则用编号
    slug=$(echo "${slug}" | sed 's/^-*//;s/-*$//')
    if [[ -z "${slug}" ]]; then
        slug="${next_num}"
    fi

    local filename="${ADR_DIR}/${next_num}-${slug}.md"
    local today
    today=$(date +%Y-%m-%d)

    # 基于模板生成
    sed \
        -e "s|{NUMBER}|${next_num}|g" \
        -e "s|{TITLE}|${title}|g" \
        -e "s|{STATUS}|Proposed|g" \
        -e "s|{DATE}|${today}|g" \
        -e "s|{DECIDERS}|等待填写|g" \
        -e "s|{CONTEXT}|待描述|g" \
        -e "s|{DECISION}|待描述|g" \
        -e "s|{CONSEQUENCES}|待描述|g" \
        "${TEMPLATE}" > "${filename}"

    echo "[adr] 已创建: ${filename}"
}

# ─── 主入口 ─────────────────────────────────────────────────

cmd="${1:-}"
case "${cmd}" in
    init)
        init
        ;;
    list|ls)
        list
        ;;
    new|add)
        new "${2:-}"
        ;;
    *)
        echo "ADR Generator — 架构决策记录管理"
        echo ""
        echo "用法:"
        echo "  adr.sh init                  创建 docs/adr/ 目录及模板"
        echo "  adr.sh new \"决策标题\"        创建新的 ADR"
        echo "  adr.sh list                  列出所有 ADR"
        exit 1
        ;;
esac