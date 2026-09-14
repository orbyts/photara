FROM rust:1.95-bookworm AS builder
WORKDIR /source
COPY . .
RUN cargo build --locked --release -p photara-service \
    --bin photara-service --bin photara-migrate

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /nonexistent \
        --shell /usr/sbin/nologin photara
COPY --from=builder /source/target/release/photara-service /usr/local/bin/
COPY --from=builder /source/target/release/photara-migrate /usr/local/bin/
USER photara
EXPOSE 8080
CMD ["/usr/local/bin/photara-service"]
