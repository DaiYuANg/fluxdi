use super::*;

impl Injector {
    pub(crate) fn store_instance<T>(&self, instance: Shared<Instance<T>>)
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        self.inner.instances.borrow_mut().insert(type_id, instance);
    }

    pub(crate) fn store_set_instance<T>(
        &self,
        provider_ref: &Shared<Provider<T>>,
        instance: Shared<Instance<T>>,
    ) where
        T: ?Sized + 'static,
    {
        let key = SetProviderKey::of::<T>(provider_ref);
        self.inner.set_instances.borrow_mut().insert(key, instance);
    }

    pub(crate) fn store_instance_named<T>(&self, name: &str, instance: Shared<Instance<T>>)
    where
        T: ?Sized + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);
        self.inner
            .named_instances
            .borrow_mut()
            .insert(key, instance);
    }

    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope;
        let graph_meta = ProviderGraphMeta::of::<T>(scope, provider.dependency_hints.clone());

        let mut providers = self.inner.providers.borrow_mut();
        if providers.contains_key(&type_id) {
            return Err(Error::provider_already_registered(
                type_name,
                scope.to_string().as_str(),
            ));
        }
        providers.insert(type_id, Shared::new(provider));
        drop(providers);
        self.inner
            .graph_providers
            .borrow_mut()
            .insert(type_id, graph_meta);

        Ok(())
    }

    pub(crate) fn store_set_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let graph_meta =
            ProviderGraphMeta::of::<T>(provider.scope, provider.dependency_hints.clone());
        let mut providers = self.inner.set_providers.borrow_mut();
        providers
            .entry(type_id)
            .or_default()
            .push(Shared::new(provider));
        drop(providers);
        self.inner
            .graph_set_providers
            .borrow_mut()
            .entry(type_id)
            .or_default()
            .push(graph_meta);
        Ok(())
    }

    pub(crate) fn store_named_provider<T>(
        &self,
        name: &str,
        provider: Provider<T>,
    ) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope;
        let graph_meta = ProviderGraphMeta::of::<T>(scope, provider.dependency_hints.clone());

        let mut providers = self.inner.named_providers.borrow_mut();
        if providers.contains_key(&key) {
            return Err(Error::provider_already_registered_named(
                type_name,
                name,
                scope.to_string().as_str(),
            ));
        }
        providers.insert(key.clone(), Shared::new(provider));
        drop(providers);
        self.inner
            .graph_named_providers
            .borrow_mut()
            .insert(key, graph_meta);

        Ok(())
    }

    pub(crate) fn replace_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let graph_meta =
            ProviderGraphMeta::of::<T>(provider.scope, provider.dependency_hints.clone());

        let mut providers = self.inner.providers.borrow_mut();
        if !providers.contains_key(&type_id) {
            return Err(Error::service_not_provided_for_override(type_name));
        }
        providers.insert(type_id, Shared::new(provider));
        drop(providers);
        self.inner
            .graph_providers
            .borrow_mut()
            .insert(type_id, graph_meta);

        self.clear_instance_cache::<T>();
        Ok(())
    }

    pub(crate) fn clear_instance_cache<T>(&self)
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.inner.instances.borrow_mut().remove(&type_id);
    }

    pub(crate) fn get_instance<T>(&self) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();

        let local = self.inner.instances.borrow().get(&type_id).cloned();

        if local.is_some() {
            return local.and_then(|instance| instance.downcast::<Instance<T>>().ok());
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_instance::<T>();
        }

        None
    }

    pub(crate) fn get_set_instance<T>(
        &self,
        provider_ref: &Shared<Provider<T>>,
    ) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + 'static,
    {
        let key = SetProviderKey::of::<T>(provider_ref);
        let local = self.inner.set_instances.borrow().get(&key).cloned();
        local.and_then(|instance| instance.downcast::<Instance<T>>().ok())
    }

    pub(crate) fn get_instance_named<T>(&self, name: &str) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);
        let local = self.inner.named_instances.borrow().get(&key).cloned();

        if local.is_some() {
            return local.and_then(|instance| instance.downcast::<Instance<T>>().ok());
        }

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            return parent_injector.get_instance_named::<T>(name);
        }

        None
    }
}
