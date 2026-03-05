use super::*;

impl Injector {
    pub(crate) fn store_instance<T>(&self, instance: Shared<Instance<T>>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        #[cfg(feature = "lock-free")]
        {
            self.inner.instances.insert(type_id, instance);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            self.inner
                .instances
                .write()
                .unwrap()
                .insert(type_id, instance);
        }
    }

    pub(crate) fn store_set_instance<T>(
        &self,
        provider_ref: &Shared<Provider<T>>,
        instance: Shared<Instance<T>>,
    ) where
        T: ?Sized + Send + Sync + 'static,
    {
        let key = SetProviderKey::of::<T>(provider_ref);

        #[cfg(feature = "lock-free")]
        {
            self.inner.set_instances.insert(key, instance);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            self.inner
                .set_instances
                .write()
                .unwrap()
                .insert(key, instance);
        }
    }

    pub(crate) fn store_instance_named<T>(&self, name: &str, instance: Shared<Instance<T>>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);

        #[cfg(feature = "lock-free")]
        {
            self.inner.named_instances.insert(key, instance);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            self.inner
                .named_instances
                .write()
                .unwrap()
                .insert(key, instance);
        }
    }

    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope;
        let scope_text = scope.to_string();
        let graph_meta = ProviderGraphMeta::of::<T>(scope, provider.dependency_hints.clone());

        #[cfg(feature = "lock-free")]
        {
            match self.inner.providers.entry(type_id) {
                DashEntry::Occupied(_) => {
                    return Err(Error::provider_already_registered(
                        type_name,
                        scope_text.as_str(),
                    ));
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(Shared::new(provider));
                }
            }
            self.inner.graph_providers.insert(type_id, graph_meta);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            let mut providers = self.inner.providers.write().unwrap();
            if providers.contains_key(&type_id) {
                return Err(Error::provider_already_registered(
                    type_name,
                    scope_text.as_str(),
                ));
            }
            providers.insert(type_id, Shared::new(provider));
            drop(providers);
            self.inner
                .graph_providers
                .write()
                .unwrap()
                .insert(type_id, graph_meta);
        }

        Ok(())
    }

    pub(crate) fn store_set_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let graph_meta =
            ProviderGraphMeta::of::<T>(provider.scope, provider.dependency_hints.clone());

        #[cfg(feature = "lock-free")]
        {
            match self.inner.set_providers.entry(type_id) {
                DashEntry::Occupied(mut entry) => {
                    entry.get_mut().push(Shared::new(provider));
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(vec![Shared::new(provider)]);
                }
            }
            match self.inner.graph_set_providers.entry(type_id) {
                DashEntry::Occupied(mut entry) => {
                    entry.get_mut().push(graph_meta);
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(vec![graph_meta]);
                }
            }
        }

        #[cfg(not(feature = "lock-free"))]
        {
            let mut providers = self.inner.set_providers.write().unwrap();
            providers
                .entry(type_id)
                .or_default()
                .push(Shared::new(provider));
            drop(providers);
            self.inner
                .graph_set_providers
                .write()
                .unwrap()
                .entry(type_id)
                .or_default()
                .push(graph_meta);
        }

        Ok(())
    }
}
