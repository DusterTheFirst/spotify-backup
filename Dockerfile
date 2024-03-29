FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
ARG DEBIAN_FRONTEND="noninteractive"
RUN set -eux; \
    apt update; \
    apt install git-crypt lld -y

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY --chown=root:root . .
RUN set -eux; \
    # Make Git happy (fly.toml does not get copied when running `fly deploy`)
    git restore fly.toml; \
    cargo build --release; \
    objcopy --compress-debug-sections target/release/spotify-backup ./spotify-backup

FROM gcr.io/distroless/cc AS runtime

COPY --from=builder /app/spotify-backup /spotify-backup

ENV STATIC_DIR="/static"
COPY ./static /static

ENV GITHUB_PRIVATE_KEY="/spotify-backup.private-key.pem"
COPY ./spotify-backup.private-key.pem /spotify-backup.private-key.pem

CMD ["/spotify-backup"]