//! Error types for the FluxDI dependency injection container.
//!
//! This module defines a lightweight error model used across the container to
//! describe failures that can occur during service registration, resolution,
//! scope handling, and module initialization.
//!
//! # Design
//!
//! - `ErrorKind` captures the error category.
//! - `Error` stores the category and a human-readable message.
//!
//! The helpers in `Error` are provided to keep call sites concise and to
//! maintain consistent error messages.
//!
//! # Feature Flags
//!
//! - `tracing`: logs errors when they are created.
//! - `debug`: enables extra diagnostic formatting in `Display`.
//!
//! # Examples
//!
//! ```
//! use fluxdi::error::Error;
//!
//! let err = Error::service_not_provided("MyService");
//! assert!(err.message.contains("MyService"));
//! ```

use core::fmt;

#[cfg(feature = "tracing")]
use tracing::error;

/// Error categories for the container.
///
/// These variants are intentionally coarse-grained to keep error handling
/// straightforward while still expressive enough for diagnostics.
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub enum ErrorKind {
    /// Service provider not found for the requested type.
    ServiceNotProvided,
    /// Type mismatch during downcast or resolution.
    TypeMismatch,
    /// Factory closure already registered for this type.
    ProviderAlreadyRegistered,
    /// Circular dependency detected in resolution chain.
    CircularDependency,
    /// Async provider was resolved through a synchronous resolve method.
    AsyncFactoryRequiresAsyncResolve,
    /// Service creation was denied by configured resource limits.
    ResourceLimitExceeded,
    /// Module lifecycle hook failed.
    ModuleLifecycleFailed,
}

/// Container error structure.
///
/// `kind` enables programmatic handling, while `message` is human-readable.
#[derive(Clone)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl Error {
    /// Creates a new error with the given kind and message.
    ///
    /// If the `tracing` feature is enabled, the error is automatically logged.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        let error = Self {
            kind: kind.clone(),
            message: message.into(),
        };

        #[cfg(feature = "tracing")]
        error!("{}", error);

        error
    }

    /// Service provider not found for the requested type.
    pub fn service_not_provided(type_name: &str) -> Self {
        Self::new(
            ErrorKind::ServiceNotProvided,
            format!("No provider registered for type: {}", type_name),
        )
    }

    /// Type mismatch during downcast or factory execution.
    ///
    /// This covers both immediate type mismatches and cached instance type mismatches.
    pub fn type_mismatch(type_name: &str) -> Self {
        Self::new(
            ErrorKind::TypeMismatch,
            format!("Type mismatch when resolving: {}", type_name),
        )
    }

    /// Provider already registered for this type.
    ///
    /// Attempting to register a provider for a type that already has one.
    pub fn provider_already_registered(type_name: &str, scope: &str) -> Self {
        Self::new(
            ErrorKind::ProviderAlreadyRegistered,
            format!(
                "Provider ({} scope) already registered for type: {}",
                scope, type_name
            ),
        )
    }

    /// Circular dependency detected in resolution chain.
    pub fn circular_dependency(dependency_chain: &[&str]) -> Self {
        Self::new(
            ErrorKind::CircularDependency,
            format!(
                "Circular dependency detected: {}",
                dependency_chain.join(" -> ")
            ),
        )
    }

    /// Async provider resolved through a synchronous resolve method.
    pub fn async_factory_requires_async_resolve(type_name: &str) -> Self {
        Self::new(
            ErrorKind::AsyncFactoryRequiresAsyncResolve,
            format!(
                "Type {} is registered with an async provider; use try_resolve_async/resolve_async",
                type_name
            ),
        )
    }

    /// Service creation was denied by configured resource limits.
    pub fn resource_limit_exceeded(type_name: &str, details: &str) -> Self {
        Self::new(
            ErrorKind::ResourceLimitExceeded,
            format!(
                "Resource limit exceeded while creating type {}: {}",
                type_name, details
            ),
        )
    }

    /// Module lifecycle hook failed.
    pub fn module_lifecycle_failed(module_name: &str, phase: &str, details: &str) -> Self {
        Self::new(
            ErrorKind::ModuleLifecycleFailed,
            format!(
                "Module lifecycle failed: module={}, phase={}, details={}",
                module_name, phase, details
            ),
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "debug")]
        {
            write!(f, "({:?}) - {}", self.kind, self.message)
        }
        #[cfg(not(feature = "debug"))]
        {
            write!(f, "{}", self.message)
        }
    }
}

#[cfg(feature = "debug")]
impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_not_provided_error() {
        let err = Error::service_not_provided("MyType");
        assert!(err.kind == ErrorKind::ServiceNotProvided);
        assert!(err.message.contains("MyType"));
        assert!(err.message.contains("provider"));
    }

    #[test]
    fn type_mismatch_error() {
        let err = Error::type_mismatch("OtherType");
        assert!(err.kind == ErrorKind::TypeMismatch);
        assert!(err.message.contains("OtherType"));
    }

    #[test]
    fn provider_already_registered_error() {
        let err = Error::provider_already_registered("Foo", "transient");
        assert!(err.kind == ErrorKind::ProviderAlreadyRegistered);
        assert!(err.message.contains("Foo"));
        assert!(err.message.contains("transient"));
    }

    #[test]
    fn circular_dependency_error() {
        let chain = ["A", "B", "A"];
        let err = Error::circular_dependency(&chain);
        assert!(err.kind == ErrorKind::CircularDependency);
        assert!(err.message.contains("A -> B -> A"));
    }

    #[test]
    fn async_factory_requires_async_resolve_error() {
        let err = Error::async_factory_requires_async_resolve("AsyncType");
        assert!(err.kind == ErrorKind::AsyncFactoryRequiresAsyncResolve);
        assert!(err.message.contains("AsyncType"));
        assert!(err.message.contains("try_resolve_async"));
    }

    #[test]
    fn resource_limit_exceeded_error() {
        let err = Error::resource_limit_exceeded("DbPool", "max_concurrent_creations=1");
        assert!(err.kind == ErrorKind::ResourceLimitExceeded);
        assert!(err.message.contains("DbPool"));
        assert!(err.message.contains("max_concurrent_creations=1"));
    }

    #[test]
    fn module_lifecycle_failed_error() {
        let err = Error::module_lifecycle_failed("WebModule", "on_start", "bind failed");
        assert!(err.kind == ErrorKind::ModuleLifecycleFailed);
        assert!(err.message.contains("WebModule"));
        assert!(err.message.contains("on_start"));
        assert!(err.message.contains("bind failed"));
    }

    #[test]
    fn display_trait() {
        let err = Error::service_not_provided("X");
        let s = format!("{}", err);
        #[cfg(feature = "debug")]
        assert!(s.contains("ServiceNotProvided"));
        assert!(s.contains("X"));
    }

    #[test]
    fn error_kind_equality() {
        let err1 = Error::type_mismatch("A");
        let err2 = Error::type_mismatch("B");
        assert!(err1.kind == err2.kind);
        assert_ne!(err1.message, err2.message);
    }
}
