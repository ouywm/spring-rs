//! Validation context storage for garde and validator runtime validation context.
//!
//! Provides a typed map ([`ValidationContextRegistry`]) for storing runtime validation
//! contexts. Users register it via `#[component]`, and validation extractors look up
//! contexts at request time.
//!
//! # Architecture
//!
//! 1. **User registers** — `#[component]` returns `ValidationContextRegistry`
//! 2. **Extractor consumes** — `Garde*` / `Validator*WithArgs` extractors look up context at request time
//!
//! # When do you need this?
//!
//! Use this registry when:
//!
//! - your garde structs use **custom context** (`#[garde(context(...))]`)
//! - your validator structs use **ValidateArgs / use_context**
//!
//! If all your rules are literal values (for example `#[garde(length(min = 1, max = 100))]`
//! or plain `validator::Validate` without arguments), you don't need a registry at all.

use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A hasher optimized for `TypeId` keys (already hashed u64 values).
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }
}

/// Trait object used by the registry to support cloning typed values stored in `Box`.
trait AnyClone: Any {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync>;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Clone + Send + Sync + 'static> AnyClone for T {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Clone for Box<dyn AnyClone + Send + Sync> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// Internal storage: a small typed map similar to `http::Extensions`.
///
/// Each entry is boxed and cloneable, which allows the whole registry to
/// implement `Clone` and therefore be stored as a normal Summer component.
type AnyMap = HashMap<TypeId, Box<dyn AnyClone + Send + Sync>, BuildHasherDefault<IdHasher>>;

/// A typed map that stores runtime validation context instances.
///
/// Built once at startup via `#[component]`, then read-only at request time.
///
/// Because Summer components are cloned when extracted, this registry also needs
/// to be cloneable. The inner typed values therefore must implement `Clone`.
///
/// # Usage
///
/// ## Step 1: Define your validation rules struct
///
/// ```rust,ignore
/// #[derive(Clone, Debug)]
/// pub struct UserRules {
///     pub min_name: usize,
///     pub max_name: usize,
/// }
/// ```
///
/// ## Step 2: Register via `#[component]`
///
/// ```rust,ignore
/// use summer_web::validation::context::ValidationContextRegistry;
///
/// #[summer::component]
/// fn create_validation_contexts() -> ValidationContextRegistry {
///     let mut registry = ValidationContextRegistry::new();
///     registry.insert(UserRules { min_name: 2, max_name: 50 });
///     registry.insert(OrderRules { max_items: 100 });
///     registry
/// }
/// ```
///
/// For larger projects, each module can export a registration function:
///
/// ```rust,ignore
/// // user/mod.rs
/// pub fn register_validation(r: &mut ValidationContextRegistry) {
///     r.insert(UserRules { min_name: 2, max_name: 50 });
///     r.insert(UserPasswordRules { min_len: 8 });
/// }
///
/// // main.rs or validation.rs
/// #[summer::component]
/// fn create_validation_contexts() -> ValidationContextRegistry {
///     let mut r = ValidationContextRegistry::new();
///     user::register_validation(&mut r);
///     role::register_validation(&mut r);
///     r
/// }
/// ```
///
/// ## Step 3A: Use in your garde struct
///
/// ```rust,ignore
/// #[derive(Debug, Deserialize, garde::Validate, summer_web::GardeSchema)]
/// #[garde(context(UserRules as ctx))]
/// pub struct CreateUserRequest {
///     #[garde(length(min = ctx.min_name, max = ctx.max_name))]
///     pub name: String,
///     #[garde(email)]
///     pub email: String,
/// }
/// ```
///
/// The `Garde<Json<CreateUserRequest>>` wrapper will automatically look up
/// `UserRules` from the registry at request time.
///
/// ## Step 3B: Use in your validator struct with `ValidateArgs`
///
/// ```rust,ignore
/// #[derive(Debug, Deserialize, validator::Validate)]
/// #[validate(context = UserRules)]
/// pub struct ListUsersRequest {
///     #[validate(custom(function = "validate_page_size", use_context))]
///     pub page_size: usize,
/// }
///
/// fn validate_page_size(
///     value: usize,
///     ctx: &UserRules,
/// ) -> Result<(), validator::ValidationError> {
///     if value > ctx.max_name {
///         return Err(validator::ValidationError::new("page_size_too_large"));
///     }
///     Ok(())
/// }
/// ```
///
/// The `ValidatorEx<Json<ListUsersRequest>>` wrapper will automatically look up
/// `UserRules` from the registry at request time.
#[derive(Clone, Default)]
pub struct ValidationContextRegistry {
    map: AnyMap,
}

impl ValidationContextRegistry {
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
        }
    }

    /// Insert a typed context value.
    pub fn insert<C: Clone + Send + Sync + 'static>(&mut self, context: C) {
        self.map.insert(TypeId::of::<C>(), Box::new(context));
    }

    /// Get a reference to a context by type.
    pub fn get<C: Send + Sync + 'static>(&self) -> Option<&C> {
        self.map
            .get(&TypeId::of::<C>())
            .and_then(|boxed| boxed.as_ref().as_any().downcast_ref::<C>())
    }

    /// Get a reference to a context by type, panicking if not found.
    pub fn get_expect<C: Send + Sync + 'static>(&self) -> &C {
        self.get::<C>().unwrap_or_else(|| {
            panic!(
                "Validation context '{}' not found in registry. \
                 Did you forget to register it?",
                type_name::<C>()
            )
        })
    }

    /// Check if a context type is registered.
    pub fn contains<C: Send + Sync + 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<C>())
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationContextRegistry;

    #[derive(Clone, Debug, PartialEq)]
    struct UserRules {
        min_name: usize,
        max_name: usize,
    }

    #[test]
    fn registry_clone_preserves_typed_lookup() {
        let mut registry = ValidationContextRegistry::new();
        registry.insert(UserRules {
            min_name: 2,
            max_name: 50,
        });

        let cloned = registry.clone();
        let rules = cloned.get::<UserRules>().unwrap();

        assert_eq!(
            rules,
            &UserRules {
                min_name: 2,
                max_name: 50,
            }
        );
    }
}
