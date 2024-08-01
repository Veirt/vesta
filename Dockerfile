FROM --platform=$TARGETPLATFORM rust:1.80-alpine AS build
ARG TARGETPLATFORM

RUN apk add musl-dev --no-cache

WORKDIR /usr/src
RUN USER=root cargo new --bin vesta
WORKDIR /usr/src/vesta
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM scratch

WORKDIR /app
COPY ./static static
COPY --from=build /usr/src/vesta/target/release/vesta /app/vesta

CMD ["/app/vesta"]
