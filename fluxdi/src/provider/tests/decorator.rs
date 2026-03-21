use super::*;

#[test]
fn test_decorator_wraps_resolved_instance() {
    let provider = Provider::singleton(|_| {
        Shared::new(TestService {
            id: 0,
            name: "base".to_string(),
        })
    })
    .with_decorator(|inner| {
        let mut svc = (*inner).clone();
        svc.name = format!("{} (decorated)", svc.name);
        Shared::new(svc)
    });

    let injector = Injector::root();
    injector.provide::<TestService>(provider);

    let resolved = injector.resolve::<TestService>();
    assert_eq!(resolved.id, 0);
    assert_eq!(resolved.name, "base (decorated)");
}

#[test]
fn test_multiple_decorators_apply_in_order() {
    let provider = Provider::singleton(|_| {
        Shared::new(TestService {
            id: 1,
            name: "x".to_string(),
        })
    })
    .with_decorator(|inner| {
        let mut s = (*inner).clone();
        s.name = format!("[{}]", s.name);
        Shared::new(s)
    })
    .with_decorator(|inner| {
        let mut s = (*inner).clone();
        s.name = format!("({})", s.name);
        Shared::new(s)
    });

    let injector = Injector::root();
    injector.provide::<TestService>(provider);

    let resolved = injector.resolve::<TestService>();
    // Order: base -> first decorator -> second decorator
    // base: "x" -> first: "[x]" -> second: "([x])"
    assert_eq!(resolved.name, "([x])");
}

#[test]
fn test_decorator_preserves_singleton_scope() {
    let provider = Provider::singleton(|_| {
        Shared::new(TestService {
            id: 42,
            name: "singleton".to_string(),
        })
    })
    .with_decorator(|inner| inner);

    let injector = Injector::root();
    injector.provide::<TestService>(provider);

    let a = injector.resolve::<TestService>();
    let b = injector.resolve::<TestService>();
    assert!(Shared::ptr_eq(&a, &b));
}
