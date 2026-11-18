# syntax=docker/dockerfile:1
# Static musl binary on scratch; the image carries only the binary + assets.

FROM oven/bun:1.4.4-alpine AS frontend-builder
WORKDIR /build
COPY package.json bun.lockb ./
RUN --mount=type=cache,target=/root/.bun/install/cache,sharing=locked \
    bun install --frozen-lockfile
COPY vite.config.ts tsconfig.json components.json ./
COPY frontend ./frontend
RUN bun run build

FROM --platform=$BUILDPLATFORM rust:1.99.1-slim-bookworm AS build
# Debian's cross gcc targets glibc, so bundled sqlite3.c compiles against
# glibc headers and emits LFS64 symbols (open64, stat64, ...) that musl does
# not provide -> undefined references at link time. zig ships a real musl
# sysroot; cargo-zigbuild drives it as the cc/linker for cross targets.
ARG ZIG_VERSION=0.15.1
ARG ZIGBUILD_VERSION=0.24.1
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        xz-utils \
    && mkdir -p /usr/local/zig \
    && curl -fsSL "https://ziglang.org/download/${ZIG_VERSION}/zig-x86_64-linux-${ZIG_VERSION}.tar.xz" \
        | tar -xJ -C /usr/local/zig --strip-components=1 \
    && rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
ENV PATH=/usr/local/zig:$PATH
RUN cargo install --locked cargo-zigbuild@"${ZIGBUILD_VERSION}"
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY migrations ./migrations
COPY docs ./docs
COPY src ./src
ARG TARGETARCH
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    mkdir -p /out /data \
    && if [ "$TARGETARCH" = "arm64" ]; then \
        cargo zigbuild --release --locked --target=aarch64-unknown-linux-musl \
        && cp target/aarch64-unknown-linux-musl/release/vesta /out/vesta; \
    else \
        cargo zigbuild --release --locked --target=x86_64-unknown-linux-musl \
        && cp target/x86_64-unknown-linux-musl/release/vesta /out/vesta; \
    fi

FROM scratch
WORKDIR /app
COPY --from=build /data /data
COPY static static
COPY --from=frontend-builder /build/static/ui static/ui
COPY --from=build /out/vesta vesta
ENV VESTA_CONFIG=/data/config.toml \
    VESTA_DATABASE=/data/vesta.sqlite \
    VESTA_PLUGINS=/data/plugins \
    VESTA_STATIC_DIR=/app/static
EXPOSE 3000
ENTRYPOINT ["/app/vesta"]
