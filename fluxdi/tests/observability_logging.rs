#![cfg(feature = "logging")]

use fluxdi::{Injector, Provider, Shared, try_init_logging};

#[test]
fn initializes_logging_and_emits_fluxdi_events() {
    try_init_logging().expect("logging subscriber should initialize");

    let injector = Injector::root();
    injector.provide::<String>(Provider::transient(|_| Shared::new("value".to_string())));

    let value = injector.try_resolve::<String>().unwrap();
    assert_eq!(value.as_str(), "value");
}
