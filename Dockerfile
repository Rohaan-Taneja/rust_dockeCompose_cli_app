# ---------- Stage 1: Build ----------
FROM rust:1.93.1 AS builder

RUN apt-get update && apt-get install -y \
    curl \
    libpq-dev \
    pkg-config \
    build-essential \
    && rm -rf /var/lib/apt/lists/*


WORKDIR /app


COPY Cargo.lock Cargo.toml ./

RUN mkdir src && echo "fn main() {}" >src/main.rs

RUN cargo build --release

RUN rm -rf src

COPY . .

RUN cargo build --release


# ---------- Stage 1: Runtime ----------
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/docker_compose_cli .


EXPOSE 8080

# CMD [ "cargo" , "run" ]
CMD ["sleep", "infinity"]
