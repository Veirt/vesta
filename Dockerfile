# syntax=docker/dockerfile:1
# Static musl binary on scratch; the image carries only the binary + assets.

FROM oven/bun:1-alpine AS frontend-builder
WORKDIR /build
COPY package.json bun.lockb ./
RUN bun install --frozen-lockfile
COPY vite.config.ts tsconfig.json components.json ./
COPY frontend ./frontend
COPY src ./src
RUN bun run build

FROM rust:1.99.1-slim-bookworm AS build
RUN apt-get update && apt-get install --no-install-recommends -y \
    musl-tools \
    gcc-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./build.rs
COPY migrations ./migrations
COPY docs ./docs
COPY src ./src
ARG TARGETARCH
RUN mkdir -p /out /data \
    && if [ "$TARGETARCH" = "arm64" ]; then \
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
COPY --from=build /data /data
COPY static static
COPY --from=frontend-builder /build/static/ui static/ui
COPY --from=build /out/vesta vesta
ENV VESTA_CONFIG=/data/config.toml \
    VESTA_DATABASE=/data/vesta.sqlite \
    VESTA_STATIC_DIR=/app/static
EXPOSE 3000
ENTRYPOINT ["/app/vesta"]
