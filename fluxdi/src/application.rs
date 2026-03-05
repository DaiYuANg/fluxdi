//! Application container for bootstrapping and managing the dependency injection system.
//!
//! This module provides the [`Application`] struct, which serves as the main entry point
//! for configuring and initializing a dependency injection container with a modular structure.
//!
//! # Overview
//!
//! The `Application` manages:
//! - Root module registration
//! - Bootstrap process for loading modules and their dependencies
//! - Access to the root injector
//! - Hierarchical module loading with proper isolation
//!
//! # Thread Safety
//!
//! When the `thread-safe` feature is enabled, the [`Application`] requires the root module
//! to implement `Send + Sync`, allowing the application to be safely shared across threads.
//!
//! # Examples
//!
//! ```
//! use fluxdi::application::Application;
//! use fluxdi::module::Module;
//! use fluxdi::injector::Injector;
//!
//! struct AppModule;
//!
//! impl Module for AppModule {
//!     fn providers(&self, injector: &Injector) {
//!         // Register providers
//!     }
//! }
//!
//! let mut app = Application::new(AppModule);
//! app.bootstrap_sync().unwrap();
//!
//! let injector = app.injector();
//! // Use injector to resolve dependencies
//! ```

use crate::Error;
use crate::injector::Injector;
use crate::module::Module;
use crate::runtime::Shared;

#[cfg(feature = "tracing")]
use tracing::{debug, info};

#[cfg(not(feature = "thread-safe"))]
type ModuleObject = Box<dyn Module>;
#[cfg(feature = "thread-safe")]
type ModuleObject = Box<dyn Module>;

struct LoadedModule {
    module: ModuleObject,
    injector: Shared<Injector>,
}

/// The main application container for dependency injection.
///
/// `Application` manages the lifecycle of modules and provides access to the root
/// dependency injector. It handles the bootstrap process, which recursively loads
/// all modules and their imports, creating a hierarchical injector structure.
///
/// # Thread Safety
///
/// With the `thread-safe` feature enabled, the application requires modules to implement
/// `Send + Sync` to ensure they can be safely shared across threads. Without this feature,
/// modules have no additional thread-safety requirements.
///
/// # Lifecycle
///
/// 1. **Creation**: Create an application with a root module using [`new()`](Application::new)
/// 2. **Bootstrap**: Call [`bootstrap_sync()`](Application::bootstrap_sync) for sync-only modules
///    or [`bootstrap()`](Application::bootstrap) for async-capable lifecycle hooks
/// 3. **Usage**: Access the injector via [`injector()`](Application::injector) to resolve dependencies
/// 4. **Shutdown (optional)**: Call [`shutdown()`](Application::shutdown) to run `on_stop` hooks
///
/// # Examples
///
/// ```
/// use fluxdi::application::Application;
/// use fluxdi::module::Module;
/// use fluxdi::injector::Injector;
///
/// struct MyAppModule;
///
/// impl Module for MyAppModule {
///     fn providers(&self, injector: &Injector) {
///         // Configure your providers
///     }
/// }
///
/// let mut app = Application::new(MyAppModule);
/// assert!(!app.is_bootstrapped());
///
/// app.bootstrap_sync().unwrap();
/// assert!(app.is_bootstrapped());
///
/// let injector = app.injector();
/// // Use injector to get services
/// ```
pub struct Application {
    root: Option<ModuleObject>,
    injector: Shared<Injector>,
    started_modules: Vec<LoadedModule>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application")
            .field("injector", &"...")
            .field("root", &"<dyn Module>")
            .field("started_modules", &self.started_modules.len())
            .finish()
    }
}

impl Application {
    /// Creates a new application with the given root module.
    ///
    /// The application is created in an un-bootstrapped state. You must call
    /// either [`bootstrap_sync()`](Application::bootstrap_sync) or
    /// [`bootstrap()`](Application::bootstrap) to load the module and its dependencies.
    ///
    /// # Parameters
    ///
    /// - `root`: The root module that defines the application's dependency graph
    ///
    /// # Returns
    ///
    /// A new `Application` instance ready to be bootstrapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct RootModule;
    ///
    /// impl Module for RootModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let app = Application::new(RootModule);
    /// assert!(!app.is_bootstrapped());
    /// ```
    pub fn new(root: impl Module + 'static) -> Self {
        #[cfg(feature = "tracing")]
        info!("Creating new Application instance with root module");

        Self {
            root: Some(Box::new(root)),
            injector: Shared::new(Injector::root()),
            started_modules: Vec::new(),
        }
    }

