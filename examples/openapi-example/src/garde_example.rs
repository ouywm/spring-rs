// Garde-based validation examples
//
// Scenario A: Context = () (zero-config, literal values)
// Scenario B: Custom Context (runtime rules via ValidationContextRegistry)

use serde::Deserialize;
use summer_web::axum::Json;
use summer_web::validation::garde::{GardeForm, GardeJson, GardePath, GardeQuery};
use summer_web::{get_api, post_api};

// GardeSchema generates JsonSchema with constraint injection from #[garde(...)] attrs.
// When all values are literals, constraints (minLength, maximum, etc.) appear in OpenAPI.

#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
pub struct GardeCreateUserRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(email)]
    pub email: String,
    #[garde(range(min = 0, max = 150))]
    pub age: Option<i32>,
}

#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
pub struct GardeListUsersQuery {
    #[garde(range(min = 1))]
    pub page: Option<i32>,
    #[garde(range(min = 1, max = 100))]
    pub size: Option<i32>,
}

#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
pub struct GardeUserIdPath {
    #[garde(range(min = 1))]
    pub id: u32,
}

#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
pub struct GardeSignupForm {
    #[garde(length(min = 1, max = 32))]
    pub username: String,
    #[garde(email)]
    pub email: String,
}

/// Create a new user — garde validation (Context = (), zero-config)
///
/// Uses `GardeJson<T>` extractor. No registry, no context — just
/// derive `garde::Validate` and it works.
///
/// @tag Garde Validation
#[post_api("/garde/users")]
async fn garde_create_user(
    GardeJson(body): GardeJson<GardeCreateUserRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1,
        "name": body.name,
        "email": body.email,
        "age": body.age,
    }))
}

/// List users — garde query validation
///
/// @tag Garde Validation
#[get_api("/garde/users")]
async fn garde_list_users(
    GardeQuery(query): GardeQuery<GardeListUsersQuery>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "page": query.page.unwrap_or(1),
        "size": query.size.unwrap_or(20),
        "total": 0,
        "items": [],
    }))
}

/// Get user by ID — garde path validation
///
/// @tag Garde Validation
#[get_api("/garde/users/{id}")]
async fn garde_get_user(GardePath(path): GardePath<GardeUserIdPath>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": path.id,
        "name": "Garde User",
        "email": "garde@example.com",
    }))
}

/// Submit signup form — garde form validation
///
/// @tag Garde Validation
#[post_api("/garde/signup-form")]
async fn garde_signup_form(GardeForm(body): GardeForm<GardeSignupForm>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 3001,
        "username": body.username,
        "email": body.email,
    }))
}

// Uses #[component] to register a ValidationContextRegistry.
// The GardeJson extractor automatically looks up the context at request time.

#[derive(Clone, Debug)]
pub struct UserValidationRules {
    pub min_name: usize,
    pub max_name: usize,
}

/// Register all validation contexts via #[component].
///
/// See `ValidationContextRegistry` docs for the module-level registration pattern.
#[summer::component]
fn create_validation_contexts() -> summer_web::validation::context::ValidationContextRegistry {
    use summer_web::validation::context::ValidationContextRegistry;
    let mut registry = ValidationContextRegistry::new();
    registry.insert(UserValidationRules {
        min_name: 2,
        max_name: 50,
    });
    registry
}

// GardeSchema generates JsonSchema without conflicting with garde context expressions.
// Runtime values (ctx.min_name) are skipped — only `email` gets `format: "email"`.
#[derive(Debug, Deserialize, garde::Validate, summer_web::GardeSchema)]
#[garde(context(UserValidationRules as ctx))]
pub struct GardeContextCreateUserRequest {
    #[garde(length(min = ctx.min_name, max = ctx.max_name))]
    pub name: String,
    #[garde(email)]
    pub email: String,
}

