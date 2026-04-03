+++
title = "Migrating from spring-rs to summer-rs"
description = "spring-rs has been officially renamed to summer-rs. This post explains the reasoning behind the rename and how to migrate your existing projects."
date = 2026-03-21T10:00:00+00:00
updated = 2026-03-21T10:00:00+00:00
draft = false
template = "blog/page.html"

[extra]
lead = "spring-rs has been officially renamed to summer-rs. This post explains the reasoning behind the rename and how to migrate your existing projects."
+++

## Why the rename?

Starting from version 0.5.0, **spring-rs** has been officially renamed to **summer-rs**.

spring-rs was originally named after Java's SpringBoot as its inspiration. As the project grew, we decided it needed its own brand identity. "Summer" keeps the seasonal connection to "spring" while giving the project its own personality — summer-rs is an independent, vibrant application framework in the Rust ecosystem.

The rename does not involve any architectural or functional breaking changes. The core API remains the same. You only need a simple find-and-replace to complete the migration.

## Migration steps

### 1. Update Cargo.toml dependencies

Replace all `spring-*` dependencies with `summer-*`:

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

Full crate name mapping:

| Old name | New name |
|----------|----------|
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

### 2. Update use statements in Rust source code

Replace all `spring` imports with `summer`:

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

Note that Rust crate names use underscores in `use` statements: `spring_web` → `summer_web`.

### 3. Update configuration files

Update the schema URL in `config/app.toml`:

```diff
-#:schema https://spring-rs.github.io/config-schema.json
+#:schema https://summer-rs.github.io/config-schema.json
```

The configuration sections themselves (e.g., `[web]`, `[sqlx]`, `[redis]`) do not need to change — they are independent of the crate names.

### 4. Quick replacement commands

Run these commands from your project root to do most of the work:

```bash
# Replace dependency names in Cargo.toml
find . -name "Cargo.toml" -exec sed -i '' 's/spring-/summer-/g; s/spring =/summer =/g' {} +

# Replace use statements in Rust source files
find . -name "*.rs" -exec sed -i '' 's/spring_/summer_/g; s/use spring/use summer/g' {} +

# Replace schema URL in config files
find . -name "app.toml" -exec sed -i '' 's/spring-rs\.github\.io/summer-rs.github.io/g' {} +
```

After replacing, run `cargo check` to verify everything compiles.

## Example after migration

Here is what a Web application looks like after migration:

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

As you can see, apart from the `use` paths changing from `spring` to `summer`, all other code remains identical. Macros, traits, and configuration patterns are unchanged.

## IDE support

The summer-rs VSCode extension has been updated accordingly. Search for `summer-rs` in the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=summer-rs.summer-rs) to install it. The extension provides route navigation, component views, dependency graph visualization, and more.

## Summary

This rename is purely a branding update with no API changes. Migration only requires a global replacement of `spring` → `summer` and can be done in minutes. If you run into any issues, feel free to report them on [GitHub Issues](https://github.com/summer-rs/summer-rs/issues).
