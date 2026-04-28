use crate::Router;
use std::collections::HashMap;

pub use inventory::submit;

/// TypeHandler is used to configure the summer-macro marked route handler
pub trait TypedHandlerRegistrar: Send + Sync + 'static {
    /// install route
    fn install_route(&self, router: Router) -> Router;

    /// Route group this handler belongs to.
    ///
    /// Used by [`auto_grouped_routers`] to bucket handlers so plugins can apply middleware
    /// (via `add_group_layer`) to only the routes belonging to their own crate/group without
    /// affecting the rest of the application.
    ///
    /// - Hand-written impls get the default `"default"` bucket.
    /// - The `#[post("/...", group = "xxx")]` macro overrides this. When no `group = "..."`
    ///   is written on the macro, it falls back to `env!("CARGO_PKG_NAME")`, so each crate
    ///   is automatically its own group.
    fn group(&self) -> &'static str {
        "default"
    }
}

/// Add typed routes marked with procedural macros
pub trait TypeRouter {
    /// Add typed routes marked with procedural macros
    fn typed_route<F: TypedHandlerRegistrar>(self, factory: F) -> Self;
}

impl TypeRouter for Router {
    fn typed_route<F: TypedHandlerRegistrar>(self, factory: F) -> Self {
        factory.install_route(self)
    }
}

inventory::collect!(&'static dyn TypedHandlerRegistrar);

/// auto_config
#[macro_export]
macro_rules! submit_typed_handler {
    ($ty:ident) => {
        ::summer_web::handler::submit! {
            &$ty as &dyn ::summer_web::handler::TypedHandlerRegistrar
        }
    };
}

#[cfg(feature = "socket_io")]
#[macro_export]
macro_rules! submit_socketio_handler {
    ($ty:ident) => {
        ::summer_web::handler::submit! {
            &$ty as &dyn ::summer_web::handler::SocketIOHandlerRegistrar
        }
    };
}

/// auto_config
pub fn auto_router() -> Router {
    #[cfg(feature = "openapi")]
    crate::enable_openapi();

    let mut router = Router::new();
    for handler in inventory::iter::<&dyn TypedHandlerRegistrar> {
        router = handler.install_route(router);
    }
    router
}

/// Routers bucketed by group tag.
///
/// - [`GroupedRouters::default`] holds handlers registered without an explicit
///   `group = "..."` — these are merged straight into the main router.
/// - [`GroupedRouters::by_group`] holds handlers that declared a specific group;
///   [`crate::WebPlugin`] applies any group-specific layers (registered via
///   `add_group_layer`) to each named group router **before** merging everything
///   into the final axum Router, so the layer only affects that group's routes.
#[derive(Default)]
pub struct GroupedRouters {
    /// Routes registered without a `group = "..."` attribute.
    pub default: Router,
    /// Routes registered with `group = "NAME"`, keyed by the group name.
    pub by_group: HashMap<String, Router>,
}

/// Collect all inventory-registered handlers bucketed by their [`TypedHandlerRegistrar::group`]
/// tag.
///
/// Handlers in the `"default"` group (e.g. hand-written impls that don't override `group()`)
/// land in [`GroupedRouters::default`]. Everything else lands in
/// [`GroupedRouters::by_group`] under its group key.
///
/// This is the bucketed counterpart of [`auto_router`]. The `auto_config` macro expands to
/// call this one instead, so plugins can target their own routes via `add_group_layer`.
pub fn auto_grouped_routers() -> GroupedRouters {
    #[cfg(feature = "openapi")]
    crate::enable_openapi();

    let mut default = Router::new();
    let mut by_group: HashMap<String, Router> = HashMap::new();

    for handler in inventory::iter::<&dyn TypedHandlerRegistrar> {
        let group = handler.group();
        if group == "default" {
            default = handler.install_route(default);
        } else {
            let existing = by_group.remove(group).unwrap_or_else(Router::new);
            by_group.insert(group.to_string(), handler.install_route(existing));
        }
    }

    GroupedRouters { default, by_group }
}

/// Collect routes for a crate group.
///
/// Returns an empty `Router` if the group doesn't exist or has no handlers.
/// This is useful for crates that want to collect their own routes and apply
/// their own middleware/layers without writing manual `router()` functions.
///
/// # Example
///
/// ```ignore
/// // In your crate's router module:
/// use summer_web::handler::grouped_router;
///
/// pub fn router() -> Router {
///     grouped_router("my-crate-group")
/// }
/// ```
pub fn grouped_router(group: &str) -> Router {
    #[cfg(feature = "openapi")]
    crate::enable_openapi();

    let mut router = Router::new();
    for handler in inventory::iter::<&dyn TypedHandlerRegistrar> {
        if handler.group() == group {
            router = handler.install_route(router);
        }
    }
    router
}

#[cfg(feature = "socket_io")]
pub trait SocketIOHandlerRegistrar: Send + Sync + 'static {
    fn install_socketio_handlers(&self, socket: &crate::socketioxide::extract::SocketRef);
}

#[cfg(feature = "socket_io")]
inventory::collect!(&'static dyn SocketIOHandlerRegistrar);

#[cfg(feature = "socket_io")]
pub fn auto_socketio_setup(socket: &crate::socketioxide::extract::SocketRef) {
    for handler in inventory::iter::<&dyn SocketIOHandlerRegistrar> {
        handler.install_socketio_handlers(socket);
    }
}
