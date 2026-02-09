# syntax=docker/dockerfile:1.7

FROM rust:1.88-bookworm AS builder

WORKDIR /app

# 先复制清单文件，提升依赖缓存命中率
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY src ./src

# 构建目标二进制：searxng_mcp
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin searxng_mcp \
    && cp /app/target/release/searxng_mcp /tmp/searxng_mcp

FROM debian:bookworm-slim AS runtime

WORKDIR /app

# 运行期仅保留必要组件（TLS 证书）
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /tmp/searxng_mcp /usr/local/bin/searxng_mcp

# 容器内默认监听 8000，确保可被宿主机访问
ENV MCP_BIND=0.0.0.0:8000

EXPOSE 8000

ENTRYPOINT ["/usr/local/bin/searxng_mcp"]
