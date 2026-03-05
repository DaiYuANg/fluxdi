use fluxdi::{Module, Provider, Shared};

use crate::core::domain::todo::TodoRepository;
use crate::core::domain::user::UserRepository;
use crate::infra::persistence::sqlite::SqliteClient;
use crate::infra::persistence::sqlite::repository::{TodoSqliteRepository, UserSqliteRepository};

pub struct RepositoriesModule;

impl Module for RepositoriesModule {
    fn providers(&self, injector: &fluxdi::Injector) {
        injector.provide::<dyn UserRepository>(Provider::root(|injector| {
            let sqlite_client = injector.resolve::<SqliteClient>();
            Shared::new(UserSqliteRepository::new(sqlite_client)) as Shared<dyn UserRepository>
        }));

        injector.provide::<dyn TodoRepository>(Provider::root(|injector| {
            let sqlite_client = injector.resolve::<SqliteClient>();
            Shared::new(TodoSqliteRepository::new(sqlite_client)) as Shared<dyn TodoRepository>
        }));
    }
}
