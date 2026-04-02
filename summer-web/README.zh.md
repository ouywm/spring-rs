[![crates.io](https://img.shields.io/crates/v/summer-web.svg)](https://crates.io/crates/summer-web)
[![Documentation](https://docs.rs/summer-web/badge.svg)](https://docs.rs/summer-web)

[Axum](https://github.com/tokio-rs/axum)是rust社区最优秀的Web框架之一，它是由tokio官方维护的一个基于[hyper](https://github.com/hyperium/hyper)的子项目。Axum提供了web路由，声明式的HTTP请求解析，HTTP响应的序列化等功能，而且能够与[tower](https://github.com/tower-rs)生态中的中间件结合。

## 依赖

```toml
summer-web = { version = "<version>" }
```

可选的**features**:
* `http2`: http2
* `multipart`: 文件上传
* `ws`: websocket
* `socket_io`：SocketIO 支持
* `openapi`: openapi文档
* `openapi-redoc`: redoc文档界面
* `openapi-scalar`: scalar文档界面
* `openapi-swagger`: swagger文档界面
* `validator`: validator 验证提取器（独立使用，不依赖 axum-valid）
* `garde`: garde 验证提取器（独立使用，支持自定义 Context）
* `axum-valid`: axum-valid 兼容层（可与 validator/garde 组合使用）

## 配置项

```toml
[web]
binding = "172.20.10.4"  # 要绑定的网卡IP地址，默认0.0.0.0
port = 8000              # 要绑定的端口号，默认8080
connect_info = false     # 是否使用客户端连接信息，默认false
graceful = true          # 是否开启优雅停机, 默认false
global_prefix = "api"    # 所有路由的全局前缀，默认为空

# web中间件配置
[web.middlewares]
compression = { enable = true }  # 开启压缩中间件
catch_panic = { enable = true }  # 捕获handler产生的panic
logger = { enable = true, level = "info" }            # 开启日志中间件
limit_payload = { enable = true, body_limit = "5MB" } # 限制请求体大小
timeout_request = { enable = true, timeout = 60000 }  # 请求超时时间60s

# 跨域配置
cors = { enable = true, allow_origins = [
    "https://summer-rs.github.io",
], allow_headers = [
    "Authentication",
], allow_methods = [
    "*",
], max_age = 60 }

# 静态资源配置
static = { enable = true, uri = "/static", path = "static", precompressed = true, fallback = "index.html" }
```

> **NOTE**: 通过上面的middleware配置可以集成tower生态中提供的中间件。当然如果你对tower生态非常熟悉，也可以不启用这些middleware，通过编写代码自行配置。下面是相关的文档链接：
> * [tower](https://docs.rs/tower/latest/tower/)
> * [tower-http](https://docs.rs/tower-http/latest/tower_http/)

## API接口

App实现了[WebConfigurator](https://docs.rs/summer-web/latest/summer_web/trait.WebConfigurator.html)特征，可以通过该特征指定路由配置：

```no_run, rust, linenos, hl_lines=6 10-18
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SqlxPlugin)
        .add_plugin(WebPlugin)
        .add_router(router())
        .run()
        .await
}

fn router() -> Router {
    Router::new().typed_route(hello_word)
}

#[get("/")]
async fn hello_word() -> impl IntoResponse {
    "hello word"
}

/// # API的标题必须用markdown格式的h1
/// API描述信息
/// API描述支持多行文本
/// get_api宏会自动收集请求参数和响应的schema
/// @tag api_tag 支持多个
#[get_api("/api")]
async fn hello_api() -> String {
   "hello api".to_string()
}
```

你也可以使用`auto_config`宏来实现自动配置，这个过程宏会自动将被过程宏标记的路由注册进app中：

```diff
+#[auto_config(WebConfigurator)]
 #[tokio::main]
 async fn main() {
     App::new()
         .add_plugin(SqlxPlugin)
         .add_plugin(WebPlugin)
-        .add_router(router())
         .run()
         .await
}
```

## 属性宏

上面例子中的[`get`](https://docs.rs/summer-macros/latest/summer_macros/attr.get.html)是一个属性宏，`summer-web`提供了八个标准HTTP METHOD的过程宏：`get`、`post`、`patch`、`put`、`delete`、`head`、`trace`、`options`。另外还提供了`get_api`、`post_api`等八个用于生成openapi文档的宏。

也可以使用[`route`](https://docs.rs/summer-macros/latest/summer_macros/attr.route.html)或[`api_route`](https://docs.rs/summer-macros/latest/summer_macros/attr.api_route.html)宏同时绑定多个method：

```rust
use summer_web::route;
use summer_web::axum::response::IntoResponse;

#[route("/test", method = "GET", method = "HEAD")]
async fn example() -> impl IntoResponse {
    "hello world"
}
```

除此之外，summer还支持一个handler绑定多个路由，这需要用到[`routes`](https://docs.rs/summer-macros/latest/summer_macros/attr.routes.html)属性宏：

```rust
use summer_web::{routes, get, delete};
use summer_web::axum::response::IntoResponse;

#[routes]
#[get("/test")]
#[get("/test2")]
#[delete("/test")]
async fn example() -> impl IntoResponse {
    "hello world"
}
```

## 提取插件注册的Component

上面的例子中`SqlxPlugin`插件为我们自动注册了一个Sqlx连接池组件，我们可以使用`Component`从State中提取这个连接池，[`Component`](https://docs.rs/summer-web/latest/summer_web/extractor/struct.Component.html)是一个axum的[extractor](https://docs.rs/axum/latest/axum/extract/index.html)。

```rust
#[get("/version")]
async fn mysql_version(Component(pool): Component<ConnectPool>) -> Result<String> {
    let version = sqlx::query("select version() as version")
        .fetch_one(&pool)
        .await
        .context("sqlx query failed")?
        .get("version");
    Ok(version)
}
```

axum也提供了其他的[extractor](https://docs.rs/axum/latest/axum/extract/index.html)，这些都被reexport到了[`summer_web::extractor`](https://docs.rs/summer-web/latest/summer_web/extractor/index.html)下。

## 读取配置

你可以用[`Config`](https://docs.rs/summer-web/latest/summer_web/extractor/struct.Config.html)抽取toml中的配置。

```rust
#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "custom"]
struct CustomConfig {
    a: u32,
    b: bool,
}

#[get("/config")]
async fn use_toml_config(Config(conf): Config<CustomConfig>) -> impl IntoResponse {
    format!("a={}, b={}", conf.a, conf.b)
}
```

在你的配置文件中添加相应配置：

```toml
[custom]
a = 1
b = true
```

完整代码参考[`web-example`](https://github.com/summer-rs/summer-rs/tree/master/examples/web-example)

## 在Middleware中使用Component抽取注册的组件

你也可以在[middleware中使用Extractor](https://docs.rs/axum/latest/axum/middleware/fn.from_fn.html)，注意需要遵循axum的规则。

```rust
use summer_web::{middlewares, axum::middleware};

/// 你可以通过middlewares宏来使用上面定义的middleware
#[middlewares(
    middleware::from_fn(problem_middleware),
)]
mod routes {
    use summer_web::{axum::{response::Response, middleware::Next, response::IntoResponse}, extractor::{Request, Component}};
    use summer_sqlx::ConnectPool;
    use summer_web::{middlewares, get, axum::middleware};
    use std::time::Duration;

    async fn problem_middleware(Component(db): Component<ConnectPool>, request: Request, next: Next) -> Response {
        // do something
        let response = next.run(request).await;

        response
    }

    #[get("/")]
    async fn hello_world() -> impl IntoResponse {
        "hello world"
    }

}
```


完整代码参考[`web-middleware-example`](https://github.com/summer-rs/summer-rs/tree/master/examples/web-middleware-example)

summer-web是围绕axum的一层薄薄的封装, 提供了一些宏以简化开发. [axum官方的examples](https://github.com/tokio-rs/axum/tree/main/examples)大多只要稍作修改即可运行在summer-web中。


# SocketIO 支持

你可以启用 `summer-web` 的 `socket_io` 功能，以使用与 [socketioxide](https://github.com/Totodore/socketioxide) 的集成。

SocketIO 是 WebSocket 的一种实现，提供更多的定义功能：

* 命名事件（例如 `chat message`、`user joined` 等），而不仅仅是普通消息
* 连接丢失时自动重连
* 心跳机制，用于检测失效连接
* 房间 / 命名空间，用于对客户端进行分组
* 如果 WebSocket 不可用，可回退到其他传输方式

你可以参考 [socketio-example](https://github.com/summer-rs/summer-rs/tree/master/examples/web-socketio-example) 来查看在 summer-web 中使用 SocketIO 的示例。

我们可以在 SocketIO 处理器中共享插件注册的组件，就像在普通 HTTP 处理器中一样，例如使用由 `SqlxPlugin` 插件注册的 Sqlx 连接池组件。

# OpenAPI 支持

你可以启用 `summer-web` 的 `openapi` 功能来生成 OpenAPI 文档。你可以参考 [openapi-example](https://github.com/summer-rs/summer-rs/tree/master/examples/openapi-example) 获取更多信息。

此外，你需要启用以下文档界面功能之一：`openapi-redoc`、`openapi-scalar` 或 `openapi-swagger`，以生成相应的文档界面。

```rust,ignore
/// 始终返回错误  
/// 
/// 此端点使用 Errors::B 和 Errors::C 的 status_codes 注解
/// @tag error
/// @status_codes Errors::B, Errors::C, Errors::SqlxError, Errors::TeaPod
#[get_api("/error")]
async fn error() -> Result<Json<String>, Errors> {
    Err(Errors::B)
}
```

要生成 OpenAPI 文档，你可以使用 `get_api`、`post_api` 等宏来定义你的 API 端点。这些宏会自动收集请求参数和响应模式以生成 OpenAPI 文档。

API 函数上方的注释用于为 OpenAPI 文档提供附加信息，例如标签（tags）和状态码（status codes）。

`status_codes` 注解指定了 API 可能返回的错误类型。这些信息将包含在 OpenAPI 文档中，使用户能够了解调用此 API 时的潜在错误响应。

如果你想定义自定义错误类型，可以使用 `ProblemDetails` 派生宏，它会自动实现 `From<T> for ProblemDetails` 和 `IntoResponse` trait，用于在 OpenAPI 文档中将错误映射成 [RFC 7807](https://www.rfc-editor.org/rfc/rfc7807)和[RFC 9457](https://www.rfc-editor.org/rfc/rfc9457.html)中定义的Problem Details标准格式。

在此示例中，我们实现了 `thiserror::Error` 以获得更好的错误处理，但这不是强制的。

```rust,ignore
use summer_web::ProblemDetails;
use summer_web::axum::http::StatusCode;

// 只需要派生 ProblemDetails - From 和 IntoResponse 都会自动生成！
#[derive(thiserror::Error, Debug, ProblemDetails)]
pub enum CustomErrors {
    #[status_code(400)]
    #[error("发生了基本错误")]
    ABasicError,

    #[status_code(500)]
    #[error(transparent)]
    SqlxError(#[from] summer_sqlx::sqlx::Error),

    #[status_code(418)]
    #[error("TeaPod 错误发生: {0:?}")]
    TeaPod(CustomErrorSchema),
}

// 不需要手动实现 IntoResponse！
// 可以直接在处理器中返回 Result<T, CustomErrors>

#[derive(Debug, JsonSchema)]
pub struct CustomErrorSchema {
    pub code: u16,
    pub message: String,
}
```

# 验证支持

summer-web 提供了两套验证框架的集成，两者都能将验证失败转换为 ProblemDetails 响应。

## Feature 组合

```toml
# 仅使用 validator（自建提取器，不依赖 axum-valid）
summer-web = { version = "<version>", features = ["validator"] }

# 仅使用 garde（自建提取器，支持自定义 Context）
summer-web = { version = "<version>", features = ["garde"] }

# 两者同时使用
summer-web = { version = "<version>", features = ["validator", "garde"] }

# 额外兼容 axum-valid（可选）
summer-web = { version = "<version>", features = ["validator", "garde", "axum-valid"] }
```

## Validator 验证

使用 `ValidatorJson`、`ValidatorQuery`、`ValidatorPath`、`ValidatorForm` 提取器：

```rust,ignore
use summer_web::validation::validator::{ValidatorJson, ValidatorQuery};
use validator::Validate;

#[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 100, message = "名称长度必须在1到100之间"))]
    pub name: String,
    #[validate(email(message = "必须是有效的邮箱地址"))]
    pub email: String,
    #[validate(range(min = 0, max = 150))]
    pub age: Option<i32>,
}

#[post_api("/users")]
async fn create_user(
    ValidatorJson(body): ValidatorJson<CreateUserRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "name": body.name, "email": body.email }))
}
```

验证失败时返回：

```json
{
  "type": "about:blank",
  "title": "Validation Error",
  "status": 422,
  "violations": [
    { "field": "email", "message": "必须是有效的邮箱地址", "in": "body" }
  ]
}
```

## Garde 验证

使用 `GardeJson`、`GardeQuery`、`GardePath`、`GardeForm` 提取器：

```rust,ignore
use summer_web::validation::garde::{GardeJson, GardeQuery};

// 场景 A：Context = ()，零配置
#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
pub struct CreateUserRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(email)]
    pub email: String,
}

#[post_api("/users")]
async fn create_user(
    GardeJson(body): GardeJson<CreateUserRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "name": body.name, "email": body.email }))
}
```

### Garde 自定义 Context

当验证规则需要运行时配置时，使用 `ValidationContextRegistry`：

```rust,ignore
use summer_web::validation::context::ValidationContextRegistry;

#[derive(Clone, Debug)]
pub struct UserRules {
    pub min_name: usize,
    pub max_name: usize,
}

// 通过 #[component] 注册验证上下文
#[summer::component]
fn create_validation_contexts() -> ValidationContextRegistry {
    let mut registry = ValidationContextRegistry::new();
    registry.insert(UserRules { min_name: 2, max_name: 50 });
    registry
}

// 使用 GardeSchema 替代 JsonSchema（解决 context 表达式冲突）
#[derive(Debug, Deserialize, garde::Validate, summer_web::GardeSchema)]
#[garde(context(UserRules as ctx))]
pub struct CreateUserRequest {
    #[garde(length(min = ctx.min_name, max = ctx.max_name))]
    pub name: String,
    #[garde(email)]
    pub email: String,
}
```

## OpenAPI 约束注入

`ValidatorSchema` 和 `GardeSchema` 宏会自动将验证属性中的字面量值注入到 JSON Schema 中，
使 OpenAPI 文档包含 `minLength`、`maximum`、`format` 等约束信息：

```rust,ignore
// 使用 ValidatorSchema（替代 JsonSchema）
#[derive(Deserialize, summer_web::ValidatorSchema, Validate)]
pub struct Foo {
    #[validate(length(min = 1, max = 100))]
    pub name: String,        // → OpenAPI: {"type": "string", "minLength": 1, "maxLength": 100}
    #[validate(email)]
    pub email: String,       // → OpenAPI: {"type": "string", "format": "email"}
}

// 使用 GardeSchema（替代 JsonSchema）
#[derive(Deserialize, summer_web::GardeSchema, garde::Validate)]
pub struct Bar {
    #[garde(length(min = 1, max = 100))]
    pub name: String,        // → OpenAPI: {"type": "string", "minLength": 1, "maxLength": 100}
    #[garde(email)]
    pub email: String,       // → OpenAPI: {"type": "string", "format": "email"}
}
```

> **注意**：如果不需要 OpenAPI 约束注入，也可以直接使用 `#[derive(JsonSchema)]`，
> 但生成的 schema 中不会包含 minLength、maximum 等验证信息。

完整代码参考 [`openapi-example`](https://github.com/summer-rs/summer-rs/tree/master/examples/openapi-example)
