FROM rust:1.93.1


WORKDIR /app


COPY Cargo.lock Cargo.toml ./

RUN mkdir src && echo "fn main() {}" >src/main.rs

RUN cargo build

RUN rm -rf src

COPY . .

EXPOSE 8080

CMD [ "cargo" , "run" ]