    /// Bootstraps the application by loading the root module and all its imports.
    ///
    /// This method recursively processes the module hierarchy:
    /// 1. Creates child injectors for each module
    /// 2. Loads all imported modules first
    /// 3. Registers the module's own providers
    ///
    /// # Panics
    ///
    /// Panics if called more than once on the same application instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct AppModule;
    ///
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {
    ///         // Register providers
    ///     }
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// app.bootstrap_sync().unwrap();
    /// assert!(app.is_bootstrapped());
    /// ```
    ///
    /// # Panics Example
    ///
    /// ```should_panic
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// app.bootstrap_sync().unwrap();
    /// app.bootstrap_sync().unwrap(); // Panics: Application already bootstrapped
    /// ```
    pub fn bootstrap_sync(&mut self) -> Result<(), Error> {
        let root = self.root.take().expect("Application already bootstrapped");

        #[cfg(feature = "tracing")]
        info!("Starting application bootstrap process");

        Self::load_module(self.injector.clone(), root)?;

        #[cfg(feature = "tracing")]
        info!("Application bootstrap completed successfully");
        Ok(())
    }

    /// Unified bootstrap that supports module async providers and lifecycle hooks.
    ///
    /// This executes:
    /// 1. `configure()` for each module (imports first)
    /// 2. `on_start()` for each module
    pub async fn bootstrap(&mut self) -> Result<(), Error> {
        let root = self.root.take().expect("Application already bootstrapped");

        #[cfg(feature = "tracing")]
        info!("Starting async application bootstrap process");

        self.started_modules = Self::load_module_async(self.injector.clone(), root).await?;

        #[cfg(feature = "tracing")]
        info!("Async application bootstrap completed successfully");
        Ok(())
    }

    /// Backward-compatible alias for the old async bootstrap name.
    pub async fn bootstrap_async(&mut self) -> Result<(), Error> {
        self.bootstrap().await
    }

    /// Executes module `on_stop()` hooks in reverse startup order.
    pub async fn shutdown(&mut self) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        info!("Starting async application shutdown process");

        while let Some(loaded) = self.started_modules.pop() {
            let module_name = std::any::type_name_of_val(&*loaded.module);
            loaded
                .module
                .on_stop(loaded.injector.clone())
                .await
                .map_err(|err| {
                    Error::module_lifecycle_failed(module_name, "on_stop", &err.to_string())
                })?;
        }

        #[cfg(feature = "tracing")]
        info!("Async application shutdown completed successfully");
        Ok(())
    }

    /// Backward-compatible alias for the old async shutdown name.
    pub async fn shutdown_async(&mut self) -> Result<(), Error> {
        self.shutdown().await
    }

    /// Backward-compatible alias for startup.
    pub async fn start(&mut self) -> Result<(), Error> {
        self.bootstrap().await
    }

    /// Backward-compatible alias for shutdown.
    pub async fn stop(&mut self) -> Result<(), Error> {
        self.shutdown().await
    }

    /// Returns a shared reference to the root injector.
    ///
    /// The injector can be used to resolve dependencies after the application
    /// has been bootstrapped. The returned reference can be cloned to share
    /// access to the injector.
    ///
    /// # Returns
    ///
    /// A shared reference to the root [`Injector`].
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// app.bootstrap_sync().unwrap();
    ///
    /// let injector = app.injector();
    /// let another_ref = app.injector();
    /// // Both references point to the same injector
    /// ```
    pub fn injector(&self) -> Shared<Injector> {
        #[cfg(feature = "tracing")]
        debug!("Accessing root injector");

        #[cfg(feature = "tracing")]
        {
            if self.is_bootstrapped() {
                debug!("Injector is available and application is bootstrapped");
            } else {
                debug!("Injector is available but application is not bootstrapped yet");
            }
        }

        self.injector.clone()
    }

