use super::*;
use crate::runtime::Shared;
use futures::executor::block_on;

struct EmptyModule;

impl Module for EmptyModule {
    fn providers(&self, _injector: &Injector) {}
}

struct ModuleWithImports {
    import_count: usize,
}

impl Module for ModuleWithImports {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        (0..self.import_count)
            .map(|_| Box::new(EmptyModule) as Box<dyn Module>)
            .collect()
    }

    fn providers(&self, _injector: &Injector) {}
}

#[test]
fn test_default_imports_returns_empty_vec() {
    let module = EmptyModule;
    let imports = module.imports();
    assert!(imports.is_empty(), "Default imports should be empty");
}

#[test]
fn test_module_can_have_imports() {
    let module = ModuleWithImports { import_count: 3 };
    let imports = module.imports();
    assert_eq!(imports.len(), 3, "Should have 3 imports");
}

#[test]
fn test_module_providers_can_be_called() {
    let module = EmptyModule;
    let injector = Injector::root();

    // Should not panic
    module.providers(&injector);
    assert!(module.configure(&injector).is_ok());
}

#[test]
fn test_module_trait_object() {
    let module: Box<dyn Module> = Box::new(EmptyModule);
    let injector = Injector::root();

    // Test that trait object works correctly
    let imports = module.imports();
    assert!(imports.is_empty());

    module.providers(&injector);
}

#[test]
fn test_multiple_modules() {
    let modules: Vec<Box<dyn Module>> = vec![
        Box::new(EmptyModule),
        Box::new(EmptyModule),
        Box::new(ModuleWithImports { import_count: 2 }),
    ];

    assert_eq!(modules.len(), 3, "Should have 3 modules");

    let injector = Injector::root();
    for module in modules {
        module.providers(&injector);
        assert!(module.configure(&injector).is_ok());
    }
}

#[test]
fn test_default_async_lifecycle_hooks_are_noop() {
    let module = EmptyModule;
    let injector = Shared::new(Injector::root());

    assert!(block_on(module.providers_async(injector.clone())).is_ok());
    assert!(block_on(module.on_start(injector.clone())).is_ok());
    assert!(block_on(module.on_stop(injector)).is_ok());
}

#[test]
fn test_nested_imports() {
    let module = ModuleWithImports { import_count: 2 };
    let imports = module.imports();

    // Each import should also be callable
    let injector = Injector::root();
    for import in imports {
        import.providers(&injector);
        assert!(
            import.imports().is_empty(),
            "Nested imports should be empty for EmptyModule"
        );
    }
}

// Note: CountingModule uses RefCell which is not Send, so it's only available
// when thread-safe feature is disabled
#[cfg(not(feature = "thread-safe"))]
struct CountingModule {
    call_count: std::cell::RefCell<usize>,
}

#[cfg(not(feature = "thread-safe"))]
impl Module for CountingModule {
    fn providers(&self, _injector: &Injector) {
        *self.call_count.borrow_mut() += 1;
    }
}

#[cfg(feature = "thread-safe")]
struct CountingModule {
    call_count: std::sync::Mutex<usize>,
}

#[cfg(feature = "thread-safe")]
impl Module for CountingModule {
    fn providers(&self, _injector: &Injector) {
        *self.call_count.lock().unwrap() += 1;
    }
}

#[test]
fn test_providers_can_have_side_effects() {
    #[cfg(not(feature = "thread-safe"))]
    let module = CountingModule {
        call_count: std::cell::RefCell::new(0),
    };

    #[cfg(feature = "thread-safe")]
    let module = CountingModule {
        call_count: std::sync::Mutex::new(0),
    };

    let injector = Injector::root();

    #[cfg(not(feature = "thread-safe"))]
    {
        assert_eq!(*module.call_count.borrow(), 0);
        module.providers(&injector);
        assert_eq!(*module.call_count.borrow(), 1);
        module.providers(&injector);
        assert_eq!(*module.call_count.borrow(), 2);
    }

    #[cfg(feature = "thread-safe")]
    {
        assert_eq!(*module.call_count.lock().unwrap(), 0);
        module.providers(&injector);
        assert_eq!(*module.call_count.lock().unwrap(), 1);
        module.providers(&injector);
        assert_eq!(*module.call_count.lock().unwrap(), 2);
    }
}
