#!/usr/bin/env bash
#
# AeroFyl dev setup
# Clones the main repo, the vscode extension repo,
# and installs the extension in VS Code if it is installed.

set -euo pipefail

# ---------- pretty printing ----------

BOLD=$'\033[1m'
GREEN=$'\033[32m'
YELLOW=$'\033[33m'
BLUE=$'\033[34m'
CYAN=$'\033[36m'
RED=$'\033[31m'
DIM=$'\033[2m'
RESET=$'\033[0m'

step_num=0
errors=0

banner() {
    echo
    echo "${CYAN}${BOLD}"
    cat <<'EOF'
      ___
     /   \    AeroFyl dev setup
    | o o |   aeroFyl + vscode extension
     \ ~ /    one script, everything installed
      ---
EOF
    echo "${RESET}"
}

step() {
    step_num=$((step_num + 1))
    echo
    echo "${BLUE}${BOLD}[${step_num}]${RESET} ${BOLD}$1${RESET}"
}

ok() {
    echo "  ${GREEN}✔${RESET} $1"
}

skip() {
    echo "  ${YELLOW}○${RESET} $1"
}

fail() {
    echo "  ${RED}✘ $1${RESET}" >&2
    errors=$((errors + 1))
}

summary() {
    echo
    if [ "$errors" -eq 0 ]; then
        echo "${GREEN}${BOLD}All done.${RESET} You are ready to code on AeroFyl."
    else
        echo "${RED}${BOLD}Finished with ${errors} error(s).${RESET} Check the output above."
    fi
    echo
}

ask() {
    # ask "question" -> returns 0 if yes, 1 if no
    local answer
    read -r -p "$(echo "${YELLOW}?${RESET} $1 [y/N] ")" answer
    [[ "$answer" =~ ^[Yy]$ ]]
}

# ---------- config ----------

AEROFYL_REPO="https://github.com/AeroForger/AeroFyl"
VSCODE_REPO="https://github.com/AeroForger/AeroFyl-vscode"

# install location: current directory by default
INSTALL_DIR="${AEROFYL_INSTALL_DIR:-$(pwd)}"

# ---------- preflight ----------

banner

step "Checking prerequisites"

if ! command -v git >/dev/null 2>&1; then
    fail "git is not installed. Install it first (https://git-scm.com)."
    summary
    exit 1
fi
ok "git found"

VS_CODE=""
if command -v code >/dev/null 2>&1; then
    VS_CODE="code"
elif command -v codium >/dev/null 2>&1; then
    VS_CODE="codium"
fi

if [ -n "$VS_CODE" ]; then
    ok "VS Code found ($VS_CODE)"
else
    skip "VS Code not found, will skip the extension install"
fi

if [ -d "$INSTALL_DIR/AeroFyl" ]; then
    skip "AeroFyl folder already exists in $INSTALL_DIR, git pull instead of clone"
    PULL_AEROFYL=1
else
    PULL_AEROFYL=0
fi

if [ -d "$INSTALL_DIR/AeroFyl-vscode" ]; then
    skip "AeroFyl-vscode folder already exists in $INSTALL_DIR, git pull instead of clone"
    PULL_VSCODE=1
else
    PULL_VSCODE=0
fi

# ---------- AeroFyl main repo ----------

step "AeroFyl main repo"

cd "$INSTALL_DIR"

if [ "$PULL_AEROFYL" -eq 1 ]; then
    if (cd AeroFyl && git pull --ff-only); then
        ok "AeroFyl updated"
    else
        fail "could not update AeroFyl (local commits may conflict)"
    fi
else
    if git clone "$AEROFYL_REPO"; then
        ok "AeroFyl cloned into $INSTALL_DIR/AeroFyl"
    else
        fail "could not clone $AEROFYL_REPO"
    fi
fi

# ---------- vscode extension repo ----------

step "AeroFyl vscode extension repo"

if [ "$PULL_VSCODE" -eq 1 ]; then
    if (cd AeroFyl-vscode && git pull --ff-only); then
        ok "AeroFyl-vscode updated"
    else
        fail "could not update AeroFyl-vscode (local commits may conflict)"
    fi
else
    if git clone "$VSCODE_REPO"; then
        ok "AeroFyl-vscode cloned into $INSTALL_DIR/AeroFyl-vscode"
    else
        fail "could not clone $VSCODE_REPO"
    fi
fi

# ---------- build AeroFyl if there is a known setup ----------

step "AeroFyl project setup"

if [ -f AeroFyl/package.json ]; then
    if command -v npm >/dev/null 2>&1; then
        echo "  ${DIM}found package.json, running npm install${RESET}"
        if (cd AeroFyl && npm install); then
            ok "npm dependencies installed"
        else
            fail "npm install failed"
        fi
    else
        skip "package.json found but npm is missing, run npm install inside AeroFyl later"
    fi
elif [ -f AeroFyl/Makefile ]; then
    echo "  ${DIM}found Makefile, nothing to auto-install${RESET}"
    skip "check the Makefile for build targets"
elif [ -f AeroFyl/Cargo.toml ]; then
    if command -v cargo >/dev/null 2>&1; then
        echo "  ${DIM}found Cargo.toml, running cargo fetch${RESET}"
        if (cd AeroFyl && cargo fetch); then
            ok "rust dependencies fetched"
        else
            fail "cargo fetch failed"
        fi
    else
        skip "Cargo.toml found but cargo is missing, install rust later"
    fi
elif [ -f AeroFyl/requirements.txt ]; then
    if command -v pip3 >/dev/null 2>&1; then
        echo "  ${DIM}found requirements.txt, installing python deps${RESET}"
        if pip3 install -r AeroFyl/requirements.txt; then
            ok "python dependencies installed"
        else
            fail "pip install failed"
        fi
    else
        skip "requirements.txt found but pip3 is missing, install deps later"
    fi
else
    skip "no known dependency file, see README.md in the repo for build steps"
fi

# ---------- vscode extension ----------

step "VS Code extension"

if [ -z "$VS_CODE" ]; then
    skip "VS Code is not installed, skipping extension install"
elif [ ! -d AeroFyl-vscode ]; then
    skip "AeroFyl-vscode folder missing, skipping extension install"
elif [ -f AeroFyl-vscode/package.json ]; then
    if ask "Install the extension ($VS_CODE --install-extension AeroFyl-vscode)?"; then
        (cd AeroFyl-vscode && npm install >/dev/null 2>&1 || true)
        if $VS_CODE --install-extension "$(pwd)/AeroFyl-vscode" 2>/dev/null; then
            ok "extension installed"
        else
            # some repos publish the extension, so try the marketplace id too
            if $VS_CODE --install-extension AeroForger.AeroFyl 2>/dev/null; then
                ok "extension installed from the marketplace"
            else
                skip "could not install automatically, open the folder in VS Code and press F5 to run it"
            fi
        fi
    else
        skip "skipped extension install"
    fi
else
    skip "no package.json in the extension repo, open the folder in VS Code and press F5 to run it"
fi

# ---------- done ----------

summary
