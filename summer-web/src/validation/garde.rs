//! Garde validation extractors returning RFC 9457 ProblemDetails on failure.
//!
//! Provides `GardeJson`, `GardeQuery`, `GardePath`, `GardeForm` extractors that:
//! 1. Deserialize the request data (JSON body, query string, path params, form)
//! 2. Validate via garde (`validate()` or `validate_with(&ctx)`)
//! 3. Convert failures to [`ProblemDetails`] with field-level violations
//!
//! # Two modes
//!
//! - **`T::Context = ()`** — zero-config, validates directly without a registry
//! - **`T::Context = SomeType`** — looks up context from [`ValidationContextRegistry`]
//!
//! # axum-valid compatibility
//!
//! When the `axum-valid` feature is also enabled, `From<GardeRejection<*Rejection>>`
//! impls are provided so users can still use `axum_valid::Garde<T>` directly if preferred.

use super::{
    form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
    query_rejection_to_problem,
};
use crate::problem_details::{ProblemDetails, ViolationLocation};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::Json;
use garde::Validate;
use std::any::TypeId;

use super::context::ValidationContextRegistry;

/// Fetch the validation context registry from request parts.
/// Returns `None` if `T::Context` is `()` (no registry needed).
fn fetch_registry<T: Validate>(
    parts: &Parts,
) -> Result<Option<ValidationContextRegistry>, ProblemDetails>
where
    T::Context: 'static,
{
    if TypeId::of::<T::Context>() == TypeId::of::<()>() {
        Ok(None)
    } else {
        use crate::extractor::RequestPartsExt;
        let registry = parts
            .get_component::<ValidationContextRegistry>()
            .map_err(|_| {
                ProblemDetails::new(500).with_detail(
                    "ValidationContextRegistry not found. \
                     Ensure WebPlugin built the validation registry.",
                )
            })?;
        Ok(Some(registry))
    }
}

/// Validate data using garde, dispatching based on whether `T::Context` is `()`.
fn validate_data<T: Validate>(
    data: &T,
    registry: Option<&ValidationContextRegistry>,
    location: ViolationLocation,
) -> Result<(), ProblemDetails>
where
    T::Context: Send + Sync + 'static,
{
    let result = if TypeId::of::<T::Context>() == TypeId::of::<()>() {
        // Context is (), no registry needed.
        // SAFETY: TypeId check guarantees T::Context is exactly ().
        // () is a ZST — any aligned non-null pointer is a valid reference.
        let unit = ();
        let ctx: &T::Context = unsafe { &*(&unit as *const () as *const T::Context) };
        data.validate_with(ctx)
    } else {
        let registry = registry.ok_or_else(|| {
            ProblemDetails::new(500).with_detail(
                "ValidationContextRegistry not available for non-() context",
            )
        })?;
        let ctx = registry.get::<T::Context>().ok_or_else(|| {
            ProblemDetails::new(500).with_detail(format!(
                "Validation context '{}' not registered for request type '{}'. \
                 Register it via ValidationContextRegistry in a #[component] function.",
                std::any::type_name::<T::Context>(),
                std::any::type_name::<T>()
            ))
        })?;
        data.validate_with(ctx)
    };

    result.map_err(|report| ProblemDetails::from_garde_report(&report, location))
}

/// Garde-validated JSON body extractor.
///
/// Deserializes the request body as JSON, then validates via garde.
/// Returns [`ProblemDetails`] on deserialization or validation failure.
pub struct GardeJson<T>(pub T);

impl<T, S> FromRequest<S> for GardeJson<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    T::Context: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Split to access extensions before body extraction
        let (parts, body) = req.into_parts();
        let registry = fetch_registry::<T>(&parts)?;
        let req = Request::from_parts(parts, body);

        let Json(data) = Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection_to_problem)?;

        validate_data(&data, registry.as_ref(), ViolationLocation::Body)?;
        Ok(GardeJson(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for GardeJson<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for GardeJson<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for GardeJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Garde-validated query string extractor.
pub struct GardeQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for GardeQuery<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    T::Context: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let registry = fetch_registry::<T>(parts)?;

        let axum::extract::Query(data) =
            axum::extract::Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(query_rejection_to_problem)?;

        validate_data(&data, registry.as_ref(), ViolationLocation::Query)?;
        Ok(GardeQuery(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for GardeQuery<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for GardeQuery<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for GardeQuery<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Garde-validated path parameter extractor.
pub struct GardePath<T>(pub T);

impl<T, S> FromRequestParts<S> for GardePath<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    T::Context: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let registry = fetch_registry::<T>(parts)?;

        let axum::extract::Path(data) =
            axum::extract::Path::<T>::from_request_parts(parts, state)
                .await
                .map_err(path_rejection_to_problem)?;

        validate_data(&data, registry.as_ref(), ViolationLocation::Path)?;
        Ok(GardePath(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for GardePath<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for GardePath<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for GardePath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


/// Garde-validated form body extractor.
pub struct GardeForm<T>(pub T);

impl<T, S> FromRequest<S> for GardeForm<T>
where
    T: serde::de::DeserializeOwned + Validate + Send + 'static,
    T::Context: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let registry = fetch_registry::<T>(&parts)?;
        let req = Request::from_parts(parts, body);

        let axum::extract::Form(data) = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(form_rejection_to_problem)?;

        validate_data(&data, registry.as_ref(), ViolationLocation::Form)?;
        Ok(GardeForm(data))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for GardeForm<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

impl<T> std::ops::Deref for GardeForm<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for GardeForm<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// When `axum-valid` feature is also enabled, provide `From<GardeRejection<*>>` impls
/// so users can use `axum_valid::Garde<T>` directly and still get ProblemDetails conversion.
#[cfg(feature = "axum-valid")]
mod axum_valid_compat {
    use super::*;
    use axum::extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection};
    use axum_valid::GardeRejection;

    impl From<GardeRejection<JsonRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<JsonRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Body)
                }
                GardeRejection::Inner(inner) => json_rejection_to_problem(inner),
            }
        }
    }

    impl From<GardeRejection<QueryRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<QueryRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Query)
                }
                GardeRejection::Inner(inner) => query_rejection_to_problem(inner),
            }
        }
    }

    impl From<GardeRejection<PathRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<PathRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Path)
                }
                GardeRejection::Inner(inner) => path_rejection_to_problem(inner),
            }
        }
    }

    impl From<GardeRejection<FormRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<FormRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Form)
                }
                GardeRejection::Inner(inner) => form_rejection_to_problem(inner),
            }
        }
    }
}
