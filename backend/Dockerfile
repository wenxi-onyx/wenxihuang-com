# Use cargo-chef to cache dependencies
FROM rust:1.91-alpine3.22 as chef
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
RUN cargo install cargo-chef
WORKDIR /app

# Analyze dependencies
FROM chef as planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies (this layer is cached unless dependencies change)
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
COPY .sqlx ./.sqlx
RUN cargo chef cook --release --recipe-path recipe.json

# Build application (only rebuilds your code, not dependencies)
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release && \
    strip /app/target/release/backend && \
    strip /app/target/release/initialize && \
    strip /app/target/release/create_admin

# Runtime stage
FROM alpine:3.22
RUN apk add --no-cache ca-certificates libgcc
WORKDIR /app
COPY --from=builder /app/target/release/backend .
COPY --from=builder /app/target/release/initialize ./initialize
COPY --from=builder /app/target/release/create_admin ./create_admin
COPY --from=builder /app/migrations ./migrations
COPY ["Table Tennis Database - Ledger.csv", "./"]
EXPOSE 8080
CMD ["./backend"]
