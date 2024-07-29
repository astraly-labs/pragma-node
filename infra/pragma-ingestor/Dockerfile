FROM rust:1.79 as builder

WORKDIR /home/pragma-ingestor

COPY pragma-ingestor/. .

RUN mkdir /home/pragma-entities

COPY pragma-entities/. /home/pragma-entities/.
COPY pragma-common/. /home/pragma-common/.

ENV CARGO_HOME="/home/pragma-ingestor/"
RUN cargo install --path .

FROM debian:bookworm-slim

RUN apt-get clean && apt-get update
RUN apt-get install -y libpq-dev bash

RUN groupadd pragma

RUN useradd -G pragma ingestor

USER ingestor:pragma

WORKDIR /home/pragma-ingestor

COPY --from=builder --chown=ingestor:pragma  /home/pragma-ingestor/bin/pragma-ingestor /usr/local/bin/pragma-ingestor
# COPY infra/pragma-ingestor/config/.env.example .env

CMD ["pragma-ingestor"]