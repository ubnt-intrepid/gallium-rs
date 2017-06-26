#!/bin/bash
set -euo pipefail
container_hash=$(docker-compose ps -q db)
docker exec -it "$container_hash" psql -U postgres -b gallium
