FROM lukemathwalker/cargo-chef:latest-rust-1.78.0 AS chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin newsletter

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt update -y \
    && apt install -y --no-install-recommends openssl ca-certificates \
    && apt autoremove -y \
    && apt clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/newsletter newsletter
COPY configuration configuration
ENV APP_ENV production

ENTRYPOINT ["./newsletter"]
