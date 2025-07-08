# Pragma Node 🧩

This repository contains the source code of the Pragma Node, a highly accurate, readily available, and fast API built on an extensive network of data providers. Pragma empowers developers to drive the evolution of next-generation applications with reliable prices and financial data at high frequency.

## Documentation

For detailed information about API endpoints and usage, visit our documentation at [docs.pragma.build/api/overview](https://docs.pragma.build/api/overview).

The Pragma Node swagger documentation is available at `http://localhost:3000/node/swagger-ui` when running locally.

The full spec is also available at [openapi.json](./openapi.json).

## Development Setup

### Quick Setup

> [!NOTE]
> The script is still minimal and does not include `pulse` or other pushing services for the offchain database.
> But it will work fine for the onchain database.

Run the setup script:

```bash
# Running the script with only "sh" will fail
bash scripts/run_dev.sh
```

You will be prompted to either use the `indexer-service` repository or use a backup file.
When using the `indexer` option, make sure you've cloned the indexer-service repository at the same level as this repo.

```bash
git clone git@github.com:astraly-labs/indexer-service.git
```

Optional environment variables:
- `APIBARA_KEY`: will be used as your Apibara API key instead of asking for it.
- `STARTING_BLOCK`: will be used as the indexer starting block.

### Manual Setup

#### 1. Start Services

We have `compose` file for dev purposes. It only spin ups required services for `pragma-node` and let you run it locally using `cargo`.

```bash
docker compose -f compose.dev.yaml up -d --build
```

#### 2. Kafka Setup

Just make sure the topics are correctly created:

```sh
make init-kafka-topics
```

#### 3. Database Setup

#### Onchain Database

**Option 1: Using the indexer**
```bash
git clone git@github.com:astraly-labs/indexer-service.git
cd indexer-service
# Index & fill the spot_entry (testnet) table
apibara run examples/pragma/testnet/sepolia-script-spot.js -A [YOUR_APIBARA_API_KEY] --connection-string postgres://postgres:test-password@localhost:5432/pragma --table-name spot_entry --timeout-duration-seconds=240
```

**Option 2: Using a backup file**
```bash
# copy the backup file to the container
docker cp /path/to/the/backup.sql pragma-node-postgre-db-1:/backup.sql
# connect to the container
docker exec -it pragma-node-postgre-db-1 bash
# execute the backup
PGPASSWORD=test-password pg_restore -h postgre-db -U postgres -d pragma /backup.sql
```

#### Offchain Database

First, make sure that you're correctly registered as a publisher before pushing prices.

You can simply execute some SQL directly into the offchain database, for example:

```sql
INSERT INTO PUBLISHERS
(
    name,
    master_key,
    active_key,
    active,
    account_address
) VALUES
(
    'YOUR_PUBLISHER_NAME', -- or any other name you want
    
    -- For the keys below, make sure they correspond to a correct Starknet Account.
    -- You can generate keys using any starknet wallet.
    -- This is needed for publishing later, since you will need your private key.
    '0x0257a51cd27e950a2ba767795446b4c6ed86116f297c820e5a7159c6b00c6ac9',
    '0x0257a51cd27e950a2ba767795446b4c6ed86116f297c820e5a7159c6b00c6ac9',
    true,
    '0x012322c5EA7A94cC027970694ee70e45434f1F71050e0e2D0d9DE83f1DE66945'
);
```

Now, you can for example use `pulse`, a Pragma price-pushing service:

```bash
git clone https://github.com/astraly-labs/pulse.git
cd pulse
cp .env.example .env # and fill the values
cargo run -- --config ./pulse.config.yaml
```

We also have the [python price-pusher](https://github.com/astraly-labs/pragma-sdk/tree/master/price-pusher) that should work with the API.

#### 4. Environment Setup

Either create a `.env` file following the `.env.example` or export the required variables:

```bash
export MODE=dev
export OFFCHAIN_DATABASE_URL="postgres://postgres:test-password@0.0.0.0:5432/pragma"
export ONCHAIN_DATABASE_URL="postgres://postgres:test-password@0.0.0.0:5433/pragma"
export DATABASE_MAX_CONN=5
export TOPIC="pragma-data"
export HOST="0.0.0.0"
export PORT=3000
export METRICS_PORT=8080
export KAFKA_BROKERS=localhost:29092
# Optional but allows you to export OTEL logs anywhere
export OTEL_EXPORTER_OTLP_ENDPOINT=localhost:4317
```


#### 5. Start Pragma Node

Now that every services are correctly running, you can run the server:

```bash
cargo run --bin pragma-node
```
