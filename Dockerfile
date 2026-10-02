# OmniPack API server (omnipack-server) for Linux.
#   docker build -t omnipack-server .
#   docker run -p 8765:8765 -e OMNIPACK_API_KEYS=change-me omnipack-server
# Drop folders: mount a volume and add --inbox /data/inbox --outbox /data/outbox.
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The desktop app is a workspace member: cargo needs its manifest, not its toolchain.
COPY app/src-tauri/Cargo.toml app/src-tauri/build.rs app/src-tauri/
COPY app/src-tauri/src app/src-tauri/src
RUN cargo build --release --locked -p omnipack-server

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 omnipack \
    && mkdir -p /data/inbox /data/outbox \
    && chown -R omnipack /data
COPY --from=build /src/target/release/omnipack-server /usr/local/bin/omnipack-server
USER omnipack
WORKDIR /data
EXPOSE 8765
# Listening on the network needs an API key: set OMNIPACK_API_KEYS.
ENV OMNIPACK_BIND=0.0.0.0:8765
ENTRYPOINT ["omnipack-server"]
