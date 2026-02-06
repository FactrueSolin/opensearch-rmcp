# openperplexity

## 1) 项目介绍

`openperplexity` 是一个面向 MCP（Model Context Protocol）的搜索服务：通过封装 **searXNG** 的聚合搜索能力，对外提供可被 LLM/Agent 直接调用的 MCP 工具接口。

本项目的核心依赖与特色：

- **searXNG**：作为底层元搜索引擎，项目通过 HTTP 调用 searXNG 的 `/search?format=json` 接口来获取搜索结果（见 [`SearxngClient::search()`](src/searxng/client.rs:33)）。
- **轨迹流动（SiliconFlow）重排序模型**：可选接入 SiliconFlow 的重排序 API，在搜索结果数量超过 `limit` 时对结果进行相关性重排（见 [`RerankClient::rerank()`](src/rerank/client.rs:45) 与 [`SearxngClient::search()`](src/searxng/client.rs:33)）。默认使用模型 [`DEFAULT_MODEL`](src/rerank/client.rs:8)（`Qwen/Qwen3-Reranker-8B`）。
- **MCP Server（HTTP Streamable）**：基于 `rmcp` 的 streamable HTTP server 方式暴露 MCP 服务（见 [`server::serve()`](src/mcp/server.rs:17)）。

项目二进制入口为 [`src/bin/searxng_mcp.rs`](src/bin/searxng_mcp.rs)，启动后会加载环境变量并运行 MCP 服务（见 [`main()`](src/bin/searxng_mcp.rs:7)）。

---

## 2) 部署教程

本项目运行时依赖一个可用的 searXNG 服务地址（见 [`SEARXNG_URL`](.env.example:1)）。你可以选择本地运行（Cargo）或 Docker/Compose 部署。

### 2.1 环境变量配置

复制模板并按需修改：

```bash
cp .env.example .env
```

环境变量说明（模板见 [`.env.example`](.env.example)）：

| 变量 | 是否必填 | 说明 |
|---|---:|---|
| [`SEARXNG_URL`](.env.example:1) | 是 | searXNG 服务地址（会自动去掉尾部 `/`，见 [`McpConfig::from_env()`](src/mcp/config.rs:11)） |
| [`MCP_BIND`](.env.example:2) | 否 | MCP HTTP 监听地址，默认 `127.0.0.1:8000`（见 [`McpConfig::from_env()`](src/mcp/config.rs:11)） |
| [`MCP_AUTH_TOKEN`](.env.example:3) | 否 | 启用后需要 `Authorization: Bearer <token>`（鉴权中间件见 [`auth_middleware()`](src/mcp/auth.rs:39)） |
| [`SILICONFLOW_API_KEY`](.env.example:4) | 否 | 启用 SiliconFlow（轨迹流动）重排序能力所需密钥（读取见 [`RerankClient::new()`](src/rerank/client.rs:19)） |

### 2.2 方式 A：本地运行（Cargo）

1) 准备 `.env`

```bash
cp .env.example .env
```

2) 运行服务（二进制：[`searxng_mcp`](src/bin/searxng_mcp.rs:1)）

```bash
cargo run --bin searxng_mcp
```

3) 健康检查

```bash
curl http://127.0.0.1:8000/health
```

健康检查路由见 [`health_check()`](src/mcp/server.rs:13)。

### 2.3 方式 B：Docker Compose（推荐）

项目已提供 Compose 编排文件 [`docker-compose.yml`](docker-compose.yml)。快速启动：

```bash
cp .env.example .env
docker compose up -d --build
```

注意 Compose 默认端口映射为宿主机 `18001 -> 容器 8000`（见 [`docker-compose.yml`](docker-compose.yml:11)），因此健康检查示例：

```bash
curl http://127.0.0.1:18001/health
```

停止并移除：

```bash
docker compose down
```

### 2.4 方式 C：Docker 单容器运行

更完整的 Docker 说明请参考 [`DOCKER.md`](DOCKER.md)。下面给出最小可用示例：

