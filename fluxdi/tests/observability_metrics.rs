#![cfg(feature = "metrics")]

use fluxdi::{ErrorKind, Injector, Provider, Shared};

#[test]
fn tracks_registration_resolution_and_failure_counters() {
    let injector = Injector::root();

    injector
        .try_provide::<String>(Provider::transient(|_| Shared::new("value".to_string())))
        .unwrap();

    let duplicate = injector
        .try_provide::<String>(Provider::transient(|_| Shared::new("other".to_string())))
        .unwrap_err();
    assert_eq!(duplicate.kind, ErrorKind::ProviderAlreadyRegistered);

    let first = injector.try_resolve::<String>().unwrap();
    assert_eq!(first.as_str(), "value");
    let second = injector.try_resolve::<String>().unwrap();
    assert_eq!(second.as_str(), "value");

    let missing = injector.try_resolve::<u64>().unwrap_err();
    assert_eq!(missing.kind, ErrorKind::ServiceNotProvided);

    let snapshot = injector.metrics_snapshot();
    assert_eq!(snapshot.provide_attempts_total, 2);
    assert_eq!(snapshot.provide_success_total, 1);
    assert_eq!(snapshot.provide_failures_total, 1);
    assert_eq!(snapshot.resolve_attempts_total, 3);
    assert_eq!(snapshot.resolve_success_total, 2);
    assert_eq!(snapshot.resolve_failures_total, 1);
    assert_eq!(snapshot.resolve_cache_hits_total, 0);
    assert_eq!(snapshot.resolve_cache_misses_total, 3);
    assert_eq!(snapshot.factory_executions_total, 2);
    assert_eq!(snapshot.resolve_duration_samples_total, 3);
}

#[test]
fn tracks_cache_hits_for_root_scoped_service() {
    let injector = Injector::root();

    injector.provide::<u32>(Provider::root(|_| Shared::new(42)));

    let first = injector.try_resolve::<u32>().unwrap();
    let second = injector.try_resolve::<u32>().unwrap();

    assert_eq!(*first, 42);
    assert_eq!(*second, 42);

    let snapshot = injector.metrics_snapshot();
    assert_eq!(snapshot.resolve_attempts_total, 2);
    assert_eq!(snapshot.resolve_success_total, 2);
    assert_eq!(snapshot.resolve_failures_total, 0);
    assert_eq!(snapshot.resolve_cache_hits_total, 1);
    assert_eq!(snapshot.resolve_cache_misses_total, 1);
    assert_eq!(snapshot.factory_executions_total, 1);
    assert_eq!(snapshot.resolve_duration_samples_total, 2);
}

#[test]
fn shares_metrics_between_parent_and_child_injectors() {
    let root = Shared::new(Injector::root());
    root.provide::<usize>(Provider::root(|_| Shared::new(7usize)));

    let child = Injector::child(root.clone());
    let value = child.try_resolve::<usize>().unwrap();
    assert_eq!(*value, 7);

    let root_snapshot = root.metrics_snapshot();
    let child_snapshot = child.metrics_snapshot();
    assert_eq!(root_snapshot, child_snapshot);
    assert_eq!(root_snapshot.resolve_attempts_total, 1);
    assert_eq!(root_snapshot.factory_executions_total, 1);
}

#[cfg(feature = "prometheus")]
#[test]
fn exports_prometheus_text_metrics() {
    let injector = Injector::root();
    injector.provide::<u8>(Provider::root(|_| Shared::new(1)));
    let _ = injector.try_resolve::<u8>().unwrap();

    let text = injector.prometheus_metrics();

    assert!(text.contains("# HELP fluxdi_resolve_attempts_total"));
    assert!(text.contains("# TYPE fluxdi_resolve_attempts_total counter"));
    assert!(text.contains("fluxdi_resolve_attempts_total 1"));
    assert!(text.contains("fluxdi_factory_executions_total 1"));
    assert!(text.contains("fluxdi_resolve_duration_seconds_total"));
}
