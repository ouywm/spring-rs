#[cfg(any(feature = "validator", feature = "garde"))]
mod header_validation {
    use axum::routing::get;
    use summer::plugin::MutableComponentRegistry;
    use summer_web::axum::body::Body;
    use summer_web::axum::http::{Request, StatusCode};
    use summer_web::axum::http::HeaderName;
    use summer_web::axum::Extension;
    use summer_web::headers::{Error, Header, HeaderValue};
    use summer_web::problem_details::{ProblemDetails, ViolationLocation};
    #[cfg(feature = "garde")]
    use summer_web::validation::context::ValidationContextRegistry;
    use summer_web::{AppState, Router};
    use tower::ServiceExt;

    static DEMO_HEADER_NAME: HeaderName = HeaderName::from_static("x-demo");

    #[cfg(feature = "validator")]
    #[derive(Clone, Debug, validator::Validate)]
    struct DemoValidatorHeader {
        #[validate(length(min = 3, max = 12))]
        value: String,
    }

    #[cfg(feature = "validator")]
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
            if !value.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                return Err(Error::invalid());
            }
            Ok(Self {
                value: value.to_string(),
            })
        }

        fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
            let value = HeaderValue::from_str(&self.value).expect("valid demo header");
            values.extend(std::iter::once(value));
        }
    }

    #[cfg(feature = "garde")]
    #[derive(Clone, Debug)]
    struct DemoHeaderRules {
        min: usize,
        max: usize,
    }

    #[cfg(feature = "garde")]
    #[derive(Clone, Debug, garde::Validate)]
    struct DemoGardeHeader {
        #[garde(length(min = 3, max = 12))]
        value: String,
    }

    #[cfg(feature = "garde")]
    impl Header for DemoGardeHeader {
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
            if !value.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                return Err(Error::invalid());
            }
            Ok(Self {
                value: value.to_string(),
            })
        }

        fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
            let value = HeaderValue::from_str(&self.value).expect("valid garde header");
            values.extend(std::iter::once(value));
        }
    }

    #[cfg(feature = "garde")]
    #[derive(Clone, Debug, garde::Validate)]
    #[garde(context(DemoHeaderRules as ctx))]
    struct DemoGardeHeaderWithContext {
        #[garde(length(min = ctx.min, max = ctx.max))]
        value: String,
    }

    #[cfg(feature = "garde")]
    impl Header for DemoGardeHeaderWithContext {
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
            if !value.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                return Err(Error::invalid());
            }
            Ok(Self {
                value: value.to_string(),
            })
        }

        fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
            let value = HeaderValue::from_str(&self.value).expect("valid garde header");
            values.extend(std::iter::once(value));
        }
    }

    #[cfg(feature = "validator")]
    async fn validator_header_handler(
        payload: summer_web::validation::validator::Validator<summer_web::TypedHeader<DemoValidatorHeader>>,
    ) -> &'static str {
        let _ = payload;
        "ok"
    }

    #[cfg(feature = "validator")]
    fn validator_router() -> Router {
        Router::new().route("/validator", get(validator_header_handler))
    }

    #[cfg(feature = "garde")]
    async fn garde_header_handler(
        payload: summer_web::validation::garde::Garde<summer_web::TypedHeader<DemoGardeHeader>>,
    ) -> &'static str {
        let _ = payload;
        "ok"
    }

    #[cfg(feature = "garde")]
    async fn garde_header_with_context_handler(
        payload: summer_web::validation::garde::Garde<summer_web::TypedHeader<DemoGardeHeaderWithContext>>,
    ) -> &'static str {
        let _ = payload;
        "ok"
    }

    #[cfg(feature = "garde")]
    async fn router_with_registry(registry: Option<ValidationContextRegistry>) -> Router {
        let mut app = summer::app::AppBuilder::default();
        if let Some(registry) = registry {
            app.add_component(registry);
        }
        let app = app.build().await.expect("app build");

        Router::new()
            .route("/garde", get(garde_header_handler))
            .route("/garde-context", get(garde_header_with_context_handler))
            .layer(Extension(AppState { app }))
    }

    #[cfg(feature = "validator")]
    fn request_with_header(value: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri("/validator");
        if let Some(value) = value {
            builder = builder.header(DEMO_HEADER_NAME.as_str(), value);
        }
        builder.body(Body::empty()).expect("request")
    }

    #[cfg(feature = "garde")]
    fn garde_request(uri: &'static str, value: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(uri);
        if let Some(value) = value {
            builder = builder.header(DEMO_HEADER_NAME.as_str(), value);
        }
        builder.body(Body::empty()).expect("request")
    }

    async fn read_problem(response: axum::response::Response) -> ProblemDetails {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("problem details json")
    }

    #[cfg(feature = "validator")]
    #[tokio::test]
    async fn validator_header_missing_maps_to_header_violation() {
        let response = validator_router()
            .oneshot(request_with_header(None))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "x-demo");
        assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    }

    #[cfg(feature = "validator")]
    #[tokio::test]
    async fn validator_header_validation_error_maps_to_header_violation() {
        let response = validator_router()
            .oneshot(request_with_header(Some("ab")))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "value");
        assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    }

    #[cfg(feature = "validator")]
    #[tokio::test]
    async fn validator_header_parse_error_maps_to_header_violation() {
        let response = validator_router()
            .oneshot(request_with_header(Some("bad!")))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "x-demo");
        assert_eq!(problem.violations[0].location, ViolationLocation::Header);
        assert_eq!(problem.violations[0].message, "invalid header value");
    }

    #[cfg(feature = "garde")]
    #[tokio::test]
    async fn garde_header_missing_maps_to_header_violation() {
        let response = router_with_registry(None)
            .await
            .oneshot(garde_request("/garde", None))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "x-demo");
        assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    }

    #[cfg(feature = "garde")]
    #[tokio::test]
    async fn garde_header_validation_error_maps_to_header_violation() {
        let response = router_with_registry(None)
            .await
            .oneshot(garde_request("/garde", Some("ab")))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "value");
        assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    }

    #[cfg(feature = "garde")]
    #[tokio::test]
    async fn garde_header_missing_registry_returns_server_error() {
        let response = router_with_registry(None)
            .await
            .oneshot(garde_request("/garde-context", Some("abcd")))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[cfg(feature = "garde")]
    #[tokio::test]
    async fn garde_header_context_validation_error_maps_to_header_violation() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(DemoHeaderRules { min: 5, max: 8 });

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(garde_request("/garde-context", Some("abcd")))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "value");
        assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    }

}
