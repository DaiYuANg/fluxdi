use axum::{Router, routing::get};
use fluxdi::{
    Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture,
};
use std::net::SocketAddr;

#[derive(Clone)]
struct ServerAddress(SocketAddr);

struct WebModule;

impl Module for WebModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<ServerAddress>(Provider::root(|_| {
            Shared::new(ServerAddress(
                "127.0.0.1:3001".parse().expect("invalid bind address"),
            ))
        }));

        injector.provide::<Router>(Provider::root(|_| {
            Shared::new(Router::new().route("/health", get(|| async { "ok" })))
        }));
        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let addr = injector.resolve::<ServerAddress>().0;
            let app = injector.resolve::<Router>().as_ref().clone();

            let listener = tokio::net::TcpListener::bind(addr).await.map_err(|err| {
                Error::module_lifecycle_failed("WebModule", "on_start", &err.to_string())
            })?;

            println!("listening on http://{addr}");
            axum::serve(listener, app).await.map_err(|err| {
                Error::module_lifecycle_failed("WebModule", "on_start", &err.to_string())
            })
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = Application::new(WebModule);
    app.bootstrap().await
}
