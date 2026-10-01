#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."
ROOT="$(pwd)"

pkill -f "${ROOT}/.*astro" || true
pkill -f "${ROOT}/target/debug/qafiyah-api" || true
