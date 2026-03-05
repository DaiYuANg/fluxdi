use std::any::{Any, TypeId};
#[cfg(not(all(feature = "thread-safe", feature = "lock-free")))]
use std::collections::HashMap;

#[cfg(all(feature = "thread-safe", feature = "lock-free"))]
use dashmap::{DashMap, mapref::entry::Entry as DashEntry};

use crate::error::Error;
use crate::instance::Instance;
#[cfg(feature = "metrics")]
use crate::observability::{MetricsSnapshot, MetricsState};
#[cfg(feature = "tracing")]
use crate::observability::{SPAN_FACTORY_EXECUTE, SPAN_PROVIDE, SPAN_RESOLVE};
use crate::provider::Provider;
use crate::resolve_guard::ResolveGuard;
use crate::runtime::Shared;
#[cfg(not(all(feature = "thread-safe", feature = "lock-free")))]
use crate::runtime::Store;
use crate::scope::Scope;

#[cfg(feature = "tracing")]
use tracing::{debug, info_span, trace};

pub struct Injector {
    inner: Shared<InjectorInner>,
}

struct InjectorInner {
    pub(crate) parent: Option<Shared<InjectorInner>>,

    #[cfg(not(feature = "thread-safe"))]
    pub(crate) providers: Store<HashMap<TypeId, Shared<dyn Any>>>,
    #[cfg(not(feature = "thread-safe"))]
    pub(crate) instances: Store<HashMap<TypeId, Shared<dyn Any>>>,

    #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
    pub(crate) providers: Store<HashMap<TypeId, Shared<dyn Any + Send + Sync>>>,
    #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
    pub(crate) instances: Store<HashMap<TypeId, Shared<dyn Any + Send + Sync>>>,

    #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
    pub(crate) providers: DashMap<TypeId, Shared<dyn Any + Send + Sync>>,
    #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
    pub(crate) instances: DashMap<TypeId, Shared<dyn Any + Send + Sync>>,

    #[cfg(feature = "metrics")]
    pub(crate) metrics: Shared<MetricsState>,
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for InjectorInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("InjectorInner");
        ds.field("parent", &self.parent.is_some());
        ds.field("providers", &self.providers);
        ds.field("instances", &self.instances);
        #[cfg(feature = "metrics")]
        ds.field("metrics", &self.metrics.snapshot());
        ds.finish()
    }
}

#[cfg(feature = "debug")]
impl std::fmt::Debug for Injector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("inner", &self.inner)
            .finish()
    }
}

impl Clone for Injector {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Injector {
    pub fn root() -> Self {
        Self {
            inner: Shared::new(InjectorInner {
                parent: None,
                #[cfg(not(feature = "thread-safe"))]
                providers: Store::new(HashMap::new()),
                #[cfg(not(feature = "thread-safe"))]
                instances: Store::new(HashMap::new()),
                #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
                providers: Store::new(HashMap::new()),
                #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
                instances: Store::new(HashMap::new()),
                #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
                providers: DashMap::new(),
                #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
                instances: DashMap::new(),
                #[cfg(feature = "metrics")]
                metrics: Shared::new(MetricsState::default()),
            }),
        }
    }

    pub fn child(parent: Shared<Injector>) -> Self {
        Self {
            inner: Shared::new(InjectorInner {
                parent: Some(parent.inner.clone()),
                #[cfg(not(feature = "thread-safe"))]
                providers: Store::new(HashMap::new()),
                #[cfg(not(feature = "thread-safe"))]
                instances: Store::new(HashMap::new()),
                #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
                providers: Store::new(HashMap::new()),
                #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
                instances: Store::new(HashMap::new()),
                #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
                providers: DashMap::new(),
                #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
                instances: DashMap::new(),
                #[cfg(feature = "metrics")]
                metrics: parent.inner.metrics.clone(),
            }),
        }
    }

    pub(crate) fn root_injector(&self) -> Injector {
        let mut current = self.clone();

        while let Some(parent) = &current.inner.parent {
            current = Injector {
                inner: parent.clone(),
            };
        }

        current
    }

    #[cfg(feature = "metrics")]
    pub fn metrics_snapshot(&self) -> MetricsSnapshot {
        self.inner.metrics.snapshot()
    }

