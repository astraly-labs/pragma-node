#!/bin/bash
set -e

DB_NAME="pragma"
CLICKHOUSE_HOST="localhost"
CLICKHOUSE_PORT="9000"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --database|-d)
            DB_NAME="$2"
            shift 2
            ;;
        --host|-h)
            CLICKHOUSE_HOST="$2"
            shift 2
            ;;
        --port|-p)
            CLICKHOUSE_PORT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./init.sh [--database <db>] [--host <host>] [--port <port>]"
            exit 1
            ;;
    esac
done

echo "Checking that the migration tool is installed"
if ! command -v goose >/dev/null 2>&1; then
    echo "goose is missing, please install it:"
    echo "  go install github.com/pressly/goose/v3/cmd/goose@latest"
    exit 1
fi

echo "Checking that the ClickHouse client is installed"
if ! command -v clickhouse-client >/dev/null 2>&1; then
    echo "clickhouse-client is missing, please install it:"
    echo "  brew install clickhouse (macOS)"
    echo "  apt-get install clickhouse-client (Debian/Ubuntu)"
    exit 1
fi

echo "Initializing ClickHouse database: $DB_NAME"

# Create database
clickhouse-client --host="$CLICKHOUSE_HOST" --port="$CLICKHOUSE_PORT" \
    --query="CREATE DATABASE IF NOT EXISTS $DB_NAME"

# Run migrations
echo "Running migrations..."
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
goose -dir "$script_dir" clickhouse "tcp://$CLICKHOUSE_HOST:$CLICKHOUSE_PORT/$DB_NAME" up

echo "ClickHouse initialization complete!"
