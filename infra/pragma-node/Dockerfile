FROM rust:1.79 as builder

WORKDIR /home/pragma-node

COPY pragma-node/. .

RUN mkdir /home/pragma-entities
RUN mkdir /home/pragma-common

COPY pragma-entities/. /home/pragma-entities/.
COPY pragma-common/. /home/pragma-common/.

ENV CARGO_HOME="/home/pragma-node/"
RUN cargo install --path .

FROM debian:bookworm-slim

# Install necessary packages including CA certificates
RUN apt-get update && apt-get install -y \
    libpq-dev \
    ca-certificates \
    bash

RUN groupadd pragma

RUN useradd -G pragma node

USER node:pragma

WORKDIR /home/pragma-node

COPY --from=builder --chown=node:pragma /home/pragma-node/bin/pragma-node /usr/local/bin/pragma-node

EXPOSE 3000

CMD ["pragma-node"]
