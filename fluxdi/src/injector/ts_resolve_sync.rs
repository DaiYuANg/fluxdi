use super::*;

impl Injector {
    pub fn try_resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_RESOLVE, type_name = type_name).entered();

        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_attempt();
        #[cfg(feature = "metrics")]
        let resolve_started = std::time::Instant::now();

        let result = (|| {
            let _guard = ResolveGuard::push(type_id)?;

            #[cfg(feature = "tracing")]
            trace!("Resolving service");

            if let Some(instance) = self.get_instance::<T>() {
                #[cfg(feature = "tracing")]
                trace!("Resolve cache hit");
                #[cfg(feature = "metrics")]
                self.inner.metrics.record_resolve_cache_hit();
                return Ok(instance.value());
            }

            #[cfg(feature = "tracing")]
            trace!("Resolve cache miss, creating instance");
            #[cfg(feature = "metrics")]
            self.inner.metrics.record_resolve_cache_miss();

            let provider = self.resolve_provider::<T>()?;

            let instance = self.resolve_instance::<T>()?;

            if provider.scope == Scope::Transient {
                #[cfg(feature = "tracing")]
                trace!("Resolved transient service");
                return Ok(instance.value());
            }

            if let Some(target) = self.cache_target_for_scope(provider.scope) {
                target.store_instance::<T>(instance.clone());
            }

            #[cfg(feature = "tracing")]
            trace!(scope = %provider.scope, "Resolved and cached service");

            Ok(instance.value())
        })();

        #[cfg(feature = "metrics")]
        match &result {
            Ok(_) => self
                .inner
                .metrics
                .record_resolve_success(resolve_started.elapsed()),
            Err(_) => self
                .inner
                .metrics
                .record_resolve_failure(resolve_started.elapsed()),
        }

        result
    }

    pub fn try_resolve_all<T>(&self) -> Result<Vec<Shared<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_RESOLVE, type_name = type_name, op = "resolve_all").entered();

        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_attempt();
        #[cfg(feature = "metrics")]
        let resolve_started = std::time::Instant::now();

        let result = (|| {
            let _guard = ResolveGuard::push(type_id)?;

            #[cfg(feature = "tracing")]
            trace!("Resolving service set");

            let providers = self.resolve_set_providers::<T>()?;
            let mut values = Vec::with_capacity(providers.len());

            for provider in providers {
                let cache_target = self.cache_target_for_scope(provider.scope);

                if let Some(target) = &cache_target {
                    if let Some(instance) = target.get_set_instance::<T>(&provider) {
                        #[cfg(feature = "metrics")]
                        self.inner.metrics.record_resolve_cache_hit();
                        values.push(instance.value());
                        continue;
                    }

                    #[cfg(feature = "metrics")]
                    self.inner.metrics.record_resolve_cache_miss();
                }

                let instance = self.resolve_instance_from_provider::<T>(&provider)?;

                if let Some(target) = cache_target {
                    target.store_set_instance::<T>(&provider, instance.clone());
                }

                values.push(instance.value());
            }

            Ok(values)
        })();

        #[cfg(feature = "metrics")]
        match &result {
            Ok(_) => self
                .inner
                .metrics
                .record_resolve_success(resolve_started.elapsed()),
            Err(_) => self
                .inner
                .metrics
                .record_resolve_failure(resolve_started.elapsed()),
        }

        result
    }

    pub fn try_resolve_named<T>(&self, name: &str) -> Result<Shared<T>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_RESOLVE, type_name = type_name, name = %name).entered();

        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_attempt();
        #[cfg(feature = "metrics")]
        let resolve_started = std::time::Instant::now();

        let result = (|| {
            let _guard = ResolveGuard::push(type_id)?;

            #[cfg(feature = "tracing")]
            trace!("Resolving named service");

            if let Some(instance) = self.get_instance_named::<T>(name) {
                #[cfg(feature = "tracing")]
                trace!("Named resolve cache hit");
                #[cfg(feature = "metrics")]
                self.inner.metrics.record_resolve_cache_hit();
                return Ok(instance.value());
            }

            #[cfg(feature = "tracing")]
            trace!("Named resolve cache miss, creating instance");
            #[cfg(feature = "metrics")]
            self.inner.metrics.record_resolve_cache_miss();

            let provider = self.resolve_provider_named::<T>(name)?;
            let instance = self.resolve_instance_named::<T>(name)?;

            if provider.scope == Scope::Transient {
                #[cfg(feature = "tracing")]
                trace!("Resolved named transient service");
                return Ok(instance.value());
            }

            if let Some(target) = self.cache_target_for_scope(provider.scope) {
                target.store_instance_named::<T>(name, instance.clone());
            }

            #[cfg(feature = "tracing")]
            trace!(scope = %provider.scope, "Resolved and cached named service");

            Ok(instance.value())
        })();

        #[cfg(feature = "metrics")]
        match &result {
            Ok(_) => self
                .inner
                .metrics
                .record_resolve_success(resolve_started.elapsed()),
            Err(_) => self
                .inner
                .metrics
                .record_resolve_failure(resolve_started.elapsed()),
        }

        result
    }

    pub fn resolve<T>(&self) -> Shared<T>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve::<T>().unwrap()
    }

    pub fn resolve_all<T>(&self) -> Vec<Shared<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve_all::<T>().unwrap()
    }

    pub fn resolve_named<T>(&self, name: &str) -> Shared<T>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve_named::<T>(name).unwrap()
    }

    pub fn optional_resolve<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve::<T>().ok()
    }

    pub fn optional_resolve_all<T>(&self) -> Option<Vec<Shared<T>>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve_all::<T>().ok()
    }

    pub fn optional_resolve_named<T>(&self, name: &str) -> Option<Shared<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve_named::<T>(name).ok()
    }
}
