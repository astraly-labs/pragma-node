#!/bin/bash

# Run database migrations for pragma-node
# This script runs the migrations that were previously run automatically on startup

set -e

# Load environment variables if .env file exists
if [ -f .env ]; then
    export $(cat .env | xargs)
fi

echo "Running database migrations..."

# Method 1: Using Diesel CLI (recommended)
echo "Using Diesel CLI..."
cd pragma-entities && diesel migration run && cd ..

# Method 2: Using Rust migration script (alternative approach)
# Uncomment the following lines if you prefer to use the Rust-based approach:
# echo "Using Rust migration script..."
# cd scripts && cargo run --bin migrate

echo "Database migrations completed successfully!" 