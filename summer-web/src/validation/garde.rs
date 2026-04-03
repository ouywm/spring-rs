//! Garde wrappers returning RFC 9457 ProblemDetails on failure.
//!
//! Provides a generic `Garde<E>` wrapper that can validate any extractor `E`
//! whose inner type implements `garde::Validate`.

use super::context::ValidationContextRegistry;
use super::{HasValidate, ValidationSource};
use crate::problem_details::{ProblemDetails, ViolationLocation};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::Json;
use garde::Validate;
use std::any::{Any, TypeId};

/// Generic garde wrapper for extractors whose inner type implements `garde::Validate`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Garde<E>(pub E);

impl<E> std::ops::Deref for Garde<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> std::ops::DerefMut for Garde<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

enum GardeInternalError {
    MissingRegistry,
    MissingContext(&'static str),
    ValidationReport(garde::Report, ViolationLocation),
}

impl From<GardeInternalError> for ProblemDetails {
    fn from(error: GardeInternalError) -> Self {
        match error {
            GardeInternalError::MissingRegistry => ProblemDetails::new(500)
                .with_detail("Server validation configuration is unavailable."),
            GardeInternalError::MissingContext(context_type) => ProblemDetails::new(500)
                .with_detail(format!(
                    "Server validation configuration for '{}' is unavailable.",
                    context_type
                )),
            GardeInternalError::ValidationReport(report, location) => {
                ProblemDetails::from_garde_report(&report, location)
            }
        }
    }
}

fn fetch_registry<T: Validate>(
    parts: &Parts,
) -> Result<Option<ValidationContextRegistry>, GardeInternalError>
where
    T::Context: 'static,
{
    if TypeId::of::<T::Context>() == TypeId::of::<()>() {
        Ok(None)
    } else {
        use crate::extractor::RequestPartsExt;
        let registry = parts
            .get_component::<ValidationContextRegistry>()
            .map_err(|_| GardeInternalError::MissingRegistry)?;
        Ok(Some(registry))
    }
}

fn validate_without_context<T: Validate>(data: &T) -> Result<(), garde::Report>
where
    T::Context: 'static,
{
    let unit = ();
    let ctx = (&unit as &dyn Any)
        .downcast_ref::<T::Context>()
        .expect("TypeId check guarantees T::Context is ()");
    data.validate_with(ctx)
}

fn validate_data<T: Validate>(
    data: &T,
    registry: Option<&ValidationContextRegistry>,
    location: ViolationLocation,
) -> Result<(), GardeInternalError>
where
    T::Context: Send + Sync + 'static,
{
    let result = if TypeId::of::<T::Context>() == TypeId::of::<()>() {
        validate_without_context(data)
    } else {
        let registry = registry.ok_or(GardeInternalError::MissingRegistry)?;
        let ctx = registry
            .get::<T::Context>()
            .ok_or(GardeInternalError::MissingContext(std::any::type_name::<T::Context>()))?;
        data.validate_with(ctx)
    };

    result.map_err(|report| GardeInternalError::ValidationReport(report, location))
}

impl<State, Extractor> FromRequest<State> for Garde<Extractor>
where
    State: Send + Sync,
    Extractor: HasValidate
        + ValidationSource
        + FromRequest<State, Rejection = <Extractor as ValidationSource>::Rejection>,
    Extractor::Validate: Validate,
    <Extractor::Validate as Validate>::Context: Send + Sync + 'static,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let registry = fetch_registry::<Extractor::Validate>(&parts)?;
        let req = Request::from_parts(parts, body);

        let inner = Extractor::from_request(req, state)
            .await
            .map_err(Extractor::rejection_to_problem)?;
        validate_data(
            inner.get_validate(),
            registry.as_ref(),
            Extractor::violation_location(),
        )?;
        Ok(Self(inner))
    }
}

impl<State, Extractor> FromRequestParts<State> for Garde<Extractor>
where
    State: Send + Sync,
    Extractor: HasValidate
        + ValidationSource
        + FromRequestParts<State, Rejection = <Extractor as ValidationSource>::Rejection>,
    Extractor::Validate: Validate,
    <Extractor::Validate as Validate>::Context: Send + Sync + 'static,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let registry = fetch_registry::<Extractor::Validate>(parts)?;
        let inner = Extractor::from_request_parts(parts, state)
            .await
            .map_err(Extractor::rejection_to_problem)?;
        validate_data(
            inner.get_validate(),
            registry.as_ref(),
            Extractor::violation_location(),
        )?;
        Ok(Self(inner))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<Json<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<axum::extract::Query<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<axum::extract::Path<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<axum::extract::Form<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(all(feature = "typed-header", feature = "openapi"))]
impl<T> aide::OperationInput for Garde<axum_extra::TypedHeader<T>>
where
    T: axum_extra::headers::Header,
{
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        add_typed_header_operation_input::<T>(ctx, operation);
    }
}

#[cfg(all(feature = "typed-header", feature = "openapi"))]
fn add_typed_header_operation_input<T>(
    ctx: &mut aide::generate::GenContext,
    operation: &mut aide::openapi::Operation,
) where
    T: axum_extra::headers::Header,
{
    use aide::openapi::{HeaderStyle, Parameter, ParameterData, ParameterSchemaOrContent};

    let schema = ctx.schema.subschema_for::<String>();
    let parameter = Parameter::Header {
        parameter_data: ParameterData {
            name: T::name().to_string(),
            description: None,
            required: true,
            format: ParameterSchemaOrContent::Schema(aide::openapi::SchemaObject {
                json_schema: schema,
                example: None,
                external_docs: None,
            }),
            extensions: Default::default(),
            deprecated: None,
            example: None,
            examples: Default::default(),
            explode: None,
        },
        style: HeaderStyle::Simple,
    };

    if operation.parameters.iter().all(|existing| match existing {
        aide::openapi::ReferenceOr::Reference { .. } => true,
        aide::openapi::ReferenceOr::Item(item) => match item {
            Parameter::Header { parameter_data, .. } => parameter_data.name != T::name().as_str(),
            _ => true,
        },
    }) {
        operation
            .parameters
            .push(aide::openapi::ReferenceOr::Item(parameter));
    }
}

#[cfg(test)]
mod tests {
    use super::GardeInternalError;
    use crate::problem_details::ViolationLocation;

    #[test]
    fn internal_validation_report_converts_to_problem_details() {
        let mut report = garde::Report::new();
        report.append(garde::Path::new("name"), garde::Error::new("too short"));

        let problem = crate::problem_details::ProblemDetails::from(
            GardeInternalError::ValidationReport(report, ViolationLocation::Body),
        );

        assert_eq!(problem.status, 400);
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "name");
        assert_eq!(problem.violations[0].message, "too short");
    }
}

/// When `axum-valid` feature is also enabled, provide `From<GardeRejection<*>>` impls
/// so users can use `axum_valid::Garde<T>` directly and still get ProblemDetails conversion.
#[cfg(feature = "axum-valid")]
mod axum_valid_compat {
    use super::*;
    use axum::extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection};
    use axum_valid::GardeRejection;
    use crate::validation::{
        form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
        query_rejection_to_problem,
    };
    #[cfg(feature = "typed-header")]
    use axum_extra::typed_header::TypedHeaderRejection;
    #[cfg(feature = "typed-header")]
    use crate::validation::typed_header_rejection_to_problem;

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

    #[cfg(feature = "typed-header")]
    impl From<GardeRejection<TypedHeaderRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<TypedHeaderRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Header)
                }
                GardeRejection::Inner(inner) => typed_header_rejection_to_problem(inner),
            }
        }
    }
}
