use fluxdi::{Application, Error, Injector, Module, Provider, Shared};

#[derive(Debug)]
struct AppName(&'static str);

struct SyncModule;

impl Module for SyncModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<AppName>(Provider::root(|_| {
            Shared::new(AppName("module-sync-example"))
        }));
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    fluxdi::init_logging();

    let mut app = Application::new(SyncModule);
    app.bootstrap_sync()?;

    let name = app.injector().resolve::<AppName>();
    println!("started {}", name.0);

    Ok(())
}
