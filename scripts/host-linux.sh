#!/bin/bash
# Host the games on a Linux machine (a desktop, a home server or a VPS): Basin Battle and Elwynn
# Survival, each on its own realm. Only the server is built here, no client, so it runs headless.
#   scripts/host-linux.sh                         password "hold"
#   scripts/host-linux.sh --password secret       pick the password everyone uses
#   scripts/host-linux.sh --public-host 1.2.3.4   your internet address (found automatically otherwise)
#   WOW_DATA=/srv/wow/Data scripts/host-linux.sh  where the 1.12.1 Data folder is, without asking
# The first run installs Rust if it's missing and builds the server (5-10 minutes); later runs
# rebuild only what changed. Every argument goes to the server as given (see scripts/host.sh).
set -euo pipefail
. "$(dirname "$0")/common.sh"
cd "$ROOT"

if ! command -v cc >/dev/null 2>&1; then
    echo "A C compiler and linker are needed to build the server. Install them, then run this again:"
    echo "  Debian/Ubuntu:  sudo apt install build-essential pkg-config curl"
    echo "  Fedora:         sudo dnf install gcc pkgconf-pkg-config curl"
    echo "  Arch:           sudo pacman -S base-devel curl"
    exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "Installing Rust (once; a few minutes)..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
fi
if [ -z "${WOW_DATA:-}" ] && [ ! -t 0 ]; then
    echo "No WoW 1.12.1 Data folder saved and no terminal to ask in: set WOW_DATA=/path/to/Data." >&2
    exit 1
fi
echo "Open TCP ports 3724, 8085 and 8086 in this machine's firewall (e.g. sudo ufw allow 3724,8085,8086/tcp)"
echo "and forward them on your router if friends join over the internet."
exec "$ROOT/scripts/host.sh" "$@"