    #[cfg(feature = "prometheus")]
    pub fn prometheus_metrics(&self) -> String {
        self.inner.metrics.to_prometheus()
    }
}

#[cfg(all(test, feature = "async-factory"))]
mod tests {
    use super::*;
    use crate::{ErrorKind, Provider, Shared};

    use futures::executor::block_on;
    #[cfg(feature = "thread-safe")]
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };
    #[cfg(feature = "thread-safe")]
    use std::thread;
    #[cfg(feature = "thread-safe")]
    use std::time::Duration;

    #[test]
    fn sync_resolve_rejects_async_provider() {
        let injector = Injector::root();
        injector.provide::<String>(Provider::transient_async(|_| async {
            Shared::new("hello".to_string())
        }));

        let err = injector.try_resolve::<String>().unwrap_err();
        assert_eq!(err.kind, ErrorKind::AsyncFactoryRequiresAsyncResolve);
    }

    #[test]
    fn async_resolve_handles_async_provider() {
        let injector = Injector::root();
        injector.provide::<String>(Provider::transient_async(|_| async {
            Shared::new("async".to_string())
        }));

        let value = block_on(injector.try_resolve_async::<String>()).unwrap();
        assert_eq!(value.as_str(), "async");
    }

    #[test]
    fn async_resolve_handles_sync_provider() {
        let injector = Injector::root();
        injector.provide::<u32>(Provider::transient(|_| Shared::new(42u32)));

        let value = block_on(injector.try_resolve_async::<u32>()).unwrap();
        assert_eq!(*value, 42);
    }

    #[test]
    fn async_root_provider_is_cached() {
        let injector = Injector::root();
        injector.provide::<String>(Provider::root_async(|_| async {
            Shared::new("cached".to_string())
        }));

        let first = block_on(injector.try_resolve_async::<String>()).unwrap();
        let second = block_on(injector.try_resolve_async::<String>()).unwrap();
        assert!(Shared::ptr_eq(&first, &second));
    }

    #[test]
    fn async_transient_provider_is_not_cached() {
        let injector = Injector::root();
        injector.provide::<String>(Provider::transient_async(|_| async {
            Shared::new("new".to_string())
        }));

        let first = block_on(injector.try_resolve_async::<String>()).unwrap();
        let second = block_on(injector.try_resolve_async::<String>()).unwrap();
        assert!(!Shared::ptr_eq(&first, &second));
    }

    #[test]
    fn optional_resolve_async_returns_none_for_missing_service() {
        let injector = Injector::root();
        let value = block_on(injector.optional_resolve_async::<String>());
        assert!(value.is_none());
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn concurrent_async_transient_resolve_returns_unique_values() {
        let injector = Arc::new(Injector::root());
        let counter = Arc::new(AtomicUsize::new(0));

        injector.provide::<usize>(Provider::transient_async({
            let counter = Arc::clone(&counter);
            move |_| {
                let counter = Arc::clone(&counter);
                async move { Shared::new(counter.fetch_add(1, Ordering::SeqCst)) }
            }
        }));

        let workers = 8;
        let barrier = Arc::new(Barrier::new(workers));
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let injector = Arc::clone(&injector);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    *block_on(injector.try_resolve_async::<usize>()).unwrap()
                })
            })
            .collect();

        let mut values: Vec<usize> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        values.sort_unstable();
        values.dedup();
        assert_eq!(values.len(), workers);
    }

    #[cfg(feature = "thread-safe")]
    #[test]
    fn concurrent_async_root_resolve_completes_and_caches() {
        let injector = Arc::new(Injector::root());
        let creations = Arc::new(AtomicUsize::new(0));

        injector.provide::<usize>(Provider::root_async({
            let creations = Arc::clone(&creations);
            move |_| {
                let creations = Arc::clone(&creations);
                async move {
                    creations.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(5));
                    Shared::new(7usize)
                }
            }
        }));

        let workers = 8;
        let barrier = Arc::new(Barrier::new(workers));
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let injector = Arc::clone(&injector);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    *block_on(injector.try_resolve_async::<usize>()).unwrap()
                })
            })
            .collect();

        for value in handles.into_iter().map(|h| h.join().unwrap()) {
            assert_eq!(value, 7);
        }

        let created = creations.load(Ordering::SeqCst);
        assert!(created >= 1);
        assert!(created <= workers);

        let first = block_on(injector.try_resolve_async::<usize>()).unwrap();
        let second = block_on(injector.try_resolve_async::<usize>()).unwrap();
        assert!(Shared::ptr_eq(&first, &second));
    }
}

