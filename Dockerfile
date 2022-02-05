FROM rustlang/rust:nightly as builder

WORKDIR /home/rust/

USER root

COPY rust-toolchain.toml .

RUN rustc --version; cargo --version; rustup --version

# Avoid having to install/build all dependencies by copying
# the Cargo files and making a dummy src/main.rs
COPY Cargo.toml .
COPY Cargo.lock .
COPY battlesnake-rs/Cargo.toml ./battlesnake-rs/
COPY web-rocket/Cargo.toml ./web-rocket/
COPY web-lambda/Cargo.toml ./web-lambda/
RUN mkdir -p ./battlesnake-rs/src/ && echo "fn foo() {}" > ./battlesnake-rs/src/lib.rs
RUN mkdir -p ./web-rocket/src/ && echo "fn main() {}" > ./web-rocket/src/main.rs
RUN mkdir -p ./web-lambda/src/ && echo "fn main() {}" > ./web-lambda/src/main.rs
RUN cargo build --release --locked --bin web-rocket

# We need to touch our real main.rs file or else docker will use
# the cached one.
COPY . .
RUN touch battlesnake-rs/src/lib.rs

RUN cargo build --release --bin web-rocket

# Start building the final image
FROM debian:buster-slim
WORKDIR /home/rust/
COPY --from=builder /home/rust/target/release/web-rocket .

ENV JSON_LOGS=1
ENV ROCKET_PORT=8000

EXPOSE 8000

ENTRYPOINT ["./web-rocket"]
