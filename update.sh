#!/bin/sh
# Refresh an existing bench install. Takes the same options as install.sh and
# keeps config.toml, index, parties/, house-rules/, encounters/ and reports/.
#
#   curl -fsSL https://raw.githubusercontent.com/atendev/homebrew-bench/master/update.sh | sh

set -eu

dir=$(CDPATH='' cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || dir=""
if [ -f "$0" ] && [ -n "$dir" ] && [ -f "$dir/install.sh" ]; then
    exec sh "$dir/install.sh" "$@"
fi

url="https://raw.githubusercontent.com/${BENCH_REPO:-atendev/homebrew-bench}/main/install.sh"
tmp=$(mktemp -d 2>/dev/null || mktemp -d -t bench-update)
trap 'rm -rf "$tmp"' EXIT INT TERM
if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$tmp/install.sh" "$url"
elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$tmp/install.sh" "$url"
else
    printf 'update.sh: need curl or wget to download %s\n' "$url" >&2
    exit 1
fi
sh "$tmp/install.sh" "$@"
