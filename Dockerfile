FROM rustlang/rust:nightly as builder

WORKDIR /home/rust/

USER root

COPY rust-toolchain.toml .

RUN rustc --version; cargo --version; rustup --version

RUN apt update && apt install cmake -y

# Avoid having to install/build all dependencies by copying
# the Cargo files and making a dummy src/main.rs
COPY Cargo.toml .
COPY Cargo.lock .
COPY battlesnake-rs/Cargo.toml ./battlesnake-rs/
COPY battlesnake-minimax/Cargo.toml ./battlesnake-minimax/
COPY web-rocket/Cargo.toml ./web-rocket/
COPY web-lambda/Cargo.toml ./web-lambda/
COPY web-axum/Cargo.toml ./web-axum/
RUN mkdir -p ./battlesnake-rs/src/ && echo "fn foo() {}" > ./battlesnake-rs/src/lib.rs
RUN mkdir -p ./battlesnake-minimax/src/ && echo "fn foo() {}" > ./battlesnake-minimax/src/lib.rs
RUN mkdir -p ./web-rocket/src/ && echo "fn main() {}" > ./web-rocket/src/main.rs
RUN mkdir -p ./web-lambda/src/ && echo "fn main() {}" > ./web-lambda/src/main.rs
RUN mkdir -p ./web-axum/src/ && echo "fn main() {}" > ./web-axum/src/main.rs
RUN cargo build --release --locked --bin web-axum

# We need to touch our real main.rs file or else docker will use
# the cached one.
COPY . .
RUN touch battlesnake-minimax/src/lib.rs && \
    touch battlesnake-rs/src/lib.rs && \
    touch web-axum/src/main.rs && \
    touch web-rocket/src/main.rs

RUN cargo build --release --locked --bin web-axum

# Start building the final image
FROM debian:buster-slim
WORKDIR /home/rust/
COPY --from=builder /home/rust/target/release/web-axum .

ENV JSON_LOGS=1
ENV PORT=8000

EXPOSE 8000

ENTRYPOINT ["./web-axum"]
