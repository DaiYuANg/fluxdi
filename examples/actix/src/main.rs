use actix_web::{App, HttpResponse, HttpServer, web};
use fluxdi::{
    Injector, Provider, Shared,
    actix::{Resolved, injector_data},
};

#[derive(Debug)]
struct GreetingService {
    message: String,
}

async fn hello(Resolved(service): Resolved<GreetingService>) -> HttpResponse {
    HttpResponse::Ok().body(service.message.clone())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let injector = Shared::new(Injector::root());
    injector.provide::<GreetingService>(Provider::root(|_| {
        Shared::new(GreetingService {
            message: "hello from fluxdi + actix".to_string(),
        })
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(injector_data(injector.clone()))
            .route("/hello", web::get().to(hello))
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
