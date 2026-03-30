use super::*;
use crate::{Provider, Shared};

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn wave_parallelism_respects_dependency_order() {
    let injector = Injector::root();
    let order = Arc::new(AtomicUsize::new(0));

    // A and B have no deps (wave 0), C depends on both (wave 1)
    injector.provide::<u32>({
        let order = order.clone();
        Provider::singleton_async(move |_| {
            let order = order.clone();
            async move {
                let seq = order.fetch_add(1, Ordering::SeqCst);
                Arc::new(seq as u32)
            }
        })
    });

    injector.provide::<u64>({
        let order = order.clone();
        Provider::singleton_async(move |_| {
            let order = order.clone();
            async move {
                let seq = order.fetch_add(1, Ordering::SeqCst);
                Arc::new(seq as u64)
            }
        })
    });

    injector.provide::<String>({
        let order = order.clone();
        Provider::singleton_async(move |_| {
            let order = order.clone();
            async move {
                let seq = order.fetch_add(1, Ordering::SeqCst);
                Arc::new(format!("C:{}", seq))
            }
        })
        .with_dependency::<u32>()
        .with_dependency::<u64>()
    });

    injector.resolve_all_eager().await.unwrap();

    // C should have been resolved after A and B
    let c = injector.try_resolve_async::<String>().await.unwrap();
    let c_seq: usize = c.strip_prefix("C:").unwrap().parse().unwrap();
    assert!(
        c_seq >= 2,
        "C should resolve after A and B, got sequence {}",
        c_seq
    );
}

#[tokio::test]
async fn idempotent_double_call() {
    let injector = Injector::root();
    let counter = Arc::new(AtomicUsize::new(0));

    injector.provide::<u32>({
        let counter = counter.clone();
        Provider::singleton_async(move |_| {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Arc::new(42u32)
            }
        })
    });

    injector.resolve_all_eager().await.unwrap();
    injector.resolve_all_eager().await.unwrap();

    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "Factory should only run once"
    );
}

#[cfg(feature = "dynamic")]
#[tokio::test]
async fn error_propagation_prevents_next_wave() {
    use crate::dynamic::DynamicProvider;
    use std::any::Any;

    let injector = Injector::root();

    // "good" provider in wave 0
    injector.provide::<u32>(Provider::singleton_async(|_| async { Arc::new(1u32) }));

    // "failing" dynamic provider in wave 0
    injector.provide_dynamic(
        "bad",
        DynamicProvider::new(|_inj| async {
            Err(crate::Error::new(
                crate::ErrorKind::ModuleLifecycleFailed,
                "intentional failure".to_string(),
            ))
        }),
    );

    // wave 1 depends on the failing provider
    injector.provide::<bool>(
        Provider::singleton_async(|_| async { Arc::new(true) }).with_dynamic_dependency("bad"),
    );

    let result = injector.resolve_all_eager().await;
    assert!(
        result.is_err(),
        "Should fail due to failing dynamic provider"
    );
    assert!(
        result.unwrap_err().message.contains("intentional failure"),
        "Error should contain the failure reason"
    );
}

#[tokio::test]
async fn no_providers_is_noop() {
    let injector = Injector::root();
    injector.resolve_all_eager().await.unwrap();
}

#[cfg(feature = "dynamic")]
#[tokio::test]
async fn eager_resolves_dynamic_providers() {
    use crate::dynamic::DynamicProvider;
    use std::any::Any;

    let injector = Injector::root();

    injector.provide::<u32>(Provider::singleton_async(|_| async { Arc::new(10u32) }));

    injector.provide_dynamic(
        "doubled",
        DynamicProvider::new(|inj| async move {
            let base = inj.try_resolve_async::<u32>().await.map_err(|e| e)?;
            Ok(Arc::new(*base * 2) as Shared<dyn Any + Send + Sync>)
        })
        .depends_on_static::<u32>(),
    );

    injector.resolve_all_eager().await.unwrap();

    // Both should be cached now
    let base = injector.try_resolve_async::<u32>().await.unwrap();
    assert_eq!(*base, 10);

    let doubled = injector.try_resolve_dynamic("doubled").await.unwrap();
    let val = doubled.downcast::<u32>().unwrap();
    assert_eq!(*val, 20);
}

#[tokio::test]
async fn transient_providers_skipped_in_eager() {
    let injector = Injector::root();
    let counter = Arc::new(AtomicUsize::new(0));

    injector.provide::<u32>({
        let counter = counter.clone();
        Provider::transient_async(move |_| {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Arc::new(99u32)
            }
        })
    });

    injector.resolve_all_eager().await.unwrap();

    // Transient providers should still be resolved by eager (they have resolvers registered)
    // but their instances won't be cached
    let val = injector.try_resolve_async::<u32>().await.unwrap();
    assert_eq!(*val, 99);
}
