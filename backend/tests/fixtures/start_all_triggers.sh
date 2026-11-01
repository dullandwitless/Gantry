#!/usr/bin/env bash
# Starts (or stops) all external services needed for the trigger e2e tests.
#
# Usage:
#   ./tests/fixtures/start_all_triggers.sh        # start everything
#   ./tests/fixtures/start_all_triggers.sh stop   # stop everything
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
ACTION="${1:-start}"

SCRIPTS=(
    "$DIR/start_mqtt.sh"
    "$DIR/start_websocket.sh"
    "$DIR/start_postgres_replication.sh"
)

if [[ "$ACTION" == "stop" ]]; then
    for s in "${SCRIPTS[@]}"; do
        echo "--- $(basename "$s" .sh) stop ---"
        bash "$s" stop
    done
    exit 0
fi

for s in "${SCRIPTS[@]}"; do
    echo "--- $(basename "$s" .sh) ---"
    bash "$s"
    echo ""
done

echo "============================================"
echo "All services ready. Run the e2e tests with:"
echo ""
echo "  cargo test --test trigger_e2e --features mqtt_trigger,websocket,postgres_trigger -- --ignored --nocapture"
