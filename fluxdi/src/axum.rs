//! Axum integration for FluxDI.
//!
//! This module provides a state wrapper and an extractor that resolves
//! dependencies directly from an `Injector`.

use std::ops::Deref;

use ::axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

#[cfg(feature = "async-factory")]
use crate::ErrorKind;
use crate::{Error, Injector, runtime::Shared};

/// Axum state wrapper that stores a shared injector reference.
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

impl IntoResponse for ResolveRejection {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

/// Axum extractor that resolves `T` from `InjectorState`.
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

impl<S, T> FromRequestParts<S> for Resolved<T>
where
    S: Send + Sync,
    InjectorState: FromRef<S>,
    T: ?Sized + Send + Sync + 'static,
{
    type Rejection = ResolveRejection;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let injector_state = InjectorState::from_ref(state);
        #[cfg(feature = "async-factory")]
        {
            match injector_state.injector.try_resolve::<T>() {
                Ok(resolved) => Ok(Self(resolved)),
                Err(error) if error.kind == ErrorKind::AsyncFactoryRequiresAsyncResolve => {
                    let resolved = injector_state.injector.try_resolve_async::<T>().await?;
                    Ok(Self(resolved))
                }
                Err(error) => Err(error.into()),
            }
        }

        #[cfg(not(feature = "async-factory"))]
        {
            let resolved = injector_state.injector.try_resolve::<T>()?;
            Ok(Self(resolved))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use futures::executor::block_on;

    use crate::Provider;

    #[test]
    fn resolved_extractor_supports_sync_provider() {
        let injector = Shared::new(Injector::root());
        injector.provide::<u32>(Provider::root(|_| Shared::new(42)));
        let state = InjectorState::new(injector);

        let (mut parts, _) = Request::builder().uri("/").body(()).unwrap().into_parts();
        let resolved = block_on(Resolved::<u32>::from_request_parts(&mut parts, &state)).unwrap();
        assert_eq!(*resolved, 42);
    }

    #[cfg(feature = "async-factory")]
    #[test]
    fn resolved_extractor_supports_async_provider() {
        let injector = Shared::new(Injector::root());
        injector.provide::<u32>(Provider::root_async(|_| async { Shared::new(7) }));
        let state = InjectorState::new(injector);

        let (mut parts, _) = Request::builder().uri("/").body(()).unwrap().into_parts();
        let resolved = block_on(Resolved::<u32>::from_request_parts(&mut parts, &state)).unwrap();
        assert_eq!(*resolved, 7);
    }
}
