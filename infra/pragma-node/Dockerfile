FROM rust:1.80 as builder

WORKDIR /home/pragma

# Copy the entire project
COPY . .

# Build the project
RUN cargo build --release --bin pragma-node

FROM debian:bookworm-slim

# Install necessary packages including CA certificates
RUN apt-get clean && apt-get update && \
    apt-get install -y libpq-dev libssl-dev ca-certificates bash && \
    rm -rf /var/lib/apt/lists/* && \
    update-ca-certificates

RUN groupadd pragma
RUN useradd -G pragma node

USER node:pragma

WORKDIR /home/pragma-node

# Copy the built binary from the builder stage
COPY --from=builder --chown=node:pragma /home/pragma/target/release/pragma-node /usr/local/bin/pragma-node

EXPOSE 3000

CMD ["pragma-node"]