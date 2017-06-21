#!/bin/bash
set -euo pipefail
docker-compose exec dev cargo install --force --root /opt/gallium