```bash
docker build -t openperplexity/searxng-mcp:latest -f Dockerfile .

docker run -d \
  --name searxng-mcp \
  --env-file .env \
  -e MCP_BIND=0.0.0.0:8000 \
  -p 8000:8000 \
  openperplexity/searxng-mcp:latest
```

---

## 3) MCP 功能的详细介绍

### 3.1 MCP 服务入口与路由

服务启动逻辑见 [`server::serve()`](src/mcp/server.rs:17)：

- `GET /health`：健康检查，返回 `OK`（见 [`health_check()`](src/mcp/server.rs:13)）。
- `/mcp`：MCP 服务入口（通过 `rmcp` 的 streamable HTTP server 暴露，见 [`StreamableHttpService::new`](src/mcp/server.rs:23)）。

#### 鉴权（可选）

当配置了 [`MCP_AUTH_TOKEN`](.env.example:3) 后，会启用鉴权中间件（见 [`auth_middleware()`](src/mcp/auth.rs:39)）：

- 客户端请求需携带 `Authorization: Bearer <token>`
- 否则返回 `401 Unauthorized`

### 3.2 工具列表

当前服务器启用 Tools 能力（见 [`SearxngTools::get_info()`](src/mcp/tools.rs:186)），并提供 1 个核心工具：

#### 3.2.1 `opensearch`

工具实现与描述见 [`SearxngTools::opensearch()`](src/mcp/tools.rs:204)。其定位是：

- 面向 searXNG 的“并发多关键词搜索”工具
- 可按 `search_type`（类别）进行搜索
- 聚合返回结构化 JSON 结果

**请求参数**（见 [`OpenSearchParams`](src/mcp/tools.rs:49)）：

- `queries: Vec<String>`：要搜索的关键词列表（不能为空；为空会返回错误，见 [`SearxngTools::run_open_search()`](src/mcp/tools.rs:72)）
- `search_type: SearchType`：搜索类别（见 [`SearchType`](src/mcp/tools.rs:18)）
- `limit: Option<usize>`：每个 query 的结果条数，默认 `20`，最大 `50`（见 [`SearxngTools::MAX_LIMIT`](src/mcp/tools.rs:63) 与 [`SearxngTools::run_open_search()`](src/mcp/tools.rs:72)）

**search_type 取值**（见 [`SearchType`](src/mcp/tools.rs:18)）：

- `general`：通用搜索（不传 categories）
- `news`：新闻
- `images`：图片
- `it`：信息技术
- `science`：学术

**返回结构**：

工具会返回 [`OpenSearchResponse`](src/searxng/types.rs:28)，其中每个 query 对应一组 [`QuerySearchResult`](src/searxng/types.rs:20)，每条结果包含 `url` 与 `description`（见 [`SearchResult`](src/searxng/types.rs:5)）。

### 3.3 与 searXNG 的集成细节

searXNG 集成入口见 [`SearxngClient`](src/searxng/client.rs:9)：

- 通过 `GET {SEARXNG_URL}/search?q=...&format=json` 获取原始结果（见 [`SearxngClient::search()`](src/searxng/client.rs:33)）。
- 根据类别进行结果映射：图片类读取 `img_src`，文本类读取 `url/content/title`（映射见 [`map_result_item()`](src/searxng/mapper.rs:3)，类型见 [`SearxngResultItem`](src/searxng/types.rs:41)）。

### 3.4 轨迹流动（SiliconFlow）重排序机制

当启用 [`SILICONFLOW_API_KEY`](.env.example:4) 并使用带重排能力的客户端时，搜索流程会在“结果数超过 limit”时触发重排序（见 [`SearxngClient::search()`](src/searxng/client.rs:33)）：

1) 将候选结果拼接为文档列表：`"{url} - {description}"`
2) 生成增强 query（注入 `category` 与用户 query）
3) 调用 SiliconFlow Rerank API（端点见 [`RERANK_API_ENDPOINT`](src/rerank/client.rs:7)）
4) 按相关性分数从高到低重排（排序逻辑见 [`RerankClient::rerank()`](src/rerank/client.rs:45)）

重排序失败时会降级为 searXNG 原始顺序返回（见 [`SearxngClient::search()`](src/searxng/client.rs:33)）。

