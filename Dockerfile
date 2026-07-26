# syntax=docker/dockerfile:1

### Builds the Modrinth downloader
FROM docker.io/library/rust:1.97.1-alpine3.22 AS builder
ARG TARGETARCH
ARG TARGETOS
ARG OPENSSL_STATIC=1
ARG OPENSSL_LIB_DIR=/usr/lib
ARG OPENSSL_INCLUDE_DIR=/usr/include
ARG CARGO_REGISTRY=/root/.cargo/registry
RUN apk add --no-cache openssl-libs-static openssl-dev musl-dev
WORKDIR /src
COPY --link Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked --release \
    && rm -rf src
COPY --link src ./src
RUN cargo build --locked --release
WORKDIR /artefacts
RUN cp /src/target/release/nema /artefacts/nema

### Downloads all selected mods and datapacks from Modrinth
FROM docker.io/library/alpine:3.24
VOLUME ["/artefacts"]
ENV MINECRAFT_VERSION="1.21.1"
ENV RUST_LOG="info,nema=debug"
COPY --from=builder --chmod=0755 /artefacts/nema /usr/local/bin/nema
WORKDIR /artefacts
ENTRYPOINT ["/usr/local/bin/nema"]
CMD ["--strict", "-s", "-o", "/artefacts", "--lockfile", "/artefacts/Modrinth.lock", "--manifest", "/artefacts/Modrinth.toml"]
