# syntax=docker/dockerfile:1.4
FROM rust:1.74.0-bullseye as env
RUN rm -f /etc/apt/apt.conf.d/docker-clean \
    && echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache
RUN --mount=type=cache,target=/var/cache/apt \
    --mount=type=cache,target=/var/lib/apt \
    apt update \
    && apt install --yes --no-install-recommends \
    cmake
WORKDIR /app
COPY ./rust-toolchain.toml .
# NOTE: `rustup show` might not force-install toolchain in the future
# See https://github.com/rust-lang/rustup/issues/1397
RUN rustup show active-toolchain

FROM env AS builder
COPY --link . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=./target \
    cargo build --bin aceton --locked --release \
    && mv ./target/release/aceton /usr/local/bin/

FROM alpine AS aceton
RUN apk add \
    zlib

COPY --from=builder /usr/local/bin/aceton /usr/local/bin/

VOLUME [ "/etc/aceton" ]
ENTRYPOINT [\
    "/usr/local/bin/aceton",\
    "--config", "/etc/aceton/aceton.toml"\
    ]
