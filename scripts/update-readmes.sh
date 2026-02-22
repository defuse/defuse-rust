#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Downloading vfy README..."
curl -sS -o static/markdown/vfy-readme.md \
    https://raw.githubusercontent.com/defuse/vfy/master/README.md

echo "Downloading passgenr README..."
curl -sS -o static/markdown/passgenr-readme.md \
    https://raw.githubusercontent.com/defuse/passgenr/master/README.md

echo "Done."
