use fluxdi::{Injector, Module, Provider};

use crate::core::{
    application::use_case::{
        todo::{
            CreateTodoUseCase, DeleteTodoUseCase, GetAllTodoUseCase, GetByIdTodoUseCase,
            UpdateStatusTodoUseCase,
        },
        user::{CreateUserUseCase, DeleteUserUseCase, GetAllUserUseCase, GetByIdUserUseCase},
    },
    domain::{todo::TodoRepository, user::UserRepository},
};

pub struct UseCasesModule;

impl Module for UseCasesModule {
    fn providers(&self, injector: &Injector) {
        // User use cases

        injector.provide::<CreateUserUseCase>(Provider::root(|injector| {
            let user_repository = injector.resolve::<dyn UserRepository>();
            CreateUserUseCase::new(user_repository).into()
        }));

        injector.provide::<DeleteUserUseCase>(Provider::root(|injector| {
            let user_repository = injector.resolve::<dyn UserRepository>();
            DeleteUserUseCase::new(user_repository).into()
        }));

        injector.provide::<GetAllUserUseCase>(Provider::root(|injector| {
            let user_repository = injector.resolve::<dyn UserRepository>();
            GetAllUserUseCase::new(user_repository).into()
        }));

        injector.provide::<GetByIdUserUseCase>(Provider::root(|injector| {
            let user_repository = injector.resolve::<dyn UserRepository>();
            GetByIdUserUseCase::new(user_repository).into()
        }));

        // Todos use cases

        injector.provide::<CreateTodoUseCase>(Provider::root(|injector| {
            let todo_repository = injector.resolve::<dyn TodoRepository>();
            CreateTodoUseCase::new(todo_repository).into()
        }));

        injector.provide::<DeleteTodoUseCase>(Provider::root(|injector| {
            let todo_repository = injector.resolve::<dyn TodoRepository>();
            DeleteTodoUseCase::new(todo_repository).into()
        }));

        injector.provide::<GetAllTodoUseCase>(Provider::root(|injector| {
            let todo_repository = injector.resolve::<dyn TodoRepository>();
            GetAllTodoUseCase::new(todo_repository).into()
        }));

        injector.provide::<GetByIdTodoUseCase>(Provider::root(|injector| {
            let todo_repository = injector.resolve::<dyn TodoRepository>();
            GetByIdTodoUseCase::new(todo_repository).into()
        }));

        injector.provide::<UpdateStatusTodoUseCase>(Provider::root(|injector| {
            let todo_repository = injector.resolve::<dyn TodoRepository>();
            UpdateStatusTodoUseCase::new(todo_repository).into()
        }));
    }
}
