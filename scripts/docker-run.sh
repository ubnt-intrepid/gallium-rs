#!/bin/bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage:"
  echo "  $0 <command> [<args>...]"
  echo ""
  echo "Example:"
  echo "  $0 diesel setup"
  exit 1
fi

container_id="$(docker-compose ps -q dev)"
script_root="$(cd $(dirname $BASH_SOURCE); pwd)"

docker exec -it "$container_id" "$1" "${@:2}"
