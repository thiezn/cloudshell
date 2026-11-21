#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
site_dir="$(cd -- "$script_dir/../site" && pwd)"

if ! command -v zola >/dev/null 2>&1; then
  printf 'Zola is required to develop the documentation site. Install it with: brew install zola\n' >&2
  exit 1
fi

cd "$site_dir"
zola check --skip-external-links
exec zola serve --interface 127.0.0.1 --port "${ZOLA_PORT:-1111}"
