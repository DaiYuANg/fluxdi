use fluxdi::{
    Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture,
};

#[derive(Debug)]
struct AppName(&'static str);

#[derive(Debug)]
struct Greeting(String);

#[derive(Debug)]
struct AsyncEndpoint(String);

struct MixedModule;

impl Module for MixedModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<AppName>(Provider::root(|_| Shared::new(AppName("mixed-module"))));

        injector.provide::<Greeting>(Provider::transient(|inj| {
            let app_name = inj.resolve::<AppName>();
            Shared::new(Greeting(format!("hello from {}", app_name.0)))
        }));

        injector.provide::<AsyncEndpoint>(Provider::root_async(|inj| async move {
            let app_name = inj.resolve::<AppName>();
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            Shared::new(AsyncEndpoint(format!("https://{}.example.dev", app_name.0)))
        }));

        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let greeting = injector.resolve::<Greeting>();
            let endpoint = injector.try_resolve_async::<AsyncEndpoint>().await?;
            println!("sync provider -> {}", greeting.0);
            println!("async provider -> {}", endpoint.0);
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = Application::new(MixedModule);
    app.bootstrap().await?;

    let injector = app.injector();
    let app_name = injector.resolve::<AppName>();
    let endpoint = injector.try_resolve_async::<AsyncEndpoint>().await?;
    println!("runtime check: app={} endpoint={}", app_name.0, endpoint.0);

    app.shutdown().await
}
