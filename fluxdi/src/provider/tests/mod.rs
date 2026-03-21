use super::*;
use crate::scope::Scope;

mod decorator;
mod factories_and_threading;
mod scopes;

#[derive(Debug, Clone, PartialEq)]
struct TestService {
    id: u32,
    name: String,
}

#[cfg(not(feature = "thread-safe"))]
#[derive(Debug)]
struct Counter {
    value: std::cell::Cell<u32>,
}

#[cfg(not(feature = "thread-safe"))]
impl Counter {
    fn new() -> Self {
        Self {
            value: std::cell::Cell::new(0),
        }
    }

    fn increment(&self) -> u32 {
        let current = self.value.get();
        self.value.set(current + 1);
        current
    }
}

#[cfg(feature = "thread-safe")]
#[derive(Debug)]
struct Counter {
    value: std::sync::atomic::AtomicU32,
}

#[cfg(feature = "thread-safe")]
impl Counter {
    fn new() -> Self {
        Self {
            value: std::sync::atomic::AtomicU32::new(0),
        }
    }

    fn increment(&self) -> u32 {
        self.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

trait Repository: std::fmt::Debug {}

#[derive(Debug)]
struct PostgresRepository {
    _connection_string: String,
}

impl Repository for PostgresRepository {}
