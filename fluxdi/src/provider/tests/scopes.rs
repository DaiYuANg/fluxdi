use super::*;

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
fn test_scoped_provider_has_scoped_scope() {
    let provider = Provider::scoped(|_| {
        Shared::new(TestService {
            id: 7,
            name: "scoped".to_string(),
        })
    });

    assert_eq!(provider.scope, Scope::Scoped);
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

    let scoped = Provider::scoped(|_| {
        Shared::new(TestService {
            id: 4,
            name: "scoped".to_string(),
        })
    });

    assert_eq!(singleton.scope, Scope::Module);
    assert_eq!(transient.scope, Scope::Transient);
    assert_eq!(root.scope, Scope::Root);
    assert_eq!(scoped.scope, Scope::Scoped);

    assert_ne!(singleton.scope, transient.scope);
    assert_ne!(singleton.scope, root.scope);
    assert_ne!(singleton.scope, scoped.scope);
    assert_ne!(transient.scope, root.scope);
    assert_ne!(transient.scope, scoped.scope);
    assert_ne!(root.scope, scoped.scope);
}