#[cfg(not(feature = "thread-safe"))]
impl Injector {
    pub fn try_provide<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let scope = provider.scope;

        #[cfg(feature = "metrics")]
        self.inner.metrics.record_provide_attempt();

        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();

        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_PROVIDE, type_name = type_name, scope = %scope).entered();

        let result = match scope {
            Scope::Root => {
                let root = self.root_injector();
                root.store_provider::<T>(provider)
            }

            Scope::Module | Scope::Transient => self.store_provider::<T>(provider),
        };

        #[cfg(feature = "tracing")]
        match &result {
            Ok(_) => debug!("Provider registered"),
            Err(error) => debug!(error = %error, "Provider registration failed"),
        }

        #[cfg(feature = "metrics")]
        match &result {
            Ok(_) => self.inner.metrics.record_provide_success(),
            Err(_) => self.inner.metrics.record_provide_failure(),
        }

        result
    }

    pub fn provide<T>(&self, provider: Provider<T>) -> &Self
    where
        T: ?Sized + 'static,
    {
        self.try_provide::<T>(provider).unwrap();
        self
    }

    pub(crate) fn get_provider<T>(&self) -> Option<Shared<dyn Any>>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        let local = self.inner.providers.borrow().get(&type_id).cloned();

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_provider::<T>();
        }

        None
    }

    pub(crate) fn resolve_provider<T>(&self) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider::<T>()
            .ok_or_else(|| Error::service_not_provided(type_name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        Ok(provider)
    }

    pub(crate) fn resolve_instance<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_FACTORY_EXECUTE, type_name = type_name).entered();

        let provider_ref = self.resolve_provider::<T>()?;
        let permit = provider_ref.acquire_creation_permit(type_name)?;

        #[cfg(feature = "async-factory")]
        if provider_ref.async_factory.is_some() {
            drop(permit);
            return Err(Error::async_factory_requires_async_resolve(type_name));
        }

        let instance = Shared::new((provider_ref.factory)(self));
        drop(permit);
        Ok(instance)
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn resolve_instance_async<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        #[cfg(feature = "tracing")]
        let _span =
            info_span!(SPAN_FACTORY_EXECUTE, type_name = std::any::type_name::<T>()).entered();

        let type_name = std::any::type_name::<T>();
        let provider_ref = self.resolve_provider::<T>()?;
        let permit = provider_ref
            .acquire_creation_permit_async(type_name)
            .await?;

        if let Some(async_factory) = &provider_ref.async_factory {
            let instance = Shared::new((async_factory)(self.clone()).await);
            drop(permit);
            return Ok(instance);
        }

        let instance = Shared::new((provider_ref.factory)(self));
        drop(permit);
        Ok(instance)
    }

    pub(crate) fn store_instance<T>(&self, instance: Shared<Instance<T>>)
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        self.inner.instances.borrow_mut().insert(type_id, instance);
    }

    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let mut providers = self.inner.providers.borrow_mut();
        if providers.contains_key(&type_id) {
            return Err(Error::provider_already_registered(
                type_name,
                provider.scope.to_string().as_str(),
            ));
        }
        providers.insert(type_id, Shared::new(provider));

        Ok(())
    }

    pub(crate) fn get_instance<T>(&self) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        let local = self.inner.instances.borrow().get(&type_id).cloned();

        if local.is_some() {
            return local.and_then(|instance| instance.downcast::<Instance<T>>().ok());
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_instance::<T>();
        }

        None
    }

    pub fn try_resolve<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();

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

            match provider.scope {
                Scope::Root => {
                    let root = self.root_injector();
                    root.store_instance::<T>(instance.clone());
                }

                Scope::Module => {
                    self.store_instance::<T>(instance.clone());
                }

                Scope::Transient => unreachable!(),
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

    pub fn resolve<T>(&self) -> Shared<T>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve::<T>().unwrap()
    }

    pub fn optional_resolve<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve::<T>().ok()
    }

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
            trace!("Resolving service asynchronously");

            if let Some(instance) = self.get_instance::<T>() {
                #[cfg(feature = "tracing")]
                trace!("Resolve cache hit");
                #[cfg(feature = "metrics")]
                self.inner.metrics.record_resolve_cache_hit();
                return Ok(instance.value());
            }

            #[cfg(feature = "tracing")]
            trace!("Resolve cache miss, creating instance asynchronously");
            #[cfg(feature = "metrics")]
            self.inner.metrics.record_resolve_cache_miss();

            let provider = self.resolve_provider::<T>()?;
            let instance = self.resolve_instance_async::<T>().await?;

            if provider.scope == Scope::Transient {
                #[cfg(feature = "tracing")]
                trace!("Resolved transient service");
                return Ok(instance.value());
            }

            match provider.scope {
                Scope::Root => {
                    let root = self.root_injector();
                    root.store_instance::<T>(instance.clone());
                }

                Scope::Module => {
                    self.store_instance::<T>(instance.clone());
                }

                Scope::Transient => unreachable!(),
            }

            #[cfg(feature = "tracing")]
            trace!(scope = %provider.scope, "Resolved and cached service");

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
    pub async fn optional_resolve_async<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + 'static,
    {
        self.try_resolve_async::<T>().await.ok()
    }
}

