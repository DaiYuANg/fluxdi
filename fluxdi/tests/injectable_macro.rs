#![cfg(feature = "macros")]

use fluxdi::{Injectable, Injector, Provider, Shared};

#[derive(Debug)]
struct Clock {
    tick: u64,
}

#[derive(Injectable)]
struct AppService {
    clock: Shared<Clock>,
}

#[test]
fn derive_injectable_resolves_shared_dependencies() {
    let injector = Injector::root();
    injector.provide::<Clock>(Provider::root(|_| Shared::new(Clock { tick: 42 })));
    injector.provide::<AppService>(Provider::root(AppService::from_injector));

    let service = injector.resolve::<AppService>();
    assert_eq!(service.clock.tick, 42);
}
