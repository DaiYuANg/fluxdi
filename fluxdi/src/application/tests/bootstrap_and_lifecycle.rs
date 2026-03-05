use super::*;

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
