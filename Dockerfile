FROM rust:1.93.1

RUN apt-get update && apt-get install -y \
    curl \
    libpq-dev \
    pkg-config \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

    
WORKDIR /app


COPY Cargo.lock Cargo.toml ./

RUN mkdir src && echo "fn main() {}" >src/main.rs

RUN cargo build

RUN rm -rf src

COPY . .

EXPOSE 8080

# CMD [ "cargo" , "run" ]
CMD ["sleep", "infinity"]
