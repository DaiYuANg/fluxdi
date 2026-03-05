//! Module system for organizing dependency injection providers.
//!
//! This module defines the [`Module`] trait, which serves as the foundation for organizing
//! and configuring dependency injection in a modular, composable way.
//!
//! # Overview
//!
//! Modules allow you to:
//! - Group related providers together
//! - Import other modules to compose functionality
//! - Configure services within an injector
//!
//! # Thread Safety
//!
//! When the `thread-safe` feature is enabled, the [`Module`] trait requires implementors
//! to be `Send + Sync`, allowing modules to be safely shared across threads.
//!
//! # Examples
//!
//! ```
//! use fluxdi::module::Module;
//! use fluxdi::injector::Injector;
//!
//! struct DatabaseModule;
//!
//! impl Module for DatabaseModule {
//!     fn providers(&self, injector: &Injector) {
//!         // Register database-related providers
//!     }
//! }
//! ```
use crate::injector::Injector;
use crate::{Error, runtime::Shared};
use std::future::Future;
use std::pin::Pin;

#[cfg(not(feature = "thread-safe"))]
pub type ModuleLifecycleFuture = Pin<Box<dyn Future<Output = Result<(), Error>> + 'static>>;

#[cfg(feature = "thread-safe")]
pub type ModuleLifecycleFuture = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'static>>;

#[cfg(not(feature = "thread-safe"))]
mod trait_non_thread_safe;
#[cfg(feature = "thread-safe")]
mod trait_thread_safe;

#[cfg(not(feature = "thread-safe"))]
pub use trait_non_thread_safe::Module;
#[cfg(feature = "thread-safe")]
pub use trait_thread_safe::Module;

#[cfg(test)]
mod tests;
