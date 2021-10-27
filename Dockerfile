FROM rust:1.56 AS chef
RUN CARGO_TERM_COLOR=always cargo install cargo-chef 
WORKDIR /app

FROM chef AS planner
COPY . .
RUN CARGO_TERM_COLOR=always cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update
RUN apt install musl-tools -y

# Build dependencies - this is the caching Docker layer!
RUN CARGO_TERM_COLOR=always cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-musl

# Build application
COPY . .
RUN CARGO_TERM_COLOR=always cargo build --release --target x86_64-unknown-linux-musl

# We do not need the Rust toolchain to run the binary!
FROM gcr.io/distroless/static-debian11 AS runtime

WORKDIR /app
VOLUME [ "/app" ]

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/spotify-backup /bin/spotify-backup
CMD ["/bin/spotify-backup"]