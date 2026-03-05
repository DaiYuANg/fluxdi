use super::*;

impl Injector {
    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_async<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
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

        let result = async {
            let _guard = ResolveGuard::push(type_id)?;

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                op = "resolve_async",
                stage = "start",
                "Starting async resolve flow"
            );

            if let Some(instance) = self.get_instance::<T>() {
                #[cfg(feature = "tracing")]
                trace!(
                    type_name = type_name,
                    op = "resolve_async",
                    cache = "hit",
                    "Resolve cache hit"
                );
                #[cfg(feature = "metrics")]
                self.inner.metrics.record_resolve_cache_hit();
                return Ok(instance.value());
            }

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                op = "resolve_async",
                cache = "miss",
                stage = "create_instance",
                "Resolve cache miss, creating instance asynchronously"
            );
            #[cfg(feature = "metrics")]
            self.inner.metrics.record_resolve_cache_miss();

            let provider = self.resolve_provider::<T>()?;
            let instance = self.resolve_instance_async::<T>().await?;

            if provider.scope == Scope::Transient {
                #[cfg(feature = "tracing")]
                trace!(
                    type_name = type_name,
                    op = "resolve_async",
                    scope = %provider.scope,
                    cached = false,
                    "Resolved transient service"
                );
                return Ok(instance.value());
            }

            if let Some(target) = self.cache_target_for_scope(provider.scope) {
                target.store_instance::<T>(instance.clone());
            }

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                op = "resolve_async",
                scope = %provider.scope,
                cached = true,
                "Resolved and cached service"
            );

            Ok(instance.value())
        }
        .await;

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

    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_all_async<T>(&self) -> Result<Vec<Shared<T>>, Error>
    where
        T: ?Sized + 'static,
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

        let result = async {
            let _guard = ResolveGuard::push(type_id)?;

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                op = "resolve_all_async",
                stage = "start",
                "Resolving service set asynchronously"
            );

            let providers = self.resolve_set_providers::<T>()?;
            let mut values = Vec::with_capacity(providers.len());

            for (index, provider) in providers.into_iter().enumerate() {
                let cache_target = self.cache_target_for_scope(provider.scope);

                if let Some(target) = &cache_target {
                    if let Some(instance) = target.get_set_instance::<T>(&provider) {
                        #[cfg(feature = "tracing")]
                        trace!(
                            type_name = type_name,
                            op = "resolve_all_async",
                            index = index,
                            scope = %provider.scope,
                            cache = "hit",
                            "Resolved async set item from cache"
                        );
                        #[cfg(feature = "metrics")]
                        self.inner.metrics.record_resolve_cache_hit();
                        values.push(instance.value());
                        continue;
                    }

                    #[cfg(feature = "tracing")]
                    trace!(
                        type_name = type_name,
                        op = "resolve_all_async",
                        index = index,
                        scope = %provider.scope,
                        cache = "miss",
                        "Async set item cache miss, creating instance"
                    );
                    #[cfg(feature = "metrics")]
                    self.inner.metrics.record_resolve_cache_miss();
                }

                let instance = self
                    .resolve_instance_from_provider_async::<T>(&provider)
                    .await?;

                if let Some(target) = cache_target {
                    target.store_set_instance::<T>(&provider, instance.clone());
                }

                values.push(instance.value());
            }

            Ok(values)
        }
        .await;

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

    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_named_async<T>(&self, name: &str) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
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

        let result = async {
            let _guard = ResolveGuard::push(type_id)?;

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                name = %name,
                op = "resolve_named_async",
                stage = "start",
                "Resolving named service asynchronously"
            );

            if let Some(instance) = self.get_instance_named::<T>(name) {
                #[cfg(feature = "tracing")]
                trace!(
                    type_name = type_name,
                    name = %name,
                    op = "resolve_named_async",
                    cache = "hit",
                    "Named resolve cache hit"
                );
                #[cfg(feature = "metrics")]
                self.inner.metrics.record_resolve_cache_hit();
                return Ok(instance.value());
            }

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                name = %name,
                op = "resolve_named_async",
                cache = "miss",
                stage = "create_instance",
                "Named resolve cache miss, creating instance asynchronously"
            );
            #[cfg(feature = "metrics")]
            self.inner.metrics.record_resolve_cache_miss();

            let provider = self.resolve_provider_named::<T>(name)?;
            let instance = self.resolve_instance_named_async::<T>(name).await?;

            if provider.scope == Scope::Transient {
                #[cfg(feature = "tracing")]
                trace!(
                    type_name = type_name,
                    name = %name,
                    op = "resolve_named_async",
                    scope = %provider.scope,
                    cached = false,
                    "Resolved named transient service"
                );
                return Ok(instance.value());
            }

            if let Some(target) = self.cache_target_for_scope(provider.scope) {
                target.store_instance_named::<T>(name, instance.clone());
            }

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                name = %name,
                op = "resolve_named_async",
                scope = %provider.scope,
                cached = true,
                "Resolved and cached named service"
            );

            Ok(instance.value())
        }
        .await;

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

    #[cfg(feature = "async-factory")]
    pub async fn resolve_async<T>(&self) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_async::<T>().await.unwrap()
    }

    #[cfg(feature = "async-factory")]
    pub async fn resolve_all_async<T>(&self) -> Vec<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_all_async::<T>().await.unwrap()
    }

    #[cfg(feature = "async-factory")]
    pub async fn resolve_named_async<T>(&self, name: &str) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_named_async::<T>(name).await.unwrap()
    }

    #[cfg(feature = "async-factory")]
    pub async fn optional_resolve_async<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_async::<T>().await.ok()
    }

    #[cfg(feature = "async-factory")]
    pub async fn optional_resolve_all_async<T>(&self) -> Option<Vec<Shared<T>>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_all_async::<T>().await.ok()
    }

    #[cfg(feature = "async-factory")]
    pub async fn optional_resolve_named_async<T>(&self, name: &str) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_named_async::<T>(name).await.ok()
    }
}
