//! Provider module for dependency injection.
//!
//! This module defines the [`Provider`] struct, which encapsulates the logic for creating
//! instances of dependencies with different lifecycle scopes (singleton, transient, root).
//!
//! # Lifecycle Scopes
//!
//! - **Singleton (Module)**: One instance per injector module
//! - **Transient**: New instance on every resolution
//! - **Root**: One instance per root injector (application-wide)
//!
//! # Thread Safety
//!
//! The module supports two compilation modes via the `thread-safe` feature flag:
//!
//! - **With `thread-safe`**: Factories must be `Send + Sync`, allowing safe concurrent access
//! - **Without `thread-safe`**: Single-threaded mode with no thread safety overhead
//!
//! # Examples
//!
//! ```
//! use fluxdi::{Provider, Injector, Shared};
//!
//! // Concrete type - singleton
//! struct Database {
//!     url: String,
//! }
//!
//! let provider = Provider::singleton(|_| {
//!     Shared::new(Database {
//!         url: "postgresql://localhost".to_string(),
//!     })
//! });
//! ```
//!
//! For trait objects:
//!
//! ```
//! use fluxdi::{Provider, Shared};
//!
//! trait Logger {}
//! struct ConsoleLogger;
//! impl Logger for ConsoleLogger {}
//!
//! let provider = Provider::<dyn Logger>::singleton(|_| {
//!     Shared::new(ConsoleLogger) as Shared<dyn Logger>
//! });
//! ```

use crate::error::Error;
use crate::injector::Injector;
use crate::instance::Instance;
use crate::runtime::Shared;
use crate::scope::Scope;

#[cfg(feature = "async-factory")]
use std::future::Future;
#[cfg(feature = "async-factory")]
use std::pin::Pin;
use std::time::Duration;

#[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[cfg(feature = "tracing")]
use tracing::{debug, info};

#[cfg(all(feature = "async-factory", not(feature = "thread-safe")))]
type AsyncFactory<T> =
    Box<dyn Fn(Injector) -> Pin<Box<dyn Future<Output = Instance<T>> + 'static>> + 'static>;

#[cfg(all(feature = "async-factory", feature = "thread-safe"))]
type AsyncFactory<T> = Box<
    dyn Fn(Injector) -> Pin<Box<dyn Future<Output = Instance<T>> + Send + 'static>>
        + Send
        + Sync
        + 'static,
>;

/// Behavior when a resource limit is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Return an error immediately when no creation slot is available.
    Deny,
    /// Block until a creation slot becomes available.
    Block,
}

/// Limits applied to provider factory execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum number of concurrent in-flight factory executions for this provider.
    ///
    /// `None` disables the limit.
    pub max_concurrent_creations: Option<usize>,
    /// Action to take when the limit is reached.
    pub policy: Policy,
    /// Optional timeout used by `Policy::Block`.
    ///
    /// - In `thread-safe` sync resolve, this bounds `Condvar` wait time.
    /// - With `resource-limit-async`, async resolve uses `tokio::time::timeout`.
    pub timeout: Option<Duration>,
}

impl Limits {
    pub const fn unlimited() -> Self {
        Self {
            max_concurrent_creations: None,
            policy: Policy::Deny,
            timeout: None,
        }
    }

    pub const fn deny(max_concurrent_creations: usize) -> Self {
        Self {
            max_concurrent_creations: Some(max_concurrent_creations),
            policy: Policy::Deny,
            timeout: None,
        }
    }

    pub const fn block(max_concurrent_creations: usize) -> Self {
        Self {
            max_concurrent_creations: Some(max_concurrent_creations),
            policy: Policy::Block,
            timeout: None,
        }
    }

    /// Applies a timeout to `Policy::Block`.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds block policy with timeout in one call.
    pub fn block_with_timeout(max_concurrent_creations: usize, timeout: Duration) -> Self {
        Self::block(max_concurrent_creations).with_timeout(timeout)
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::unlimited()
    }
}

#[cfg(not(feature = "thread-safe"))]
#[derive(Debug)]
pub(crate) struct Limiter {
    max: usize,
    policy: Policy,
    current: std::cell::Cell<usize>,
    timeout: Option<Duration>,
}

