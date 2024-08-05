FROM --platform=$BUILDPLATFORM oven/bun:1-alpine AS css-builder
WORKDIR /temp
COPY package.json bun.lockb ./
RUN bun install --frozen-lockfile
COPY gridPlugin.js tailwind.config.js ./
COPY src ./src
RUN bunx tailwindcss -i ./src/style.css -o ./out.css --minify

FROM --platform=$BUILDPLATFORM rust:1.80 AS build
ARG TARGETPLATFORM

RUN apt-get update && apt-get install --no-install-recommends -y \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl 
RUN apt-get update && apt-get install --no-install-recommends -y \
    gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

# Set up the build environment
WORKDIR /usr/src
RUN USER=root cargo new --bin vesta
WORKDIR /usr/src/vesta
COPY Cargo.toml Cargo.lock ./
COPY src ./src

COPY docker/installer/linux linux
RUN ./$TARGETPLATFORM.sh

FROM scratch
WORKDIR /app
COPY ./static static
COPY --from=css-builder /temp/out.css ./static/style.css

COPY --from=build /usr/src/vesta/vesta /app/vesta


CMD ["/app/vesta"]
