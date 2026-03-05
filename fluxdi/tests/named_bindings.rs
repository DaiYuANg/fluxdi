use std::sync::atomic::{AtomicUsize, Ordering};

use fluxdi::{ErrorKind, Injector, Provider, Shared};

trait CacheBackend: Send + Sync {
    fn id(&self) -> &'static str;
}

struct RedisBackend;

impl CacheBackend for RedisBackend {
    fn id(&self) -> &'static str {
        "redis"
    }
}

struct MemoryBackend;

impl CacheBackend for MemoryBackend {
    fn id(&self) -> &'static str {
        "memory"
    }
}

#[test]
fn can_register_and_resolve_multiple_named_bindings_for_same_trait() {
    let injector = Injector::root();

    injector.provide_named::<dyn CacheBackend>(
        "primary",
        Provider::singleton(|_| Shared::new(RedisBackend) as Shared<dyn CacheBackend>),
    );
    injector.provide_named::<dyn CacheBackend>(
        "fallback",
        Provider::singleton(|_| Shared::new(MemoryBackend) as Shared<dyn CacheBackend>),
    );

    let primary = injector.resolve_named::<dyn CacheBackend>("primary");
    let fallback = injector.resolve_named::<dyn CacheBackend>("fallback");

    assert_eq!(primary.id(), "redis");
    assert_eq!(fallback.id(), "memory");
}

#[test]
fn named_registration_rejects_duplicate_name_for_same_type() {
    let injector = Injector::root();

    injector.provide_named::<dyn CacheBackend>(
        "primary",
        Provider::singleton(|_| Shared::new(RedisBackend) as Shared<dyn CacheBackend>),
    );

    let err = injector
        .try_provide_named::<dyn CacheBackend>(
            "primary",
            Provider::singleton(|_| Shared::new(MemoryBackend) as Shared<dyn CacheBackend>),
        )
        .unwrap_err();

    assert_eq!(err.kind, ErrorKind::ProviderAlreadyRegistered);
    assert!(err.message.contains("primary"));
}

#[test]
fn missing_named_binding_includes_name_context() {
    let injector = Injector::root();
    let err = match injector.try_resolve_named::<dyn CacheBackend>("missing") {
        Ok(_) => panic!("expected named resolve to fail"),
        Err(err) => err,
    };

    assert_eq!(err.kind, ErrorKind::ServiceNotProvided);
    assert!(err.message.contains("missing"));
}

#[test]
fn named_root_scope_is_shared_across_child_injectors() {
    let parent = Injector::root();
    parent.provide_named::<String>(
        "dsn",
        Provider::root(|_| Shared::new("sqlite::memory:".to_string())),
    );

    let child = Injector::child(Shared::new(parent.clone()));

    let from_parent = parent.resolve_named::<String>("dsn");
    let from_child = child.resolve_named::<String>("dsn");

    assert!(Shared::ptr_eq(&from_parent, &from_child));
}

#[test]
fn named_module_scope_is_cached_per_injector() {
    static CREATIONS: AtomicUsize = AtomicUsize::new(0);

    CREATIONS.store(0, Ordering::SeqCst);

    let parent = Injector::root();
    parent.provide_named::<usize>(
        "module-local",
        Provider::singleton(|_| Shared::new(CREATIONS.fetch_add(1, Ordering::SeqCst))),
    );

    let child = Injector::child(Shared::new(parent.clone()));

    let child_first = child.resolve_named::<usize>("module-local");
    let child_second = child.resolve_named::<usize>("module-local");
    let parent_first = parent.resolve_named::<usize>("module-local");
    let parent_second = parent.resolve_named::<usize>("module-local");

    assert!(Shared::ptr_eq(&child_first, &child_second));
    assert!(Shared::ptr_eq(&parent_first, &parent_second));
    assert_ne!(*parent_first, *child_first);
}

#[cfg(feature = "async-factory")]
#[test]
fn named_async_provider_requires_async_resolve_path() {
    use futures::executor::block_on;

    let injector = Injector::root();
    injector.provide_named::<String>(
        "remote",
        Provider::singleton_async(|_| async { Shared::new("ready".to_string()) }),
    );

    let sync_err = injector.try_resolve_named::<String>("remote").unwrap_err();
    assert_eq!(sync_err.kind, ErrorKind::AsyncFactoryRequiresAsyncResolve);

    let value = block_on(injector.try_resolve_named_async::<String>("remote")).unwrap();
    assert_eq!(value.as_str(), "ready");
}