#[cfg(feature = "thread-safe")]
#[derive(Debug)]
pub(crate) struct Limiter {
    max: usize,
    policy: Policy,
    current: std::sync::Mutex<usize>,
    condvar: std::sync::Condvar,
    timeout: Option<Duration>,
    #[cfg(feature = "resource-limit-async")]
    async_semaphore: Shared<Semaphore>,
}

#[derive(Debug)]
pub(crate) enum CreationPermit {
    Sync {
        limiter: Shared<Limiter>,
    },
    #[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
    Async(OwnedSemaphorePermit),
}

impl Drop for CreationPermit {
    fn drop(&mut self) {
        #[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
        match self {
            Self::Sync { limiter } => limiter.release(),
            Self::Async(_permit) => {}
        }

        #[cfg(not(all(feature = "thread-safe", feature = "resource-limit-async")))]
        {
            let Self::Sync { limiter } = self;
            limiter.release();
        }
    }
}

impl Limiter {
    fn from_limits(limits: Limits) -> Option<Shared<Self>> {
        let max = limits.max_concurrent_creations?;

        #[cfg(feature = "thread-safe")]
        {
            Some(Shared::new(Self {
                max,
                policy: limits.policy,
                current: std::sync::Mutex::new(0),
                condvar: std::sync::Condvar::new(),
                timeout: limits.timeout,
                #[cfg(feature = "resource-limit-async")]
                async_semaphore: Shared::new(Semaphore::new(max)),
            }))
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            Some(Shared::new(Self {
                max,
                policy: limits.policy,
                current: std::cell::Cell::new(0),
                timeout: limits.timeout,
            }))
        }
    }

