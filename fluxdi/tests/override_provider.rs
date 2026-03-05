use fluxdi::{ErrorKind, Injector, Provider, Shared};

#[test]
fn override_root_provider_replaces_cached_instance() {
    let injector = Injector::root();
    injector.provide::<String>(Provider::root(|_| Shared::new("old".to_string())));

    let first = injector.resolve::<String>();
    assert_eq!(first.as_str(), "old");

    injector
        .try_override_provider::<String>(Provider::root(|_| Shared::new("new".to_string())))
        .unwrap();

    let second = injector.resolve::<String>();
    assert_eq!(second.as_str(), "new");
    assert!(!Shared::ptr_eq(&first, &second));
}

#[test]
fn override_module_provider_replaces_cached_instance() {
    let injector = Injector::root();
    injector.provide::<u32>(Provider::singleton(|_| Shared::new(1)));

    let first = injector.resolve::<u32>();
    assert_eq!(*first, 1);

    injector
        .try_override_provider::<u32>(Provider::singleton(|_| Shared::new(2)))
        .unwrap();

    let second = injector.resolve::<u32>();
    assert_eq!(*second, 2);
    assert!(!Shared::ptr_eq(&first, &second));
}

#[test]
fn override_requires_existing_provider() {
    let injector = Injector::root();
    let error = injector
        .try_override_provider::<u32>(Provider::root(|_| Shared::new(10)))
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::ServiceNotProvided);
    assert!(error.message.contains("Cannot override provider"));
}

#[test]
fn child_can_override_root_provider() {
    let root = Shared::new(Injector::root());
    root.provide::<String>(Provider::root(|_| Shared::new("old-root".to_string())));

    let child = Injector::child(root.clone());
    child
        .try_override_provider::<String>(Provider::root(|_| Shared::new("new-root".to_string())))
        .unwrap();

    let from_root = root.resolve::<String>();
    let from_child = child.resolve::<String>();

    assert_eq!(from_root.as_str(), "new-root");
    assert_eq!(from_child.as_str(), "new-root");
}
