#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Downloading vfy README..."
curl -sS -o static/markdown/vfy-readme.md \
    https://raw.githubusercontent.com/defuse/vfy/master/README.md

echo "Done."