    fn try_acquire(limiter: &Shared<Self>, type_name: &str) -> Result<CreationPermit, Error> {
        #[cfg(feature = "thread-safe")]
        {
            if limiter.max == 0 {
                return Err(Error::resource_limit_exceeded(
                    type_name,
                    "max_concurrent_creations must be greater than 0",
                ));
            }

            let mut current = limiter.current.lock().unwrap();
            let deadline = limiter
                .timeout
                .map(|timeout| std::time::Instant::now() + timeout);
            loop {
                if *current < limiter.max {
                    *current += 1;
                    return Ok(CreationPermit::Sync {
                        limiter: limiter.clone(),
                    });
                }

                match limiter.policy {
                    Policy::Deny => {
                        return Err(Error::resource_limit_exceeded(
                            type_name,
                            format!("max_concurrent_creations={}", limiter.max).as_str(),
                        ));
                    }
                    Policy::Block => {
                        if let Some(deadline) = deadline {
                            let now = std::time::Instant::now();
                            if now >= deadline {
                                return Err(Error::resource_limit_exceeded(
                                    type_name,
                                    format!(
                                        "max_concurrent_creations={} timeout={:?}",
                                        limiter.max,
                                        limiter.timeout.unwrap_or_default()
                                    )
                                    .as_str(),
                                ));
                            }

                            let remaining = deadline.saturating_duration_since(now);
                            let (next_guard, wait_result) =
                                limiter.condvar.wait_timeout(current, remaining).unwrap();
                            current = next_guard;

                            if wait_result.timed_out() && *current >= limiter.max {
                                return Err(Error::resource_limit_exceeded(
                                    type_name,
                                    format!(
                                        "max_concurrent_creations={} timeout={:?}",
                                        limiter.max,
                                        limiter.timeout.unwrap_or_default()
                                    )
                                    .as_str(),
                                ));
                            }
                        } else {
                            current = limiter.condvar.wait(current).unwrap();
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            if limiter.max == 0 {
                return Err(Error::resource_limit_exceeded(
                    type_name,
                    "max_concurrent_creations must be greater than 0",
                ));
            }

            let current = limiter.current.get();
            if current < limiter.max {
                limiter.current.set(current + 1);
                return Ok(CreationPermit::Sync {
                    limiter: limiter.clone(),
                });
            }

            match limiter.policy {
                Policy::Deny => Err(Error::resource_limit_exceeded(
                    type_name,
                    format!("max_concurrent_creations={}", limiter.max).as_str(),
                )),
                Policy::Block => Err(Error::resource_limit_exceeded(
                    type_name,
                    if limiter.timeout.is_some() {
                        "policy=Block (with timeout) requires `thread-safe` feature"
                    } else {
                        "policy=Block requires `thread-safe` feature"
                    },
                )),
            }
        }
    }

    #[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
    async fn try_acquire_async(
        limiter: &Shared<Self>,
        type_name: &str,
    ) -> Result<CreationPermit, Error> {
        if limiter.max == 0 {
            return Err(Error::resource_limit_exceeded(
                type_name,
                "max_concurrent_creations must be greater than 0",
            ));
        }

        match limiter.policy {
            Policy::Deny => limiter
                .async_semaphore
                .clone()
                .try_acquire_owned()
                .map(CreationPermit::Async)
                .map_err(|_| {
                    Error::resource_limit_exceeded(
                        type_name,
                        format!("max_concurrent_creations={}", limiter.max).as_str(),
                    )
                }),
            Policy::Block => {
                if let Some(timeout) = limiter.timeout {
                    let acquire = limiter.async_semaphore.clone().acquire_owned();
                    match tokio::time::timeout(timeout, acquire).await {
                        Ok(Ok(permit)) => Ok(CreationPermit::Async(permit)),
                        Ok(Err(_)) => Err(Error::resource_limit_exceeded(
                            type_name,
                            "async semaphore closed",
                        )),
                        Err(_) => Err(Error::resource_limit_exceeded(
                            type_name,
                            format!(
                                "max_concurrent_creations={} timeout={:?}",
                                limiter.max, timeout
                            )
                            .as_str(),
                        )),
                    }
                } else {
                    limiter
                        .async_semaphore
                        .clone()
                        .acquire_owned()
                        .await
                        .map(CreationPermit::Async)
                        .map_err(|_| {
                            Error::resource_limit_exceeded(type_name, "async semaphore closed")
                        })
                }
            }
        }
    }

    fn release(&self) {
        #[cfg(feature = "thread-safe")]
        {
            let mut current = self.current.lock().unwrap();
            if *current > 0 {
                *current -= 1;
            }

            if self.policy == Policy::Block {
                self.condvar.notify_one();
            }
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            let current = self.current.get();
            if current > 0 {
                self.current.set(current - 1);
            }
        }
    }
}

/// A provider encapsulates the factory logic for creating instances of type `T`.
///
/// The provider stores:
/// - The lifecycle [`Scope`] (singleton, transient, or root)
/// - A factory function that creates [`Instance<T>`] when invoked
///
/// # Type Parameters
///
/// - `T`: The type being provided. Can be `?Sized` to support trait objects.
///
/// # Thread Safety
///
/// When compiled with the `thread-safe` feature, the factory function must be
/// `Send + Sync` to allow safe sharing across threads.
///
/// # Examples
///
/// Creating a singleton provider:
///
/// ```
/// use fluxdi::{Provider, Shared};
///
/// struct Service {
///     name: String,
/// }
///
/// let provider = Provider::singleton(|_injector| {
///     Shared::new(Service {
///         name: "MyService".to_string(),
///     })
/// });
/// ```
pub struct Provider<T: ?Sized + 'static> {
    /// The lifecycle scope of this provider
    pub scope: Scope,

    /// The factory function that creates instances
    ///
    /// In single-threaded mode, the factory only needs to be `'static`.
    /// In thread-safe mode, the factory must also be `Send + Sync`.
    #[allow(clippy::type_complexity)]
    #[cfg(not(feature = "thread-safe"))]
    pub factory: Box<dyn Fn(&Injector) -> Instance<T> + 'static>,

    /// The factory function that creates instances (thread-safe variant)
    #[allow(clippy::type_complexity)]
    #[cfg(feature = "thread-safe")]
    pub factory: Box<dyn Fn(&Injector) -> Instance<T> + Send + Sync + 'static>,

    /// Optional async factory used by `Injector::try_resolve_async`.
    #[cfg(feature = "async-factory")]
    pub async_factory: Option<AsyncFactory<T>>,

    /// Optional resource limits for this provider factory.
    pub limits: Limits,

    limiter: Option<Shared<Limiter>>,
}

#[cfg(feature = "debug")]
impl<T: ?Sized + 'static> std::fmt::Debug for Provider<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct(std::any::type_name::<Self>());

        ds.field("scope", &self.scope);

        #[cfg(feature = "thread-safe")]
        {
            ds.field(
                "factory",
                &"Box<dyn Fn(&Injector) -> Instance<T> + Send + Sync + 'static>",
            );
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            ds.field(
                "factory",
                &"Box<dyn Fn(&Injector) -> Instance<T> + 'static>",
            );
        }

        #[cfg(feature = "async-factory")]
        {
            ds.field("async_factory", &self.async_factory.is_some());
        }

        ds.field("limits", &self.limits);
        ds.field("limiter", &self.limiter.is_some());

        ds.finish()
    }
}

impl<T: ?Sized + 'static> Provider<T> {
    /// Applies resource limits to this provider.
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self.limiter = Limiter::from_limits(limits);
        self
    }

    pub(crate) fn acquire_creation_permit(
        &self,
        type_name: &str,
    ) -> Result<Option<CreationPermit>, Error> {
        if let Some(limiter) = &self.limiter {
            return Limiter::try_acquire(limiter, type_name).map(Some);
        }

        Ok(None)
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn acquire_creation_permit_async(
        &self,
        type_name: &str,
    ) -> Result<Option<CreationPermit>, Error> {
        if let Some(limiter) = &self.limiter {
            #[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
            {
                return Limiter::try_acquire_async(limiter, type_name)
                    .await
                    .map(Some);
            }

            #[cfg(not(all(feature = "thread-safe", feature = "resource-limit-async")))]
            {
                return Limiter::try_acquire(limiter, type_name).map(Some);
            }
        }

        Ok(None)
    }
}

