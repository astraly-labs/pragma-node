FROM rust:1.80 as builder

# Install additional dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    libclang-dev \
    cmake

WORKDIR /home/pragma

# Copy the entire project
COPY . .

# Build the project
RUN cargo build --release --bin pragma-ingestor

FROM debian:bookworm-slim

RUN apt-get clean && apt-get update
RUN apt-get install -y libpq-dev bash

RUN groupadd pragma
RUN useradd -G pragma ingestor

USER ingestor:pragma

WORKDIR /home/pragma-ingestor

# Copy the built binary from the builder stage
COPY --from=builder --chown=ingestor:pragma /home/pragma/target/release/pragma-ingestor /usr/local/bin/pragma-ingestor

CMD ["pragma-ingestor"]