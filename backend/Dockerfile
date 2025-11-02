FROM rust:1.91-alpine3.22 as builder
WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM alpine:3.22
RUN apk add --no-cache ca-certificates libgcc
WORKDIR /app
COPY --from=builder /app/target/release/backend .
COPY --from=builder /app/migrations ./migrations
EXPOSE 8080
CMD ["./backend"]
