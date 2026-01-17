FROM rust:1.89-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM alpine:3.22 AS certs
RUN apk --update add ca-certificates

FROM gcr.io/distroless/cc-debian12

WORKDIR /
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/target/release/simple-http /usr/local/bin/simple-http

LABEL org.opencontainers.image.source="https://github.com/mipsel64/simple-http"
LABEL org.opencontainers.image.description="Simple HTTP server with IP + path counting"
LABEL org.opencontainers.image.licenses="MIT"

ENTRYPOINT ["simple-http"]