#[cfg(not(feature = "thread-safe"))]
impl<T: ?Sized + 'static> Provider<T> {
    /// Creates a singleton provider with module scope (single-threaded).
    ///
    /// A singleton provider creates **one instance per injector module**.
    /// Once created, the same instance is returned on subsequent resolutions
    /// within the same module.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates the instance when first requested
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    ///
    /// struct Config {
    ///     debug: bool,
    /// }
    ///
    /// let provider = Provider::singleton(|_injector| {
    ///     Shared::new(Config { debug: true })
    /// });
    /// ```
    ///
    /// # Note
    ///
    /// This is the single-threaded version. The factory does not need to be `Send + Sync`.
    pub fn singleton<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating singleton provider with Module scope (not thread-safe)");

        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing singleton factory for type instantiation");

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a transient provider (single-threaded).
    ///
    /// A transient provider creates a **new instance on every resolution**.
    /// No caching or instance reuse occurs.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates a new instance on each invocation
    ///
    /// # Use Cases
    ///
    /// - Request handlers
    /// - Short-lived operations
    /// - Stateful services that should not be shared
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    ///
    /// struct RequestHandler {
    ///     id: u64,
    /// }
    ///
    /// let provider = Provider::transient(|_injector| {
    ///     Shared::new(RequestHandler {
    ///         id: std::time::SystemTime::now()
    ///             .duration_since(std::time::UNIX_EPOCH)
    ///             .unwrap()
    ///             .as_nanos() as u64,
    ///     })
    /// });
    /// ```
    ///
    /// # Note
    ///
    /// This is the single-threaded version. The factory does not need to be `Send + Sync`.
    pub fn transient<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating transient provider with Transient scope (not thread-safe)");

        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing transient factory - creating new instance");

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a root-scoped provider (single-threaded).
    ///
    /// A root provider creates **one instance per root injector** (application-wide).
    /// This is the highest level of singleton, shared across all child injectors.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates the instance when first requested
    ///
    /// # Use Cases
    ///
    /// - Application configuration
    /// - Logging infrastructure
    /// - Connection pools
    /// - Global caches
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    ///
    /// struct AppConfig {
    ///     version: String,
    /// }
    ///
    /// let provider = Provider::root(|_injector| {
    ///     Shared::new(AppConfig {
    ///         version: "1.0.0".to_string(),
    ///     })
    /// });
    /// ```
    ///
    /// # Note
    ///
    /// This is the single-threaded version. The factory does not need to be `Send + Sync`.
    pub fn root<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating root provider with Root scope (not thread-safe)");

        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing root factory for type instantiation");

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a singleton provider with resource limits.
    pub fn singleton_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        Self::singleton(factory).with_limits(limits)
    }

    /// Creates a transient provider with resource limits.
    pub fn transient_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        Self::transient(factory).with_limits(limits)
    }

    /// Creates a root provider with resource limits.
    pub fn root_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + 'static,
    {
        Self::root(factory).with_limits(limits)
    }

    /// Creates a singleton provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn singleton_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + 'static,
        Fut: Future<Output = Shared<T>> + 'static,
    {
        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move { Instance::new(future.await) }
                })
            })),
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a transient provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn transient_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + 'static,
        Fut: Future<Output = Shared<T>> + 'static,
    {
        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move { Instance::new(future.await) }
                })
            })),
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a root-scoped provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn root_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + 'static,
        Fut: Future<Output = Shared<T>> + 'static,
    {
        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move { Instance::new(future.await) }
                })
            })),
            limits: Limits::default(),
            limiter: None,
        }
    }
}

