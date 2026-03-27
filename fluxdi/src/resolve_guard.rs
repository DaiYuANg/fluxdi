use std::collections::HashMap;
use std::sync::Mutex;
use std::any::TypeId;

use crate::error::{Error, ErrorKind};
#[cfg(feature = "tracing")]
use crate::observability::EVENT_CIRCULAR_DEPENDENCY;

#[cfg(feature = "tracing")]
use tracing::debug;

/// Key identifying a resolution scope.
///
/// When running inside a tokio task (requires `lifecycle` or `resource-limit-async`
/// feature which pulls in tokio), each task gets its own resolve stack keyed by
/// task ID. This prevents stack corruption when the runtime migrates async tasks
/// between worker threads.
///
/// Without tokio, falls back to thread ID so that each thread gets its own
/// independent stack.
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct ScopeKey(u64);

impl ScopeKey {
    fn current() -> Self {
        // When tokio is available, use task ID for async-safe scoping.
        // tokio::task::try_id() returns Some(Id) inside a tokio task, None outside.
        #[cfg(feature = "lifecycle")]
        if let Some(id) = tokio::task::try_id() {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            id.hash(&mut hasher);
            return Self(hasher.finish());
        }
        // Fallback for sync contexts or when tokio is not available: use thread ID.
        Self(thread_id_as_u64())
    }
}

fn thread_id_as_u64() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    hasher.finish()
}

/// Global resolve stacks, keyed by scope (task ID or thread ID).
///
/// Uses a global Mutex instead of thread_local! so that guards pushed on
/// one worker thread and dropped on another (after async task migration)
/// still find their stack. Contention is minimal: the critical section is
/// a HashMap lookup + Vec push/pop.
static RESOLVE_STACKS: Mutex<Option<HashMap<ScopeKey, Vec<TypeId>>>> = Mutex::new(None);

fn with_stacks<R>(f: impl FnOnce(&mut HashMap<ScopeKey, Vec<TypeId>>) -> R) -> R {
    let mut guard = RESOLVE_STACKS.lock().unwrap_or_else(|e| e.into_inner());
    let stacks = guard.get_or_insert_with(HashMap::new);
    f(stacks)
}

pub struct ResolveGuard {
    type_id: TypeId,
    scope_key: ScopeKey,
}

impl ResolveGuard {
    pub fn push(type_id: TypeId) -> Result<Self, Error> {
        let scope_key = ScopeKey::current();

        with_stacks(|stacks| {
            let stack = stacks.entry(scope_key).or_default();

            if stack.contains(&type_id) {
                #[cfg(feature = "tracing")]
                debug!(
                    event = EVENT_CIRCULAR_DEPENDENCY,
                    type_id = ?type_id,
                    depth = stack.len(),
                    "Circular dependency detected during resolve"
                );

                return Err(Error::new(
                    ErrorKind::CircularDependency,
                    format!(
                        "Circular dependency detected while resolving type_id: {:?}",
                        type_id
                    ),
                ));
            }

            stack.push(type_id);
            Ok(Self {
                type_id,
                scope_key,
            })
        })
    }
}

impl Drop for ResolveGuard {
    fn drop(&mut self) {
        with_stacks(|stacks| {
            if let Some(stack) = stacks.get_mut(&self.scope_key) {
                if let Some(last) = stack.pop() {
                    if last != self.type_id {
                        panic!(
                            "ResolveGuard stack corrupted: expected to pop {:?} but popped {:?}",
                            self.type_id, last
                        );
                    }
                    if stack.is_empty() {
                        stacks.remove(&self.scope_key);
                    }
                } else {
                    panic!("ResolveGuard stack corrupted: attempted to pop from an empty stack");
                }
            } else {
                panic!(
                    "ResolveGuard stack corrupted: no stack found for scope {:?}",
                    self.scope_key.0
                );
            }
        });
    }
}
