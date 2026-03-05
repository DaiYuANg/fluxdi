use std::sync::atomic::{AtomicUsize, Ordering};

use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct AppName(&'static str);

#[derive(Debug)]
struct RequestContext {
    request_id: usize,
}

static NEXT_REQUEST_ID: AtomicUsize = AtomicUsize::new(1);

fn main() {
    fluxdi::init_logging();

    let injector = Injector::root();

    injector.provide::<AppName>(Provider::root(|_| Shared::new(AppName("fluxdi"))));
    injector.provide::<RequestContext>(Provider::scoped(|_| {
        Shared::new(RequestContext {
            request_id: NEXT_REQUEST_ID.fetch_add(1, Ordering::SeqCst),
        })
    }));

    let scope_a = injector.create_scope();
    let scope_b = injector.create_scope();

    let app_a = scope_a.resolve::<AppName>();
    let app_b = scope_b.resolve::<AppName>();
    let request_a1 = scope_a.resolve::<RequestContext>();
    let request_a2 = scope_a.resolve::<RequestContext>();
    let request_b1 = scope_b.resolve::<RequestContext>();

    assert!(Shared::ptr_eq(&app_a, &app_b));
    assert!(Shared::ptr_eq(&request_a1, &request_a2));
    assert!(!Shared::ptr_eq(&request_a1, &request_b1));

    println!(
        "app={}, scope_a_request_id={}, scope_b_request_id={}",
        app_a.0, request_a1.request_id, request_b1.request_id
    );
}
