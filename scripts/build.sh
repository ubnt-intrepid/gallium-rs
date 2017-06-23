#!/bin/bash
set -euo pipefail

script_root="$(cd $(dirname $BASH_SOURCE); pwd)"

sudo rsync -C --filter=":- .dockerignore" -acz --delete "$script_root/.." "$script_root/../data/source"
$script_root/cargo.sh install --force --root /opt/gallium

docker-compose restart gallium
