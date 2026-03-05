use super::*;

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
