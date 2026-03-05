use super::*;

#[cfg(all(feature = "thread-safe", feature = "async-factory"))]
impl<T: ?Sized + 'static> Provider<T> {
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
            dependency_hints: Vec::new(),
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
            dependency_hints: Vec::new(),
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
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }

    /// Creates a scope-scoped provider whose factory resolves asynchronously.
    #[cfg(feature = "async-factory")]
    pub fn scoped_async<F, Fut>(factory: F) -> Provider<T>
    where
        F: Fn(Injector) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Shared<T>> + Send + 'static,
    {
        Provider::<T> {
            scope: Scope::Scoped,
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
            dependency_hints: Vec::new(),
            limiter: None,
        }
    }
}
