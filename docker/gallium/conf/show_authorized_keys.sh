#!/bin/bash
set -euo pipefail

source /opt/gallium/.env
exec /opt/gallium/bin/pubkey show --database-url "${DATABASE_URL}"
