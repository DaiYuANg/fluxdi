use super::*;

impl Injector {
    pub(crate) fn get_provider<T>(&self) -> Option<Shared<dyn Any>>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();

        let local = self.inner.providers.borrow().get(&type_id).cloned();

        if local.is_some() {
            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                op = "provider_lookup",
                source = "local",
                hit = true,
                "Provider lookup hit"
            );
            return local;
        }

        #[cfg(feature = "tracing")]
        trace!(
            type_name = type_name,
            op = "provider_lookup",
            source = "local",
            hit = false,
            has_parent = self.inner.parent.is_some(),
            "Provider lookup miss"
        );

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                op = "provider_lookup",
                source = "parent",
                "Falling back to parent injector"
            );

            return parent_injector.get_provider::<T>();
        }

        #[cfg(feature = "tracing")]
        trace!(
            type_name = type_name,
            op = "provider_lookup",
            source = "none",
            "Provider not found in injector hierarchy"
        );

        None
    }

    pub(crate) fn get_provider_named<T>(&self, name: &str) -> Option<Shared<dyn Any>>
    where
        T: ?Sized + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();
        let local = self.inner.named_providers.borrow().get(&key).cloned();

        if local.is_some() {
            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                name = %name,
                op = "provider_lookup_named",
                source = "local",
                hit = true,
                "Named provider lookup hit"
            );
            return local;
        }

        #[cfg(feature = "tracing")]
        trace!(
            type_name = type_name,
            name = %name,
            op = "provider_lookup_named",
            source = "local",
            hit = false,
            has_parent = self.inner.parent.is_some(),
            "Named provider lookup miss"
        );

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };

            #[cfg(feature = "tracing")]
            trace!(
                type_name = type_name,
                name = %name,
                op = "provider_lookup_named",
                source = "parent",
                "Falling back to parent injector for named provider"
            );

            return parent_injector.get_provider_named::<T>(name);
        }

        #[cfg(feature = "tracing")]
        trace!(
            type_name = type_name,
            name = %name,
            op = "provider_lookup_named",
            source = "none",
            "Named provider not found in injector hierarchy"
        );

        None
    }

    pub(crate) fn resolve_provider<T>(&self) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider::<T>()
            .ok_or_else(|| Error::service_not_provided(type_name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            op = "provider_resolve",
            "Resolved provider metadata"
        );

        Ok(provider)
    }

    pub(crate) fn resolve_provider_named<T>(&self, name: &str) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider_named::<T>(name)
            .ok_or_else(|| Error::service_not_provided_named(type_name, name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            name = %name,
            op = "provider_resolve_named",
            "Resolved named provider metadata"
        );

        Ok(provider)
    }

    pub(crate) fn collect_set_providers<T>(
        &self,
        providers: &mut Vec<Shared<Provider<T>>>,
    ) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            parent_injector.collect_set_providers::<T>(providers)?;
        }

        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let local = self
            .inner
            .set_providers
            .borrow()
            .get(&type_id)
            .cloned()
            .unwrap_or_default();
        #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
        let local_count = local.len();

        for any_provider in local {
            let provider = any_provider
                .downcast::<Provider<T>>()
                .map_err(|_| Error::type_mismatch(type_name))?;
            providers.push(provider);
        }

        #[cfg(feature = "tracing")]
        trace!(
            type_name = type_name,
            op = "provider_collect_set",
            local_count = local_count,
            aggregate_count = providers.len(),
            "Collected set providers from injector node"
        );

        Ok(())
    }

    pub(crate) fn resolve_set_providers<T>(&self) -> Result<Vec<Shared<Provider<T>>>, Error>
    where
        T: ?Sized + 'static,
    {
        let type_name = std::any::type_name::<T>();
        let mut providers = Vec::new();
        self.collect_set_providers::<T>(&mut providers)?;

        if providers.is_empty() {
            return Err(Error::service_not_provided(type_name));
        }

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            op = "provider_resolve_set",
            provider_count = providers.len(),
            "Resolved provider set"
        );

        Ok(providers)
    }
}
