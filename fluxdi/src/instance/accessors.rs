use super::*;

impl<T: ?Sized + 'static> Instance<T> {
    /// Returns a reference to the wrapped value.
    ///
    /// This provides direct, immutable access to the underlying value without
    /// requiring an additional clone of the `Shared` wrapper. The reference
    /// is valid for as long as the `Instance` exists.
    ///
    /// # Returns
    ///
    /// An immutable reference to the wrapped value of type `&T`.
    ///
    /// # Performance
    ///
    /// This is a zero-cost operation that simply dereferences the `Shared`
    /// pointer. No cloning or additional allocations occur.
    ///
    /// # Examples
    ///
    /// Accessing fields:
    ///
    /// ```
    /// use fluxdi::{Instance, Shared};
    ///
    /// struct Config {
    ///     debug: bool,
    ///     timeout_ms: u64,
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Config {
    ///     debug: true,
    ///     timeout_ms: 5000,
    /// }));
    ///
    /// assert!(instance.get().debug);
    /// assert_eq!(instance.get().timeout_ms, 5000);
    /// ```
    ///
    /// Calling methods:
    ///
    /// ```
    /// use fluxdi::{Instance, Shared};
    ///
    /// struct Calculator {
    ///     base: i32,
    /// }
    ///
    /// impl Calculator {
    ///     fn add(&self, x: i32) -> i32 {
    ///         self.base + x
    ///     }
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Calculator { base: 10 }));
    /// assert_eq!(instance.get().add(5), 15);
    /// ```
    ///
    /// Using with trait objects:
    ///
    /// ```
    /// use fluxdi::{Instance, Shared};
    ///
    /// trait Greeter {
    ///     fn greet(&self) -> String;
    /// }
    ///
    /// struct EnglishGreeter;
    /// impl Greeter for EnglishGreeter {
    ///     fn greet(&self) -> String {
    ///         "Hello!".to_string()
    ///     }
    /// }
    ///
    /// let greeter: Shared<dyn Greeter> = Shared::new(EnglishGreeter);
    /// let instance = Instance::<dyn Greeter>::new(greeter);
    ///
    /// assert_eq!(instance.get().greet(), "Hello!");
    /// ```
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Returns a clone of the underlying `Shared<T>` reference.
    ///
    /// This creates a new strong reference to the same underlying value,
    /// incrementing the reference count. The cloned `Shared` can be stored,
    /// passed to other functions, or used to create additional `Instance`
    /// wrappers.
    ///
    /// # Returns
    ///
    /// A `Shared<T>` that points to the same allocation as this instance.
    ///
    /// # Performance
    ///
    /// This operation performs a reference count increment, which is:
    /// - Atomic (when using `thread-safe` feature with `Arc`)
    /// - Non-atomic but very fast (when using `Rc` without `thread-safe`)
    ///
    /// No deep cloning of the actual value occurs.
    ///
    /// # Use Cases
    ///
    /// - **Sharing dependencies**: Pass the value to multiple components
    /// - **Storing references**: Keep a reference in a struct or collection
    /// - **Background tasks**: Send the value to async tasks or threads (with `thread-safe`)
    /// - **Testing**: Create test instances that share the same mock
    ///
    /// # Examples
    ///
    /// Storing multiple references:
    ///
    /// ```
    /// use fluxdi::{Instance, Shared};
    ///
    /// struct Config {
    ///     max_connections: u32,
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Config { max_connections: 100 }));
    ///
    /// let shared1 = instance.value();
    /// let shared2 = instance.value();
    ///
    /// // All point to the same allocation
    /// assert!(Shared::ptr_eq(&shared1, &shared2));
    /// assert!(Shared::ptr_eq(&shared1, &instance.value()));
    /// ```
    ///
    /// Passing to multiple services:
    ///
    /// ```
    /// use fluxdi::{Instance, Shared};
    ///
    /// struct Database {
    ///     url: String,
    /// }
    ///
    /// struct UserService {
    ///     db: Shared<Database>,
    /// }
    ///
    /// struct OrderService {
    ///     db: Shared<Database>,
    /// }
    ///
    /// let db_instance = Instance::new(Shared::new(Database {
    ///     url: "postgresql://localhost".to_string(),
    /// }));
    ///
    /// let user_service = UserService {
    ///     db: db_instance.value(),
    /// };
    ///
    /// let order_service = OrderService {
    ///     db: db_instance.value(),
    /// };
    ///
    /// // Both services share the same database connection
    /// assert!(Shared::ptr_eq(&user_service.db, &order_service.db));
    /// ```
    ///
    /// # Thread Safety
    ///
    /// When the `thread-safe` feature is enabled, the returned `Shared<T>`
    /// (which is `Arc<T>`) can be safely sent to other threads:
    ///
    /// ```no_run
    /// # #[cfg(feature = "thread-safe")]
    /// # {
    /// use fluxdi::{Instance, Shared};
    /// use std::thread;
    ///
    /// struct Counter {
    ///     value: std::sync::atomic::AtomicU32,
    /// }
    ///
    /// let instance = Instance::new(Shared::new(Counter {
    ///     value: std::sync::atomic::AtomicU32::new(0),
    /// }));
    ///
    /// let shared = instance.value();
    /// let handle = thread::spawn(move || {
    ///     shared.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    /// });
    ///
    /// handle.join().unwrap();
    /// # }
    /// ```
    pub fn value(&self) -> Shared<T> {
        self.value.clone()
    }
}
