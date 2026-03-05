use super::*;

impl Application {
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
}
