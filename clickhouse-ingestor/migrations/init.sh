#!/bin/bash
set -e

DB_NAME="kafka"
CLICKHOUSE_HOST="localhost"
CLICKHOUSE_PORT="9000"
CLICKHOUSE_USER="default"
CLICKHOUSE_PASSWORD=""
USE_SECURE=""

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
        --user|-u)
            CLICKHOUSE_USER="$2"
            shift 2
            ;;
        --password)
            CLICKHOUSE_PASSWORD="$2"
            shift 2
            ;;
        --secure|-s)
            USE_SECURE="true"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./init.sh [options]"
            echo ""
            echo "Options:"
            echo "  --database, -d    Database name (default: kafka)"
            echo "  --host, -h        ClickHouse host (default: localhost)"
            echo "  --port, -p        ClickHouse port (default: 9000)"
            echo "  --user, -u        ClickHouse user (default: default)"
            echo "  --password        ClickHouse password"
            echo "  --secure, -s      Use secure connection (TLS)"
            echo ""
            echo "Examples:"
            echo "  Local:            ./init.sh --database kafka"
            echo "  ClickHouse Cloud: ./init.sh --host xxx.clickhouse.cloud --port 9440 --user default --password xxx --secure --database kafka"
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

# Build connection string for goose
if [ -n "$USE_SECURE" ]; then
    GOOSE_CONN="tcp://$CLICKHOUSE_HOST:$CLICKHOUSE_PORT/$DB_NAME?username=$CLICKHOUSE_USER&password=$CLICKHOUSE_PASSWORD&secure=true"
else
    GOOSE_CONN="tcp://$CLICKHOUSE_HOST:$CLICKHOUSE_PORT/$DB_NAME?username=$CLICKHOUSE_USER&password=$CLICKHOUSE_PASSWORD"
fi

# Build clickhouse-client arguments
CLIENT_ARGS="--host=$CLICKHOUSE_HOST --port=$CLICKHOUSE_PORT --user=$CLICKHOUSE_USER"
[ -n "$CLICKHOUSE_PASSWORD" ] && CLIENT_ARGS="$CLIENT_ARGS --password=$CLICKHOUSE_PASSWORD"
[ -n "$USE_SECURE" ] && CLIENT_ARGS="$CLIENT_ARGS --secure"

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

echo "Initializing ClickHouse database: $DB_NAME on $CLICKHOUSE_HOST:$CLICKHOUSE_PORT"

# Create database
clickhouse-client $CLIENT_ARGS --query="CREATE DATABASE IF NOT EXISTS $DB_NAME"

# Run migrations with goose
echo "Running migrations..."
goose -dir "$script_dir" clickhouse "$GOOSE_CONN" up

echo "ClickHouse initialization complete!"
