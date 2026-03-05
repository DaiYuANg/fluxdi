use super::*;

#[test]
fn test_application_can_be_created_with_different_modules() {
    let _app1 = Application::new(EmptyModule);

    #[cfg(not(feature = "thread-safe"))]
    let _app2 = Application::new(CountingModule {
        counter: Rc::new(RefCell::new(0)),
    });
    #[cfg(feature = "thread-safe")]
    let _app2 = Application::new(CountingModule {
        counter: Arc::new(Mutex::new(0)),
    });

    // Should compile and work with different module types
}

#[test]
fn test_injector_accessible_before_bootstrap() {
    let app = Application::new(EmptyModule);
    let _injector = app.injector();
    // Should not panic - injector is available even before bootstrap
}

#[test]
fn test_multiple_injector_clones() {
    let mut app = Application::new(EmptyModule);
    app.bootstrap_sync().unwrap();

    let injectors: Vec<_> = (0..5).map(|_| app.injector()).collect();
    assert_eq!(injectors.len(), 5);

    #[cfg(feature = "thread-safe")]
    assert_eq!(std::sync::Arc::strong_count(&injectors[0]), 6); // app + 5 in vec

    #[cfg(not(feature = "thread-safe"))]
    assert_eq!(std::rc::Rc::strong_count(&injectors[0]), 6); // app + 5 in vec
}

#[cfg(feature = "debug")]
#[test]
fn test_debug_implementation() {
    let app = Application::new(EmptyModule);
    let debug_str = format!("{:?}", app);
    assert!(
        debug_str.contains("Application"),
        "Debug output should contain 'Application'"
    );
}

#[test]
fn test_deeply_nested_modules() {
    #[cfg(not(feature = "thread-safe"))]
    let counter = Rc::new(RefCell::new(0));
    #[cfg(feature = "thread-safe")]
    let counter = Arc::new(Mutex::new(0));

    let module = NestedImportModule {
        counter: counter.clone(),
        depth: 5,
    };

    let mut app = Application::new(module);
    app.bootstrap_sync().unwrap();

    // depth 5, 4, 3, 2, 1, 0 = 6 modules total
    #[cfg(not(feature = "thread-safe"))]
    assert_eq!(*counter.borrow(), 6, "All nested modules should be loaded");
    #[cfg(feature = "thread-safe")]
    assert_eq!(
        *counter.lock().unwrap(),
        6,
        "All nested modules should be loaded"
    );
}
