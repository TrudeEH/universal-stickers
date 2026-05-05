#!/usr/bin/env bash
set -euo pipefail

no_run=0

for arg in "$@"; do
    case "$arg" in
        --no-run)
            no_run=1
            ;;
        -h|--help)
            echo "Usage: ./run.sh [--no-run]"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: ./run.sh [--no-run]" >&2
            exit 2
            ;;
    esac
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app_path="$repo_root/target/debug/universal-stickers"

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        if [[ "$1" == "cargo" ]]; then
            echo "Install Rust from https://rustup.rs/ or your distro packages." >&2
            exit 1
        fi

        echo "On Ubuntu/Debian, install the common build dependencies with:" >&2
        echo "  sudo apt-get install build-essential libadwaita-1-dev libgtk-4-dev pkg-config" >&2
        exit 1
    fi
}

require_command cargo

cargo build -p universal-stickers

if [[ "$no_run" -eq 0 ]]; then
    "$app_path"
fi
