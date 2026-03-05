use super::*;

mod advanced;
mod basic;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestData {
    id: u32,
    name: String,
}

#[derive(Debug, PartialEq)]
struct Counter {
    value: u32,
}

trait Service: std::fmt::Debug {
    fn name(&self) -> &str;
    fn execute(&self) -> String;
}

#[derive(Debug)]
struct DatabaseService {
    connection_string: String,
}

impl Service for DatabaseService {
    fn name(&self) -> &str {
        "DatabaseService"
    }

    fn execute(&self) -> String {
        format!("Connected to: {}", self.connection_string)
    }
}

#[derive(Debug)]
struct CacheService {
    max_size: usize,
}

impl Service for CacheService {
    fn name(&self) -> &str {
        "CacheService"
    }

    fn execute(&self) -> String {
        format!("Cache with max size: {}", self.max_size)
    }
}
