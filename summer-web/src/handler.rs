use crate::Router;
use std::collections::HashMap;

pub use inventory::submit;

/// Group name used by hand-written registrars that don't override
/// [`TypedHandlerRegistrar::group`].
pub const DEFAULT_GROUP: &str = "default";

/// TypeHandler is used to configure the summer-macro marked route handler
pub trait TypedHandlerRegistrar: Send + Sync + 'static {
    /// install route
    fn install_route(&self, router: Router) -> Router;

    /// Route group this handler belongs to.
    ///
    /// Used by [`auto_grouped_routers`] to bucket handlers so callers can compose routers
    /// per crate/group and apply group-specific middleware without affecting the rest of
    /// the application.
    ///
    /// - Hand-written impls get the default `"default"` bucket.
    /// - The `#[post("/...", group = "xxx")]` macro overrides this. When no `group = "..."`
    ///   is written on the macro, it falls back to `env!("CARGO_PKG_NAME")`, so each crate
    ///   is automatically its own group.
    fn group(&self) -> &'static str {
        DEFAULT_GROUP
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
/// Use [`GroupedRouters::take_default`] and [`GroupedRouters::take_group`] to build
/// the final router explicitly after a single inventory collection pass.
#[derive(Default)]
pub struct GroupedRouters {
    /// Routes registered without a `group = "..."` attribute.
    default: Router,
    /// Routes registered with `group = "NAME"`, keyed by the group name.
    by_group: HashMap<String, Router>,
}

impl GroupedRouters {
    /// Remove and return the default group router.
    pub fn take_default(&mut self) -> Router {
        std::mem::replace(&mut self.default, Router::new())
    }

    /// Remove and return the router for `group`.
    ///
    /// Missing groups return an empty router. Passing [`DEFAULT_GROUP`] is equivalent
    /// to [`Self::take_default`].
    pub fn take_group(&mut self, group: impl AsRef<str>) -> Router {
        let group = group.as_ref();
        if group == DEFAULT_GROUP {
            return self.take_default();
        }

        self.by_group.remove(group).unwrap_or_else(Router::new)
    }

    pub(crate) fn into_parts(self) -> (Router, HashMap<String, Router>) {
        (self.default, self.by_group)
    }
}

/// Collect all inventory-registered handlers bucketed by their [`TypedHandlerRegistrar::group`]
/// tag.
///
/// Handlers in the [`DEFAULT_GROUP`] group (e.g. hand-written impls that don't override `group()`)
/// are returned by [`GroupedRouters::take_default`]. Everything else is returned by
/// [`GroupedRouters::take_group`] under its group key.
///
/// This is the bucketed counterpart of [`auto_router`]. The `auto_config` macro expands to
/// call this one so named groups can be merged separately by the web plugin or taken
/// explicitly by applications.
pub fn auto_grouped_routers() -> GroupedRouters {
    #[cfg(feature = "openapi")]
    crate::enable_openapi();

    let mut default = Router::new();
    let mut by_group: HashMap<String, Router> = HashMap::new();

    for handler in inventory::iter::<&dyn TypedHandlerRegistrar> {
        let group = handler.group();
        if group == DEFAULT_GROUP {
            default = handler.install_route(default);
        } else {
            let existing = by_group.remove(group).unwrap_or_else(Router::new);
            by_group.insert(group.to_string(), handler.install_route(existing));
        }
    }

    GroupedRouters { default, by_group }
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
