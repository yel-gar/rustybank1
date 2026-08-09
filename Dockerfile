FROM rust:1-alpine3.24 AS builder
WORKDIR /app/build
COPY Cargo.lock Cargo.toml ./
COPY src/ ./src/
RUN cargo install --path .

FROM alpine:3.24 AS runner
COPY --from=builder /usr/local/cargo/bin/rustybank1 /usr/local/bin/rustybank1
ENV RUST_LOG="rustybank1=info"
CMD ["rustybank1"]
