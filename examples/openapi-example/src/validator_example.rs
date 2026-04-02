// Validator-based validation examples (standalone extractors)

use serde::Deserialize;
use summer_web::axum::Json;
use summer_web::validation::validator::{
    ValidatorForm, ValidatorJson, ValidatorPath, ValidatorQuery,
};
use summer_web::{get_api, post_api};
use validator::Validate;

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

/// Create a new user (body validation via framework extractor)
///
/// @tag Validation
#[post_api("/validation/users")]
async fn create_user(
    ValidatorJson(body): ValidatorJson<CreateUserRequest>,
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
    ValidatorQuery(query): ValidatorQuery<ListUsersQuery>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "page": query.page.unwrap_or(1),
        "size": query.size.unwrap_or(20),
        "total": 0,
        "items": [],
    }))
}

/// Get user by ID (path parameter validation via framework extractor)
///
/// @tag Validation
#[get_api("/validation/users/{id}")]
async fn get_user_by_id(ValidatorPath(path): ValidatorPath<UserIdPath>) -> Json<serde_json::Value> {
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
    ValidatorForm(body): ValidatorForm<CreateUserForm>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 2001,
        "username": body.username,
        "email": body.email,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn validator_extractors_support_openapi_input() {
        assert_operation_input::<ValidatorJson<CreateUserRequest>>();
        assert_operation_input::<ValidatorQuery<ListUsersQuery>>();
        assert_operation_input::<ValidatorPath<UserIdPath>>();
        assert_operation_input::<ValidatorForm<CreateUserForm>>();
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

        // plain — no constraints
        let f = &props["plain_field"];
        assert!(f.get("format").is_none());
        assert!(f.get("pattern").is_none());
        assert!(f.get("minLength").is_none());
    }
}
