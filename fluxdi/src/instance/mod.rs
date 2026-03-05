//! Instance wrapper for dependency injection values.
//!
//! This module provides the [`Instance`] type, which wraps a [`Shared`] reference
//! to provide convenient access to dependency-injected values. It serves as the
//! primary interface for consuming resolved dependencies.
//!
//! # Core Responsibilities
//!
//! - **Value Access**: Provides ergonomic access to the underlying value via references
//! - **Shared Ownership**: Maintains shared ownership through reference counting
//! - **Type Safety**: Preserves type information including support for trait objects
//!
//! # Design Philosophy
//!
//! The `Instance` type is a thin wrapper around `Shared<T>` that provides a clear
//! semantic distinction between "a resolved dependency" and "a shared reference".
//! This makes the DI container's API more intuitive and self-documenting.
//!
//! # Thread Safety
//!
//! When compiled with the `thread-safe` feature, `Instance<T>` can be safely shared
//! across threads (assuming `T: Send + Sync`). The underlying `Shared` type will be
//! `Arc<T>`, providing atomic reference counting.
//!
//! # Examples
//!
//! Creating and using an instance:
//!
//! ```
//! use fluxdi::{Instance, Shared};
//!
//! struct Config {
//!     debug: bool,
//!     port: u16,
//! }
//!
//! let shared_config = Shared::new(Config { debug: true, port: 8080 });
//! let instance = Instance::new(shared_config);
//!
//! // Access via reference
//! assert_eq!(instance.get().port, 8080);
//!
//! // Get a cloned Shared reference
//! let shared = instance.value();
//! assert_eq!(shared.port, 8080);
//! ```
//!
//! With trait objects:
//!
//! ```
//! use fluxdi::{Instance, Shared};
//!
//! trait Logger {
//!     fn log(&self, message: &str);
//! }
//!
//! struct ConsoleLogger;
//! impl Logger for ConsoleLogger {
//!     fn log(&self, message: &str) {
//!         println!("{}", message);
//!     }
//! }
//!
//! let logger: Shared<dyn Logger> = Shared::new(ConsoleLogger);
//! let instance = Instance::<dyn Logger>::new(logger);
//!
//! instance.get().log("Hello, world!");
//! ```

use crate::Shared;

/// A wrapper around a shared reference to a dependency-injected value.
///
/// `Instance<T>` provides a convenient interface for accessing values resolved
/// by the dependency injection container. It maintains shared ownership of the
/// underlying value through reference counting.
///
/// # Type Parameters
///
/// - `T`: The type of the wrapped value. Can be `?Sized` to support trait objects.
///
/// # Invariants
///
/// - Always contains a valid `Shared<T>` reference
/// - The wrapped value is immutable (interior mutability requires explicit use
///   of `Mutex`, `RwLock`, `RefCell`, etc.)
///
/// # Memory Management
///
/// The instance holds a strong reference to the underlying value. The value will
/// be deallocated when all `Instance` wrappers and `Shared` references are dropped.
///
/// # Examples
///
/// Basic usage with a concrete type:
///
/// ```
/// use fluxdi::{Instance, Shared};
///
/// #[derive(Debug, PartialEq)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user = Shared::new(User {
///     id: 1,
///     name: "Alice".to_string(),
/// });
///
/// let instance = Instance::new(user);
/// assert_eq!(instance.get().id, 1);
/// assert_eq!(instance.get().name, "Alice");
/// ```
///
/// Multiple instances sharing the same value:
///
/// ```
/// use fluxdi::{Instance, Shared};
///
/// let shared = Shared::new(vec![1, 2, 3]);
/// let instance1 = Instance::new(shared.clone());
/// let instance2 = Instance::new(shared.clone());
///
/// // Both instances point to the same allocation
/// assert!(Shared::ptr_eq(&instance1.value(), &instance2.value()));
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Instance<T: ?Sized + 'static> {
    /// The shared reference to the actual value
    value: Shared<T>,
}

mod accessors;
mod constructors;

#[cfg(test)]
mod tests;
