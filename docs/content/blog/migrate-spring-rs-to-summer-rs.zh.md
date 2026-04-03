+++
title = "从 spring-rs 迁移到 summer-rs"
description = "spring-rs 正式更名为 summer-rs。本文介绍更名的原因以及如何将现有项目从 spring-rs 迁移到 summer-rs。"
date = 2026-03-21T10:00:00+00:00
updated = 2026-03-21T10:00:00+00:00
draft = false
template = "blog/page.html"

[extra]
lead = "spring-rs 正式更名为 summer-rs。本文介绍更名的原因以及如何将现有项目从 spring-rs 迁移到 summer-rs。"
+++

## 为什么更名？

从 0.5.0 版本开始，**spring-rs** 正式更名为 **summer-rs**。

spring-rs 最初以 Java SpringBoot 为灵感命名，但随着项目的发展，我们意识到需要一个独立的品牌标识。"summer" 既保留了与 "spring" 的季节关联，又赋予了项目自己的个性——在 Rust 生态中，summer-rs 是一个独立的、充满活力的应用框架。

更名不涉及任何架构或功能上的 breaking change，核心 API 保持不变。你只需要做一些简单的查找替换就能完成迁移。

## 迁移步骤

### 1. 更新 Cargo.toml 依赖

将所有 `spring-*` 依赖替换为 `summer-*`：

```diff
 [dependencies]
-spring = "0.4"
-spring-web = "0.4"
-spring-sqlx = "0.4"
-spring-job = "0.4"
-spring-redis = "0.4"
+summer = "0.5"
+summer-web = "0.5"
+summer-sqlx = "0.5"
+summer-job = "0.5"
+summer-redis = "0.5"
```

完整的 crate 名称映射：

| 旧名称 | 新名称 |
|--------|--------|
| `spring` | `summer` |
| `spring-macros` | `summer-macros` |
| `spring-web` | `summer-web` |
| `spring-sqlx` | `summer-sqlx` |
| `spring-postgres` | `summer-postgres` |
| `spring-sea-orm` | `summer-sea-orm` |
| `spring-redis` | `summer-redis` |
| `spring-mail` | `summer-mail` |
| `spring-job` | `summer-job` |
| `spring-stream` | `summer-stream` |
| `spring-opentelemetry` | `summer-opentelemetry` |
| `spring-grpc` | `summer-grpc` |
| `spring-opendal` | `summer-opendal` |
| `spring-apalis` | `summer-apalis` |
| `spring-sa-token` | `summer-sa-token` |

### 2. 更新 Rust 源码中的 use 语句

将所有 `spring` 开头的 import 替换为 `summer`：

```diff
-use spring::{auto_config, App};
-use spring_web::{get, route};
-use spring_web::{
-    error::Result, extractor::{Path, Component},
-    WebConfigurator, WebPlugin,
-};
-use spring_sqlx::{sqlx, ConnectPool, SqlxPlugin};
+use summer::{auto_config, App};
+use summer_web::{get, route};
+use summer_web::{
+    error::Result, extractor::{Path, Component},
+    WebConfigurator, WebPlugin,
+};
+use summer_sqlx::{sqlx, ConnectPool, SqlxPlugin};
```

注意 Rust 的 crate 名称在 `use` 语句中使用下划线：`spring_web` → `summer_web`。

### 3. 更新配置文件

配置文件 `config/app.toml` 中的 schema 地址需要更新：

```diff
-#:schema https://spring-rs.github.io/config-schema.json
+#:schema https://summer-rs.github.io/config-schema.json
```

配置项本身（如 `[web]`、`[sqlx]`、`[redis]` 等）不需要修改，它们与 crate 名称无关。

### 4. 快速替换命令

在项目根目录下，可以用以下命令一键完成大部分替换：

```bash
# 替换 Cargo.toml 中的依赖名
find . -name "Cargo.toml" -exec sed -i '' 's/spring-/summer-/g; s/spring =/summer =/g' {} +

# 替换 Rust 源码中的 use 语句
find . -name "*.rs" -exec sed -i '' 's/spring_/summer_/g; s/use spring/use summer/g' {} +

# 替换配置文件中的 schema 地址
find . -name "app.toml" -exec sed -i '' 's/spring-rs\.github\.io/summer-rs.github.io/g' {} +
```

替换完成后，运行 `cargo check` 确认编译通过。

## 迁移后的示例

迁移后的 Web 应用代码如下：

```rust
use summer::{auto_config, App};
use summer_sqlx::{
    sqlx::{self, Row},
    ConnectPool, SqlxPlugin,
};
use summer_web::{get, route};
use summer_web::{
    error::Result,
    extractor::{Component, Path},
    axum::response::IntoResponse,
    WebConfigurator, WebPlugin,
};
use anyhow::Context;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SqlxPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[get("/")]
async fn hello_world() -> impl IntoResponse {
    "hello world"
}

#[route("/hello/{name}", method = "GET", method = "POST")]
async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    format!("hello {name}")
}

#[get("/version")]
async fn sqlx_request_handler(Component(pool): Component<ConnectPool>) -> Result<String> {
    let version = sqlx::query("select version() as version")
        .fetch_one(&pool)
        .await
        .context("sqlx query failed")?
        .get("version");
    Ok(version)
}
```

可以看到，除了 `use` 路径从 `spring` 变成了 `summer`，其他所有代码完全一致。宏、trait、配置方式都没有变化。

## IDE 支持

summer-rs 的 VSCode 扩展也已同步更新，在 [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=summer-rs.summer-rs) 搜索 `summer-rs` 即可安装。扩展提供了路由导航、组件视图、依赖图可视化等功能。

## 总结

这次更名是一次纯粹的品牌升级，不涉及 API 变更。迁移过程只需要全局替换 `spring` → `summer`，几分钟即可完成。如果遇到任何问题，欢迎在 [GitHub Issues](https://github.com/summer-rs/summer-rs/issues) 中反馈。
