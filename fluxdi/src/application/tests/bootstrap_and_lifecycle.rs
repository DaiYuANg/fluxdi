use super::*;
use crate::application::options::{BootstrapOptions, ShutdownOptions};

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
fn test_shutdown_aggregates_multiple_module_errors() {
    let mut app = Application::new(FailingShutdownModule);
    block_on(app.bootstrap()).unwrap();

    let err = block_on(app.shutdown()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::ModuleLifecycleFailed);
    assert!(
        err.message.contains("2 module(s) reported errors"),
        "expected aggregated message, got: {}",
        err.message
    );
    assert!(err.message.contains("root_stop_failed"));
    assert!(err.message.contains("import_stop_failed"));
}

#[test]
fn test_bootstrap_parallel_aggregates_multiple_module_errors() {
    let mut app = Application::new(FailingBootstrapModule);
    let opts = BootstrapOptions::default().with_parallel_start(true);
    let err = block_on(app.bootstrap_with_options(opts)).unwrap_err();
    assert_eq!(err.kind, ErrorKind::ModuleLifecycleFailed);
    assert!(
        err.message.contains("2 module(s) reported errors"),
        "expected aggregated message, got: {}",
        err.message
    );
    assert!(err.message.contains("root_start_failed"));
    assert!(err.message.contains("import_start_failed"));
}

#[test]
fn test_bootstrap_rollback_calls_on_stop_on_already_started_modules() {
    #[cfg(not(feature = "thread-safe"))]
    let log: EventLog = Rc::new(RefCell::new(Vec::new()));
    #[cfg(feature = "thread-safe")]
    let log: EventLog = Arc::new(Mutex::new(Vec::new()));

    let root = RollbackTestModule { log: log.clone() };
    let mut app = Application::new(root);
    let err = block_on(app.bootstrap()).unwrap_err();

    assert_eq!(err.kind, ErrorKind::ModuleLifecycleFailed);
    assert!(err.message.contains("root_start_failed"));
    // Rollback should have called on_stop on import (which had already started)
    assert_eq!(
        event_snapshot(&log),
        vec![
            "providers:import".to_string(),
            "on_start:import".to_string(),
            "providers:root".to_string(),
            "on_start:root".to_string(),
            "on_stop:import".to_string(), // rollback
        ],
        "rollback should call on_stop on already-started modules"
    );
}

#[test]
fn test_bootstrap_parallel_rollback_calls_on_stop_on_started_modules() {
    // RollbackTestModule: import succeeds, root fails. With parallel_start, we rollback import.
    #[cfg(not(feature = "thread-safe"))]
    let log: EventLog = Rc::new(RefCell::new(Vec::new()));
    #[cfg(feature = "thread-safe")]
    let log: EventLog = Arc::new(Mutex::new(Vec::new()));

    let root = RollbackTestModule { log: log.clone() };
    let mut app = Application::new(root);
    let opts = BootstrapOptions::default().with_parallel_start(true);
    let err = block_on(app.bootstrap_with_options(opts)).unwrap_err();

    assert_eq!(err.kind, ErrorKind::ModuleLifecycleFailed);
    assert!(err.message.contains("root_start_failed"));
    assert!(
        event_snapshot(&log).contains(&"on_stop:import".to_string()),
        "rollback should call on_stop on successfully-started import; log: {:?}",
        event_snapshot(&log)
    );
}

#[test]
fn test_bootstrap_with_options_default_succeeds() {
    let mut app = Application::new(EmptyModule);
    let opts = BootstrapOptions::default();
    block_on(app.bootstrap_with_options(opts)).unwrap();
    assert!(app.is_bootstrapped());
}

#[test]
fn test_shutdown_with_options_after_bootstrap() {
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
    block_on(app.bootstrap()).unwrap();

    let opts = ShutdownOptions::default();
    block_on(app.shutdown_with_options(opts)).unwrap();

    assert_eq!(
        event_snapshot(&log),
        vec![
            "providers:root".to_string(),
            "on_start:root".to_string(),
            "on_stop:root".to_string(),
        ]
    );
}

#[tokio::test]
#[cfg(feature = "lifecycle")]
async fn test_shutdown_graceful_timeout_attempts_all_modules() {
    use crate::module::ModuleLifecycleFuture;
    use std::time::Duration;

    struct SlowShutdownRoot;

    impl Module for SlowShutdownRoot {
        fn imports(&self) -> Vec<Box<dyn Module>> {
            vec![Box::new(SlowShutdownImport)]
        }

        fn providers(&self, _: &Injector) {}

        fn on_stop(&self, _: Shared<Injector>) -> ModuleLifecycleFuture {
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            })
        }
    }

    struct SlowShutdownImport;

    impl Module for SlowShutdownImport {
        fn providers(&self, _: &Injector) {}

        fn on_stop(&self, _: Shared<Injector>) -> ModuleLifecycleFuture {
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            })
        }
    }

    let mut app = Application::new(SlowShutdownRoot);
    app.bootstrap().await.unwrap();

    let opts = ShutdownOptions::default().with_timeout(Duration::from_millis(50));
    let err = app.shutdown_with_options(opts).await.unwrap_err();

    assert_eq!(err.kind, ErrorKind::ModuleLifecycleFailed);
    assert!(
        err.message.contains("2 module(s) reported errors"),
        "graceful shutdown should attempt all modules; got: {}",
        err.message
    );
    assert!(
        err.message.contains("timed out"),
        "expected timeout in error: {}",
        err.message
    );
}

#[tokio::test]
#[cfg(feature = "lifecycle")]
async fn test_bootstrap_timeout_fails_when_exceeded() {
    use crate::module::ModuleLifecycleFuture;
    use std::time::Duration;

    struct SlowModule;

    impl Module for SlowModule {
        fn providers(&self, _: &Injector) {}

        fn on_start(&self, _: Shared<Injector>) -> ModuleLifecycleFuture {
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(())
            })
        }
    }

    let mut app = Application::new(SlowModule);
    let opts = BootstrapOptions::default().with_timeout(Duration::from_millis(50));
    let err = app.bootstrap_with_options(opts).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::ModuleLifecycleFailed);
    assert!(err.message.contains("timed out"));
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
