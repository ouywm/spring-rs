//! Validation extractors that return RFC 9457 ProblemDetails on failure.
//!
//! This module wraps [axum-valid](https://docs.rs/axum-valid)'s `Valid<E>` extractor
//! so that validation errors are automatically returned as structured
//! [`ProblemDetails`](crate::problem_details::ProblemDetails) responses with
//! field-level [`Violation`](crate::problem_details::Violation) information.
//!
//! # Usage
//!
//! ```rust,ignore
//! use axum::Json;
//! use axum_valid::Valid;
//! use serde::Deserialize;
//! use validator::Validate;
//! use summer_web::extractor::{Query, Path};
//!
//! #[derive(Deserialize, Validate)]
//! struct CreateUser {
//!     #[validate(length(min = 1, max = 100))]
//!     name: String,
//!     #[validate(email)]
//!     email: String,
//! }
//!
//! // Just use Valid<Json<T>> — validation failures automatically become ProblemDetails
//! async fn create_user(Valid(Json(body)): Valid<Json<CreateUser>>) -> Json<String> {
//!     Json(format!("Created user: {}", body.name))
//! }
//! ```
//!
//! # How it works
//!
//! axum-valid's `Valid<E>` performs two-phase extraction:
//! 1. Inner extractor (e.g. `Json<T>`) deserializes the request
//! 2. `validator::Validate::validate()` checks business rules
//!
//! When either phase fails, axum-valid returns a `ValidationRejection`.
//! This module provides `From<ValidationRejection> for ProblemDetails` so that
//! the rejection is automatically converted to a structured error response.

use crate::problem_details::{ProblemDetails, Violation, ViolationLocation};
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum_valid::ValidationRejection;
use validator::ValidationErrors;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ValidationRejection → ProblemDetails conversions
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Convert `Valid<Json<T>>` rejection into ProblemDetails.
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

/// Convert `Valid<Query<T>>` rejection into ProblemDetails.
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

/// Convert `Valid<Path<T>>` rejection into ProblemDetails.
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Axum rejection → ProblemDetails (phase 1: deserialization errors)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// NOTE: These functions parse axum 0.8's error message format.
// If axum changes its error messages in a future version, these
// parsers may need updating.

fn json_rejection_to_problem(rejection: JsonRejection) -> ProblemDetails {
    let msg = rejection.body_text();
    match rejection {
        JsonRejection::JsonDataError(_) => {
            let stripped = msg
                .strip_prefix("Failed to deserialize the JSON body into the target type: ")
                .unwrap_or(&msg);

            let (field, detail) = split_field_message(stripped);

            if !field.is_empty() && field != "." {
                ProblemDetails::validation_error(vec![Violation::body(
                    field,
                    humanize_serde_message(detail),
                )])
            } else if let Some(name) = extract_backtick_value(stripped, "missing field `") {
                ProblemDetails::validation_error(vec![Violation::body(
                    name,
                    "this field is required",
                )])
            } else {
                ProblemDetails::validation_error_simple(humanize_serde_message(stripped))
            }
        }
        JsonRejection::JsonSyntaxError(_) => {
            ProblemDetails::validation_error_simple("request body is not valid JSON")
        }
        JsonRejection::MissingJsonContentType(_) => ProblemDetails::new(415)
            .with_detail("expected Content-Type: application/json"),
        _ => ProblemDetails::validation_error_simple("failed to read request body"),
    }
}

fn query_rejection_to_problem(rejection: QueryRejection) -> ProblemDetails {
    let msg = rejection.body_text();
    let stripped = msg
        .strip_prefix("Failed to deserialize query string: ")
        .unwrap_or(&msg);

    if let Some(name) = extract_backtick_value(stripped, "missing field `") {
        ProblemDetails::validation_error(vec![Violation::query(
            name,
            "this query parameter is required",
        )])
    } else {
        let (field, detail) = split_field_message(stripped);
        if !field.is_empty() {
            ProblemDetails::validation_error(vec![Violation::query(
                field,
                humanize_serde_message(detail),
            )])
        } else {
            ProblemDetails::validation_error(vec![Violation::query(
                "query",
                humanize_serde_message(stripped),
            )])
        }
    }
}

fn path_rejection_to_problem(rejection: PathRejection) -> ProblemDetails {
    let msg = rejection.body_text();

    // axum 0.8 format: "Invalid URL: Cannot parse `id` with value `abc` to a `u32`"
    if let Some(name) = extract_backtick_value(&msg, "Cannot parse `") {
        let expected = extract_backtick_value(&msg, "to a `").unwrap_or("valid value");
        ProblemDetails::validation_error(vec![Violation::path(
            name,
            format!("must be a valid {expected}"),
        )])
    } else {
        ProblemDetails::validation_error(vec![Violation::path(
            "path",
            "invalid path parameter",
        )])
    }
}

// ── Parsing helpers ────────────────────────────────────────────────

fn split_field_message(s: &str) -> (&str, &str) {
    match s.find(": ") {
        Some(pos) => (&s[..pos], &s[pos + 2..]),
        None => ("", s),
    }
}

/// Extract the value between backticks after a given prefix.
/// e.g. `extract_backtick_value(msg, "missing field \`")` → Some("name")
fn extract_backtick_value<'a>(msg: &'a str, prefix: &str) -> Option<&'a str> {
    let start = msg.find(prefix)?;
    let rest = &msg[start + prefix.len()..];
    let end = rest.find('`')?;
    Some(&rest[..end])
}

/// Turn serde's raw error into a human-friendly sentence.
fn humanize_serde_message(raw: &str) -> String {
    // Strip " at line X column Y" suffix
    let clean = if let Some(pos) = raw.find(" at line ") {
        &raw[..pos]
    } else {
        raw
    };

    if clean.starts_with("missing field") {
        if let Some(name) = extract_backtick_value(clean, "missing field `") {
            return format!("field '{name}' is required");
        }
    }

    if clean.starts_with("invalid type:") {
        let detail = clean.strip_prefix("invalid type: ").unwrap_or(clean);
        return format!("invalid type: {detail}");
    }

    clean.to_string()
}
