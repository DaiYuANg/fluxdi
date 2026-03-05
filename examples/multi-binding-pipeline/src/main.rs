use fluxdi::{Injector, Provider, Shared};

trait Middleware: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, input: String) -> String;
}

struct Trim;
impl Middleware for Trim {
    fn name(&self) -> &'static str {
        "trim"
    }

    fn apply(&self, input: String) -> String {
        input.trim().to_string()
    }
}

struct Uppercase;
impl Middleware for Uppercase {
    fn name(&self) -> &'static str {
        "uppercase"
    }

    fn apply(&self, input: String) -> String {
        input.to_uppercase()
    }
}

struct Prefix;
impl Middleware for Prefix {
    fn name(&self) -> &'static str {
        "prefix"
    }

    fn apply(&self, input: String) -> String {
        format!("[processed] {input}")
    }
}

fn main() {
    fluxdi::init_logging();

    let injector = Injector::root();

    // Registration order defines execution order.
    injector.provide_into_set::<dyn Middleware>(Provider::singleton(|_| {
        Shared::new(Trim) as Shared<dyn Middleware>
    }));
    injector.provide_into_set::<dyn Middleware>(Provider::singleton(|_| {
        Shared::new(Uppercase) as Shared<dyn Middleware>
    }));
    injector.provide_into_set::<dyn Middleware>(Provider::singleton(|_| {
        Shared::new(Prefix) as Shared<dyn Middleware>
    }));

    let pipeline = injector.resolve_all::<dyn Middleware>();
    let before = "  hello fluxdi  ".to_string();

    let mut value = before.clone();
    for step in pipeline {
        println!("apply: {}", step.name());
        value = step.apply(value);
    }

    println!("before: {before}");
    println!("after:  {value}");
}
