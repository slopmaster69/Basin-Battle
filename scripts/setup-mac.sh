#!/bin/bash
# One-time setup on a Mac: installs the tools, downloads the benilla client and builds everything.
# Safe to run again; it picks up where it left off. Takes 10-30 minutes the first time.
set -euo pipefail
. "$(dirname "$0")/common.sh"
cd "$ROOT"

if [ "$(uname)" = "Darwin" ] && ! xcode-select -p >/dev/null 2>&1; then
    echo "Installing Apple's command line tools (a window will open)..."
    xcode-select --install || true
    echo "When that installer finishes, run this script again."
    exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
fi
if [ ! -d benilla/.git ]; then
    echo "Downloading the benilla client..."
    git clone "$BENILLA_REPO" benilla
fi
git -C benilla fetch -q origin

find_wow_data
echo "Game data: $WOW_DATA"

echo "Building the Basin Battle server..."
cargo build --release -p htl-server
echo "Building the benilla client (the long part)..."
rm -f benilla/.htl-patches
build_benilla_if_needed
echo
echo "All set. Start a game with:  scripts/host.sh"
echo "Then play with:              scripts/play.sh"
