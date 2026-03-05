use std::sync::atomic::{AtomicUsize, Ordering};

use actix_web::{App, HttpResponse, HttpServer, web};
use fluxdi::{
    Injector, Provider, Shared,
    actix::{InjectorState, Resolved, injector_data},
};

#[derive(Debug)]
struct GreetingService {
    message: String,
}

#[derive(Debug)]
struct RequestContext {
    request_scope_id: usize,
}

static NEXT_REQUEST_SCOPE_ID: AtomicUsize = AtomicUsize::new(1);

async fn hello(Resolved(service): Resolved<GreetingService>) -> HttpResponse {
    HttpResponse::Ok().body(service.message.clone())
}

async fn hello_scoped(
    state: web::Data<InjectorState>,
    Resolved(service): Resolved<GreetingService>,
) -> HttpResponse {
    let scoped = state.injector().create_scope();
    let context_a = scoped.resolve::<RequestContext>();
    let context_b = scoped.resolve::<RequestContext>();

    HttpResponse::Ok().body(format!(
        "{} | request_scope_id={} | same_instance_within_scope={}",
        service.message,
        context_a.request_scope_id,
        Shared::ptr_eq(&context_a, &context_b)
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let injector = Shared::new(Injector::root());
    injector.provide::<GreetingService>(Provider::root(|_| {
        Shared::new(GreetingService {
            message: "hello from fluxdi + actix".to_string(),
        })
    }));
    injector.provide::<RequestContext>(Provider::scoped(|_| {
        Shared::new(RequestContext {
            request_scope_id: NEXT_REQUEST_SCOPE_ID.fetch_add(1, Ordering::SeqCst),
        })
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(injector_data(injector.clone()))
            .route("/hello", web::get().to(hello))
            .route("/hello/scoped", web::get().to(hello_scoped))
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
