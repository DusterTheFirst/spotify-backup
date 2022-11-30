FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
ARG DEBIAN_FRONTEND="noninteractive"

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN set -eux; \
    cargo build --release; \
    objcopy --compress-debug-sections target/release/spotify-backup ./spotify-backup

FROM gcr.io/distroless/cc AS runtime

COPY --from=builder /app/spotify-backup /spotify-backup
COPY --from=builder /app/static /static

ENV STATIC_DIR="/static"

CMD ["/spotify-banger-backend"]