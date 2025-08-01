# syntax=docker/dockerfile:1

### Builds the Modrinth downloader
FROM docker.io/library/rust:1.88.0-alpine3.22 AS builder
ARG TARGETARCH
ARG TARGETOS
ARG OPENSSL_STATIC=1
ARG OPENSSL_LIB_DIR=/usr/lib
ARG OPENSSL_INCLUDE_DIR=/usr/include
#ARG SCCACHE_VERSION
#ARG SCCACHE_CHECKSUM
#ARG AWS_ACCESS_KEY_ID
#ARG AWS_SECRET_ACCESS_KEY
#ARG SCCACHE_BUCKET
#ARG SCCACHE_ENDPOINT
#ARG SCCACHE_REGION=auto
#ARG SCCACHE_S3_USE_SSL=true
#ARG SCCACHE_S3_SERVER_SIDE_ENCRYPTION=true
#ARG RUSTC_WRAPPER=/usr/local/bin/sccache
ARG CARGO_REGISTRY=/root/.cargo/registry
RUN apk add --no-cache openssl-libs-static openssl-dev musl-dev
# WORKDIR /tmp
# ADD --link --checksum=${SCCACHE_CHECKSUM} https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-${TARGETARCH}-unknown-${TARGETOS}-musl.tar.gz ./sccache-${SCCACHE_VERSION}.tar.gz
# RUN <<-EOF
# set -ex;
# tar -xzf ./sccache-${SCCACHE_VERSION}.tar.gz sccache-${SCCACHE_VERSION}/sccache;
# install -o root -g root -m 0755 ./sccache-${SCCACHE_VERSION}/sccache /usr/local/bin/sccache;
# sccache --show-stats;
# EOF
WORKDIR /src
COPY --link Cargo.toml .
COPY --link Cargo.lock .
COPY --link src ./src
RUN --mount=type=cache,target=/root/.cargo cargo fetch
RUN --mount=type=cache,target=/root/.cargo --mount=type=cache,target=/src/target cargo build --locked --release
WORKDIR /artefacts
RUN --mount=type=cache,target=/src/target cp /src/target/release/modrinth /artefacts/modrinth

### Downloads all selected mods and datapacks from Modrinth
FROM docker.io/library/alpine:3.22
VOLUME ["/artefacts"]
ENV MINECRAFT_VERSION="1.21.1"
ENV RUST_LOG="info,modrinth=debug"
COPY --from=builder --chmod=0755 /artefacts/modrinth /usr/local/bin/modrinth
WORKDIR /artefacts
ENTRYPOINT ["/usr/local/bin/modrinth"]
CMD ["--strict", "-s", "-o", "/artefacts", "--lockfile", "/artefacts/Modrinth.lock", "--manifest", "/artefacts/Modrinth.toml"]
