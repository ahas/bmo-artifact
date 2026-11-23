[English](README.md) | [한국어](README.ko.md) | [日本語](README.ja.md) | **简体中文**

# bmo-artifact

用于保存、搜索和共享文档的单一服务，带有自己的 API。它管理记录、学习书、导入的网页和 HTML 页面，每份文档都有其文件、
所引用的页面、共享链接，以及与之共享以供阅读的读者所在公司的成员。harness 通过 MCP 服务器 `bmo-artifact mcp` 和
`bmo-artifact` 技能访问它，其他调用方则通过 HTTP API 和 Rust 客户端访问。它通过 bmo-explorer 的客户端读取网页，
不调用其他任何单元。

## 目录结构

| 文件夹 | 内容 |
|---|---|
| `src/` | 库和服务的可执行文件 `bmo-artifact`：路由、存储、客户端、MCP 服务器和命令 |
| `tools/` | 工具宿主放置的工具：清单、模式和可执行文件 `artifact-tool` |
| `lang/` | 文档措辞文件的英文默认值 |
| `skills/` | `bmo-artifact` 技能 |
| `client/` | 控制台包 |
| `tests/` | 测试，其中包括仓库检查 |
| `deploy/` | 中性的服务单元模板，以及部署发布的应用声明 |
| `docs/` | 描述服务行为的规范文档 |

根目录下有 Cargo 清单、`VERSION`、`build.rs`、`testbed.toml`、`install.toml`、`rustfmt.toml`、`CLAUDE.md`、
`PERSONAL-DATA.md`、`LICENSE`、`settings.toml`，以及部署时提供的本地设置示例 `settings.local.toml.example`。

## 文档

| 文档 | 内容 |
|---|---|
| `docs/api.md` | API：路由、事件、命令、客户端、MCP 工具以及对 bmo-explorer 的调用 |
| `docs/openapi.json` | 由代码生成的 OpenAPI 3.1 文档形式的 API |
| `docs/artifact.md` | 文档的设计，由 API 访问、由布局安排 |
| `docs/layout.md` | 布局、管道、控制台包、存储、版本、声明和检查 |
| `PERSONAL-DATA.md` | 任何文件都不得包含的个人数据，以及保证这一点的检查 |

## 版本

- **整个仓库只有一个版本，** 首次发行前为 `0.1.0`，之后为 `0.1.0-rev.<n>`。它保存在 `VERSION` 中，crate 和
  控制台包的版本与之相同，健康检查路由、OpenAPI 文档和 MCP 服务器都返回这个版本。

## 命令

```sh
cargo check --all-targets
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --bin bmo-artifact -- serve
cargo run --bin bmo-artifact -- mcp
cargo run --bin bmo-artifact -- openapi
```
