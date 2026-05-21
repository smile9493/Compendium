#!/usr/bin/env bash
# Bulk-delete failed GitHub Actions workflow runs for a repository.
#
# Usage:
#   ./scripts/delete-failed-actions-runs.sh [owner/repo]
#   GH_TOKEN=ghp_xxx ./scripts/delete-failed-actions-runs.sh smile9493/Compendium
#
# Auth (first match wins): GH_TOKEN / GITHUB_TOKEN, or `git credential fill` for github.com.

set -euo pipefail

REPO="${1:-smile9493/Compendium}"
PER_PAGE=100
API="https://api.github.com/repos/${REPO}/actions/runs"

github_token() {
  if [[ -n "${GH_TOKEN:-}" ]]; then
    printf '%s' "$GH_TOKEN"
    return
  fi
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    printf '%s' "$GITHUB_TOKEN"
    return
  fi
  if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    gh auth token
    return
  fi
  printf 'protocol=https\nhost=github.com\n\n' | git credential fill 2>/dev/null \
    | awk -F= '/^password=/{print $2; exit}'
}

TOKEN="$(github_token)"
if [[ -z "${TOKEN}" ]]; then
  echo "error: no GitHub token. Run: gh auth login  OR  export GH_TOKEN=..." >&2
  exit 1
fi

auth_header() {
  printf 'Authorization: Bearer %s' "$TOKEN"
}

list_failed_ids() {
  local page=1
  while true; do
    local body
    body="$(curl -fsSL \
      -H "$(auth_header)" \
      -H 'Accept: application/vnd.github+json' \
      -H 'X-GitHub-Api-Version: 2022-11-28' \
      "${API}?status=failure&per_page=${PER_PAGE}&page=${page}")"
    local ids
    ids="$(printf '%s' "$body" | python3 -c "
import json, sys
data = json.load(sys.stdin)
runs = data.get('workflow_runs') or []
for r in runs:
    print(r['id'])
")"
    if [[ -z "${ids}" ]]; then
      break
    fi
    printf '%s\n' "$ids"
    local count
    count="$(printf '%s\n' "$ids" | wc -l | tr -d ' ')"
    if [[ "${count}" -lt "${PER_PAGE}" ]]; then
      break
    fi
    page=$((page + 1))
  done
}

delete_run() {
  local run_id="$1"
  curl -fsSL -X DELETE \
    -H "$(auth_header)" \
    -H 'Accept: application/vnd.github+json' \
    -H 'X-GitHub-Api-Version: 2022-11-28' \
    "${API}/${run_id}" >/dev/null
}

echo "Listing failed runs for ${REPO} ..."
mapfile -t RUN_IDS < <(list_failed_ids | sort -u)

if [[ "${#RUN_IDS[@]}" -eq 0 ]]; then
  echo "No failed workflow runs to delete."
  exit 0
fi

echo "Deleting ${#RUN_IDS[@]} failed run(s) ..."
deleted=0
for id in "${RUN_IDS[@]}"; do
  if delete_run "${id}"; then
    deleted=$((deleted + 1))
    echo "  deleted run ${id}"
  else
    echo "  failed to delete run ${id}" >&2
  fi
done

echo "Done. Deleted ${deleted}/${#RUN_IDS[@]} failed run(s)."
