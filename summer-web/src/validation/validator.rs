//! Validator wrappers returning RFC 9457 ProblemDetails on failure.
//!
//! Provides two generic wrappers:
//! 1. `Validator<E>` for `validator::Validate`
//! 2. `ValidatorEx<E>` for `validator::ValidateArgs`
//!
//! The wrapped extractor `E` can be `Json<T>`, `Query<T>`, `Path<T>`, `Form<T>`,
//! `TypedHeader<T>`, or any custom extractor implementing the shared validation
//! traits from `validation::mod`.

use super::context::ValidationContextRegistry;
use super::{HasValidate, ValidationSource};
use crate::problem_details::{ProblemDetails, ViolationLocation};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use validator::{Validate, ValidateArgs};

/// Runtime metadata for validator types that use `ValidateArgs`.
pub trait ValidatorContextType {
    type Context: Send + Sync + 'static;
}

/// Trait for wrappers to access the inner value validated via `ValidateArgs`.
pub trait HasValidateArgs {
    type ValidateArgs;
    fn get_validate_args(&self) -> &Self::ValidateArgs;
}

impl<T> HasValidateArgs for axum::Json<T> {
    type ValidateArgs = T;
    fn get_validate_args(&self) -> &Self::ValidateArgs {
        &self.0
    }
}

impl<T> HasValidateArgs for axum::extract::Query<T> {
    type ValidateArgs = T;
    fn get_validate_args(&self) -> &Self::ValidateArgs {
        &self.0
    }
}

impl<T> HasValidateArgs for axum::extract::Path<T> {
    type ValidateArgs = T;
    fn get_validate_args(&self) -> &Self::ValidateArgs {
        &self.0
    }
}

impl<T> HasValidateArgs for axum::extract::Form<T> {
    type ValidateArgs = T;
    fn get_validate_args(&self) -> &Self::ValidateArgs {
        &self.0
    }
}

#[cfg(feature = "typed-header")]
impl<T> HasValidateArgs for axum_extra::TypedHeader<T> {
    type ValidateArgs = T;
    fn get_validate_args(&self) -> &Self::ValidateArgs {
        &self.0
    }
}

enum ValidatorInternalError {
    MissingRegistry,
    MissingArgs(&'static str),
    ValidationErrors(validator::ValidationErrors, ViolationLocation),
}

impl From<ValidatorInternalError> for ProblemDetails {
    fn from(error: ValidatorInternalError) -> Self {
        match error {
            ValidatorInternalError::MissingRegistry => ProblemDetails::new(500)
                .with_detail("Server validation configuration is unavailable."),
            ValidatorInternalError::MissingArgs(args_type) => ProblemDetails::new(500)
                .with_detail(format!(
                    "Server validation configuration for '{}' is unavailable.",
                    args_type
                )),
            ValidatorInternalError::ValidationErrors(errors, location) => {
                ProblemDetails::from_validation_errors(&errors, location)
            }
        }
    }
}

fn validate_data<T: Validate>(
    data: &T,
    location: ViolationLocation,
) -> Result<(), ValidatorInternalError> {
    data.validate()
        .map_err(|errors| ValidatorInternalError::ValidationErrors(errors, location))
}

fn fetch_registry(parts: &Parts) -> Result<ValidationContextRegistry, ValidatorInternalError> {
    use crate::extractor::RequestPartsExt;
    parts.get_component::<ValidationContextRegistry>()
        .map_err(|_| ValidatorInternalError::MissingRegistry)
}

fn validate_data_with_args<T>(
    data: &T,
    registry: &ValidationContextRegistry,
    location: ViolationLocation,
) -> Result<(), ValidatorInternalError>
where
    T: ValidatorContextType,
    for<'v> T: ValidateArgs<'v, Args = &'v T::Context>,
{
    let args = registry
        .get::<T::Context>()
        .ok_or(ValidatorInternalError::MissingArgs(std::any::type_name::<T::Context>()))?;

    data.validate_with_args(args)
        .map_err(|errors| ValidatorInternalError::ValidationErrors(errors, location))
}

/// Generic validator wrapper for extractors whose inner type implements `Validate`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Validator<E>(pub E);

impl<E> std::ops::Deref for Validator<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> std::ops::DerefMut for Validator<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<State, Extractor> FromRequest<State> for Validator<Extractor>
where
    State: Send + Sync,
    Extractor: HasValidate
        + ValidationSource
        + FromRequest<State, Rejection = <Extractor as ValidationSource>::Rejection>,
    Extractor::Validate: Validate,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let inner = Extractor::from_request(req, state)
            .await
            .map_err(Extractor::rejection_to_problem)?;
        validate_data(inner.get_validate(), Extractor::violation_location())?;
        Ok(Self(inner))
    }
}

