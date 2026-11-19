#!/usr/bin/env bash
# Build Open Mining and install it into an Open CAD Studio plugins folder,
# exactly as the Plugin Manager would (<plugins>/opencad.mining/{plugin.toml,lib}).
#
#   scripts/dev-install.sh                 # Linux build -> ~/.config/OpenCADStudio/plugins (WSL/WSLg)
#   scripts/dev-install.sh --windows       # Windows .dll -> %APPDATA%\OpenCADStudio\plugins (from WSL)
#   OCS_PLUGINS_DIR=/tmp/p scripts/dev-install.sh   # any folder (isolated tests)
#
# Restart Open CAD Studio afterwards (the runner keeps the library loaded).
set -euo pipefail
cd "$(dirname "$0")/.."
ID=opencad.mining

if [[ "${1:-}" == "--windows" ]]; then
  TARGET=x86_64-pc-windows-gnu
  rustup target list --installed | grep -q "$TARGET" || rustup target add "$TARGET"
  command -v x86_64-w64-mingw32-gcc >/dev/null || {
    echo "Missing MinGW linker. Install it once with:  sudo apt install -y mingw-w64" >&2; exit 1; }
  cargo build -p opencad_mining --release --target "$TARGET"
  LIB="target/$TARGET/release/opencad_mining.dll"
  APPDATA_WIN=$(cmd.exe /c "echo %APPDATA%" 2>/dev/null | tr -d '\r')
  DEST="${OCS_PLUGINS_DIR:-$(wslpath "$APPDATA_WIN")/OpenCADStudio/plugins}/$ID"
  ASSET="$ID-windows-x86_64.dll"
else
  cargo build -p opencad_mining --release
  LIB="target/release/libopencad_mining.so"
  DEST="${OCS_PLUGINS_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/OpenCADStudio/plugins}/$ID"
  ASSET="$ID-linux-x86_64.so"
fi

mkdir -p "$DEST"
# Remove a previous build (any name) so only one library is found.
find "$DEST" -maxdepth 1 -type f \( -name '*.so' -o -name '*.dll' -o -name '*.dylib' \) -delete
cp "$LIB" "$DEST/$ASSET"
cp plugin.toml "$DEST/plugin.toml"
echo "Installed $ASSET -> $DEST"
