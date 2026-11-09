#!/usr/bin/env bash
#
# AeroFyl dev setup
# Clones or updates the AeroFyl repo and installs its dependencies.
#
# Usage: ./install-dev.sh [-h]
#   -h, --help   show this help
#
# Env:
#   AEROFYL_INSTALL_DIR   folder to put the repo in (default: current directory)

set -euo pipefail

AEROFYL_REPO="https://github.com/AeroForger/AeroFyl"
INSTALL_DIR="${AEROFYL_INSTALL_DIR:-$PWD}"

for arg in "$@"; do
    case "$arg" in
        -h|--help)
            cat <<'EOF'
Usage: ./install-dev.sh [-h]
  -h, --help   show this help
Env: AEROFYL_INSTALL_DIR  folder to put the repo in (default: current directory)
EOF
            exit 0 ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

# ---------- pretty printing (colors only when output is a terminal) ----------

if [ -t 1 ]; then
    BOLD=$'\033[1m'; RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'
    BLUE=$'\033[34m'; CYAN=$'\033[36m'; DIM=$'\033[2m'; RESET=$'\033[0m'
else
    BOLD=""; RED=""; GREEN=""; YELLOW=""; BLUE=""; CYAN=""; DIM=""; RESET=""
fi

step_num=0
errors=0

banner() {
    printf '\n%s%s' "$CYAN" "$BOLD"
    cat <<'EOF'
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣠⣤⣶⡶⠊⠀⢿⣿⣿⣿⣿⣿⣿⣿⣷⣶⣦⣤⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣤⣶⣿⣿⣿⡿⠋⠀⠀⡇⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣴⣾⣿⣿⣿⣿⡿⠋⠀⠀⠀⣸⣧⠘⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣾⣿⣿⣿⣿⣿⣿⠟⠁⠀⠀⠀⢠⣿⣿⠀⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣾⣿⣿⣿⣿⣿⣿⡿⠁⠀⠀⠀⠀⠀⣾⣿⣿⡇⠸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣼⣿⣿⣿⣿⣿⣿⣿⠏⠀⠀⠀⠀⠀⠀⣸⣿⣿⣿⣿⡀⢻⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⣿⣿⣿⡿⠁⠀⠀⠀⠀⠀⠀⢰⣿⣿⣿⣿⣿⣧⠈⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⣿⣿⣿⠟⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⣿⣿⣿⣆⠘⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⣤⣴⣶⣿⡆⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⣿⣿⣿⠋⠀⠀⠀⠀⠀⠀⠀⢀⣾⣿⣿⣿⣿⣿⣿⣿⣿⣆⠘⣿⠿⠿⠟⣛⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡀⠀⠀⣀⣠⣤⣴⣶⣿⣿⣿⠟⣫⡾⠋⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⣿⣿⣿⣿⣿⡿⠁⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⣿⣿⣿⣿⣿⣿⠿⢛⣩⣴⠖⠈⣠⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣾⣿⣿⣿⣿⣿⣿⠟⢋⣤⣾⠟⠁⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣸⣿⣿⣿⣿⣿⡟⠁⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⣿⣿⣿⡿⠟⣩⣴⣾⡿⠋⠀⣠⣾⣿⣿⣿⣿⣿⠿⠿⢟⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⢟⠋⣢⣴⣿⠟⠁⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⣿⣿⣿⣿⠟⠀⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⣿⣿⠟⣩⣴⣿⣿⣿⠟⠁⠀⣴⣿⣿⡿⠟⣋⣥⠖⢊⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠿⣛⠵⢊⣥⣾⣿⠟⠁⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⠏⠀⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⡿⢋⣴⣾⣿⣿⣿⡿⠋⠀⢀⣾⡿⠟⣩⣴⣾⡿⠃⢠⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠟⣛⣭⠖⢋⣡⣶⣿⣿⠟⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⣿⣿⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⣼⡿⢋⣴⣿⣿⣿⣿⣿⡟⠁⠀⢀⡾⢋⣴⣾⣿⣿⠟⠀⢠⣿⣿⣿⣿⣿⣿⣿⣿⣿⠿⢟⣛⣭⣵⡾⠟⠉⣠⣴⣿⣿⣿⡿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⡼⢋⣴⣿⣿⣿⣿⣿⣿⡟⠀⠀⠀⢊⣴⣿⣿⣿⣿⠏⠀⠠⠿⠿⢛⣛⣛⣋⣭⣭⣶⣶⣿⡿⠟⠋⢁⣠⣶⣿⣿⣿⣿⡿⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣾⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⢀⢈⣴⣿⣿⣿⣿⣿⣿⣿⠏⠀⠀⢀⣴⣿⣿⣿⣿⣿⡏⠀⢠⣿⣿⣿⣿⣿⣿⣿⣿⠿⠟⠋⠁⣀⣤⣾⣿⣿⣿⣿⣿⣿⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣿⣿⣿⣿⣿⣿⣿⣿⡟⠀⠀⣴⣿⣿⣿⣿⣿⣿⣿⠀⣰⣿⣿⣿⣿⠿⠟⠋⠉⠀⢀⣠⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣰⣿⠇⠀⠀⠀⠀⠀⠀⠀⠀⢠⣾⣿⣿⣿⣿⣿⣿⣿⣿⡿⠀⢠⣾⣿⣿⣿⣿⣿⣿⣿⡇⠰⠟⠛⠉⠁⠀⠀⣀⣤⣴⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⠏⠀⠀⠀⠀⠀⠀⠀⢀⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠃⣰⣿⣿⠿⠿⠛⠛⠉⠉⠀⠀⠀⠀⢀⣀⣤⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣿⡏⠀⠀⠀⠀⠀⠀⠀⣠⣿⣿⣿⣿⣿⣿⡿⠿⠿⠛⠛⠋⠈⠁⠀⠀⠀⠀⠀⠀⠀⠀⣀⣤⣴⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⣾⡟⠀⠀⠀⠀⠀⠀⢀⣾⣿⠿⠛⠋⠉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⣤⣶⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⣸⡿⠀⠀⠀⠀⠀⢀⠴⠋⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⣤⣴⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⢰⣿⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⣤⣤⣶⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⢠⣿⠃⠀⠀⠀⠀⠀⠀⠀⢀⣀⣤⣴⣶⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⢀⣾⡏⠀⠀⠀⢀⣠⣴⣶⣿⣿⣿⡿⠟⠋⠁⠈⠻⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⣼⡿⠀⢀⣴⣾⣿⣿⣿⠿⠛⠉⠁⠀⠀⠀⠀⠀⠀⠀⠙⠻⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠟⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⣸⣿⣣⣶⣿⣿⠿⠛⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠿⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠟⠋⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⢿⣿⠿⠛⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠙⠛⠛⠿⠿⠿⠿⠿⠿⠿⠿⠿⠛⠛⠋⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
EOF
    printf '%s\n' "$RESET"
    printf '  %sdev setup%s\n' "$DIM" "$RESET"
}
step() {
    step_num=$((step_num + 1))
    printf '\n%s[%d]%s %s%s%s\n' "$BLUE$BOLD" "$step_num" "$RESET" "$BOLD" "$1" "$RESET"
}

