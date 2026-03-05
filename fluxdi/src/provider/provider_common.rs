use super::*;

impl<T: ?Sized + 'static> Provider<T> {
    /// Applies resource limits to this provider.
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self.limiter = Limiter::from_limits(limits);
        self
    }

    pub(crate) fn acquire_creation_permit(
        &self,
        type_name: &str,
    ) -> Result<Option<CreationPermit>, Error> {
        if let Some(limiter) = &self.limiter {
            return Limiter::try_acquire(limiter, type_name).map(Some);
        }

        Ok(None)
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn acquire_creation_permit_async(
        &self,
        type_name: &str,
    ) -> Result<Option<CreationPermit>, Error> {
        if let Some(limiter) = &self.limiter {
            #[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
            {
                return Limiter::try_acquire_async(limiter, type_name)
                    .await
                    .map(Some);
            }

            #[cfg(not(all(feature = "thread-safe", feature = "resource-limit-async")))]
            {
                return Limiter::try_acquire(limiter, type_name).map(Some);
            }
        }

        Ok(None)
    }

    /// Declares that this provider depends on a single service `D`.
    ///
    /// This hint is used by graph tooling (`dependency_graph`, `validate_graph`)
    /// and does not change runtime resolution behavior.
    pub fn with_dependency<D>(mut self) -> Self
    where
        D: ?Sized + 'static,
    {
        self.dependency_hints.push(DependencyHint::one::<D>());
        self
    }

    /// Declares that this provider depends on a named service `D`.
    ///
    /// This hint is used by graph tooling (`dependency_graph`, `validate_graph`)
    /// and does not change runtime resolution behavior.
    pub fn with_named_dependency<D>(mut self, name: impl Into<String>) -> Self
    where
        D: ?Sized + 'static,
    {
        self.dependency_hints
            .push(DependencyHint::named::<D>(name.into()));
        self
    }

    /// Declares that this provider depends on all set bindings of service `D`.
    ///
    /// This hint is used by graph tooling (`dependency_graph`, `validate_graph`)
    /// and does not change runtime resolution behavior.
    pub fn with_set_dependency<D>(mut self) -> Self
    where
        D: ?Sized + 'static,
    {
        self.dependency_hints.push(DependencyHint::all::<D>());
        self
    }
}
