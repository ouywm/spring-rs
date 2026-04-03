// Validator-based validation examples (standalone extractors)

use serde::Deserialize;
use summer_web::axum::Json;
use summer_web::axum::http::HeaderName;
use summer_web::headers::{Error, Header, HeaderValue};
use summer_web::TypedHeader;
use summer_web::validation::context::ValidationContextRegistry;
use summer_web::validation::validator::{
    Validator, ValidatorEx,
};
use summer_web::{get_api, post_api};
use validator::{Validate, ValidationError};

/// ValidatorSchema generates JsonSchema with constraint injection from #[validate(...)] attrs.
#[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
pub struct CreateUserRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[validate(email(message = "must be a valid email address"))]
    pub email: String,
    #[validate(range(min = 0, max = 150, message = "age must be between 0 and 150"))]
    pub age: Option<i32>,
}

#[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
pub struct ListUsersQuery {
    #[validate(range(min = 1, message = "page must be at least 1"))]
    pub page: Option<i32>,
    #[validate(range(min = 1, max = 100, message = "size must be between 1 and 100"))]
    pub size: Option<i32>,
}

#[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
pub struct UserIdPath {
    pub id: u32,
}

#[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
pub struct CreateUserForm {
    #[validate(length(
        min = 1,
        max = 32,
        message = "username must be between 1 and 32 characters"
    ))]
    pub username: String,
    #[validate(email(message = "must be a valid email address"))]
    pub email: String,
}

#[derive(Clone, Debug)]
pub struct PageRules {
    pub max_page_size: usize,
}

fn validate_page_size(value: usize, ctx: &PageRules) -> Result<(), ValidationError> {
    if value > ctx.max_page_size {
        return Err(ValidationError::new("page_size_too_large"));
    }
    Ok(())
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Validate, summer_web::ValidatorContext)]
#[validate(context = PageRules)]
pub struct ValidatorContextQuery {
    #[validate(custom(function = "validate_page_size", use_context))]
    pub page_size: usize,
}

#[derive(Debug, Validate)]
pub struct DemoValidatorHeader {
    #[validate(length(
        min = 3,
        max = 16,
        message = "header value must be between 3 and 16 characters"
    ))]
    pub value: String,
}

static DEMO_HEADER_NAME: HeaderName = HeaderName::from_static("x-demo");

impl Header for DemoValidatorHeader {
    fn name() -> &'static HeaderName {
        &DEMO_HEADER_NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(Error::invalid)?;
        let value = value.to_str().map_err(|_| Error::invalid())?;
        Ok(Self {
            value: value.to_string(),
        })
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_str(&self.value).expect("header value");
        values.extend(std::iter::once(value));
    }
}

#[summer::component]
fn create_validator_contexts() -> ValidationContextRegistry {
    let mut registry = ValidationContextRegistry::new();
    registry.insert(PageRules { max_page_size: 100 });
    registry
}

/// Create a new user (body validation via framework extractor)
///
/// @tag Validation
#[post_api("/validation/users")]
async fn create_user(
    Validator(Json(body)): Validator<Json<CreateUserRequest>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1,
        "name": body.name,
        "email": body.email,
        "age": body.age,
    }))
}

/// List users (query parameter validation via framework extractor)
///
/// @tag Validation
#[get_api("/validation/users")]
async fn list_users(
    Validator(summer_web::axum::extract::Query(query)): Validator<summer_web::axum::extract::Query<ListUsersQuery>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "page": query.page.unwrap_or(1),
        "size": query.size.unwrap_or(20),
        "total": 0,
        "items": [],
    }))
}

/// Validator validation with runtime context arguments
///
/// @tag Validation
#[post_api("/validation/users/context")]
async fn validator_context_users(
    ValidatorEx(Json(payload)): ValidatorEx<Json<ValidatorContextQuery>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "page_size": payload.page_size,
        "rules": "max_page_size=100 (from ValidationContextRegistry)",
    }))
}

/// Get user by ID (path parameter validation via framework extractor)
///
/// @tag Validation
#[get_api("/validation/users/{id}")]
async fn get_user_by_id(
    Validator(summer_web::axum::extract::Path(path)): Validator<summer_web::axum::extract::Path<UserIdPath>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": path.id,
        "name": "Example User",
        "email": "user@example.com",
    }))
}

/// Submit a signup form (form validation via framework extractor)
///
/// @tag Validation
#[post_api("/validation/signup-form")]
async fn signup_form(
    Validator(summer_web::axum::extract::Form(body)): Validator<summer_web::axum::extract::Form<CreateUserForm>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 2001,
        "username": body.username,
        "email": body.email,
    }))
}

