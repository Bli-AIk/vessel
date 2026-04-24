#!/usr/bin/env bash
# tokei_check.sh — Lint for code quality: max total lines + max code lines + no mod.rs files
# Usage:
#   ./tokei_check.sh [max_total_lines] [max_code_lines] [search_dir]
#   ./tokei_check.sh --workspace [max_total_lines] [max_code_lines]
#
#   max_total_lines — maximum allowed total lines per Rust file (default: 800)
#   max_code_lines  — maximum allowed Rust code lines per file via tokei (default: 500)
#   search_dir      — directory to scan in local mode (default: src tests)
#
# Local mode checks the current repository.
# Workspace mode is accepted for compatibility and behaves the same in Vessel.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

workspace_mode_requested() {
    [[ "${1:-}" == "--workspace" || "${1:-}" == "workspace" ]]
}

if workspace_mode_requested "${1:-}"; then
    shift
fi

# Colors (only if stdout is a terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    CYAN=''
    BOLD=''
    RESET=''
fi

MAX_TOTAL_LINES="${1:-800}"
MAX_CODE_LINES="${2:-500}"
SEARCH_PATHS="${3:-src tests}"

errors=0

readarray -t SEARCH_DIRS < <(printf '%s\n' "$SEARCH_PATHS" | tr ' ' '\n' | sed '/^$/d')

build_find_roots() {
    local roots=""
    for dir in "${SEARCH_DIRS[@]}"; do
        if [ -d "$REPO_ROOT/$dir" ]; then
            roots="$roots \"$dir\""
        fi
    done
    printf '%s' "$roots"
}

FIND_ROOTS="$(build_find_roots)"

if [ -z "$FIND_ROOTS" ]; then
    echo -e "${YELLOW}${BOLD}Warning:${RESET} No search directories found for tokei checks."
    exit 0
fi

# --- Check 1: No mod.rs files (Rust 2018+ module style) ---
mod_files=$(eval "cd '$REPO_ROOT' && find $FIND_ROOTS -name 'mod.rs' -type f -not -path '*/examples/*' -print" 2>/dev/null || true)
if [ -n "$mod_files" ]; then
    echo -e "${RED}${BOLD}Error:${RESET} Found mod.rs files. Use Rust 2018+ module naming instead:"
    echo "$mod_files" | while read -r f; do echo -e "  ${YELLOW}$f${RESET}"; done
    errors=1
fi

# --- Check 2: No Rust file exceeds max total lines ---
rust_files=$(eval "cd '$REPO_ROOT' && find $FIND_ROOTS -path '*/target' -prune -o -path '*/examples' -prune -o -name '*.rs' -type f -print" 2>/dev/null || true)
if [ -n "$rust_files" ]; then
    while IFS= read -r file; do
        [ -z "$file" ] && continue
        lines=$(wc -l < "$REPO_ROOT/$file")
        if [ "$lines" -gt "$MAX_TOTAL_LINES" ]; then
            echo -e "${RED}${BOLD}Error:${RESET} ${YELLOW}$file${RESET} has ${CYAN}$lines${RESET} total lines (max ${CYAN}$MAX_TOTAL_LINES${RESET})"
            errors=1
        fi
    done <<< "$rust_files"
fi

# --- Check 3: No Rust file exceeds max code lines (via tokei) ---
tokei_report=$(cd "$REPO_ROOT" && tokei $SEARCH_PATHS --output json --files 2>/dev/null || true)
if [ -n "$tokei_report" ]; then
    over_code_limit=$(printf '%s' "$tokei_report" | jq -r --argjson max "$MAX_CODE_LINES" '
        .Rust.reports[]?
        | select((.name | contains("/target/") | not) and (.name | contains("/examples/") | not))
        | select(.stats.code > $max)
        | "\(.name)\t\(.stats.code)"
    ' 2>/dev/null || true)
    if [ -n "$over_code_limit" ]; then
        while IFS=$'\t' read -r file code_lines; do
            [ -z "$file" ] && continue
            echo -e "${RED}${BOLD}Error:${RESET} ${YELLOW}$file${RESET} has ${CYAN}$code_lines${RESET} lines of code via tokei (max ${CYAN}$MAX_CODE_LINES${RESET})"
            errors=1
        done <<< "$over_code_limit"
    fi
fi

if [ "$errors" -ne 0 ]; then
    exit 1
else
    echo -e "${GREEN}${BOLD}Tokei OK:${RESET} All Rust files under ${CYAN}$MAX_TOTAL_LINES${RESET} total lines and ${CYAN}$MAX_CODE_LINES${RESET} lines of code, no mod.rs found."
fi

# --- Check 4: No allow(clippy::...) anywhere — use clippy.toml for global config ---
allow_hits=$(grep -rn 'allow(clippy::' "$REPO_ROOT" --include="*.rs" "$REPO_ROOT/src" "$REPO_ROOT/tests" 2>/dev/null || true)
if [ -n "$allow_hits" ]; then
    echo -e "${RED}${BOLD}Error:${RESET} Found allow(clippy::...). Use clippy.toml for global config or #[expect] for individual cases:"
    echo "$allow_hits" | while read -r line; do echo -e "  ${YELLOW}$line${RESET}"; done
    errors=1
fi

# --- Check 5: #[expect(clippy::...)] must have a // reason: comment ---
expect_no_reason=$(grep -rl '#\[expect(clippy::' "$REPO_ROOT/src" "$REPO_ROOT/tests" --include="*.rs" 2>/dev/null | \
    xargs -r awk '
    prev_expect && FILENAME != prev_file {
        print prev_loc ": " prev_line
        prev_expect = 0
    }
    prev_expect {
        if (/\/\/ reason:/) { prev_expect = 0; next }
        print prev_loc ": " prev_line
        prev_expect = 0
    }
    /#\[expect\(clippy::/ {
        if (/\/\/ reason:/) next
        prev_expect = 1; prev_file = FILENAME; prev_loc = FILENAME ":" FNR; prev_line = $0
    }
    END { if (prev_expect) print prev_loc ": " prev_line }
    ' 2>/dev/null || true)
if [ -n "$expect_no_reason" ]; then
    echo -e "${RED}${BOLD}Error:${RESET} Found #[expect(clippy::...)] without // reason: comment:"
    echo "$expect_no_reason" | while read -r line; do echo -e "  ${YELLOW}$line${RESET}"; done
    errors=1
fi

if [ "$errors" -ne 0 ]; then
    exit 1
else
    echo -e "${GREEN}${BOLD}Lint OK:${RESET} No #[allow(clippy::...)] found, all #[expect] have reasons."
fi