    /// Checks whether the application has been bootstrapped.
    ///
    /// Returns `true` if bootstrap has been called through either
    /// [`bootstrap_sync()`](Application::bootstrap_sync) or [`bootstrap()`](Application::bootstrap),
    /// `false` otherwise.
    ///
    /// # Returns
    ///
    /// - `true` if the application is bootstrapped
    /// - `false` if the application has not been bootstrapped yet
    ///
    /// # Examples
    ///
    /// ```
    /// use fluxdi::application::Application;
    /// use fluxdi::module::Module;
    /// use fluxdi::injector::Injector;
    ///
    /// struct AppModule;
    /// impl Module for AppModule {
    ///     fn providers(&self, injector: &Injector) {}
    /// }
    ///
    /// let mut app = Application::new(AppModule);
    /// assert!(!app.is_bootstrapped());
    ///
    /// app.bootstrap_sync().unwrap();
    /// assert!(app.is_bootstrapped());
    /// ```
    pub fn is_bootstrapped(&self) -> bool {
        let bootstrapped = self.root.is_none();

        #[cfg(feature = "tracing")]
        debug!("Checking application bootstrap state: {}", bootstrapped);

        bootstrapped
    }

    /// Recursively loads a module and its imports into the injector hierarchy.
    ///
    /// Creates a child injector for the module, loads all imported modules first,
    /// then registers the module's own providers. This ensures proper dependency
    /// resolution order.
    ///
    /// # Parameters
    ///
    /// - `parent`: The parent injector to create a child from
    /// - `module`: The module to load
    fn load_module(parent: Shared<Injector>, module: ModuleObject) -> Result<(), Error> {
        #[cfg(feature = "tracing")]
        debug!("Loading module into injector hierarchy");

        let module_injector = Shared::new(Injector::child(parent.clone()));

        #[cfg(feature = "tracing")]
        debug!("Created child injector for module");

        let imports = module.imports();
        #[cfg(feature = "tracing")]
        if !imports.is_empty() {
            debug!("Module has {} imports, loading them first", imports.len());
        }

        #[allow(unused_variables)]
        for (index, import) in imports.into_iter().enumerate() {
            #[cfg(feature = "tracing")]
            debug!("Loading import {}", index + 1);

            Self::load_module(module_injector.clone(), import)?;
        }

        #[cfg(feature = "tracing")]
        debug!("Registering module providers");

        let module_name = std::any::type_name_of_val(&*module);
        module.configure(&module_injector).map_err(|err| {
            Error::module_lifecycle_failed(module_name, "configure", &err.to_string())
        })?;

        #[cfg(feature = "tracing")]
        debug!("Module loaded successfully");
        Ok(())
    }

