use super::*;

#[test]
fn test_instance_new_creates_instance() {
    let data = Shared::new(TestData {
        id: 1,
        name: "test".to_string(),
    });

    let instance = Instance::new(data);

    assert_eq!(instance.get().id, 1);
    assert_eq!(instance.get().name, "test");
}

#[test]
fn test_instance_new_with_primitive() {
    let value = Shared::new(42);
    let instance = Instance::new(value);

    assert_eq!(*instance.get(), 42);
}

#[test]
fn test_instance_new_with_string() {
    let text = Shared::new(String::from("Hello, world!"));
    let instance = Instance::new(text);

    assert_eq!(instance.get().as_str(), "Hello, world!");
}

#[test]
fn test_instance_new_with_vec() {
    let numbers = Shared::new(vec![1, 2, 3, 4, 5]);
    let instance = Instance::new(numbers);

    assert_eq!(instance.get().len(), 5);
    assert_eq!(instance.get()[2], 3);
}

#[test]
fn test_instance_new_with_complex_struct() {
    #[derive(Debug, PartialEq)]
    struct ComplexData {
        values: Vec<i32>,
        metadata: std::collections::HashMap<String, String>,
    }

    let mut map = std::collections::HashMap::new();
    map.insert("key1".to_string(), "value1".to_string());

    let complex = Shared::new(ComplexData {
        values: vec![10, 20, 30],
        metadata: map,
    });

    let instance = Instance::new(complex);

    assert_eq!(instance.get().values.len(), 3);
    assert_eq!(instance.get().metadata.get("key1").unwrap(), "value1");
}

#[test]
fn test_get_returns_reference() {
    let data = Shared::new(TestData {
        id: 42,
        name: "reference-test".to_string(),
    });

    let instance = Instance::new(data);
    let reference = instance.get();

    assert_eq!(reference.id, 42);
    assert_eq!(reference.name, "reference-test");
}

#[test]
fn test_get_multiple_times_returns_same_data() {
    let data = Shared::new(Counter { value: 100 });
    let instance = Instance::new(data);

    let ref1 = instance.get();
    let ref2 = instance.get();

    // Both references point to the same data
    assert_eq!(ref1.value, ref2.value);
    assert_eq!(ref1.value, 100);
}

#[test]
fn test_get_allows_field_access() {
    let data = Shared::new(TestData {
        id: 5,
        name: "field-test".to_string(),
    });

    let instance = Instance::new(data);

    // Direct field access
    assert_eq!(instance.get().id, 5);
    assert_eq!(instance.get().name, "field-test");
}

#[test]
fn test_get_allows_method_calls() {
    #[derive(Debug)]
    struct Calculator {
        base: i32,
    }

    impl Calculator {
        fn add(&self, x: i32) -> i32 {
            self.base + x
        }

        fn multiply(&self, x: i32) -> i32 {
            self.base * x
        }
    }

    let calc = Shared::new(Calculator { base: 10 });
    let instance = Instance::new(calc);

    assert_eq!(instance.get().add(5), 15);
    assert_eq!(instance.get().multiply(3), 30);
}

#[test]
fn test_get_with_nested_access() {
    #[derive(Debug)]
    struct Inner {
        value: String,
    }

    #[derive(Debug)]
    struct Outer {
        inner: Inner,
        count: usize,
    }

    let outer = Shared::new(Outer {
        inner: Inner {
            value: "nested".to_string(),
        },
        count: 5,
    });

    let instance = Instance::new(outer);

    assert_eq!(instance.get().inner.value, "nested");
    assert_eq!(instance.get().count, 5);
}

#[test]
fn test_value_returns_cloned_shared() {
    let data = Shared::new(TestData {
        id: 99,
        name: "clone-test".to_string(),
    });

    let instance = Instance::new(data);
    let shared1 = instance.value();
    let shared2 = instance.value();

    // Both point to the same allocation
    assert!(Shared::ptr_eq(&shared1, &shared2));
}

#[test]
fn test_value_increments_reference_count() {
    let data = Shared::new(TestData {
        id: 1,
        name: "refcount".to_string(),
    });

    let instance = Instance::new(data.clone());

    // Initial count: 2 (data + instance)
    #[cfg(feature = "thread-safe")]
    let initial_count = std::sync::Arc::strong_count(&data);
    #[cfg(not(feature = "thread-safe"))]
    let initial_count = std::rc::Rc::strong_count(&data);

    let _shared1 = instance.value();

    #[cfg(feature = "thread-safe")]
    let after_one = std::sync::Arc::strong_count(&data);
    #[cfg(not(feature = "thread-safe"))]
    let after_one = std::rc::Rc::strong_count(&data);

    assert_eq!(after_one, initial_count + 1);

    let _shared2 = instance.value();

    #[cfg(feature = "thread-safe")]
    let after_two = std::sync::Arc::strong_count(&data);
    #[cfg(not(feature = "thread-safe"))]
    let after_two = std::rc::Rc::strong_count(&data);

    assert_eq!(after_two, initial_count + 2);
}

#[test]
fn test_value_can_be_stored() {
    let data = Shared::new(TestData {
        id: 10,
        name: "storage-test".to_string(),
    });

    let instance = Instance::new(data);

    // Store in a vector
    let storage: Vec<Shared<TestData>> = vec![instance.value(), instance.value(), instance.value()];

    // All stored references point to the same data
    assert!(Shared::ptr_eq(&storage[0], &storage[1]));
    assert!(Shared::ptr_eq(&storage[1], &storage[2]));

    // Data is accessible through stored references
    assert_eq!(storage[0].id, 10);
    assert_eq!(storage[2].name, "storage-test");
}

#[test]
fn test_value_enables_sharing_across_components() {
    struct ServiceA {
        data: Shared<TestData>,
    }

    struct ServiceB {
        data: Shared<TestData>,
    }

    let data = Shared::new(TestData {
        id: 50,
        name: "shared".to_string(),
    });

    let instance = Instance::new(data);

    let service_a = ServiceA {
        data: instance.value(),
    };

    let service_b = ServiceB {
        data: instance.value(),
    };

    // Both services share the same data
    assert!(Shared::ptr_eq(&service_a.data, &service_b.data));
    assert_eq!(service_a.data.id, 50);
    assert_eq!(service_b.data.id, 50);
}

#[test]
fn test_instance_with_trait_object() {
    let service: Shared<dyn Service> = Shared::new(DatabaseService {
        connection_string: "postgresql://localhost".to_string(),
    });

    let instance = Instance::<dyn Service>::new(service);

    assert_eq!(instance.get().name(), "DatabaseService");
    assert!(instance.get().execute().contains("postgresql"));
}

#[test]
fn test_instance_trait_object_polymorphism() {
    let db_service: Shared<dyn Service> = Shared::new(DatabaseService {
        connection_string: "postgresql://localhost".to_string(),
    });

    let cache_service: Shared<dyn Service> = Shared::new(CacheService { max_size: 1000 });

    let instance1 = Instance::<dyn Service>::new(db_service);
    let instance2 = Instance::<dyn Service>::new(cache_service);

    assert_eq!(instance1.get().name(), "DatabaseService");
    assert_eq!(instance2.get().name(), "CacheService");
}

#[test]
fn test_trait_object_value_cloning() {
    let service: Shared<dyn Service> = Shared::new(CacheService { max_size: 500 });

    let instance = Instance::<dyn Service>::new(service);

    let shared1 = instance.value();
    let shared2 = instance.value();

    assert!(Shared::ptr_eq(&shared1, &shared2));
    assert_eq!(shared1.name(), "CacheService");
}
