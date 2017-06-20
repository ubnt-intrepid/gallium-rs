#!/bin/bash
set -euo pipefail

database_url="${DATABASE_URL:-postgres://postgres@db:5432/gallium}"

exec /opt/bin/show_pubkeys --database-url "${database_url}"
