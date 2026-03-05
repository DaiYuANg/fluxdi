use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct Database;

trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
}

struct AuthPlugin;
impl Plugin for AuthPlugin {
    fn name(&self) -> &'static str {
        "auth"
    }
}

struct MetricsPlugin;
impl Plugin for MetricsPlugin {
    fn name(&self) -> &'static str {
        "metrics"
    }
}

#[derive(Debug)]
struct AppService;

fn main() {
    let injector = Injector::root();

    injector.provide::<Database>(Provider::singleton(|_| Shared::new(Database)));
    injector.provide_into_set::<dyn Plugin>(Provider::singleton(|_| {
        Shared::new(AuthPlugin) as Shared<dyn Plugin>
    }));
    injector.provide_into_set::<dyn Plugin>(Provider::singleton(|_| {
        Shared::new(MetricsPlugin) as Shared<dyn Plugin>
    }));

    injector.provide::<AppService>(
        Provider::singleton(|_| Shared::new(AppService))
            .with_dependency::<Database>()
            .with_set_dependency::<dyn Plugin>(),
    );

    let report = injector.validate_graph();
    println!("graph valid: {}", report.is_valid());
    if !report.is_valid() {
        for issue in report.issues {
            println!("issue: {}", issue.message);
        }
    }

    let graph = injector.dependency_graph();
    println!("nodes: {}", graph.nodes.len());
    println!("edges: {}", graph.edges.len());

    println!("\nDOT:\n{}", graph.to_dot());
    println!("\nMermaid:\n{}", graph.to_mermaid());

    let plugins = injector.resolve_all::<dyn Plugin>();
    for plugin in plugins {
        println!("plugin: {}", plugin.name());
    }
}
