use super::*;

impl Injector {
    pub(crate) fn resolve_instance<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_FACTORY_EXECUTE, type_name = type_name).entered();

        let provider_ref = self.resolve_provider::<T>()?;
        let permit = provider_ref.acquire_creation_permit(type_name)?;

        #[cfg(feature = "async-factory")]
        if provider_ref.async_factory.is_some() {
            drop(permit);
            return Err(Error::async_factory_requires_async_resolve(type_name));
        }

        let instance = Shared::new((provider_ref.factory)(self));
        drop(permit);
        Ok(instance)
    }

    pub(crate) fn resolve_instance_from_provider<T>(
        &self,
        provider_ref: &Shared<Provider<T>>,
    ) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(
            SPAN_FACTORY_EXECUTE,
            type_name = type_name,
            op = "resolve_set"
        )
        .entered();

        let permit = provider_ref.acquire_creation_permit(type_name)?;

        #[cfg(feature = "async-factory")]
        if provider_ref.async_factory.is_some() {
            drop(permit);
            return Err(Error::async_factory_requires_async_resolve(type_name));
        }

        let instance = Shared::new((provider_ref.factory)(self));
        drop(permit);
        Ok(instance)
    }

    pub(crate) fn resolve_instance_named<T>(&self, name: &str) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let _span = info_span!(SPAN_FACTORY_EXECUTE, type_name = type_name, name = %name).entered();

        let provider_ref = self.resolve_provider_named::<T>(name)?;
        let permit = provider_ref.acquire_creation_permit(type_name)?;

        #[cfg(feature = "async-factory")]
        if provider_ref.async_factory.is_some() {
            drop(permit);
            return Err(Error::async_factory_requires_async_resolve(type_name));
        }

        let instance = Shared::new((provider_ref.factory)(self));
        drop(permit);
        Ok(instance)
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn resolve_instance_async<T>(&self) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        #[cfg(feature = "tracing")]
        let span = info_span!(SPAN_FACTORY_EXECUTE, type_name = std::any::type_name::<T>());

        let operation = async {
            let type_name = std::any::type_name::<T>();
            let provider_ref = self.resolve_provider::<T>()?;
            let permit = provider_ref
                .acquire_creation_permit_async(type_name)
                .await?;

            if let Some(async_factory) = &provider_ref.async_factory {
                let instance = Shared::new((async_factory)(self.clone()).await);
                drop(permit);
                return Ok(instance);
            }

            let instance = Shared::new((provider_ref.factory)(self));
            drop(permit);
            Ok(instance)
        };

        #[cfg(feature = "tracing")]
        {
            use tracing::Instrument;
            operation.instrument(span).await
        }

        #[cfg(not(feature = "tracing"))]
        {
            operation.await
        }
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn resolve_instance_named_async<T>(
        &self,
        name: &str,
    ) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        #[cfg(feature = "tracing")]
        let span =
            info_span!(SPAN_FACTORY_EXECUTE, type_name = std::any::type_name::<T>(), name = %name);

        let operation = async {
            let type_name = std::any::type_name::<T>();
            let provider_ref = self.resolve_provider_named::<T>(name)?;
            let permit = provider_ref
                .acquire_creation_permit_async(type_name)
                .await?;

            if let Some(async_factory) = &provider_ref.async_factory {
                let instance = Shared::new((async_factory)(self.clone()).await);
                drop(permit);
                return Ok(instance);
            }

            let instance = Shared::new((provider_ref.factory)(self));
            drop(permit);
            Ok(instance)
        };

        #[cfg(feature = "tracing")]
        {
            use tracing::Instrument;
            operation.instrument(span).await
        }

        #[cfg(not(feature = "tracing"))]
        {
            operation.await
        }
    }

    #[cfg(feature = "async-factory")]
    pub(crate) async fn resolve_instance_from_provider_async<T>(
        &self,
        provider_ref: &Shared<Provider<T>>,
    ) -> Result<Shared<Instance<T>>, Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        #[cfg(feature = "metrics")]
        self.inner.metrics.record_factory_execution();

        #[cfg(feature = "tracing")]
        let span = info_span!(
            SPAN_FACTORY_EXECUTE,
            type_name = std::any::type_name::<T>(),
            op = "resolve_set"
        );

        let operation = async {
            let type_name = std::any::type_name::<T>();
            let permit = provider_ref
                .acquire_creation_permit_async(type_name)
                .await?;

            if let Some(async_factory) = &provider_ref.async_factory {
                let instance = Shared::new((async_factory)(self.clone()).await);
                drop(permit);
                return Ok(instance);
            }

            let instance = Shared::new((provider_ref.factory)(self));
            drop(permit);
            Ok(instance)
        };

        #[cfg(feature = "tracing")]
        {
            use tracing::Instrument;
            operation.instrument(span).await
        }

        #[cfg(not(feature = "tracing"))]
        {
            operation.await
        }
    }
}
