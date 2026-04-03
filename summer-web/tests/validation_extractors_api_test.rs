#[cfg(all(feature = "validator", feature = "openapi"))]
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

    #[derive(Clone, Debug)]
    struct PageRules {
        max_page_size: usize,
    }

    fn validate_page_size(value: usize, ctx: &PageRules) -> Result<(), validator::ValidationError> {
        if value > ctx.max_page_size {
            return Err(validator::ValidationError::new("page_size_too_large"));
        }
        Ok(())
    }

    #[derive(Debug, Deserialize, JsonSchema, Validate, summer_web::ValidatorContext)]
    #[validate(context = PageRules)]
    struct Paginator {
        #[validate(custom(function = "validate_page_size", use_context))]
        page_size: usize,
    }

    #[cfg(feature = "typed-header")]
    impl Header for Paginator {
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
            let page_size = value.parse::<usize>().map_err(|_| Error::invalid())?;
            Ok(Self { page_size })
        }

        fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
            let value = HeaderValue::from_str(&self.page_size.to_string()).expect("header");
            values.extend(std::iter::once(value));
        }
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

    #[cfg(feature = "typed-header")]
    use axum_extra::headers::{Error, Header, HeaderValue};
    #[cfg(feature = "typed-header")]
    use summer_web::axum::http::HeaderName;

    #[cfg(feature = "typed-header")]
    #[derive(Debug, Validate)]
    struct DemoHeader {
        #[validate(length(min = 3, max = 32))]
        value: String,
    }

    #[cfg(feature = "typed-header")]
    static DEMO_HEADER_NAME: HeaderName = HeaderName::from_static("x-demo");

    #[cfg(feature = "typed-header")]
    impl Header for DemoHeader {
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
            let value = HeaderValue::from_str(&self.value).expect("header");
            values.extend(std::iter::once(value));
        }
    }

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn validator_extractors_support_openapi_input() {
        assert_operation_input::<summer_web::validation::validator::Validator<summer_web::axum::Json<CreateUserRequest>>>(
        );
        assert_operation_input::<summer_web::validation::validator::Validator<summer_web::axum::extract::Query<ListUsersQuery>>>(
        );
        assert_operation_input::<summer_web::validation::validator::Validator<summer_web::axum::extract::Path<UserIdPath>>>();
        assert_operation_input::<summer_web::validation::validator::Validator<summer_web::axum::extract::Form<SignupForm>>>();
        assert_operation_input::<summer_web::validation::validator::ValidatorEx<summer_web::axum::Json<Paginator>>>();
        assert_operation_input::<summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Query<Paginator>>>();
        assert_operation_input::<summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Path<Paginator>>>();
        assert_operation_input::<summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Form<Paginator>>>();
        #[cfg(feature = "typed-header")]
        assert_operation_input::<summer_web::validation::validator::Validator<summer_web::TypedHeader<DemoHeader>>>();
        #[cfg(feature = "typed-header")]
        assert_operation_input::<summer_web::validation::validator::ValidatorEx<summer_web::TypedHeader<Paginator>>>();
    }
}

#[cfg(all(feature = "garde", feature = "openapi"))]
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

    #[cfg(feature = "typed-header")]
    use axum_extra::headers::{Error, Header, HeaderValue};
    #[cfg(feature = "typed-header")]
    use summer_web::axum::http::HeaderName;

    #[cfg(feature = "typed-header")]
    #[derive(Debug, Validate)]
    struct DemoHeader {
        #[garde(length(min = 3, max = 32))]
        value: String,
    }

    #[cfg(feature = "typed-header")]
    static DEMO_HEADER_NAME: HeaderName = HeaderName::from_static("x-demo");

    #[cfg(feature = "typed-header")]
    impl Header for DemoHeader {
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
            let value = HeaderValue::from_str(&self.value).expect("header");
            values.extend(std::iter::once(value));
        }
    }

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn garde_extractors_support_openapi_input() {
        assert_operation_input::<summer_web::validation::garde::Garde<summer_web::axum::Json<CreateUserRequest>>>();
        assert_operation_input::<summer_web::validation::garde::Garde<summer_web::axum::extract::Query<ListUsersQuery>>>();
        assert_operation_input::<summer_web::validation::garde::Garde<summer_web::axum::extract::Path<UserIdPath>>>();
        assert_operation_input::<summer_web::validation::garde::Garde<summer_web::axum::extract::Form<SignupForm>>>();
        #[cfg(feature = "typed-header")]
        assert_operation_input::<summer_web::validation::garde::Garde<summer_web::TypedHeader<DemoHeader>>>();
    }
}
