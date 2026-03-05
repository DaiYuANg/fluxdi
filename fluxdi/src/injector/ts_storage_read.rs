use super::*;

impl Injector {
    pub(crate) fn store_named_provider<T>(
        &self,
        name: &str,
        provider: Provider<T>,
    ) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope;
        let scope_text = scope.to_string();
        let graph_meta = ProviderGraphMeta::of::<T>(scope, provider.dependency_hints.clone());

        #[cfg(feature = "lock-free")]
        {
            match self.inner.named_providers.entry(key.clone()) {
                DashEntry::Occupied(_) => {
                    return Err(Error::provider_already_registered_named(
                        type_name,
                        name,
                        scope_text.as_str(),
                    ));
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(Shared::new(provider));
                }
            }
            self.inner.graph_named_providers.insert(key, graph_meta);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            let mut providers = self.inner.named_providers.write().unwrap();
            if providers.contains_key(&key) {
                return Err(Error::provider_already_registered_named(
                    type_name,
                    name,
                    scope_text.as_str(),
                ));
            }
            providers.insert(key.clone(), Shared::new(provider));
            drop(providers);
            self.inner
                .graph_named_providers
                .write()
                .unwrap()
                .insert(key, graph_meta);
        }

        Ok(())
    }

    pub(crate) fn replace_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let graph_meta =
            ProviderGraphMeta::of::<T>(provider.scope, provider.dependency_hints.clone());

        #[cfg(feature = "lock-free")]
        {
            match self.inner.providers.entry(type_id) {
                DashEntry::Occupied(mut entry) => {
                    entry.insert(Shared::new(provider));
                }
                DashEntry::Vacant(_) => {
                    return Err(Error::service_not_provided_for_override(type_name));
                }
            }
            self.inner.graph_providers.insert(type_id, graph_meta);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            let mut providers = self.inner.providers.write().unwrap();
            if !providers.contains_key(&type_id) {
                return Err(Error::service_not_provided_for_override(type_name));
            }
            providers.insert(type_id, Shared::new(provider));
            drop(providers);
            self.inner
                .graph_providers
                .write()
                .unwrap()
                .insert(type_id, graph_meta);
        }

        self.clear_instance_cache::<T>();
        Ok(())
    }

    pub(crate) fn clear_instance_cache<T>(&self)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "lock-free")]
        {
            self.inner.instances.remove(&type_id);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            self.inner.instances.write().unwrap().remove(&type_id);
        }
    }

    pub(crate) fn get_instance<T>(&self) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .instances
            .get(&type_id)
            .map(|value| value.value().clone());
        #[cfg(not(feature = "lock-free"))]
        let local = self.inner.instances.read().unwrap().get(&type_id).cloned();

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
        T: ?Sized + Send + Sync + 'static,
    {
        let key = SetProviderKey::of::<T>(provider_ref);

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .set_instances
            .get(&key)
            .map(|value| value.value().clone());
        #[cfg(not(feature = "lock-free"))]
        let local = self.inner.set_instances.read().unwrap().get(&key).cloned();

        local.and_then(|instance| instance.downcast::<Instance<T>>().ok())
    }

    pub(crate) fn get_instance_named<T>(&self, name: &str) -> Option<Shared<Instance<T>>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);

        #[cfg(feature = "lock-free")]
        let local = self
            .inner
            .named_instances
            .get(&key)
            .map(|value| value.value().clone());
        #[cfg(not(feature = "lock-free"))]
        let local = self
            .inner
            .named_instances
            .read()
            .unwrap()
            .get(&key)
            .cloned();

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
