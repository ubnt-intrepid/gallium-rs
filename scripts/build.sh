#!/bin/bash
set -euo pipefail
docker-compose run --rm dev cargo install --force --root /opt/gallium
