# Multi-stage build: compile against the bundled ko-dic, ship a slim runtime.
FROM rust:1.94-slim AS build
WORKDIR /src

# Cache deps separately from source.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && \
	cargo build --release && rm -rf src target/release/deps/seo_for_korean_gateway*

COPY src ./src
RUN cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
	rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/seo-for-korean-gateway /usr/local/bin/

ENV RUST_LOG=info
ENV BIND=0.0.0.0:8787
EXPOSE 8787

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s \
	CMD wget -qO- http://127.0.0.1:8787/health || exit 1

ENTRYPOINT ["/usr/local/bin/seo-for-korean-gateway"]
