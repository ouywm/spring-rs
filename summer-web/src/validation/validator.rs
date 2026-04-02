//! Validator validation extractors returning RFC 9457 ProblemDetails on failure.
//!
//! Provides `ValidatorJson`, `ValidatorQuery`, `ValidatorPath`, `ValidatorForm`
//! extractors that:
//! 1. Deserialize the request data (JSON body, query string, path params, form)
//! 2. Validate via validator (`validate()`)
//! 3. Convert failures to [`ProblemDetails`] with field-level violations
//!
//! # axum-valid compatibility
//!
//! When the `axum-valid` feature is also enabled, `From<ValidationRejection<*>>`
//! impls are provided so users can still use `axum_valid::Valid<T>` directly.

use super::{
    form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
    query_rejection_to_problem,
};
use crate::problem_details::{ProblemDetails, ViolationLocation};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::Json;
use validator::Validate;

/// Validate data and convert errors to ProblemDetails.
fn validate_data<T: Validate>(data: &T, location: ViolationLocation) -> Result<(), ProblemDetails> {
    data.validate()
        .map_err(|errors| ProblemDetails::from_validation_errors(&errors, location))
}

/// Validator-validated JSON body extractor.
pub struct ValidatorJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatorJson<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection_to_problem)?;

        validate_data(&data, ViolationLocation::Body)?;
        Ok(ValidatorJson(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorJson<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for ValidatorJson<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatorJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Validator-validated query string extractor.
pub struct ValidatorQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatorQuery<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(data) =
            axum::extract::Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(query_rejection_to_problem)?;

        validate_data(&data, ViolationLocation::Query)?;
        Ok(ValidatorQuery(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorQuery<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for ValidatorQuery<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatorQuery<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Validator-validated path parameter extractor.
pub struct ValidatorPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatorPath<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(data) =
            axum::extract::Path::<T>::from_request_parts(parts, state)
                .await
                .map_err(path_rejection_to_problem)?;

        validate_data(&data, ViolationLocation::Path)?;
        Ok(ValidatorPath(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorPath<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for ValidatorPath<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatorPath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Validator-validated form body extractor.
pub struct ValidatorForm<T>(pub T);

impl<T, S> FromRequest<S> for ValidatorForm<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Form(data) = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(form_rejection_to_problem)?;

        validate_data(&data, ViolationLocation::Form)?;
        Ok(ValidatorForm(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorForm<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for ValidatorForm<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatorForm<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// When `axum-valid` feature is also enabled, provide `From<ValidationRejection<*>>`
/// impls so users can use `axum_valid::Valid<T>` directly and still get ProblemDetails.
#[cfg(feature = "axum-valid")]
mod axum_valid_compat {
    use super::*;
    use axum::extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection};
    use axum_valid::ValidationRejection;
    use validator::ValidationErrors;

    impl From<ValidationRejection<ValidationErrors, JsonRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, JsonRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Body)
                }
                ValidationRejection::Inner(inner) => json_rejection_to_problem(inner),
            }
        }
    }

    impl From<ValidationRejection<ValidationErrors, QueryRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, QueryRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Query)
                }
                ValidationRejection::Inner(inner) => query_rejection_to_problem(inner),
            }
        }
    }

    impl From<ValidationRejection<ValidationErrors, PathRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, PathRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Path)
                }
                ValidationRejection::Inner(inner) => path_rejection_to_problem(inner),
            }
        }
    }

    impl From<ValidationRejection<ValidationErrors, FormRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, FormRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Form)
                }
                ValidationRejection::Inner(inner) => form_rejection_to_problem(inner),
            }
        }
    }
}
