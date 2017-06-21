#!/bin/bash
set -euo pipefail

script_root="$(cd $(dirname $BASH_SOURCE); pwd)"
$script_root/docker-run.sh cargo "$@"
