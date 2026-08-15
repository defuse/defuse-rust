#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Downloading vfy README..."
curl -sS -o static/markdown/vfy-readme.md \
    https://raw.githubusercontent.com/defuse/vfy/master/README.md

echo "Downloading passgenr README..."
curl -sS -o static/markdown/passgenr-readme.md \
    https://raw.githubusercontent.com/defuse/passgenr/master/README.md

echo "Downloading crackstation-hashdb README..."
curl -sS -o static/markdown/crackstation-hashdb-readme.md \
    https://raw.githubusercontent.com/defuse/crackstation-hashdb/master/README.md

echo "Downloading dawr README..."
curl -sS -o static/markdown/dawr-readme.md \
    https://raw.githubusercontent.com/defuse/dawr/master/README.md

echo "Downloading wavetool README..."
curl -sS -o static/markdown/wavetool-readme.md \
    https://raw.githubusercontent.com/defuse/wavetool/master/README.md

echo "Downloading claude-statusline README..."
curl -sS -o static/markdown/claude-statusline-readme.md \
    https://raw.githubusercontent.com/defuse/claude-statusline/main/README.md

echo "Downloading auditician README..."
curl -sS -o static/markdown/auditician-readme.md \
    https://raw.githubusercontent.com/defuse/auditician/main/README.md

echo "Done."
