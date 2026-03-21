//! Lifecycle options for bootstrap and shutdown.
//!
//! Enable the `lifecycle` feature for timeout support.

use std::time::Duration;

/// Options for application bootstrap.
///
/// Use with [`Application::bootstrap_with_options`](super::Application::bootstrap_with_options).
///
/// # Timeout
///
/// When the `lifecycle` feature is enabled, `timeout` limits how long the bootstrap
/// may take. If exceeded, an error is returned.
///
/// # Parallel Start
///
/// When `parallel_start` is `true`, module `on_start` hooks run concurrently.
/// Failures are aggregated into a single error.
///
/// # Examples
///
/// With timeout:
///
/// ```
/// use std::time::Duration;
/// use fluxdi::application::{Application, BootstrapOptions};
/// use fluxdi::{Injector, Module};
///
/// struct AppModule;
/// impl Module for AppModule {
///     fn providers(&self, _: &Injector) {}
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut app = Application::new(AppModule);
///     let opts = BootstrapOptions::default()
///         .with_timeout(Duration::from_secs(30));
///     app.bootstrap_with_options(opts).await?;
///     Ok(())
/// }
/// ```
///
/// With parallel module startup:
///
/// ```
/// use fluxdi::application::{Application, BootstrapOptions};
/// use fluxdi::{Injector, Module};
///
/// struct AppModule;
/// impl Module for AppModule {
///     fn providers(&self, _: &Injector) {}
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut app = Application::new(AppModule);
///     let opts = BootstrapOptions::default().with_parallel_start(true);
///     app.bootstrap_with_options(opts).await?;
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct BootstrapOptions {
    /// Maximum time allowed for the entire bootstrap process.
    /// When `None`, no timeout is applied.
    /// Requires the `lifecycle` feature to take effect.
    pub timeout: Option<Duration>,
    /// When `true`, run module `on_start` hooks in parallel.
    /// When `false` (default), run sequentially in dependency order.
    pub parallel_start: bool,
}

impl BootstrapOptions {
    /// Sets the bootstrap timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Enables parallel execution of module `on_start` hooks.
    pub fn with_parallel_start(mut self, parallel: bool) -> Self {
        self.parallel_start = parallel;
        self
    }
}

/// Options for application shutdown.
///
/// Use with [`Application::shutdown_with_options`](super::Application::shutdown_with_options).
///
/// # Timeout
///
/// When the `lifecycle` feature is enabled, `timeout` limits how long the shutdown
/// may take. If exceeded, an error is returned.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use fluxdi::application::{Application, ShutdownOptions};
/// use fluxdi::{Injector, Module};
///
/// struct AppModule;
/// impl Module for AppModule {
///     fn providers(&self, _: &Injector) {}
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut app = Application::new(AppModule);
///     app.bootstrap().await?;
///
///     let opts = ShutdownOptions::default()
///         .with_timeout(Duration::from_secs(10));
///     app.shutdown_with_options(opts).await?;
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct ShutdownOptions {
    /// Maximum time allowed for the entire shutdown process.
    /// When `None`, no timeout is applied.
    /// Requires the `lifecycle` feature to take effect.
    pub timeout: Option<Duration>,
}

impl ShutdownOptions {
    /// Sets the shutdown timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}
