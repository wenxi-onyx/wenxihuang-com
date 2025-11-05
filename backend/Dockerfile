FROM rust:1.91-alpine3.22 as builder
WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx
RUN cargo build --release

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
