use fluxdi::{
    Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture,
};

#[derive(Debug)]
struct AsyncConfig {
    endpoint: String,
}

struct AsyncModule;

impl Module for AsyncModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<AsyncConfig>(Provider::root_async(|_| async {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            Shared::new(AsyncConfig {
                endpoint: "https://api.example.dev".to_string(),
            })
        }));
        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let config = injector.try_resolve_async::<AsyncConfig>().await?;
            println!("async instance ready: {}", config.endpoint);
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = Application::new(AsyncModule);
    app.bootstrap().await?;
    app.shutdown().await
}
