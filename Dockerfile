FROM rust:1.80 AS build

WORKDIR /usr/src
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends musl-tools && \
    rustup target add x86_64-unknown-linux-musl


RUN USER=root cargo new --bin vesta
WORKDIR /usr/src/vesta
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .


FROM scratch

WORKDIR /app
COPY ./static static
COPY --from=build /usr/src/vesta/target/x86_64-unknown-linux-musl/release/vesta .

CMD ["/app/vesta"]