#[cfg(feature = "thread-safe")]
impl<T: ?Sized + 'static> Provider<T> {
    /// Creates a singleton provider with module scope (thread-safe).
    ///
    /// A singleton provider creates **one instance per injector module**.
    /// Once created, the same instance is returned on subsequent resolutions
    /// within the same module. The instance can be safely shared across threads.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`.
    ///   Must be `Send + Sync` for thread safety.
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates the instance when first requested.
    ///   The closure must be `Send + Sync`.
    ///
    /// # Thread Safety
    ///
    /// The factory function must be `Send + Sync` because it may be called
    /// from any thread. The returned `Shared<T>` (which is `Arc<T>` in thread-safe mode)
    /// ensures safe concurrent access to the instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    ///
    /// #[derive(Debug)]
    /// struct Database {
    ///     connection_count: std::sync::atomic::AtomicUsize,
    /// }
    ///
    /// let provider = Provider::singleton(|_injector| {
    ///     Shared::new(Database {
    ///         connection_count: std::sync::atomic::AtomicUsize::new(0),
    ///     })
    /// });
    /// ```
    ///
    /// With trait objects:
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    /// use std::sync::Arc;
    ///
    /// trait Cache: Send + Sync {}
    /// struct MemoryCache;
    /// impl Cache for MemoryCache {}
    ///
    /// let provider = Provider::<dyn Cache>::singleton(|_injector| {
    ///     Arc::new(MemoryCache) as Arc<dyn Cache>
    /// });
    /// ```
    pub fn singleton<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating singleton provider with Module scope (thread-safe)");

        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing singleton factory for type instantiation");

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a transient provider (thread-safe).
    ///
    /// A transient provider creates a **new instance on every resolution**.
    /// No caching or instance reuse occurs. Each instance can be safely used
    /// across threads.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`.
    ///   Must be `Send + Sync` for thread safety.
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates a new instance on each invocation.
    ///   The closure must be `Send + Sync`.
    ///
    /// # Use Cases
    ///
    /// - Per-request services in web applications
    /// - Task-specific handlers
    /// - Stateful operations that should not be shared
    ///
    /// # Thread Safety
    ///
    /// While each resolution creates a new instance, the factory itself must
    /// be thread-safe (`Send + Sync`) as it may be called from multiple threads.
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    /// use std::sync::atomic::{AtomicU64, Ordering};
    ///
    /// static COUNTER: AtomicU64 = AtomicU64::new(0);
    ///
    /// struct RequestHandler {
    ///     id: u64,
    /// }
    ///
    /// let provider = Provider::transient(|_injector| {
    ///     Shared::new(RequestHandler {
    ///         id: COUNTER.fetch_add(1, Ordering::SeqCst),
    ///     })
    /// });
    /// ```
    pub fn transient<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating transient provider with Transient scope (thread-safe)");

        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing transient factory - creating new instance");

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a root-scoped provider (thread-safe).
    ///
    /// A root provider creates **one instance per root injector** (application-wide).
    /// This is the highest level of singleton, shared across all child injectors
    /// and safe to access from any thread.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Factory function type that takes an [`Injector`] reference and returns `Shared<T>`.
    ///   Must be `Send + Sync` for thread safety.
    ///
    /// # Arguments
    ///
    /// - `factory`: A closure that creates the instance when first requested.
    ///   The closure must be `Send + Sync`.
    ///
    /// # Use Cases
    ///
    /// - Application-wide configuration
    /// - Logging infrastructure
    /// - Thread-safe connection pools
    /// - Global metrics collectors
    /// - Shared caches
    ///
    /// # Thread Safety
    ///
    /// The root-scoped instance is shared across all threads and modules.
    /// Both the factory and the instance must be thread-safe.
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::{Provider, Shared};
    /// use std::sync::RwLock;
    ///
    /// struct GlobalConfig {
    ///     settings: RwLock<std::collections::HashMap<String, String>>,
    /// }
    ///
    /// let provider = Provider::root(|_injector| {
    ///     Shared::new(GlobalConfig {
    ///         settings: RwLock::new(std::collections::HashMap::new()),
    ///     })
    /// });
    /// ```
    pub fn root<F>(factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        #[cfg(feature = "tracing")]
        info!("Creating root provider with Root scope (thread-safe)");

        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(move |injector| {
                #[cfg(feature = "tracing")]
                debug!("Executing root factory for type instantiation");

                Instance::new(factory(injector))
            }),
            #[cfg(feature = "async-factory")]
            async_factory: None,
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a singleton provider with resource limits.
    pub fn singleton_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        Self::singleton(factory).with_limits(limits)
    }

    /// Creates a transient provider with resource limits.
    pub fn transient_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        Self::transient(factory).with_limits(limits)
    }

    /// Creates a root provider with resource limits.
    pub fn root_with_limits<F>(limits: Limits, factory: F) -> Provider<T>
    where
        F: Fn(&Injector) -> Shared<T> + Send + Sync + 'static,
    {
        Self::root(factory).with_limits(limits)
    }

    /// Creates a singleton provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn singleton_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Shared<T>> + Send + 'static,
    {
        Provider::<T> {
            scope: Scope::Module,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move { Instance::new(future.await) }
                })
            })),
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a transient provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn transient_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Shared<T>> + Send + 'static,
    {
        Provider::<T> {
            scope: Scope::Transient,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move { Instance::new(future.await) }
                })
            })),
            limits: Limits::default(),
            limiter: None,
        }
    }

    /// Creates a root-scoped provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn root_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Shared<T>> + Send + 'static,
    {
        Provider::<T> {
            scope: Scope::Root,
            factory: Box::new(|_| {
                panic!(
                    "async provider cannot be used with try_resolve/resolve; use try_resolve_async/resolve_async"
                )
            }),
            async_factory: Some(Box::new(move |injector| {
                Box::pin({
                    let future = factory(injector);
                    async move { Instance::new(future.await) }
                })
            })),
            limits: Limits::default(),
            limiter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::Scope;

    #[derive(Debug, Clone, PartialEq)]
    struct TestService {
        id: u32,
        name: String,
    }

    #[cfg(not(feature = "thread-safe"))]
    #[derive(Debug)]
    struct Counter {
        value: std::cell::Cell<u32>,
    }

    #[cfg(not(feature = "thread-safe"))]
    impl Counter {
        fn new() -> Self {
            Self {
                value: std::cell::Cell::new(0),
            }
        }

        fn increment(&self) -> u32 {
            let current = self.value.get();
            self.value.set(current + 1);
            current
        }
    }

    #[cfg(feature = "thread-safe")]
    #[derive(Debug)]
    struct Counter {
        value: std::sync::atomic::AtomicU32,
    }

    #[cfg(feature = "thread-safe")]
    impl Counter {
        fn new() -> Self {
            Self {
                value: std::sync::atomic::AtomicU32::new(0),
            }
        }

        fn increment(&self) -> u32 {
            self.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        }
    }

    trait Repository: std::fmt::Debug {}

    #[derive(Debug)]
    struct PostgresRepository {
        _connection_string: String,
    }

    impl Repository for PostgresRepository {}

    #[test]
    fn test_singleton_provider_has_module_scope() {
        let provider = Provider::singleton(|_| {
            Shared::new(TestService {
                id: 1,
                name: "test".to_string(),
            })
        });

        assert_eq!(provider.scope, Scope::Module);
    }

    #[test]
    fn test_singleton_provider_creates_instance() {
        let provider = Provider::singleton(|_| {
            Shared::new(TestService {
                id: 42,
                name: "singleton".to_string(),
            })
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);
        let value = instance.get();

        assert_eq!(value.id, 42);
        assert_eq!(value.name, "singleton");
    }

    #[test]
    fn test_singleton_provider_with_counter() {
        let counter = Shared::new(Counter::new());
        let counter_clone = counter.clone();

        let provider = Provider::singleton(move |_| {
            let id = counter_clone.increment();
            Shared::new(TestService {
                id,
                name: format!("service-{}", id),
            })
        });

        let injector = Injector::root();

        let instance1 = (provider.factory)(&injector);
        let instance2 = (provider.factory)(&injector);

        // Each call to factory creates new instance (counter increments)
        assert_eq!(instance1.get().id, 0);
        assert_eq!(instance2.get().id, 1);
    }

    #[test]
    fn test_singleton_provider_with_trait_object() {
        let provider = Provider::<dyn Repository>::singleton(|_| {
            Shared::new(PostgresRepository {
                _connection_string: "postgresql://localhost".to_string(),
            }) as Shared<dyn Repository>
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);

        // Just verify it compiles and runs
        let _repo = instance.get();
    }

    #[test]
    fn test_transient_provider_has_transient_scope() {
        let provider = Provider::transient(|_| {
            Shared::new(TestService {
                id: 1,
                name: "test".to_string(),
            })
        });

        assert_eq!(provider.scope, Scope::Transient);
    }

    #[test]
    fn test_transient_provider_creates_new_instances() {
        let counter = Shared::new(Counter::new());
        let counter_clone = counter.clone();

        let provider = Provider::transient(move |_| {
            let id = counter_clone.increment();
            Shared::new(TestService {
                id,
                name: format!("transient-{}", id),
            })
        });

        let injector = Injector::root();

        let instance1 = (provider.factory)(&injector);
        let instance2 = (provider.factory)(&injector);
        let instance3 = (provider.factory)(&injector);

        // Each call creates a new instance with incremented ID
        assert_eq!(instance1.get().id, 0);
        assert_eq!(instance2.get().id, 1);
        assert_eq!(instance3.get().id, 2);
    }

    #[test]
    fn test_transient_provider_with_trait_object() {
        let counter = Shared::new(Counter::new());
        let counter_clone = counter.clone();

        let provider = Provider::<dyn Repository>::transient(move |_| {
            let id = counter_clone.increment();
            Shared::new(PostgresRepository {
                _connection_string: format!("postgresql://localhost/{}", id),
            }) as Shared<dyn Repository>
        });

        let injector = Injector::root();
        let _instance1 = (provider.factory)(&injector);
        let _instance2 = (provider.factory)(&injector);

        // Verify counter was incremented twice
        assert_eq!(counter.increment(), 2);
    }

    #[test]
    fn test_root_provider_has_root_scope() {
        let provider = Provider::root(|_| {
            Shared::new(TestService {
                id: 1,
                name: "test".to_string(),
            })
        });

        assert_eq!(provider.scope, Scope::Root);
    }

    #[test]
    fn test_root_provider_creates_instance() {
        let provider = Provider::root(|_| {
            Shared::new(TestService {
                id: 100,
                name: "root-service".to_string(),
            })
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);
        let value = instance.get();

        assert_eq!(value.id, 100);
        assert_eq!(value.name, "root-service");
    }

    #[test]
    fn test_root_provider_with_static_data() {
        let provider = Provider::root(|_| {
            Shared::new(TestService {
                id: 0,
                name: "global-config".to_string(),
            })
        });

        let injector1 = Injector::root();
        let injector2 = Injector::root();

        let instance1 = (provider.factory)(&injector1);
        let instance2 = (provider.factory)(&injector2);

        // Both instances have the same configuration
        assert_eq!(instance1.get().name, "global-config");
        assert_eq!(instance2.get().name, "global-config");
    }

    #[test]
    fn test_different_scopes_create_different_providers() {
        let singleton = Provider::singleton(|_| {
            Shared::new(TestService {
                id: 1,
                name: "singleton".to_string(),
            })
        });

        let transient = Provider::transient(|_| {
            Shared::new(TestService {
                id: 2,
                name: "transient".to_string(),
            })
        });

        let root = Provider::root(|_| {
            Shared::new(TestService {
                id: 3,
                name: "root".to_string(),
            })
        });

        assert_eq!(singleton.scope, Scope::Module);
        assert_eq!(transient.scope, Scope::Transient);
        assert_eq!(root.scope, Scope::Root);

        assert_ne!(singleton.scope, transient.scope);
        assert_ne!(singleton.scope, root.scope);
        assert_ne!(transient.scope, root.scope);
    }

    #[test]
    fn test_factory_can_capture_environment() {
        let prefix = "test";
        let counter = 42;

        let provider = Provider::singleton(move |_| {
            Shared::new(TestService {
                id: counter,
                name: format!("{}-{}", prefix, counter),
            })
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);

        assert_eq!(instance.get().name, "test-42");
    }

    #[test]
    fn test_factory_receives_injector_reference() {
        let provider = Provider::singleton(|injector| {
            // We can use the injector inside the factory
            // For this test, just verify it's accessible
            let _ = injector;

            Shared::new(TestService {
                id: 999,
                name: "injector-test".to_string(),
            })
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);

        assert_eq!(instance.get().id, 999);
    }

    #[test]
    fn test_instance_get_returns_reference() {
        let provider = Provider::singleton(|_| {
            Shared::new(TestService {
                id: 55,
                name: "instance-test".to_string(),
            })
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);

        let value1 = instance.get();
        let value2 = instance.get();

        // Both references point to the same data
        assert_eq!(value1.id, value2.id);
        assert_eq!(value1.name, value2.name);
    }

    #[test]
    fn test_instance_value_returns_shared() {
        let provider = Provider::singleton(|_| {
            Shared::new(TestService {
                id: 77,
                name: "shared-test".to_string(),
            })
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);

        let shared1 = instance.value();
        let shared2 = instance.value();

        // Both Shared references point to same allocation
        assert!(Shared::ptr_eq(&shared1, &shared2));
    }

    #[test]
    fn test_nested_provider_creation() {
        // Create a provider that depends on another service
        let dependency = Shared::new(TestService {
            id: 1,
            name: "dependency".to_string(),
        });

        let dep_clone = dependency.clone();
        let provider = Provider::singleton(move |_| {
            let dep_id = dep_clone.id;
            Shared::new(TestService {
                id: dep_id + 100,
                name: format!("depends-on-{}", dep_id),
            })
        });

        let injector = Injector::root();
        let instance = (provider.factory)(&injector);

        assert_eq!(instance.get().id, 101);
        assert_eq!(instance.get().name, "depends-on-1");
    }

    #[test]
    fn test_provider_with_multiple_trait_objects() {
        trait Logger: std::fmt::Debug {}

        #[derive(Debug)]
        struct ConsoleLogger;
        impl Logger for ConsoleLogger {}

        #[derive(Debug)]
        struct FileLogger;
        impl Logger for FileLogger {}

        let console_provider =
            Provider::<dyn Logger>::singleton(|_| Shared::new(ConsoleLogger) as Shared<dyn Logger>);

        let file_provider =
            Provider::<dyn Logger>::transient(|_| Shared::new(FileLogger) as Shared<dyn Logger>);

        let injector = Injector::root();
        let _console = (console_provider.factory)(&injector);
        let _file = (file_provider.factory)(&injector);

        // Just verify both work with different scopes
        assert_eq!(console_provider.scope, Scope::Module);
        assert_eq!(file_provider.scope, Scope::Transient);
    }

    #[cfg(feature = "debug")]
    #[test]
    fn test_provider_debug_format() {
        let provider = Provider::singleton(|_| {
            Shared::new(TestService {
                id: 1,
                name: "debug".to_string(),
            })
        });

        let debug_str = format!("{:?}", provider);

        // Should contain type name and scope
        assert!(debug_str.contains("Provider"));
        assert!(debug_str.contains("scope"));
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn test_provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        // This test ensures Provider<T> is Send + Sync when thread-safe is enabled
        assert_send_sync::<Provider<TestService>>();
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn test_provider_can_be_shared_across_threads() {
        use std::sync::Arc;
        use std::thread;

        let provider = Arc::new(Provider::singleton(|_| {
            Shared::new(TestService {
                id: 123,
                name: "thread-test".to_string(),
            })
        }));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let provider_clone = Arc::clone(&provider);
                thread::spawn(move || {
                    let injector = Injector::root();
                    let instance = (provider_clone.factory)(&injector);
                    instance.get().id
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert_eq!(result, 123);
        }
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn test_transient_provider_creates_different_instances_per_thread() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::thread;

        static GLOBAL_COUNTER: AtomicU32 = AtomicU32::new(0);

        let provider = Arc::new(Provider::transient(|_| {
            let id = GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
            Shared::new(TestService {
                id,
                name: format!("thread-{}", id),
            })
        }));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let provider_clone = Arc::clone(&provider);
                thread::spawn(move || {
                    let injector = Injector::root();
                    let instance = (provider_clone.factory)(&injector);
                    instance.get().id
                })
            })
            .collect();

        let mut ids: Vec<u32> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        ids.sort();

        // Each thread should get a unique ID
        assert_eq!(ids, vec![0, 1, 2, 3]);
    }
}
