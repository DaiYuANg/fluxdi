use super::*;

impl Injector {
    pub(crate) fn get_provider<T>(&self) -> Option<Shared<dyn Any + Send + Sync>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .providers
            .get(&type_id)
            .map(|value| value.value().clone());
        #[cfg(not(feature = "lock-free"))]
        let local = self.inner.providers.read().unwrap().get(&type_id).cloned();

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_provider::<T>();
        }

        None
    }

    pub(crate) fn get_provider_named<T>(&self, name: &str) -> Option<Shared<dyn Any + Send + Sync>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .named_providers
            .get(&key)
            .map(|value| value.value().clone());
        #[cfg(not(feature = "lock-free"))]
        let local = self
            .inner
            .named_providers
            .read()
            .unwrap()
            .get(&key)
            .cloned();

        if local.is_some() {
            return local;
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_provider_named::<T>(name);
        }

        None
    }

    pub(crate) fn resolve_provider<T>(&self) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider::<T>()
            .ok_or_else(|| Error::service_not_provided(type_name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        Ok(provider)
    }

    pub(crate) fn resolve_provider_named<T>(&self, name: &str) -> Result<Shared<Provider<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();

        let any_provider = self
            .get_provider_named::<T>(name)
            .ok_or_else(|| Error::service_not_provided_named(type_name, name))?;

        let provider = any_provider
            .downcast::<Provider<T>>()
            .map_err(|_| Error::type_mismatch(type_name))?;

        Ok(provider)
    }

    pub(crate) fn collect_set_providers<T>(
        &self,
        providers: &mut Vec<Shared<Provider<T>>>,
    ) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            parent_injector.collect_set_providers::<T>(providers)?;
        }

        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .set_providers
            .get(&type_id)
            .map(|value| value.value().clone())
            .unwrap_or_default();
        #[cfg(not(feature = "lock-free"))]
        let local = self
            .inner
            .set_providers
            .read()
            .unwrap()
            .get(&type_id)
            .cloned()
            .unwrap_or_default();

        for any_provider in local {
            let provider = any_provider
                .downcast::<Provider<T>>()
                .map_err(|_| Error::type_mismatch(type_name))?;
            providers.push(provider);
        }

        Ok(())
    }

    pub(crate) fn resolve_set_providers<T>(&self) -> Result<Vec<Shared<Provider<T>>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();
        let mut providers = Vec::new();
        self.collect_set_providers::<T>(&mut providers)?;

        if providers.is_empty() {
            return Err(Error::service_not_provided(type_name));
        }

        Ok(providers)
    }
}