ok()   { printf '  %s✔%s %s\n' "$GREEN" "$RESET" "$1"; }
skip() { printf '  %s○%s %s\n' "$YELLOW" "$RESET" "$1"; }
note() { printf '  %s%s%s\n' "$DIM" "$1" "$RESET"; }
fail() { printf '  %s✘ %s%s\n' "$RED" "$1" "$RESET" >&2; errors=$((errors + 1)); }
have() { command -v "$1" >/dev/null 2>&1; }

summary() {
    echo
    if [ "$errors" -eq 0 ]; then
        printf '%sAll done.%s You are ready to code on AeroFyl.\n\n' "$GREEN$BOLD" "$RESET"
    else
        printf '%sFinished with %d error(s).%s Check the output above.\n\n' "$RED$BOLD" "$errors" "$RESET"
    fi
}

# ---------- helpers ----------

# sync_repo <url> <dir>: pull if already cloned, otherwise clone.
sync_repo() {
    local url=$1 dir=$2 name
    name=$(basename "$dir")
    if [ -d "$dir/.git" ]; then
        if git -C "$dir" pull --ff-only --quiet; then
            ok "$name updated"
        else
            fail "could not update $name (uncommitted or diverged changes?)"
        fi
    elif [ -e "$dir" ]; then
        fail "$dir exists but is not a git repo"
    elif git clone --quiet "$url" "$dir"; then
        ok "$name cloned"
    else
        fail "could not clone $url"
    fi
}

# npm_install <dir>: use npm ci when there is a lockfile, otherwise npm install.
npm_install() {
    if [ -f "$1/package-lock.json" ]; then
        (cd "$1" && npm ci --no-audit --no-fund)
    else
        (cd "$1" && npm install --no-audit --no-fund)
    fi
}

# install_deps <dir>: install every dependency set the repo declares.
install_deps() {
    local dir=$1 found=0

    if [ ! -d "$dir" ]; then
        skip "$dir is missing, skipping dependencies"
        return
    fi

    if [ -f "$dir/package.json" ]; then
        found=1
        if ! have npm; then
            skip "package.json found but npm is missing, install Node.js and run npm install later"
        elif npm_install "$dir"; then
            ok "npm dependencies installed"
        else
            fail "npm install failed in $dir"
        fi
    fi

    if [ -f "$dir/Cargo.toml" ]; then
        found=1
        if ! have cargo; then
            skip "Cargo.toml found but cargo is missing, install Rust later"
        elif (cd "$dir" && cargo fetch); then
            ok "rust dependencies fetched"
        else
            fail "cargo fetch failed in $dir"
        fi
    fi

    if [ -f "$dir/requirements.txt" ]; then
        found=1
        if ! have python3; then
            skip "requirements.txt found but python3 is missing, install it later"
        # A venv avoids the "externally managed environment" error on Arch and other distros.
        elif python3 -m venv "$dir/.venv" && "$dir/.venv/bin/pip" install --quiet -r "$dir/requirements.txt"; then
            ok "python dependencies installed in $dir/.venv"
        else
            fail "pip install failed in $dir"
        fi
    fi

    if [ -f "$dir/Makefile" ]; then
        found=1
        skip "Makefile found, check it for build targets (make -C $dir)"
    fi

    if [ "$found" -eq 0 ]; then
        skip "no known dependency file in $dir, see its README for build steps"
    fi
}

# ---------- run ----------

banner

step "Checking prerequisites"

if ! have git; then
    fail "git is not installed, install it first (https://git-scm.com)"
    summary
    exit 1
fi
ok "git found"

step "Fetching repository"

mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"
sync_repo "$AEROFYL_REPO" "$INSTALL_DIR/AeroFyl"

step "Installing AeroFyl dependencies"

install_deps "$INSTALL_DIR/AeroFyl"

summary

if [ "$errors" -ne 0 ]; then
    exit 1
fi