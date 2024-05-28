# Pragma Node 🧩

This repository contains the source code of the Pragma Node, which comprises several services and libraries:

- **Pragma Node**: Service for querying and storing data.
- **Pragma Ingestor**: Service running to ingest data from Data Sources.
- **Pragma Common**: Library containing common models and functions.
- **Pragma Entities**: Library containing models/Data Transfer Objects (DTOs) and functions for entities.
- **Pragma Offchain Service**: Offchain service aggregating and storing data from Data Providers.

## Getting Started

Follow these steps to get started with your Rust backend project based on this template:

1. Clone this repository:

   ```bash
   git clone https://github.com/astraly-labs/pragma-node.git
   ```

2. Choose a specific module/framework branch or work with the default configuration.

3. Customize the project to your needs.

4. Build and run your Rust backend:

   ```bash
   cargo run
   ```

# Services description

### Pragma Node

The Pragma Node service allows querying and storing data within the Pragma database. It retrieves, verifies, and sends data to the Kafka service. It also provides the ability to query data stored in the database.

### Pragma Ingestor

This service listens on the Kafka service and stores the retrieved data in the database. It performs certain checks on the collected data.

### Pragma Common

This library contains the models and functions common to different services.

### Pragma Entities

This library contains models and DTOs related to the entities used in the services and Pragma's database.

# Services Structure

The project follows a modular structure to keep the code organized and maintainable. Here's a brief overview of the project structure:

### Pragma Node

- `src/`: Contains the main source code of the application.
  - `handlers/`: Define your API handlers.
  - `infra/`: Define your infrastructure logic.
    - `kafka/`: Kafka logic.
    - `repositories`: Repositories logic.
  - `utils`: Defines utility functions.
  - `config.rs`: File containing the configuration structure.
  - `errors.rs`: Contains error kinds and error formatting logic.
  - `main.rs`: Application's entry point.
  - `routes.rs`: Defines application routes.

### Pragma Ingestor

- `src/`: Contains the main source code of the application.
  - `main.rs`: Application's entry point.
  - `config.rs`: File containing the configuration structure.
  - `consumer.rs`: Defines message consumption logic.
  - `errors.rs`: Contains error kinds and error formatting logic.

### Pragma Entities

- `migrations`: Contains database migrations.
- `src/`: Contains the main source code of the application.
  - `models/`: Defines application models.
  - `dto/`: Defines application DTOs.
  - `errors.rs`: Contains error kinds and error formatting logic.
  - `schema.rs`: Defines the database schema.
  - `connection.rs`: Defines the database connection.
  - `db.rs`: Defines the logic for executing migrations (@TODO: To be moved).
  - `lib.rs`: Defines the library's entry point.

### Pragma Common

- `src/`: Contains the main source code of the application.
  - `lib.rs`: Defines the library's entry point.
  - `tracing.rs`: Defines common tracing logic.

## Development

For faster iterations, you can deploy every needed services required by `pragma-node` using `compose.dev.yaml` & Docker compose:

### 1. Start the services:

```bash
docker compose -f compose.dev.yaml up -d --build
```

### 2. Fill the database

The database tables are created automatically using the migrations in the `infra/pragma-node/postgres_migrations` folder.
However, you need to fill the tables with data. To do so, you can either run the indexer or use a backup:

#### Run the indexer:

```bash
git clone git@github.com:astraly-labs/indexer-service.git
cd indexer-service
# Index & fill the spot_entry (testnet) table
apibara run examples/pragma/testnet/sepolia-script-spot.js -A [YOUR_APIBARA_API_KEY] --connection-string postgres://postgres:test-password@localhost:5433/pragma --table-name spot_entry --timeout-duration-seconds=240
```

#### Use the backup (ask for a file):

```bash
# copy the backup file to the container
docker cp /path/to/the/backup.sql pragma-node-postgre-db-1:/backup.sql
# connect to the container
docker exec -it pragma-node-postgre-db-1 bash
# execute the backup
PGPASSWORD=test-password pg_restore -h postgre-db -U postgres -d pragma /backup.sql
```

### 3. Export the required environment variables:

```bash
export TIMESCALE_DATABASE_URL="postgres://postgres:test-password@0.0.0.0:5432/pragma"
export POSTGRES_DATABASE_URL="postgres://postgres:test-password@0.0.0.0:5433/pragma"
export DATABASE_MAX_CONN=5
export TOPIC="pragma-data"
export HOST="0.0.0.0"
export PORT=3000
export KAFKA_BROKERS="0.0.0.0:9092"
```

### 4. Start the Pragma Node service:

```bash
cargo run --bin pragma-node
```

The pragma-node swagger documentation is available at `http://localhost:3000/node/swagger-ui`.
