#!/usr/bin/env bash
# Builds Oche and installs it for the current user (~/.local), so Walker can find it.
set -euo pipefail
cd "$(dirname "$0")/.."

npx tauri build --no-bundle

install -Dm755 src-tauri/target/release/oche "$HOME/.local/bin/oche"
install -Dm644 packaging/oche.desktop "$HOME/.local/share/applications/oche.desktop"
install -Dm644 packaging/icon.svg "$HOME/.local/share/icons/hicolor/scalable/apps/oche.svg"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true

echo "Zainstalowano: ~/.local/bin/oche"
