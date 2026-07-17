# syntax=docker/dockerfile:1

### Builds the Modrinth downloader
FROM docker.io/library/rust:1.88.0-alpine3.22 AS builder
ARG TARGETARCH
ARG TARGETOS
ARG OPENSSL_STATIC=1
ARG OPENSSL_LIB_DIR=/usr/lib
ARG OPENSSL_INCLUDE_DIR=/usr/include
ARG CARGO_REGISTRY=/root/.cargo/registry
RUN apk add --no-cache openssl-libs-static openssl-dev musl-dev
WORKDIR /src
COPY --link Cargo.toml .
COPY --link Cargo.lock .
COPY --link src ./src
RUN --mount=type=cache,target=/root/.cargo cargo fetch
RUN --mount=type=cache,target=/root/.cargo --mount=type=cache,target=/src/target cargo build --locked --release
WORKDIR /artefacts
RUN --mount=type=cache,target=/src/target cp /src/target/release/nema /artefacts/nema

### Downloads all selected mods and datapacks from Modrinth
FROM docker.io/library/alpine:3.22
VOLUME ["/artefacts"]
ENV MINECRAFT_VERSION="1.21.1"
ENV RUST_LOG="info,nema=debug"
COPY --from=builder --chmod=0755 /artefacts/nema /usr/local/bin/nema
WORKDIR /artefacts
ENTRYPOINT ["/usr/local/bin/nema"]
CMD ["--strict", "-s", "-o", "/artefacts", "--lockfile", "/artefacts/Modrinth.lock", "--manifest", "/artefacts/Modrinth.toml"]
