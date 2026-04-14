# Build stage
FROM rust:1.94-slim AS builder

WORKDIR /usr/src/app

COPY . ./microgate-demo

WORKDIR /usr/src/app/microgate-demo

RUN cargo build --release

# Run stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/microgate-demo/target/release/microgate-demo /app/microgate-demo
COPY --from=builder /usr/src/app/microgate-demo/public /app/public

ENV PORT=8080
EXPOSE 8080

CMD ["./microgate-demo"]