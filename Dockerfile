FROM rust:1.82-slim-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler clang libclang-dev cmake
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/plim-node /usr/local/bin/
EXPOSE 30333 9944 9945 9615
ENTRYPOINT ["plim-node"]
