#!/usr/bin/env bash
# Compatibility wrapper: Steam Linux packaging lives in package-steam.sh.
exec "$(cd "$(dirname "$0")" && pwd)/package-steam.sh" "$@"