/// Create user with runtime context rules — garde validation (Scenario B)
///
/// The validation rules (min_name=2, max_name=50) come from the
/// `UserValidationRules` context in the `ValidationContextRegistry`,
/// registered via `#[component]` at startup.
///
/// @tag Garde Context Validation
#[post_api("/garde/context/users")]
async fn garde_context_create_user(
    GardeJson(body): GardeJson<GardeContextCreateUserRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1,
        "name": body.name,
        "email": body.email,
        "rules": "min_name=2, max_name=50 (from UserValidationRules context)",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn garde_extractors_support_openapi_input() {
        assert_operation_input::<GardeJson<GardeCreateUserRequest>>();
        assert_operation_input::<GardeQuery<GardeListUsersQuery>>();
        assert_operation_input::<GardePath<GardeUserIdPath>>();
        assert_operation_input::<GardeForm<GardeSignupForm>>();
        assert_operation_input::<GardeJson<GardeContextCreateUserRequest>>();
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
    struct AllConstraints {
        // length → minLength / maxLength
        #[garde(length(min = 1, max = 100))]
        length_field: String,

        // length(equal) → minLength = maxLength = N
        #[garde(length(equal = 10))]
        exact_length_field: String,

        // range → minimum / maximum (integers)
        #[garde(range(min = 0, max = 150))]
        range_int_field: i32,

        // range → minimum / maximum (floats)
        #[garde(range(min = -1.5, max = 99.9))]
        range_float_field: f64,

        // email → format: "email"
        #[garde(email)]
        email_field: String,

        // url → format: "uri"
        #[garde(url)]
        url_field: String,

        // ip → format: "ip"
        #[garde(ip)]
        ip_field: String,

        // alphanumeric → pattern: "^[a-zA-Z0-9]*$"
        #[garde(alphanumeric)]
        alphanumeric_field: String,

        // pattern("regex") → pattern
        #[garde(pattern(r"^[0-9]{3}-[0-9]{4}$"))]
        pattern_field: String,

        // contains("str") → pattern (escaped)
        #[garde(contains("hello"))]
        contains_field: String,

        // prefix("str") → pattern: "^str"
        #[garde(prefix("https://"))]
        prefix_field: String,

        // suffix("str") → pattern: "str$"
        #[garde(suffix(".rs"))]
        suffix_field: String,

        // ascii — no JSON Schema mapping (should be empty)
        #[garde(ascii)]
        ascii_field: String,

        // skip — no constraints
        #[garde(skip)]
        skip_field: String,
    }

    #[test]
    fn garde_schema_all_constraints() {
        use schemars::SchemaGenerator;

        let mut gen = SchemaGenerator::default();
        let schema = <AllConstraints as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json["properties"].as_object().unwrap();

        // length(min = 1, max = 100) → minLength/maxLength
        let f = &props["length_field"];
        assert_eq!(f["minLength"], 1);
        assert_eq!(f["maxLength"], 100);

        // length(equal = 10) → minLength = maxLength = 10
        let f = &props["exact_length_field"];
        assert_eq!(f["minLength"], 10);
        assert_eq!(f["maxLength"], 10);

        // range int
        let f = &props["range_int_field"];
        assert_eq!(f["minimum"], 0);
        assert_eq!(f["maximum"], 150);

        // range float (negative min)
        let f = &props["range_float_field"];
        assert_eq!(f["minimum"], -1.5);
        assert_eq!(f["maximum"], 99.9);

        // email
        assert_eq!(props["email_field"]["format"], "email");

        // url
        assert_eq!(props["url_field"]["format"], "uri");

        // ip
        assert_eq!(props["ip_field"]["format"], "ip");

        // alphanumeric
        assert_eq!(props["alphanumeric_field"]["pattern"], "^[a-zA-Z0-9]*$");

        // pattern (raw regex passthrough)
        assert_eq!(props["pattern_field"]["pattern"], r"^[0-9]{3}-[0-9]{4}$");

        // contains("hello") → pattern with escaped string
        assert_eq!(props["contains_field"]["pattern"], "hello");

        // prefix("https://") → "^https://"
        assert_eq!(props["prefix_field"]["pattern"], "^https://");

        // suffix(".rs") → "\\.rs$"  (dot escaped)
        assert_eq!(props["suffix_field"]["pattern"], "\\.rs$");

        // ascii — no constraint mapped
        let f = &props["ascii_field"];
        assert!(f.get("format").is_none());
        assert!(f.get("pattern").is_none());

        // skip — no constraint mapped
        let f = &props["skip_field"];
        assert!(f.get("format").is_none());
        assert!(f.get("pattern").is_none());
    }

    #[test]
    fn garde_schema_injects_constraints() {
        use schemars::SchemaGenerator;

        let mut gen = SchemaGenerator::default();
        let schema = <GardeCreateUserRequest as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json["properties"].as_object().unwrap();

        let name = &props["name"];
        assert_eq!(name["minLength"], 1, "name.minLength");
        assert_eq!(name["maxLength"], 100, "name.maxLength");

        let email = &props["email"];
        assert_eq!(email["format"], "email", "email.format");

        let age = &props["age"];
        assert_eq!(age["minimum"], 0, "age.minimum");
        assert_eq!(age["maximum"], 150, "age.maximum");
    }

    #[test]
    fn garde_schema_context_skips_runtime_exprs() {
        use schemars::SchemaGenerator;

        let mut gen = SchemaGenerator::default();
        let schema =
            <GardeContextCreateUserRequest as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json["properties"].as_object().unwrap();

        let name = &props["name"];
        assert!(name.get("minLength").is_none(), "runtime expr should be skipped");
        assert!(name.get("maxLength").is_none(), "runtime expr should be skipped");

        let email = &props["email"];
        assert_eq!(email["format"], "email", "email.format should still be injected");
    }
}
