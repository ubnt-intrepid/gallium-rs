#!/bin/bash
set -euo pipefail

script_dir="$(cd $(dirname $BASH_SOURCE); pwd)"
dst="$script_dir/../src/schema.rs"

echo "[Generating schema...]"
schema="$(docker-compose run --rm gallium diesel print-schema)"

echo "[Writing schema to schema.rs...]"

rm -f $dst
echo > "$dst"  "// This file is automatically generated by diesel_cli."
echo >> "$dst" ""
echo >> "$dst" "$schema"
