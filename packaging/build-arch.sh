#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION=$(grep -m1 '^version = ' "$ROOT_DIR/Cargo.toml" | cut -d '"' -f2)
TAG="v$VERSION"

TARGET_DIR="${1:-$ROOT_DIR/target/dist}"
mkdir -p "$TARGET_DIR"
OUT_DIR="$(cd "$TARGET_DIR" && pwd)"

cd "$SCRIPT_DIR"

# Build Arch package using packaging/PKGBUILD
PKGDEST="$OUT_DIR" makepkg -f "${@:2}"

# Normalize to arithmetic-v<VERSION>-x86_64.pkg.tar.zst (no -1 suffix)
for f in "$OUT_DIR"/arithmetic-*.pkg.tar.zst; do
    if [[ -f "$f" && "$f" != *"debug"* && "$f" != *"-$TAG-"* ]]; then
        mv "$f" "$OUT_DIR/arithmetic-${TAG}-x86_64.pkg.tar.zst"
    fi
done
rm -f "$OUT_DIR"/*debug*.pkg.tar.zst
