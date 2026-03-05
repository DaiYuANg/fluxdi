use fluxdi::{Injector, Provider, Shared};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
struct AppConfig {
    name: String,
}

#[derive(Debug)]
struct RequestId(u64);

#[tokio::main]
async fn main() {
    fluxdi::init_logging();

    let injector = Injector::root();
    let counter = Arc::new(AtomicU64::new(1));

    injector.provide::<AppConfig>(Provider::root_async(|_| async {
        Shared::new(AppConfig {
            name: "async-factory-demo".to_string(),
        })
    }));

    injector.provide::<RequestId>(Provider::transient_async({
        let counter = Arc::clone(&counter);
        move |_| {
            let counter = Arc::clone(&counter);
            async move { Shared::new(RequestId(counter.fetch_add(1, Ordering::SeqCst))) }
        }
    }));

    // Sync providers can also be resolved through async resolve APIs.
    injector.provide::<usize>(Provider::root(|_| Shared::new(2026usize)));

    let config = injector.resolve_async::<AppConfig>().await;
    let request_a = injector.resolve_async::<RequestId>().await;
    let request_b = injector.resolve_async::<RequestId>().await;
    let year = injector.resolve_async::<usize>().await;

    println!("App: {}", config.name);
    println!("Request IDs: {}, {}", request_a.0, request_b.0);
    println!("Year from sync provider via async resolve: {}", year);
}
