# Pragma Node 🧩

This repository contains the source code of the Pragma Node, which comprises several services and libraries:

- **Pragma Node**: Service for querying and storing data.
- **Pragma Ingestor**: Service running to ingest data from Data Sources.
- **Pragma Common**: Library containing common models and functions.
- **Pragma Entities**: Library containing models/Data Transfer Objects (DTOs) and functions for entities.
- **Pragma Offchain Service**: Offchain service aggregating and storing data from Data Providers.

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