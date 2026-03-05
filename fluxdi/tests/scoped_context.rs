use std::sync::atomic::{AtomicUsize, Ordering};

use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct ScopedService {
    serial: usize,
}

#[test]
fn scoped_instances_are_reused_within_scope_and_isolated_across_scopes() {
    static CREATED: AtomicUsize = AtomicUsize::new(0);
    CREATED.store(0, Ordering::SeqCst);

    let injector = Injector::root();
    injector.provide::<ScopedService>(Provider::scoped(|_| {
        Shared::new(ScopedService {
            serial: CREATED.fetch_add(1, Ordering::SeqCst),
        })
    }));

    let scope_a = injector.create_scope();
    let scope_b = injector.create_scope();

    let a1 = scope_a.resolve::<ScopedService>();
    let a2 = scope_a.resolve::<ScopedService>();
    let b1 = scope_b.resolve::<ScopedService>();

    assert!(Shared::ptr_eq(&a1, &a2));
    assert!(!Shared::ptr_eq(&a1, &b1));
    assert_eq!(a1.serial, 0);
    assert_eq!(b1.serial, 1);
    assert_eq!(CREATED.load(Ordering::SeqCst), 2);
}

#[derive(Debug)]
struct RootService {
    serial: usize,
}

#[derive(Debug)]
struct ModuleService {
    serial: usize,
}

#[test]
fn root_and_module_scopes_work_with_runtime_scopes() {
    static ROOT_CREATED: AtomicUsize = AtomicUsize::new(0);
    static MODULE_CREATED: AtomicUsize = AtomicUsize::new(0);
    ROOT_CREATED.store(0, Ordering::SeqCst);
    MODULE_CREATED.store(0, Ordering::SeqCst);

    let injector = Injector::root();
    injector.provide::<RootService>(Provider::root(|_| {
        Shared::new(RootService {
            serial: ROOT_CREATED.fetch_add(1, Ordering::SeqCst),
        })
    }));
    injector.provide::<ModuleService>(Provider::singleton(|_| {
        Shared::new(ModuleService {
            serial: MODULE_CREATED.fetch_add(1, Ordering::SeqCst),
        })
    }));

    let scope_a = injector.create_scope();
    let scope_b = injector.create_scope();

    let root_a = scope_a.resolve::<RootService>();
    let root_b = scope_b.resolve::<RootService>();
    assert!(Shared::ptr_eq(&root_a, &root_b));
    assert_eq!(root_a.serial, 0);
    assert_eq!(ROOT_CREATED.load(Ordering::SeqCst), 1);

    let module_a1 = scope_a.resolve::<ModuleService>();
    let module_a2 = scope_a.resolve::<ModuleService>();
    let module_b1 = scope_b.resolve::<ModuleService>();

    assert!(Shared::ptr_eq(&module_a1, &module_a2));
    assert!(!Shared::ptr_eq(&module_a1, &module_b1));
    assert_eq!(module_a1.serial, 0);
    assert_eq!(module_b1.serial, 1);
    assert_eq!(MODULE_CREATED.load(Ordering::SeqCst), 2);
}

#[derive(Debug)]
struct ScopedNamed {
    serial: usize,
}

#[derive(Debug)]
struct ScopedSet {
    serial: usize,
}

#[test]
fn scoped_named_and_set_bindings_are_scope_local() {
    static NAMED_CREATED: AtomicUsize = AtomicUsize::new(0);
    static SET_CREATED: AtomicUsize = AtomicUsize::new(0);
    NAMED_CREATED.store(0, Ordering::SeqCst);
    SET_CREATED.store(0, Ordering::SeqCst);

    let injector = Injector::root();
    injector.provide_named::<ScopedNamed>(
        "main",
        Provider::scoped(|_| {
            Shared::new(ScopedNamed {
                serial: NAMED_CREATED.fetch_add(1, Ordering::SeqCst),
            })
        }),
    );
    injector.provide_into_set::<ScopedSet>(Provider::scoped(|_| {
        Shared::new(ScopedSet {
            serial: SET_CREATED.fetch_add(1, Ordering::SeqCst),
        })
    }));

    let scope_a = injector.create_scope();
    let scope_b = injector.create_scope();

    let named_a1 = scope_a.resolve_named::<ScopedNamed>("main");
    let named_a2 = scope_a.resolve_named::<ScopedNamed>("main");
    let named_b1 = scope_b.resolve_named::<ScopedNamed>("main");
    assert!(Shared::ptr_eq(&named_a1, &named_a2));
    assert!(!Shared::ptr_eq(&named_a1, &named_b1));
    assert_eq!(named_a1.serial, 0);
    assert_eq!(named_b1.serial, 1);

    let set_a1 = scope_a.resolve_all::<ScopedSet>();
    let set_a2 = scope_a.resolve_all::<ScopedSet>();
    let set_b1 = scope_b.resolve_all::<ScopedSet>();
    assert_eq!(set_a1.len(), 1);
    assert_eq!(set_b1.len(), 1);
    assert!(Shared::ptr_eq(&set_a1[0], &set_a2[0]));
    assert!(!Shared::ptr_eq(&set_a1[0], &set_b1[0]));
    assert_eq!(set_a1[0].serial, 0);
    assert_eq!(set_b1[0].serial, 1);
}
