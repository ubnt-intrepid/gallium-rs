#!/bin/bash
set -euo pipefail

script_root="$(cd $(dirname $BASH_SOURCE); pwd)"

$script_root/sync_src.sh
$script_root/docker-run.sh cargo install --force --root /opt/gallium

docker-compose restart gallium
