FROM rust:alpine3.22 AS builder

WORKDIR /app

RUN : \
    && apk add --no-cache \
        musl-dev \
        pkgconfig \
        perl \
        openssl-dev \
        make \
        libgcc \
        libstdc++ \
        musl \
        build-base \
    && :

COPY Cargo.toml .
COPY Cargo.lock .

RUN \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    : \
    && mkdir src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --target x86_64-unknown-linux-musl --release \
    && rm -f /app/target/x86_64-unknown-linux-musl/release/deps/nur_builder* \
    && rm -rf src \
    && :

COPY src src

RUN \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    : \
    && cargo build --target x86_64-unknown-linux-musl --release \
    && cp /app/target/x86_64-unknown-linux-musl/release/nur-builder /app/nur-builder \
    && chmod +x /app/nur-builder \
    && :

FROM alpine:3.22

WORKDIR /

RUN : \
    && apk add --no-cache \
        git \
        podman \
        iptables \
        fuse-overlayfs \
    && :

COPY --from=builder /app/nur-builder /nur-builder

CMD ["/nur-builder"]
