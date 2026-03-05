//! Actix-web integration for FluxDI.
//!
//! This module provides a state wrapper, service config extension, and an
//! extractor that resolves dependencies directly from an `Injector`.

use std::{
    future::{Ready, ready},
    ops::Deref,
};

use ::actix_web::{
    Error as ActixError, FromRequest, HttpRequest, HttpResponse, ResponseError,
    dev::Payload,
    http::StatusCode,
    web::{self, ServiceConfig},
};

use crate::{Error, Injector, runtime::Shared};

/// Actix state wrapper that stores a shared injector reference.
#[derive(Clone)]
pub struct InjectorState {
    injector: Shared<Injector>,
}

impl InjectorState {
    /// Builds state from a shared injector.
    pub fn new(injector: Shared<Injector>) -> Self {
        Self { injector }
    }

    /// Returns a cloned shared injector handle.
    pub fn injector(&self) -> Shared<Injector> {
        self.injector.clone()
    }
}

/// Builds typed app data for `actix_web::App::app_data(...)`.
pub fn injector_data(injector: Shared<Injector>) -> web::Data<InjectorState> {
    web::Data::new(InjectorState::new(injector))
}

/// Extension trait that wires injector state into `ServiceConfig`.
pub trait ServiceConfigExt {
    fn with_injector(&mut self, injector: Shared<Injector>) -> &mut Self;
}

impl ServiceConfigExt for ServiceConfig {
    fn with_injector(&mut self, injector: Shared<Injector>) -> &mut Self {
        self.app_data(injector_data(injector))
    }
}

/// Rejection type used by [`Resolved`].
#[derive(Debug)]
pub struct ResolveRejection {
    pub status: StatusCode,
    pub message: String,
}

impl ResolveRejection {
    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }
}

impl From<Error> for ResolveRejection {
    fn from(value: Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl std::fmt::Display for ResolveRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl ResponseError for ResolveRejection {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).body(self.message.clone())
    }
}

/// Actix extractor that resolves `T` from [`InjectorState`].
pub struct Resolved<T: ?Sized + Send + Sync + 'static>(pub Shared<T>);

impl<T: ?Sized + Send + Sync + 'static> Resolved<T> {
    pub fn into_shared(self) -> Shared<T> {
        self.0
    }
}

impl<T: ?Sized + Send + Sync + 'static> Deref for Resolved<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<T: ?Sized + Send + Sync + 'static> From<Resolved<T>> for Shared<T> {
    fn from(value: Resolved<T>) -> Self {
        value.0
    }
}

impl<T> FromRequest for Resolved<T>
where
    T: ?Sized + Send + Sync + 'static,
{
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = req
            .app_data::<web::Data<InjectorState>>()
            .cloned()
            .ok_or_else(|| {
                ResolveRejection::internal(
                    "InjectorState is not configured in app_data".to_string(),
                )
            })
            .and_then(|state| {
                state
                    .injector()
                    .try_resolve::<T>()
                    .map_err(ResolveRejection::from)
            })
            .map(Self)
            .map_err(ActixError::from);

        ready(result)
    }
}
