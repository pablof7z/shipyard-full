FROM rust:1.88-bookworm AS builder
ARG BIN
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --bin "${BIN}"

FROM debian:bookworm-slim
ARG BIN
ENV BIN=${BIN}
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/${BIN} /usr/local/bin/${BIN}
CMD ["/bin/sh", "-c", "exec /usr/local/bin/${BIN}"]
