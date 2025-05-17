# Pragma Historical Funding Rates

Fetches historical funding rates from Hyperliquid or Paradex and stores them in a CSV file and TimescaleDB.

## Prerequisites
- Rust (latest stable)
- TimescaleDB
- `timescaledb-parallel-copy` (see [installation](https://github.com/timescale/timescaledb-parallel-copy))
- PostgreSQL database

## Installation
1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd <repository-directory>
   ```
2. Build the project:
   ```bash
   cargo build --release
   ```

## Usage
Run the program with the following command:

```bash
cargo run --bin pragma-historical -- --source hyperliquid --range 1746057600000,1748736000000 --csv-output funding_rates.csv --connection postgres://postgres:test-password@0.0.0.0:5432/pragma
```

- `--source`: Data source (`hyperliquid` or `paradex`)
- `--range`: Time range in Unix milliseconds (start,end; last month: Apr 1, 2025 - Apr 30, 2025)
- `--csv-output`: Output CSV file path
- `--connection`: PostgreSQL connection string

## Notes
- Ensure `timescaledb-parallel-copy` is installed.
- Update the database connection string with your credentials.