#[cfg(all(feature = "validator", feature = "axum-valid"))]
#[test]
fn validator_form_rejection_maps_to_form_violation() {
    use axum::extract::rejection::FormRejection;
    use axum_valid::ValidationRejection;
    use summer_web::problem_details::{ProblemDetails, ViolationLocation};
    use validator::{ValidationError, ValidationErrors};

    let mut errors = ValidationErrors::new();
    errors.add(
        "email",
        ValidationError::new("email").with_message("must be a valid email address".into()),
    );

    let problem: ProblemDetails =
        ValidationRejection::<ValidationErrors, FormRejection>::Valid(errors).into();

    assert_eq!(problem.status, 400);
    assert_eq!(problem.violations.len(), 1);
    assert_eq!(problem.violations[0].field, "email");
    assert_eq!(problem.violations[0].location, ViolationLocation::Form);
    assert_eq!(
        problem.violations[0].message,
        "must be a valid email address"
    );
}

#[cfg(all(feature = "garde", feature = "axum-valid"))]
#[test]
fn garde_form_rejection_maps_to_form_violation() {
    use axum::extract::rejection::FormRejection;
    use axum_valid::GardeRejection;
    use garde::{Error, Path, Report};
    use summer_web::problem_details::{ProblemDetails, ViolationLocation};

    let mut report = Report::new();
    report.append(
        Path::new("items").join(0).join("name"),
        Error::new("must not be blank"),
    );

    let problem: ProblemDetails = GardeRejection::<FormRejection>::Valid(report).into();

    assert_eq!(problem.status, 400);
    assert_eq!(problem.violations.len(), 1);
    assert_eq!(problem.violations[0].field, "items[0].name");
    assert_eq!(problem.violations[0].location, ViolationLocation::Form);
    assert_eq!(problem.violations[0].message, "must not be blank");
}

#[cfg(all(feature = "validator", feature = "axum-valid", feature = "typed-header"))]
#[test]
fn validator_typed_header_rejection_maps_to_header_violation() {
    use axum_extra::typed_header::TypedHeaderRejection;
    use axum_valid::ValidationRejection;
    use summer_web::problem_details::{ProblemDetails, ViolationLocation};
    use validator::{ValidationError, ValidationErrors};

    let mut errors = ValidationErrors::new();
    errors.add(
        "value",
        ValidationError::new("length").with_message("invalid demo header".into()),
    );

    let problem: ProblemDetails =
        ValidationRejection::<ValidationErrors, TypedHeaderRejection>::Valid(errors).into();

    assert_eq!(problem.status, 400);
    assert_eq!(problem.violations.len(), 1);
    assert_eq!(problem.violations[0].field, "value");
    assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    assert_eq!(problem.violations[0].message, "invalid demo header");
}

#[cfg(all(feature = "garde", feature = "axum-valid", feature = "typed-header"))]
#[test]
fn garde_typed_header_rejection_maps_to_header_violation() {
    use axum_extra::typed_header::TypedHeaderRejection;
    use axum_valid::GardeRejection;
    use garde::{Error, Path, Report};
    use summer_web::problem_details::{ProblemDetails, ViolationLocation};

    let mut report = Report::new();
    report.append(Path::new("value"), Error::new("invalid demo header"));

    let problem: ProblemDetails = GardeRejection::<TypedHeaderRejection>::Valid(report).into();

    assert_eq!(problem.status, 400);
    assert_eq!(problem.violations.len(), 1);
    assert_eq!(problem.violations[0].field, "value");
    assert_eq!(problem.violations[0].location, ViolationLocation::Header);
    assert_eq!(problem.violations[0].message, "invalid demo header");
}
