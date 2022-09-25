FROM rust:bullseye AS builder
WORKDIR /app
ARG DEBIAN_FRONTEND="noninteractive"

COPY . .

RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.rustup \
    set -eux; \
    cargo build --release; \
    objcopy --compress-debug-sections target/release/spotify-backup ./spotify-backup

FROM debian:bullseye-slim
WORKDIR /app

COPY --from=builder /app/spotify-backup ./spotify-backup
COPY --from=builder /app/static ./static

ENV STATIC_DIR="/app/static"

CMD ["./spotify-banger-backend"]