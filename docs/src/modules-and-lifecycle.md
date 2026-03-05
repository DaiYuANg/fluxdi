# Modules And Lifecycle

`Module::configure(...)` is the unified provider registration entry.

For async startup/shutdown orchestration, use:

- `Module::on_start(...)`
- `Module::on_stop(...)`
- `Application::bootstrap().await`
- `Application::shutdown().await`

Sync-only bootstrapping is still available through `bootstrap_sync()`.

```rust
use fluxdi::{Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture};

struct AppModule;

impl Module for AppModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<String>(Provider::root(|_| Shared::new("ok".to_string())));
        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let value = injector.resolve::<String>();
            println!("started with {}", value);
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = Application::new(AppModule);
    app.bootstrap().await?;
    app.shutdown().await
}
```