impl<State, Extractor> FromRequestParts<State> for Validator<Extractor>
where
    State: Send + Sync,
    Extractor: HasValidate
        + ValidationSource
        + FromRequestParts<State, Rejection = <Extractor as ValidationSource>::Rejection>,
    Extractor::Validate: Validate,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let inner = Extractor::from_request_parts(parts, state)
            .await
            .map_err(Extractor::rejection_to_problem)?;
        validate_data(inner.get_validate(), Extractor::violation_location())?;
        Ok(Self(inner))
    }
}

/// Generic validator wrapper for extractors whose inner type implements `ValidateArgs`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatorEx<E>(pub E);

impl<E> std::ops::Deref for ValidatorEx<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> std::ops::DerefMut for ValidatorEx<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<State, Extractor> FromRequest<State> for ValidatorEx<Extractor>
where
    State: Send + Sync,
    Extractor: ValidationSource
        + FromRequest<State, Rejection = <Extractor as ValidationSource>::Rejection>
        + HasValidateArgs,
    <Extractor as HasValidateArgs>::ValidateArgs: ValidatorContextType,
    for<'v> <Extractor as HasValidateArgs>::ValidateArgs:
        ValidateArgs<'v, Args = &'v <<Extractor as HasValidateArgs>::ValidateArgs as ValidatorContextType>::Context>,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let registry = fetch_registry(&parts)?;
        let req = Request::from_parts(parts, body);

        let inner = Extractor::from_request(req, state)
            .await
            .map_err(Extractor::rejection_to_problem)?;
        validate_data_with_args(
            inner.get_validate_args(),
            &registry,
            Extractor::violation_location(),
        )?;
        Ok(Self(inner))
    }
}

impl<State, Extractor> FromRequestParts<State> for ValidatorEx<Extractor>
where
    State: Send + Sync,
    Extractor: ValidationSource
        + FromRequestParts<State, Rejection = <Extractor as ValidationSource>::Rejection>
        + HasValidateArgs,
    <Extractor as HasValidateArgs>::ValidateArgs: ValidatorContextType,
    for<'v> <Extractor as HasValidateArgs>::ValidateArgs:
        ValidateArgs<'v, Args = &'v <<Extractor as HasValidateArgs>::ValidateArgs as ValidatorContextType>::Context>,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let registry = fetch_registry(parts)?;
        let inner = Extractor::from_request_parts(parts, state)
            .await
            .map_err(Extractor::rejection_to_problem)?;
        validate_data_with_args(
            inner.get_validate_args(),
            &registry,
            Extractor::violation_location(),
        )?;
        Ok(Self(inner))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::Json<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::extract::Query<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::extract::Path<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::extract::Form<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(all(feature = "typed-header", feature = "openapi"))]
impl<T> aide::OperationInput for Validator<axum_extra::TypedHeader<T>>
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

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::Json<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::extract::Query<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::extract::Path<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::extract::Form<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(all(feature = "typed-header", feature = "openapi"))]
impl<T> aide::OperationInput for ValidatorEx<axum_extra::TypedHeader<T>>
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
    use super::ValidatorInternalError;
    use crate::problem_details::ViolationLocation;

    #[test]
    fn internal_validation_errors_convert_to_problem_details() {
        let mut errors = validator::ValidationErrors::new();
        errors.add("name", validator::ValidationError::new("required"));

        let problem = crate::problem_details::ProblemDetails::from(
            ValidatorInternalError::ValidationErrors(errors, ViolationLocation::Body),
        );

        assert_eq!(problem.status, 400);
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "name");
        assert_eq!(problem.violations[0].location, ViolationLocation::Body);
    }
}

/// When `axum-valid` feature is also enabled, provide `From<ValidationRejection<*>>`
/// impls so users can use `axum_valid::Valid<T>` directly and still get ProblemDetails.
#[cfg(feature = "axum-valid")]
mod axum_valid_compat {
    use super::*;
    use axum::extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection};
    use axum_valid::ValidationRejection;
    use crate::validation::{
        form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
        query_rejection_to_problem,
    };
    use validator::ValidationErrors;
    #[cfg(feature = "typed-header")]
    use axum_extra::typed_header::TypedHeaderRejection;
    #[cfg(feature = "typed-header")]
    use crate::validation::typed_header_rejection_to_problem;

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

    #[cfg(feature = "typed-header")]
    impl From<ValidationRejection<ValidationErrors, TypedHeaderRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, TypedHeaderRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Header)
                }
                ValidationRejection::Inner(inner) => typed_header_rejection_to_problem(inner),
            }
        }
    }
}