#[cfg(feature = "thread-safe")]
impl Injector {
    pub fn try_provide<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let scope = provider.scope;

        #[cfg(feature = "metrics")]
        self.inner.metrics.record_provide_attempt();

        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();

        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_PROVIDE, type_name = type_name, scope = %scope).entered();

        let result = match scope {
            Scope::Root => {
                let root = self.root_injector();
                root.store_provider::<T>(provider)
            }

            Scope::Module | Scope::Transient => self.store_provider::<T>(provider),
        };

        #[cfg(feature = "tracing")]
        match &result {
            Ok(_) => debug!("Provider registered"),
            Err(error) => debug!(error = %error, "Provider registration failed"),
        }

        #[cfg(feature = "metrics")]
        match &result {
            Ok(_) => self.inner.metrics.record_provide_success(),
            Err(_) => self.inner.metrics.record_provide_failure(),
        }

        result
    }

    pub fn provide<T>(&self, provider: Provider<T>) -> &Self
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_provide::<T>(provider).unwrap();
        self
    }

    pub(crate) fn get_provider<T>(&self) -> Option<Shared<dyn Any + Send + Sync>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .providers
            .get(&type_id)
            .map(|value| value.value().clone());
        #[cfg(not(feature = "lock-free"))]
        let local = self.inner.providers.read().unwrap().get(&type_id).cloned();

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_provider::<T>();
        }

        None
    }

    pub(crate) fn resolve_provider<T>(&self) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider::<T>()
            .ok_or_else(|| Error::service_not_provided(type_name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        Ok(provider)
    }

    pub(crate) fn resolve_instance<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_FACTORY_EXECUTE, type_name = type_name).entered();

        let provider_ref = self.resolve_provider::<T>()?;
        let permit = provider_ref.acquire_creation_permit(type_name)?;

        #[cfg(feature = "async-factory")]
        if provider_ref.async_factory.is_some() {
            drop(permit);
            return Err(Error::async_factory_requires_async_resolve(type_name));
        }

        let instance = Shared::new((provider_ref.factory)(self));
        drop(permit);
        Ok(instance)
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn resolve_instance_async<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        #[cfg(feature = "tracing")]
        let span = info_span!(SPAN_FACTORY_EXECUTE, type_name = std::any::type_name::<T>());

        let operation = async {
            let type_name = std::any::type_name::<T>();
            let provider_ref = self.resolve_provider::<T>()?;
            let permit = provider_ref
                .acquire_creation_permit_async(type_name)
                .await?;

            if let Some(async_factory) = &provider_ref.async_factory {
                let instance = Shared::new((async_factory)(self.clone()).await);
                drop(permit);
                return Ok(instance);
            }

            let instance = Shared::new((provider_ref.factory)(self));
            drop(permit);
            Ok(instance)
        };

        #[cfg(feature = "tracing")]
        {
            use tracing::Instrument;
            operation.instrument(span).await
        }

        #[cfg(not(feature = "tracing"))]
        {
            operation.await
        }
    }

    pub(crate) fn store_instance<T>(&self, instance: Shared<Instance<T>>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "lock-free")]
        {
            self.inner.instances.insert(type_id, instance);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            self.inner
                .instances
                .write()
                .unwrap()
                .insert(type_id, instance);
        }
    }

    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope.to_string();

        #[cfg(feature = "lock-free")]
        {
            match self.inner.providers.entry(type_id) {
                DashEntry::Occupied(_) => {
                    return Err(Error::provider_already_registered(
                        type_name,
                        scope.as_str(),
                    ));
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(Shared::new(provider));
                }
            }
        }

        #[cfg(not(feature = "lock-free"))]
        {
            let mut providers = self.inner.providers.write().unwrap();
            if providers.contains_key(&type_id) {
                return Err(Error::provider_already_registered(
                    type_name,
                    scope.as_str(),
                ));
            }
            providers.insert(type_id, Shared::new(provider));
        }

        Ok(())
    }

    pub(crate) fn get_instance<T>(&self) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .instances
            .get(&type_id)
            .map(|value| value.value().clone());
        #[cfg(not(feature = "lock-free"))]
        let local = self.inner.instances.read().unwrap().get(&type_id).cloned();

        if local.is_some() {
            return local.and_then(|instance| instance.downcast::<Instance<T>>().ok());
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_instance::<T>();
        }

        None
    }

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

            match provider.scope {
                Scope::Root => {
                    let root = self.root_injector();
                    root.store_instance::<T>(instance.clone());
                }

                Scope::Module => {
                    self.store_instance::<T>(instance.clone());
                }

                Scope::Transient => unreachable!(),
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

    pub fn resolve<T>(&self) -> Shared<T>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve::<T>().unwrap()
    }

    pub fn optional_resolve<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve::<T>().ok()
    }

    #[cfg(feature = "async-factory")]
    pub async fn try_resolve_async<T>(&self) -> Result<Shared<T>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let span = info_span!(SPAN_RESOLVE, type_name = type_name);

        #[cfg(feature = "metrics")]
        self.inner.metrics.record_resolve_attempt();
        #[cfg(feature = "metrics")]
        let resolve_started = std::time::Instant::now();

        let operation = async {
            let _guard = ResolveGuard::push(type_id)?;

            #[cfg(feature = "tracing")]
            trace!("Resolving service asynchronously");

            if let Some(instance) = self.get_instance::<T>() {
                #[cfg(feature = "tracing")]
                trace!("Resolve cache hit");
                #[cfg(feature = "metrics")]
                self.inner.metrics.record_resolve_cache_hit();
                return Ok(instance.value());
            }

            #[cfg(feature = "tracing")]
            trace!("Resolve cache miss, creating instance asynchronously");
            #[cfg(feature = "metrics")]
            self.inner.metrics.record_resolve_cache_miss();

            let provider = self.resolve_provider::<T>()?;
            let instance = self.resolve_instance_async::<T>().await?;

            if provider.scope == Scope::Transient {
                #[cfg(feature = "tracing")]
                trace!("Resolved transient service");
                return Ok(instance.value());
            }

            match provider.scope {
                Scope::Root => {
                    let root = self.root_injector();
                    root.store_instance::<T>(instance.clone());
                }

                Scope::Module => {
                    self.store_instance::<T>(instance.clone());
                }

                Scope::Transient => unreachable!(),
            }

            #[cfg(feature = "tracing")]
            trace!(scope = %provider.scope, "Resolved and cached service");

            Ok(instance.value())
        };

        #[cfg(feature = "tracing")]
        let result = {
            use tracing::Instrument;
            operation.instrument(span).await
        };

        #[cfg(not(feature = "tracing"))]
        let result = operation.await;

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
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve_async::<T>().await.unwrap()
    }

    #[cfg(feature = "async-factory")]
    pub async fn optional_resolve_async<T>(&self) -> Option<Shared<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.try_resolve_async::<T>().await.ok()
    }
}
