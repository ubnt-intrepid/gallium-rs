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

docker-compose exec dev "$1" "${@:2}"
