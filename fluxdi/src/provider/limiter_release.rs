use super::*;

impl Limiter {
    pub(super) fn release(&self) {
        #[cfg(feature = "thread-safe")]
        {
            let mut current = self.current.lock().unwrap();
            if *current > 0 {
                *current -= 1;
            }

            if self.policy == Policy::Block {
                self.condvar.notify_one();
            }
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            let current = self.current.get();
            if current > 0 {
                self.current.set(current - 1);
            }
        }
    }
}