    async fn load_module_async(
        parent: Shared<Injector>,
        module: ModuleObject,
    ) -> Result<Vec<LoadedModule>, Error> {
        enum Frame {
            Enter {
                parent: Shared<Injector>,
                module: ModuleObject,
            },
            Exit {
                module_injector: Shared<Injector>,
                module: ModuleObject,
            },
        }

        let mut stack = vec![Frame::Enter { parent, module }];
        let mut loaded = Vec::new();

        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter { parent, module } => {
                    let module_injector = Shared::new(Injector::child(parent));
                    let imports = module.imports();

                    stack.push(Frame::Exit {
                        module_injector: module_injector.clone(),
                        module,
                    });

                    for import in imports.into_iter().rev() {
                        stack.push(Frame::Enter {
                            parent: module_injector.clone(),
                            module: import,
                        });
                    }
                }
                Frame::Exit {
                    module_injector,
                    module,
                } => {
                    let module_name = std::any::type_name_of_val(&*module);
                    module.configure(&module_injector).map_err(|err| {
                        Error::module_lifecycle_failed(module_name, "configure", &err.to_string())
                    })?;

                    module
                        .on_start(module_injector.clone())
                        .await
                        .map_err(|err| {
                            Error::module_lifecycle_failed(
                                module_name,
                                "on_start",
                                &err.to_string(),
                            )
                        })?;

                    loaded.push(LoadedModule {
                        module,
                        injector: module_injector,
                    });
                }
            }
        }

        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::ModuleLifecycleFuture;
    use crate::{Error, ErrorKind};
    use futures::executor::block_on;

    #[cfg(not(feature = "thread-safe"))]
    use std::cell::RefCell;
    #[cfg(not(feature = "thread-safe"))]
    use std::rc::Rc;

    #[cfg(feature = "thread-safe")]
    use std::sync::{Arc, Mutex};

    struct EmptyModule;

    impl Module for EmptyModule {
        fn providers(&self, _injector: &Injector) {}
    }

    // CountingModule with conditional thread safety
    #[cfg(not(feature = "thread-safe"))]
    struct CountingModule {
        counter: Rc<RefCell<usize>>,
    }

    #[cfg(not(feature = "thread-safe"))]
    impl Module for CountingModule {
        fn providers(&self, _injector: &Injector) {
            *self.counter.borrow_mut() += 1;
        }
    }

    #[cfg(feature = "thread-safe")]
    struct CountingModule {
        counter: Arc<Mutex<usize>>,
    }

    #[cfg(feature = "thread-safe")]
    impl Module for CountingModule {
        fn providers(&self, _injector: &Injector) {
            *self.counter.lock().unwrap() += 1;
        }
    }

    // ModuleWithImports with conditional thread safety
    #[cfg(not(feature = "thread-safe"))]
    struct ModuleWithImports {
        counter: Rc<RefCell<usize>>,
    }

    #[cfg(not(feature = "thread-safe"))]
    impl Module for ModuleWithImports {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
            ]
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.borrow_mut() += 1;
        }
    }

    #[cfg(feature = "thread-safe")]
    struct ModuleWithImports {
        counter: Arc<Mutex<usize>>,
    }

    #[cfg(feature = "thread-safe")]
    impl Module for ModuleWithImports {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
                Box::new(CountingModule {
                    counter: self.counter.clone(),
                }),
            ]
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.lock().unwrap() += 1;
        }
    }

    #[cfg(not(feature = "thread-safe"))]
    type EventLog = Rc<RefCell<Vec<String>>>;
    #[cfg(feature = "thread-safe")]
    type EventLog = Arc<Mutex<Vec<String>>>;

    #[cfg(not(feature = "thread-safe"))]
    fn push_event(log: &EventLog, event: String) {
        log.borrow_mut().push(event);
    }

    #[cfg(feature = "thread-safe")]
    fn push_event(log: &EventLog, event: String) {
        log.lock().unwrap().push(event);
    }

    #[cfg(not(feature = "thread-safe"))]
    fn event_snapshot(log: &EventLog) -> Vec<String> {
        log.borrow().clone()
    }

    #[cfg(feature = "thread-safe")]
    fn event_snapshot(log: &EventLog) -> Vec<String> {
        log.lock().unwrap().clone()
    }

    struct LifecycleModule {
        name: &'static str,
        log: EventLog,
        import_child: bool,
    }

    impl Module for LifecycleModule {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            if !self.import_child {
                return vec![];
            }

            vec![Box::new(LifecycleModule {
                name: "import",
                log: self.log.clone(),
                import_child: false,
            })]
        }

        fn providers(&self, _injector: &Injector) {
            push_event(&self.log, format!("providers:{}", self.name));
        }

        fn on_start(&self, _injector: Shared<Injector>) -> ModuleLifecycleFuture {
            let log = self.log.clone();
            let name = self.name.to_string();
            Box::pin(async move {
                push_event(&log, format!("on_start:{}", name));
                Ok(())
            })
        }

        fn on_stop(&self, _injector: Shared<Injector>) -> ModuleLifecycleFuture {
            let log = self.log.clone();
            let name = self.name.to_string();
            Box::pin(async move {
                push_event(&log, format!("on_stop:{}", name));
                Ok(())
            })
        }
    }

    struct FailingLifecycleModule;

    impl Module for FailingLifecycleModule {
        fn on_start(&self, _injector: Shared<Injector>) -> ModuleLifecycleFuture {
            Box::pin(async {
                Err(Error::module_lifecycle_failed(
                    "FailingLifecycleModule",
                    "on_start",
                    "intentional test failure",
                ))
            })
        }
    }

    #[test]
    fn test_new_creates_unbootstrapped_application() {
        let app = Application::new(EmptyModule);
        assert!(
            !app.is_bootstrapped(),
            "New application should not be bootstrapped"
        );
    }

    #[test]
    fn test_bootstrap_changes_state() {
        let mut app = Application::new(EmptyModule);
        assert!(!app.is_bootstrapped());

        app.bootstrap_sync().unwrap();
        assert!(
            app.is_bootstrapped(),
            "Application should be bootstrapped after bootstrap()"
        );
    }

    #[test]
    #[should_panic(expected = "Application already bootstrapped")]
    fn test_bootstrap_twice_panics() {
        let mut app = Application::new(EmptyModule);
        app.bootstrap_sync().unwrap();
        app.bootstrap_sync().unwrap(); // Should panic
    }

    #[test]
    fn test_injector_returns_shared_reference() {
        let mut app = Application::new(EmptyModule);
        app.bootstrap_sync().unwrap();

        let injector1 = app.injector();
        let _injector2 = app.injector();

        // Both should reference the same underlying injector
        #[cfg(feature = "thread-safe")]
        assert_eq!(std::sync::Arc::strong_count(&injector1), 3); // app + injector1 + injector2

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(std::rc::Rc::strong_count(&injector1), 3); // app + injector1 + injector2
    }

    #[test]
    fn test_bootstrap_calls_module_providers() {
        #[cfg(not(feature = "thread-safe"))]
        let counter = Rc::new(RefCell::new(0));
        #[cfg(feature = "thread-safe")]
        let counter = Arc::new(Mutex::new(0));

        let module = CountingModule {
            counter: counter.clone(),
        };

        let mut app = Application::new(module);

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(*counter.borrow(), 0);
        #[cfg(feature = "thread-safe")]
        assert_eq!(*counter.lock().unwrap(), 0);

        app.bootstrap_sync().unwrap();

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(
            *counter.borrow(),
            1,
            "Module providers should be called during bootstrap"
        );
        #[cfg(feature = "thread-safe")]
        assert_eq!(
            *counter.lock().unwrap(),
            1,
            "Module providers should be called during bootstrap"
        );
    }

    #[test]
    fn test_bootstrap_loads_imports_first() {
        #[cfg(not(feature = "thread-safe"))]
        let counter = Rc::new(RefCell::new(0));
        #[cfg(feature = "thread-safe")]
        let counter = Arc::new(Mutex::new(0));

        let module = ModuleWithImports {
            counter: counter.clone(),
        };

        let mut app = Application::new(module);
        app.bootstrap_sync().unwrap();

        // 2 imports + 1 root module = 3 calls
        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(*counter.borrow(), 3, "All modules should be loaded");
        #[cfg(feature = "thread-safe")]
        assert_eq!(*counter.lock().unwrap(), 3, "All modules should be loaded");
    }

    #[test]
    fn test_bootstrap_async_runs_lifecycle_hooks_and_shutdown_reverse_order() {
        #[cfg(not(feature = "thread-safe"))]
        let log: EventLog = Rc::new(RefCell::new(Vec::new()));
        #[cfg(feature = "thread-safe")]
        let log: EventLog = Arc::new(Mutex::new(Vec::new()));

        let root = LifecycleModule {
            name: "root",
            log: log.clone(),
            import_child: true,
        };

        let mut app = Application::new(root);
        block_on(app.bootstrap_async()).unwrap();

        assert_eq!(
            event_snapshot(&log),
            vec![
                "providers:import".to_string(),
                "on_start:import".to_string(),
                "providers:root".to_string(),
                "on_start:root".to_string(),
            ]
        );

        block_on(app.shutdown_async()).unwrap();
        assert_eq!(
            event_snapshot(&log),
            vec![
                "providers:import".to_string(),
                "on_start:import".to_string(),
                "providers:root".to_string(),
                "on_start:root".to_string(),
                "on_stop:root".to_string(),
                "on_stop:import".to_string(),
            ]
        );
    }

    #[test]
    fn test_bootstrap_async_propagates_lifecycle_errors() {
        let mut app = Application::new(FailingLifecycleModule);
        let error = block_on(app.bootstrap_async()).unwrap_err();
        assert_eq!(error.kind, ErrorKind::ModuleLifecycleFailed);
        assert!(error.message.contains("on_start"));
    }

    #[test]
    fn test_start_stop_unified_api() {
        #[cfg(not(feature = "thread-safe"))]
        let log: EventLog = Rc::new(RefCell::new(Vec::new()));
        #[cfg(feature = "thread-safe")]
        let log: EventLog = Arc::new(Mutex::new(Vec::new()));

        let root = LifecycleModule {
            name: "root",
            log: log.clone(),
            import_child: false,
        };

        let mut app = Application::new(root);
        block_on(app.start()).unwrap();
        block_on(app.stop()).unwrap();

        assert_eq!(
            event_snapshot(&log),
            vec![
                "providers:root".to_string(),
                "on_start:root".to_string(),
                "on_stop:root".to_string(),
            ]
        );
    }

    #[test]
    fn test_application_can_be_created_with_different_modules() {
        let _app1 = Application::new(EmptyModule);

        #[cfg(not(feature = "thread-safe"))]
        let _app2 = Application::new(CountingModule {
            counter: Rc::new(RefCell::new(0)),
        });
        #[cfg(feature = "thread-safe")]
        let _app2 = Application::new(CountingModule {
            counter: Arc::new(Mutex::new(0)),
        });

        // Should compile and work with different module types
    }

    #[test]
    fn test_injector_accessible_before_bootstrap() {
        let app = Application::new(EmptyModule);
        let _injector = app.injector();
        // Should not panic - injector is available even before bootstrap
    }

    #[test]
    fn test_multiple_injector_clones() {
        let mut app = Application::new(EmptyModule);
        app.bootstrap_sync().unwrap();

        let injectors: Vec<_> = (0..5).map(|_| app.injector()).collect();
        assert_eq!(injectors.len(), 5);

        #[cfg(feature = "thread-safe")]
        assert_eq!(std::sync::Arc::strong_count(&injectors[0]), 6); // app + 5 in vec

        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(std::rc::Rc::strong_count(&injectors[0]), 6); // app + 5 in vec
    }

    #[cfg(feature = "debug")]
    #[test]
    fn test_debug_implementation() {
        let app = Application::new(EmptyModule);
        let debug_str = format!("{:?}", app);
        assert!(
            debug_str.contains("Application"),
            "Debug output should contain 'Application'"
        );
    }

    // NestedImportModule with conditional thread safety
    #[cfg(not(feature = "thread-safe"))]
    struct NestedImportModule {
        counter: Rc<RefCell<usize>>,
        depth: usize,
    }

    #[cfg(not(feature = "thread-safe"))]
    impl Module for NestedImportModule {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            if self.depth > 0 {
                vec![Box::new(NestedImportModule {
                    counter: self.counter.clone(),
                    depth: self.depth - 1,
                })]
            } else {
                vec![]
            }
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.borrow_mut() += 1;
        }
    }

    #[cfg(feature = "thread-safe")]
    struct NestedImportModule {
        counter: Arc<Mutex<usize>>,
        depth: usize,
    }

    #[cfg(feature = "thread-safe")]
    impl Module for NestedImportModule {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            if self.depth > 0 {
                vec![Box::new(NestedImportModule {
                    counter: self.counter.clone(),
                    depth: self.depth - 1,
                })]
            } else {
                vec![]
            }
        }

        fn providers(&self, _injector: &Injector) {
            *self.counter.lock().unwrap() += 1;
        }
    }

    #[test]
    fn test_deeply_nested_modules() {
        #[cfg(not(feature = "thread-safe"))]
        let counter = Rc::new(RefCell::new(0));
        #[cfg(feature = "thread-safe")]
        let counter = Arc::new(Mutex::new(0));

        let module = NestedImportModule {
            counter: counter.clone(),
            depth: 5,
        };

        let mut app = Application::new(module);
        app.bootstrap_sync().unwrap();

        // depth 5, 4, 3, 2, 1, 0 = 6 modules total
        #[cfg(not(feature = "thread-safe"))]
        assert_eq!(*counter.borrow(), 6, "All nested modules should be loaded");
        #[cfg(feature = "thread-safe")]
        assert_eq!(
            *counter.lock().unwrap(),
            6,
            "All nested modules should be loaded"
        );
    }
}
