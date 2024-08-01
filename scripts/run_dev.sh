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

# Step 1: Start the services
echo "Starting services..."
docker compose -f compose.dev.yaml up -d --build

# Step 2: Fill the onchain database
echo "Would you like to run the indexer or use a backup to fill the onchain database? (indexer/backup)"
read -r fill_method

if [ "$fill_method" = "indexer" ]; then
    echo "Preparing to run indexer..."
    if [ ! -d "../indexer-service" ]; then
        echo "Error: ../indexer-service directory not found. Please ensure it exists."
        exit 1
    fi
    read -p "Enter your Apibara API key: " apibara_api_key

    # Fetch the latest block number
    # TODO: get env variable, if empty get latest_block
    latest_block=$(curl -s --location 'https://juno.sepolia.dev.pragma.build' \
    --header 'Content-Type: application/json' \
    --data '{
        "jsonrpc": "2.0",
        "method": "starknet_blockNumber",         
        "params": [],                    
        "id": 1
    }' | jq '.result')

    # Calculate the starting block
    starting_block=$((latest_block - 1000))
    
    echo "Latest block: $latest_block"
    echo "Starting block: $starting_block"

    # Create a separate script to run the indexer
    cat << EOF > run_indexer.sh
#!/bin/bash
cd ../indexer-service
export STARTING_BLOCK=$starting_block
apibara run --allow-env-from-env=STARTING_BLOCK examples/pragma/testnet/sepolia-script-spot.js -A "$apibara_api_key" --connection-string postgres://postgres:test-password@localhost:5433/pragma --table-name spot_entry --timeout-duration-seconds=240
EOF

    chmod +x run_indexer.sh

    echo "Running indexer in the background..."
    ./run_indexer.sh > indexer.log 2>&1 &
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