//! Validation context storage for garde's `#[garde(context(...))]`.
//!
//! Provides a typed map ([`ValidationContextRegistry`]) for storing garde validation
//! contexts. Users register it via `#[component]`, extractors look up contexts at
//! request time.
//!
//! # Architecture
//!
//! 1. **User registers** — `#[component]` returns `ValidationContextRegistry`
//! 2. **Extractor consumes** — `GardeJson<T>` etc. look up context at request time
//!
//! # When do you need this?
//!
//! Only when your garde structs use **custom context** (`#[garde(context(...))]`).
//! If all your rules are literal values (e.g. `#[garde(length(min = 1, max = 100))]`),
//! you don't need a registry at all — the `GardeJson<T>` extractor handles it automatically.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;


/// A hasher optimized for `TypeId` keys (already hashed u64 values).
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

/// Internal storage: `Arc<dyn Any>` makes each entry cheaply Clone-able,
/// so `ValidationContextRegistry` itself is Clone without requiring
/// context types to implement Clone.
type AnyMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>, BuildHasherDefault<IdHasher>>;

/// A typed map that stores garde validation context instances.
///
/// Built once at startup via `#[component]`, then read-only at request time.
/// Internally uses `Arc<dyn Any>` so the registry is `Clone` without requiring
/// context types to implement `Clone`.
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
/// ## Step 3: Use in your garde struct
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
/// The `GardeJson<CreateUserRequest>` extractor will automatically look up
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
    pub fn insert<C: Send + Sync + 'static>(&mut self, context: C) {
        self.map.insert(TypeId::of::<C>(), Arc::new(context));
    }

    /// Get a reference to a context by type.
    pub fn get<C: Send + Sync + 'static>(&self) -> Option<&C> {
        self.map
            .get(&TypeId::of::<C>())
            .and_then(|arc| arc.downcast_ref::<C>())
    }

    /// Get a reference to a context by type, panicking if not found.
    pub fn get_expect<C: Send + Sync + 'static>(&self) -> &C {
        self.get::<C>().unwrap_or_else(|| {
            panic!(
                "Validation context '{}' not found in registry. \
                 Did you forget to register it?",
                std::any::type_name::<C>()
            )
        })
    }

    /// Check if a context type is registered.
    pub fn contains<C: Send + Sync + 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<C>())
    }
}
