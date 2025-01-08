#!/bin/bash
# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check for required commands
for cmd in docker git cargo apibara cargo-watch; do
    if ! command_exists $cmd; then
        echo "Error: $cmd is not installed. Please install it and try again."
        exit 1
    fi
done

# Step 1: Check if services are running and start them if needed
echo "Checking services..."
if ! docker ps --format '{{.Names}}' | grep -q "pragma-node-offchain-db-1\|pragma-node-onchain-db-1\|pragma-node-pragma-kafka\|pragma-node-pragma-zookeeper|pragma-node-pragma-ingestor-1"; then
    echo "Starting services..."
    docker compose -f compose.dev.yaml up -d
else
    echo "Services are already running"
fi

# Step 2: Fill the onchain database
echo "Would you like to run the indexer or use a backup to fill the onchain database? (indexer/backup)"
read -r fill_method

if [ "$fill_method" = "indexer" ]; then
    echo "Preparing to run indexer..."
    if [ ! -d "../indexer-service" ]; then
        echo "Error: ../indexer-service directory not found. Please ensure it exists."
        exit 1
    fi

    # Check if APIBARA_KEY environment variable is set
    if [[ -z "${APIBARA_KEY}" ]]; then
        # If not set, prompt the user for input
        read -p "Enter your Apibara API key: " apibara_api_key
    else
        echo "Env variable APIBARA_KEY is set: using it as your Apibara API key."
        # If set, use the environment variable
        apibara_api_key="${APIBARA_KEY}"
    fi

    # Check if STARTING_BLOCK environment variable is set & valid
    if [[ -n "${STARTING_BLOCK}" ]] && [[ "${STARTING_BLOCK}" =~ ^[1-9][0-9]*$ ]]; then
        starting_block="${STARTING_BLOCK}"
        echo "Using STARTING_BLOCK from environment: ${starting_block}"
    else
        # Fetch the latest block number
        latest_block=$(curl -s --location 'https://mainnet-pragma.karnot.xyz' \
            --header 'Content-Type: application/json' \
            --data '{
            "jsonrpc": "2.0",
            "method": "starknet_blockNumber",         
            "params": [],                    
            "id": 1
        }' | jq '.result')

        # Calculate the starting block
        echo "Latest block: $latest_block"
        starting_block=$((latest_block - 1000))
        echo "Calculated starting block: ${starting_block}"
    fi

    # Create a separate script to run the indexer
    cat <<EOF >run_indexer.sh
#!/bin/bash
cd ../indexer-service
export STARTING_BLOCK=$starting_block
apibara run --allow-env-from-env=STARTING_BLOCK examples/pragma/mainnet/mainnet-script-spot.js -A "$apibara_api_key" --connection-string postgres://postgres:test-password@localhost:5433/pragma --table-name spot_entry --timeout-duration-seconds=240
EOF

    chmod +x run_indexer.sh

    echo "Running indexer in the background..."
    ./run_indexer.sh >indexer.log 2>&1 &
    echo "Indexer is running in the background. PID: $!"
    echo "Check indexer.log for progress."
elif [ "$fill_method" = "backup" ]; then
    echo "Using backup..."
    read -p "Enter the path to your backup file: " backup_path
    docker cp "$backup_path" pragma-node-onchain-db-1:/backup.sql
    docker exec -it pragma-node-onchain-db-1 bash -c "PGPASSWORD=test-password pg_restore -h onchain-db -U postgres -d pragma /backup.sql"
else
    echo "Invalid option. Skipping database fill."
fi

# Step 3: Export environment variables
echo "Exporting environment variables..."
export MODE=dev
export OFFCHAIN_DATABASE_URL="postgres://postgres:test-password@0.0.0.0:5432/pragma"
export ONCHAIN_DATABASE_URL="postgres://postgres:test-password@0.0.0.0:5433/pragma"
export DATABASE_MAX_CONN=25
export TOPIC="pragma-data"
export HOST="0.0.0.0"
export PORT=3000
export METRICS_PORT=8080
export KAFKA_BROKERS="0.0.0.0:9092"

# Step 4: Start the Pragma Node service
echo "Starting Pragma Node service..."
cargo watch -x "run --bin pragma-node"
