#!/usr/bin/env bash
set -euo pipefail

./scripts/dev/compose.sh run --rm --build --no-deps -e SEARCH_INDEXER_FORCE=true search-indexer "$@"
