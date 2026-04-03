# Agents 指南

本文档面向在本仓库中协助开发或审查代码的自动化代理（AI Agent）与人类贡献者，说明项目结构、常用命令与约定。

## 项目是什么

**summer-rs** 是受 Java Spring Boot 启发的 Rust 应用框架：约定优于配置、可插拔插件、TOML 配置。核心 crate 为 `summer`；生态插件以 `summer-*` 命名（如 `summer-web`、`summer-sqlx`）。

- 官方文档：<https://summer-rs.github.io/>
- 仓库：<https://github.com/summer-rs/summer-rs>

## 工作区布局

| 路径 | 说明 |
|------|------|
| `summer/` | 核心：配置、插件、组件与 `App` |
| `summer-macros/` | 过程宏（如 `#[auto_config]`） |
| `summer-web/`、`summer-grpc/`、`summer-job/` 等 | 官方插件 |
| `examples/*` | 示例应用（工作区成员；根目录 `default-members` 不含示例，全量测试请用 `cargo test --workspace`） |
| `docs/` | 站点与文档源码（Zola 等） |

插件需实现 `Plugin`；配置需实现 `Configurable`；可作为组件注入的类型需 `Clone`。详见 `summer/Plugin.md` 与 `summer/DI.md`。

## 构建与检查

在仓库根目录执行：

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
```

修改某 crate 时，可只针对该路径缩短迭代，例如：

```bash
cargo test -p summer
cargo test -p summer-web
```

## 修改时的约定

- **范围**：优先只改与任务直接相关的文件与 crate，避免无关格式化或大范围重排。
- **风格**：与相邻代码保持一致（命名、错误处理、`async`/trait 用法）；新代码在提交前应能通过 `cargo fmt` 与 `clippy`。
- **依赖**：工作区统一版本在根目录 `Cargo.toml` 的 `[workspace.dependencies]`；子 crate 用 `workspace = true` 引用。
- **文档**：除非任务要求，不要随意新增或大面积改写用户文档；核心行为变更时考虑同步 `CHANGELOG` 或对应 README。

## 快速定位

- 应用入口与插件装配：`summer` 中的 `App`、`AppBuilder`。
- Web 路由与提取器：`summer-web`。
- 数据库：`summer-sqlx`、`summer-sea-orm`、`summer-postgres` 等按场景选用。

若不确定行为，优先阅读对应 crate 下的 `README.md` 及 `summer/` 内专题 Markdown（如 `Config.md`、`COMPONENT_MACRO.md`）。
