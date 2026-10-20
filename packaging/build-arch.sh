#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_DIR="$ROOT_DIR/target/dist"

mkdir -p "$OUT_DIR"
cd "$SCRIPT_DIR"

PKGDEST="$OUT_DIR" makepkg -f -d --noconfirm
rm -f "$OUT_DIR"/*debug*.pkg.tar.zst