/// Validate a typed request header via framework extractor
///
/// @tag Validation
#[get_api("/validation/header")]
async fn validator_header_demo(
    Validator(TypedHeader(header)): Validator<TypedHeader<DemoValidatorHeader>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "header": header.value,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn validator_extractors_support_openapi_input() {
        assert_operation_input::<Validator<Json<CreateUserRequest>>>();
        assert_operation_input::<Validator<summer_web::axum::extract::Query<ListUsersQuery>>>();
        assert_operation_input::<Validator<summer_web::axum::extract::Path<UserIdPath>>>();
        assert_operation_input::<Validator<summer_web::axum::extract::Form<CreateUserForm>>>();
        assert_operation_input::<Validator<TypedHeader<DemoValidatorHeader>>>();
        assert_operation_input::<ValidatorEx<Json<ValidatorContextQuery>>>();
    }

    #[test]
    fn validator_schema_injects_constraints() {
        use schemars::SchemaGenerator;

        let mut gen = SchemaGenerator::default();
        let schema = <CreateUserRequest as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json["properties"].as_object().unwrap();

        // length(min = 1, max = 100) → minLength/maxLength
        let name = &props["name"];
        assert_eq!(name["minLength"], 1, "name.minLength");
        assert_eq!(name["maxLength"], 100, "name.maxLength");

        // email(message = "...") → format: "email"
        let email = &props["email"];
        assert_eq!(email["format"], "email", "email.format");

        // range(min = 0, max = 150) → minimum/maximum
        let age = &props["age"];
        assert_eq!(age["minimum"], 0, "age.minimum");
        assert_eq!(age["maximum"], 150, "age.maximum");
    }

    // Comprehensive test struct for all validator → JSON Schema mappings
    #[allow(dead_code)]
    #[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
    struct AllValidatorConstraints {
        #[validate(length(min = 2, max = 50))]
        length_field: String,

        #[validate(length(equal = 8))]
        exact_length_field: String,

        #[validate(range(min = -10, max = 999))]
        range_field: i32,

        #[validate(email)]
        email_field: String,

        #[validate(url)]
        url_field: String,

        #[validate(contains(pattern = "hello"))]
        contains_field: String,

        #[validate(required)]
        required_field: Option<String>,

        #[validate(length(min = 1, max = 3))]
        items_field: Vec<String>,

        plain_optional_field: Option<String>,

        // no constraint attrs → no injection
        plain_field: String,
    }

    #[test]
    fn validator_schema_all_constraints() {
        use schemars::SchemaGenerator;

        let mut gen = SchemaGenerator::default();
        let schema = <AllValidatorConstraints as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json["properties"].as_object().unwrap();

        // length
        let f = &props["length_field"];
        assert_eq!(f["minLength"], 2);
        assert_eq!(f["maxLength"], 50);

        // length(equal)
        let f = &props["exact_length_field"];
        assert_eq!(f["minLength"], 8);
        assert_eq!(f["maxLength"], 8);

        // range (with negative min)
        let f = &props["range_field"];
        assert_eq!(f["minimum"], -10);
        assert_eq!(f["maximum"], 999);

        // email
        assert_eq!(props["email_field"]["format"], "email");

        // url
        assert_eq!(props["url_field"]["format"], "uri");

        // contains
        assert_eq!(props["contains_field"]["pattern"], "hello");

        // required(option)
        let required = json["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "required_field"));
        assert!(!required.iter().any(|v| v == "plain_optional_field"));

        // collection length -> minItems/maxItems
        let f = &props["items_field"];
        assert_eq!(f["minItems"], 1);
        assert_eq!(f["maxItems"], 3);
        assert!(f.get("minLength").is_none());

        // plain — no constraints
        let f = &props["plain_field"];
        assert!(f.get("format").is_none());
        assert!(f.get("pattern").is_none());
        assert!(f.get("minLength").is_none());
    }

    #[test]
    fn validator_schema_preserves_schemars_and_serde_metadata() {
        use schemars::SchemaGenerator;

        #[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
        #[schemars(description = "validator schema description")]
        struct PreservedMetadata {
            #[serde(rename = "user_name")]
            #[schemars(description = "renamed field description")]
            #[validate(length(min = 2, max = 30))]
            username: String,
        }

        let mut gen = SchemaGenerator::default();
        let schema = <PreservedMetadata as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();

        assert_eq!(json["description"], "validator schema description");
        let props = json["properties"].as_object().unwrap();
        assert!(props.get("username").is_none());
        let renamed = &props["user_name"];
        assert_eq!(renamed["description"], "renamed field description");
        assert_eq!(renamed["minLength"], 2);
        assert_eq!(renamed["maxLength"], 30);
    }

    #[test]
    fn validator_schema_preserves_rename_all_for_injected_constraints() {
        use schemars::SchemaGenerator;

        #[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
        #[serde(rename_all = "camelCase")]
        struct RenameAllSerde {
            #[validate(length(min = 2, max = 30))]
            user_name: String,
        }

        #[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
        #[schemars(rename_all = "kebab-case")]
        struct RenameAllSchemars {
            #[validate(length(min = 1, max = 3))]
            item_count: Vec<String>,
            #[validate(required)]
            display_name: Option<String>,
        }

        let mut gen = SchemaGenerator::default();
        let schema = <RenameAllSerde as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json["properties"].as_object().unwrap();
        assert!(props.get("user_name").is_none());
        let renamed = &props["userName"];
        assert_eq!(renamed["minLength"], 2);
        assert_eq!(renamed["maxLength"], 30);

        let mut gen = SchemaGenerator::default();
        let schema = <RenameAllSchemars as schemars::JsonSchema>::json_schema(&mut gen);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json["properties"].as_object().unwrap();
        assert!(props.get("item_count").is_none());
        let renamed = &props["item-count"];
        assert_eq!(renamed["minItems"], 1);
        assert_eq!(renamed["maxItems"], 3);

        let required = json["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "display-name"));
    }
}
