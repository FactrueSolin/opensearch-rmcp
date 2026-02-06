# Docker 部署指南

本文说明如何将 `openperplexity`（二进制：`searxng_mcp`）以 Docker 方式部署运行。

## 1. 前置条件

- 已安装 Docker Engine 24+（或 Docker Desktop 最新稳定版）
- 可选：安装 Docker Compose（`docker compose` 子命令）
- 已准备可用的 SearxNG 服务地址（`SEARXNG_URL`）

可通过以下命令确认环境：

```bash
docker --version
docker compose version
```

## 2. 快速开始

推荐使用 Compose 一键启动：

1. 复制环境变量模板
2. 根据实际环境修改 `.env`
3. 启动服务

```bash
cp .env.example .env
docker compose up -d --build
```

启动后可检查健康检查接口：

```bash
curl http://127.0.0.1:8000/health
```

## 3. 构建镜像

项目根目录执行：

```bash
docker build -t openperplexity/searxng-mcp:latest -f Dockerfile .
```

说明：
- `Dockerfile` 使用多阶段构建（builder + runtime）
- builder 基于 Rust 官方镜像编译 `src/bin/searxng_mcp.rs`
- runtime 基于 `debian:bookworm-slim`，仅保留运行所需内容，减小最终镜像体积

## 4. 运行容器

### 方式 A：使用 `.env` 文件

```bash
docker run -d \
  --name searxng-mcp \
  --env-file .env \
  -e MCP_BIND=0.0.0.0:8000 \
  -p 8000:8000 \
  openperplexity/searxng-mcp:latest
```

### 方式 B：直接传递环境变量

```bash
docker run -d \
  --name searxng-mcp \
  -e SEARXNG_URL=http://your-searxng:8080 \
  -e MCP_BIND=0.0.0.0:8000 \
  -e MCP_AUTH_TOKEN=your_token \
  -e SILICONFLOW_API_KEY=your_siliconflow_api_key \
  -p 8000:8000 \
  openperplexity/searxng-mcp:latest
```

日志查看：

```bash
docker logs -f searxng-mcp
```

## 5. 环境变量说明

以下变量参考 `.env.example`：

| 变量名                | 必填                 | 默认值                       | 说明                                                             |
| --------------------- | -------------------- | ---------------------------- | ---------------------------------------------------------------- |
| `SEARXNG_URL`         | 是                   | 无                           | SearxNG 服务地址，例如 `http://10.26.0.12:8888`                  |
| `MCP_BIND`            | 否                   | `127.0.0.1:8000`（程序默认） | MCP HTTP 服务监听地址；容器中建议设为 `0.0.0.0:8000`             |
| `MCP_AUTH_TOKEN`      | 否                   | 无                           | MCP 鉴权 token，设置后请求需携带 `Authorization: Bearer <token>` |
| `SILICONFLOW_API_KEY` | 否（按业务能力决定） | 无                           | 重排/模型相关能力所需密钥                                        |

## 6. Docker Compose

本项目已提供 `docker-compose.yml`，可直接使用：

```bash
cp .env.example .env
docker compose up -d --build
```

停止并移除容器：

```bash
docker compose down
```

查看日志：

```bash
docker compose logs -f
```

## 7. 常见问题

### 1) 容器启动后无法访问 `:8000`

- 检查是否映射端口：`-p 8000:8000`
- 检查 `MCP_BIND` 是否为 `0.0.0.0:8000`
- 检查宿主机防火墙策略

### 2) 报错 `SEARXNG_URL is required`

- 未设置 `SEARXNG_URL` 或值为空
- 检查 `.env` 文件是否被正确加载（`--env-file .env`）

### 3) 请求返回未授权（401/403）

- 若设置了 `MCP_AUTH_TOKEN`，请求必须带 `Authorization: Bearer <token>`
- 检查 token 是否包含多余空格

### 4) 拉取依赖或构建速度慢

- 可在网络较好的环境先构建镜像，再分发
- 结合 CI 缓存 Docker 层与 Cargo 依赖缓存

