FROM oven/bun:1-alpine AS css-builder
WORKDIR /temp
COPY package.json bun.lockb ./
RUN bun install --frozen-lockfile

COPY tailwind.config.js .
COPY src ./src
RUN bunx tailwindcss -i ./src/style.css -o ./out.css --minify

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
COPY --from=css-builder /temp/out.css ./static/style.css
COPY --from=build /usr/src/vesta/target/release/vesta /app/vesta

CMD ["/app/vesta"]
