# just 命令说明

本项目的根目录维护一个 `justfile`，实际脚本都放在 `just/` 目录中。脚本会根据自身位置反推项目根目录，因此可以在不同机器、不同项目路径下重新部署。

## 前置条件

- 安装 `just`
- 安装 Rust/Cargo
- macOS 部署命令需要 `launchctl`
- 项目根目录存在 `.env`

`.env` 中继续维护运行时配置，例如：

- `SEARXNG_URL`
- `MCP_BIND`
- `MCP_AUTH_TOKEN`
- `SILICONFLOW_API_KEY`

## 常用命令

```bash
just check
```

加载 `.env` 后执行 `cargo check`。如果 `.env` 不存在，也可以只做代码检查。

```bash
just build
```

加载 `.env` 后构建 release 二进制：`target/release/searxng_mcp`。如果 `.env` 不存在，也可以只做构建。

```bash
just run
```

在项目根目录启动本地服务，程序会从 `.env` 读取配置。

```bash
just deploy
```

构建 release 二进制，并部署为当前用户的 macOS `LaunchAgent`。默认服务标签为：

```text
com.factrue.openperplexity.searxng-mcp
```

部署后会生成：

```text
~/Library/LaunchAgents/com.factrue.openperplexity.searxng-mcp.plist
```

plist 中只保存项目路径、二进制路径、日志路径和 `RUST_LOG`，不会把 `.env` 的密钥写入 plist。服务启动时的工作目录是项目根目录，因此程序会自动读取该目录下的 `.env`。

```bash
just status
```

查看 launchd 中的服务状态。

```bash
just restart
```

重启已经部署的 launchd 服务。

```bash
just logs
just logs 300
```

查看 launchd 标准输出和标准错误日志，默认最近 120 行。

```bash
just undeploy
```

从 launchd 卸载服务并删除 plist，不会删除 release 二进制或日志文件。

## 可配置环境变量

可以在执行 just 命令时覆盖以下变量：

```bash
OPENPERPLEXITY_SERVICE_LABEL=com.example.openperplexity just deploy
OPENPERPLEXITY_BIN=searxng_mcp just build
OPENPERPLEXITY_RUST_LOG=debug just deploy
```

- `OPENPERPLEXITY_SERVICE_LABEL`：macOS 服务标签，默认 `com.factrue.openperplexity.searxng-mcp`
- `OPENPERPLEXITY_BIN`：Cargo bin 名称，默认 `searxng_mcp`
- `OPENPERPLEXITY_RUST_LOG`：写入 launchd plist 的 `RUST_LOG`，默认 `info`

## 重新部署

如果项目移动到了新目录，或者换到另一台机器，重新执行：

```bash
just deploy
```

脚本会重新构建二进制并写入新的绝对路径。
