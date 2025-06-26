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
    && :

COPY . .

RUN : \
    && apk add --no-cache \
        build-base \
    && :

RUN \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    : \
    && cargo build --target x86_64-unknown-linux-musl --release \
    && mv /app/target/x86_64-unknown-linux-musl/release/nur-builder /app/nur-builder \
    && :

FROM alpine:3.22

WORKDIR /

COPY --from=builder /app/nur-builder /nur-builder

CMD ["/nur-builder"]
