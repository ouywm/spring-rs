#[cfg(feature = "validator")]
mod validator_args {
    use summer::plugin::MutableComponentRegistry;
    use summer_web::axum::body::Body;
    use summer_web::axum::http::{Request, StatusCode};
    #[cfg(feature = "typed-header")]
    use summer_web::axum::http::HeaderName;
    use summer_web::axum::Extension;
    use summer_web::axum::Json;
    #[cfg(feature = "typed-header")]
    use summer_web::headers::{Error, Header, HeaderValue};
    use summer_web::problem_details::{ProblemDetails, ViolationLocation};
    use summer_web::validation::context::ValidationContextRegistry;
    use summer_web::{AppState, Router};
    use tower::ServiceExt;
    use validator::{Validate, ValidationError};

    use serde::Deserialize;

    #[derive(Clone, Debug)]
    struct PageRules {
        max_page_size: usize,
    }

    #[test]
    fn validation_context_registry_is_available_for_validator() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(PageRules { max_page_size: 100 });

        let rules = registry.get::<PageRules>().expect("page rules");
        assert_eq!(rules.max_page_size, 100);
    }

    fn validate_page_size(value: usize, ctx: &PageRules) -> Result<(), ValidationError> {
        if value > ctx.max_page_size {
            return Err(ValidationError::new("page_size_too_large"));
        }
        Ok(())
    }

    #[derive(Debug, Deserialize, Validate, summer_web::ValidatorContext)]
    #[validate(context = PageRules)]
    struct Paginator {
        #[validate(custom(function = "validate_page_size", use_context))]
        page_size: usize,
    }

    #[cfg(feature = "typed-header")]
    static DEMO_HEADER_NAME: HeaderName = HeaderName::from_static("x-page-size");

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

    async fn paginator_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::Json<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn paginator_query_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Query<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn paginator_path_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Path<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn paginator_form_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Form<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    #[cfg(feature = "typed-header")]
    async fn paginator_header_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::TypedHeader<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn router_with_registry(registry: Option<ValidationContextRegistry>) -> Router {
        let mut app = summer::app::AppBuilder::default();
        if let Some(registry) = registry {
            app.add_component(registry);
        }
        let app = app.build().await.expect("app build");

        let router = Router::new()
            .route("/paginator", axum::routing::post(paginator_handler))
            .route("/paginator-query", axum::routing::get(paginator_query_handler))
            .route("/paginator-path/{page_size}", axum::routing::get(paginator_path_handler))
            .route("/paginator-form", axum::routing::post(paginator_form_handler));

        #[cfg(feature = "typed-header")]
        let router = router.route("/paginator-header", axum::routing::get(paginator_header_handler));

        router.layer(Extension(AppState { app }))
    }

    fn json_request(body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/paginator")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request")
    }

    async fn read_problem(response: axum::response::Response) -> ProblemDetails {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("problem details json")
    }

    fn query_request(page_size: usize) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/paginator-query?page_size={page_size}"))
            .body(Body::empty())
            .expect("request")
    }

    fn path_request(page_size: usize) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/paginator-path/{page_size}"))
            .body(Body::empty())
            .expect("request")
    }

    fn form_request(page_size: usize) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/paginator-form")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("page_size={page_size}")))
            .expect("request")
    }

    #[cfg(feature = "typed-header")]
    fn header_request(page_size: usize) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri("/paginator-header")
            .header(DEMO_HEADER_NAME.as_str(), page_size.to_string())
            .body(Body::empty())
            .expect("request")
    }

    #[tokio::test]
    async fn validator_json_with_args_uses_registered_context() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(PageRules { max_page_size: 100 });

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(json_request(serde_json::json!({ "page_size": 20 })))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn validator_json_with_args_validation_error_maps_to_body_violation() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(PageRules { max_page_size: 10 });

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(json_request(serde_json::json!({ "page_size": 20 })))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "page_size");
        assert_eq!(problem.violations[0].location, ViolationLocation::Body);
    }

    #[tokio::test]
    async fn validator_json_with_args_missing_registry_returns_server_error() {
        let response = router_with_registry(None)
            .await
            .oneshot(json_request(serde_json::json!({ "page_size": 20 })))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn validator_json_with_args_missing_concrete_args_returns_server_error() {
        let registry = ValidationContextRegistry::new();

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(json_request(serde_json::json!({ "page_size": 20 })))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn validator_query_with_args_validation_error_maps_to_query_violation() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(PageRules { max_page_size: 10 });

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(query_request(20))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations[0].location, ViolationLocation::Query);
    }

    #[tokio::test]
    async fn validator_path_with_args_validation_error_maps_to_path_violation() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(PageRules { max_page_size: 10 });

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(path_request(20))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations[0].location, ViolationLocation::Path);
    }

    #[tokio::test]
    async fn validator_form_with_args_validation_error_maps_to_form_violation() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(PageRules { max_page_size: 10 });

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(form_request(20))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations[0].location, ViolationLocation::Form);
    }

    #[cfg(feature = "typed-header")]
    #[tokio::test]
    async fn validator_header_with_args_validation_error_maps_to_header_violation() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(PageRules { max_page_size: 10 });

        let response = router_with_registry(Some(registry))
            .await
            .oneshot(header_request(20))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    }
}
