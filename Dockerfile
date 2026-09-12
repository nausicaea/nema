# syntax=docker/dockerfile:1@sha256:ecfaec9ed6d810b56388c508f4121597bfbba70d41a6dfeee4d8cad5f295fc32

FROM docker.io/library/rust:1.98.0-alpine3.24@sha256:a10e64dd139b7387337c7fbe8aca31b959b57b2fd4c8ae20a02cf1d6ea424dce AS build
ARG FEATURES=""
ARG TARGETARCH
RUN case "${TARGETARCH}" in \
        amd64) echo "x86_64-unknown-linux-musl"   > /rust_target ;; \
        arm64) echo "aarch64-unknown-linux-musl"  > /rust_target ;; \
        *)     echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac
ENV OPENSSL_STATIC=1 \
    OPENSSL_LIB_DIR=/usr/lib \
    OPENSSL_INCLUDE_DIR=/usr/include \
    CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN apk add --no-cache openssl-dev openssl-libs-static musl-dev \
    && rustup target add "$(cat /rust_target)"
WORKDIR /workdir
COPY --link Cargo.toml Cargo.lock ./
RUN RUST_TARGET="$(cat /rust_target)" \
    && mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && RUSTFLAGS="-C target-feature=+crt-static" \
       cargo build --locked --release --target "${RUST_TARGET}" \
    && rm -rf src "target/${RUST_TARGET}/release/.fingerprint/nema-*"
COPY --link src/ ./src/
RUN RUST_TARGET="$(cat /rust_target)" \
    && RUSTFLAGS="-C target-feature=+crt-static" \
       cargo build --locked --release --target "${RUST_TARGET}" ${FEATURES} \
    && strip "target/${RUST_TARGET}/release/nema" \
    && cp "target/${RUST_TARGET}/release/nema" /nema

FROM docker.io/library/alpine:3.24.1@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b AS certs
RUN apk add --no-cache ca-certificates

FROM scratch
LABEL \
    org.opencontainers.image.title="nema" \
    org.opencontainers.image.description="Utility for specifying and downloading Modrinth mods" \
    org.opencontainers.image.authors="nausicaea" \
    org.opencontainers.image.source="https://github.com/nausicaea/nema" \
    org.opencontainers.image.version="0.5.2" \
    org.opencontainers.image.licenses="GPL-3.0-only"
ENV MINECRAFT_VERSION="1.21.1" \
    RUST_LOG="info,nema=debug"
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build --chmod=0755 /nema /usr/local/bin/nema
USER 10001:10001
VOLUME ["/artefacts"]
WORKDIR /artefacts
ENTRYPOINT ["/usr/local/bin/nema"]
CMD ["--strict", "-s", "-o", "/artefacts", \
     "--lockfile", "/artefacts/Modrinth.lock", \
     "--manifest", "/artefacts/Modrinth.toml"]
