#!/bin/bash
# Download the latest KaTeX release and update static/katex/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
KATEX_DIR="$PROJECT_DIR/static/katex"
TMP_DIR=$(mktemp -d)

trap 'rm -rf "$TMP_DIR"' EXIT

# Get latest version from GitHub
LATEST=$(curl -sI "https://github.com/KaTeX/KaTeX/releases/latest" | grep -i '^location:' | sed 's|.*/tag/||' | tr -d '\r')
echo "Latest KaTeX version: $LATEST"

# Download and extract
echo "Downloading..."
curl -sL "https://github.com/KaTeX/KaTeX/releases/download/${LATEST}/katex.tar.gz" -o "$TMP_DIR/katex.tar.gz"
tar xzf "$TMP_DIR/katex.tar.gz" -C "$TMP_DIR"

# Update static files
echo "Updating static/katex/..."
if [[ "$KATEX_DIR" != */static/katex ]]; then
    echo "ERROR: KATEX_DIR doesn't end with static/katex: $KATEX_DIR" >&2
    exit 1
fi
rm -rf "$KATEX_DIR"
mkdir -p "$KATEX_DIR/contrib" "$KATEX_DIR/fonts"
cp "$TMP_DIR/katex/katex.min.css" "$KATEX_DIR/"
cp "$TMP_DIR/katex/katex.min.js" "$KATEX_DIR/"
cp "$TMP_DIR/katex/contrib/auto-render.min.js" "$KATEX_DIR/contrib/"
cp "$TMP_DIR/katex/fonts/"* "$KATEX_DIR/fonts/"

echo "Done. Updated to $LATEST"
