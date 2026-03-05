use super::*;

impl<T: ?Sized + 'static> Instance<T> {
    /// Creates a new `Instance` wrapping the given shared value.
    ///
    /// This is typically called by the dependency injection container after
    /// resolving a dependency, but can also be used directly for testing or
    /// manual dependency management.
    ///
    /// # Arguments
    ///
    /// * `value` - A `Shared<T>` reference to wrap
    ///
    /// # Examples
    ///
    /// Creating an instance with a concrete type:
    ///
    /// ```
    /// use fluxdi::{Instance, Shared};
    ///
    /// struct Database {
    ///     url: String,
    /// }
    ///
    /// let db = Shared::new(Database {
    ///     url: "postgresql://localhost".to_string(),
    /// });
    ///
    /// let instance = Instance::new(db);
    /// ```
    ///
    /// Creating an instance with a trait object:
    ///
    /// ```
    /// use fluxdi::{Instance, Shared};
    ///
    /// trait Repository {}
    /// struct UserRepository;
    /// impl Repository for UserRepository {}
    ///
    /// let repo: Shared<dyn Repository> = Shared::new(UserRepository);
    /// let instance = Instance::<dyn Repository>::new(repo);
    /// ```
    pub fn new(value: Shared<T>) -> Self {
        Self { value }
    }
}
