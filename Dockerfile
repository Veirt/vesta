# syntax=docker/dockerfile:1
# Static musl binary on scratch; the image carries only the binary + assets.

FROM oven/bun:1-alpine AS css-builder
WORKDIR /build
COPY package.json bun.lockb ./
RUN bun install --frozen-lockfile
COPY src ./src
RUN bunx @tailwindcss/cli -i ./src/style.css -o /out/style.css --minify

FROM rust:1.99.1-slim-bookworm AS build
RUN apt-get update && apt-get install --no-install-recommends -y \
    musl-tools \
    gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "arm64" ]; then \
        export CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc \
               CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc; \
        cargo build --release --target=aarch64-unknown-linux-musl; \
        cp target/aarch64-unknown-linux-musl/release/vesta /out/vesta; \
    else \
        cargo build --release --target=x86_64-unknown-linux-musl; \
        cp target/x86_64-unknown-linux-musl/release/vesta /out/vesta; \
    fi

FROM scratch
WORKDIR /app
COPY --from=css-builder /out/style.css static/style.css
COPY static static
COPY --from=build /out/vesta vesta
ENV VESTA_CONFIG=/app/config/vesta.toml \
    VESTA_STATIC_DIR=/app/static
EXPOSE 3000
ENTRYPOINT ["/app/vesta"]
