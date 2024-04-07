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

#### How to run

#### Requirements

1. Create a network for the services to communicate:

```bash
docker network create pragma-network
```

2. Set up a Timescale database. You can use the following Docker command to run it inside a container:

```bash
docker pull timescale/timescaledb-ha:pg16
docker run --name postgres -e POSTGRES_PASSWORD=pragma -p 5432:5432 -d --network pragma-network timescale/timescaledb-ha:pg16
```

3. Set up a Kafka service. You can use the following Docker command to run a Kafka container:

```bash
docker run --name zookeeper --network pragma-network -e ALLOW_ANONYMOUS_LOGIN=yes -p 2181:2181 -d bitnami/zookeeper:latest

docker run --name pragma-kafka --network pragma-network -e KAFKA_CFG_ZOOKEEPER_CONNECT=zookeeper:2181 -e KAFKA_CFG_ADVERTISED_LISTENERS=PLAINTEXT://pragma-kafka:9092 -e ALLOW_PLAINTEXT_LISTENER=yes -p 9092:9092 -d bitnami/kafka:latest
```

4. Set up the environments variables in the `infra/pragma-node/config/.env` file.
   You can use the `.env.example` file as a template (located in `infra/pragma-node/config`).

#### Running the service

Move to the root of the repository & build + run the Docker image using the Dockerfiles in the `infra` directory:

```bash
docker build -t pragma-node -f infra/pragma-node/Dockerfile .
docker run --network pragma-network -p 3000:3000 pragma-node:latest
```

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
