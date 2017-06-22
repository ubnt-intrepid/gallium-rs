#!/bin/bash
set -euo pipefail

script_root="$(cd $(dirname $BASH_SOURCE); pwd)"
$script_root/cargo.sh install --force --root /opt/gallium
docker-compose restart gallium
