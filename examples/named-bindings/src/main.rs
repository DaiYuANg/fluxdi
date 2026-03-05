use fluxdi::{Injector, Provider, Shared};

trait CacheBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn get(&self, key: &str) -> String;
}

struct RedisCache;

impl CacheBackend for RedisCache {
    fn id(&self) -> &'static str {
        "redis"
    }

    fn get(&self, key: &str) -> String {
        format!("redis:{key}")
    }
}

struct MemoryCache;

impl CacheBackend for MemoryCache {
    fn id(&self) -> &'static str {
        "memory"
    }

    fn get(&self, key: &str) -> String {
        format!("memory:{key}")
    }
}

fn main() {
    let injector = Injector::root();

    injector.provide_named::<dyn CacheBackend>(
        "primary",
        Provider::root(|_| Shared::new(RedisCache) as Shared<dyn CacheBackend>),
    );
    injector.provide_named::<dyn CacheBackend>(
        "fallback",
        Provider::root(|_| Shared::new(MemoryCache) as Shared<dyn CacheBackend>),
    );

    let primary = injector.resolve_named::<dyn CacheBackend>("primary");
    let fallback = injector.resolve_named::<dyn CacheBackend>("fallback");

    println!("primary backend: {}", primary.id());
    println!("fallback backend: {}", fallback.id());
    println!("primary fetch: {}", primary.get("session:42"));
    println!("fallback fetch: {}", fallback.get("session:42"));
}
