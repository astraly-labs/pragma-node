# Pragma Node 🧩

This repo contains the following:

- Pragma Node: Node run by Data Providers.
- Pragma Offchain Service: Offchain service aggregating and storing data from Data Providers.

## Getting Started

Follow these steps to get started with your Rust backend project based on this template:

1. Clone this repository:

   ```bash
   git clone https://github.com/Quentin-Piot/axum-diesel-real-world.git
      ```

2. Choose a specific module/framework branch or work with the default configuration.

3. Customize the project to your needs.

4. Build and run your Rust backend:

    ```bash
    cargo run
    ```

## Project Structure

The project follows a modular structure to keep your code organized and maintainable. Here's a brief overview of the
project structure:

- `src/`: Contains the main source code of your application.
  - `domain/`: Define your domain logic using DDD principles.
    - `models/`: Define your domain models.
  - `handlers/`: Define your API handlers.
  - `infra/`: Define your infrastructure logic.
    - `db/`: Define your database logic.
    - `repositories/`: Define your repositories.
  - `utils/`: Define your utility functions.
    - `custom_extractors/`: Define your custom extractors for Axum.
  - `main.rs`: Application entry point.
  - `routes.rs`: Define your API routes.
  - `config.rs`: Define your application configuration : use OnceCell for static config file.
  - `error.rs`: Define your custom global error types.

- `migrations/`: Database migration files for Diesel (if applicable).

### License

This project is licensed under the MIT License.
