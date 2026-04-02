#[cfg(all(feature = "validator", feature = "axum-valid", feature = "openapi"))]
mod validator_api {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    struct CreateUserRequest {
        #[validate(length(min = 1, max = 100))]
        name: String,
        #[validate(email)]
        email: String,
    }

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    struct ListUsersQuery {
        #[validate(range(min = 1))]
        page: Option<i32>,
    }

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    #[allow(dead_code)]
    struct UserIdPath {
        id: u32,
    }

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    struct SignupForm {
        #[validate(length(min = 1, max = 32))]
        username: String,
        #[validate(email)]
        email: String,
    }

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn validator_extractors_support_openapi_input() {
        assert_operation_input::<summer_web::validation::validator::ValidatorJson<CreateUserRequest>>(
        );
        assert_operation_input::<summer_web::validation::validator::ValidatorQuery<ListUsersQuery>>(
        );
        assert_operation_input::<summer_web::validation::validator::ValidatorPath<UserIdPath>>();
        assert_operation_input::<summer_web::validation::validator::ValidatorForm<SignupForm>>();
    }
}

#[cfg(all(feature = "garde", feature = "axum-valid", feature = "openapi"))]
mod garde_api {
    use garde::Validate;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    struct CreateUserRequest {
        #[garde(length(min = 1, max = 100))]
        name: String,
        #[garde(length(min = 3, max = 255))]
        username: String,
    }

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    struct ListUsersQuery {
        #[garde(range(min = 1))]
        page: Option<i32>,
    }

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    #[allow(dead_code)]
    struct UserIdPath {
        #[garde(skip)]
        id: u32,
    }

    #[derive(Debug, Deserialize, JsonSchema, Validate)]
    struct SignupForm {
        #[garde(length(min = 1, max = 32))]
        username: String,
        #[garde(length(min = 3, max = 255))]
        contact: String,
    }

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn garde_extractors_support_openapi_input() {
        assert_operation_input::<summer_web::validation::garde::GardeJson<CreateUserRequest>>();
        assert_operation_input::<summer_web::validation::garde::GardeQuery<ListUsersQuery>>();
        assert_operation_input::<summer_web::validation::garde::GardePath<UserIdPath>>();
        assert_operation_input::<summer_web::validation::garde::GardeForm<SignupForm>>();
    }
}
