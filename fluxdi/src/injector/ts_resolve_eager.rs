use super::*;
use std::pin::Pin;

impl Injector {
    /// Resolves all registered async providers in dependency order,
    /// maximizing parallelism for independent providers.
    ///
    /// Uses topological sort (Kahn's algorithm) to group providers into waves.
    /// Within each wave, all providers whose dependencies are satisfied run
    /// concurrently. Already-cached instances are skipped (idempotent).
    ///
    /// Requires that `.with_dependency()` hints accurately reflect actual
    /// dependencies. Call `validate_graph()` beforehand to check.
    pub async fn resolve_all_eager(&self) -> Result<(), Error> {
        // 1. Collect graph state and eager resolvers
        let mut state = GraphBuildState::default();
        self.collect_graph_state(&mut state);
        let resolvers = self.collect_eager_resolvers();

        // 2. Build node_id maps and dependency adjacency
        let mut single_ids: HashMap<TypeId, String> = HashMap::new();
        let mut named_ids: HashMap<NamedTypeKey, String> = HashMap::new();
        #[cfg(feature = "dynamic")]
        let mut dynamic_ids: HashMap<String, String> = HashMap::new();
        let mut node_deps: HashMap<String, Vec<String>> = HashMap::new();

        for (type_id, meta) in &state.singles {
            let node_id = format!("single::{}", meta.type_name);
            single_ids.insert(*type_id, node_id.clone());
            node_deps.insert(node_id, Vec::new());
        }

        for (key, meta) in &state.named {
            let node_id = format!("named::{}::{}", meta.type_name, key.name);
            named_ids.insert(key.clone(), node_id.clone());
            node_deps.insert(node_id, Vec::new());
        }

        #[cfg(feature = "dynamic")]
        for name in state.dynamics.keys() {
            let node_id = format!("dynamic::{}", name);
            dynamic_ids.insert(name.clone(), node_id.clone());
            node_deps.insert(node_id, Vec::new());
        }

        // Resolve dependency hints to node_id targets
        let resolve_dep_targets = |dep: &crate::graph::DependencyHint| -> Vec<String> {
            if dep.is_dynamic {
                #[cfg(feature = "dynamic")]
                if let Some(name) = &dep.name {
                    return dynamic_ids.get(name).cloned().into_iter().collect();
                }
                #[cfg(not(feature = "dynamic"))]
                return Vec::new();
                #[allow(unreachable_code)]
                Vec::new()
            } else {
                match dep.cardinality {
                    DependencyCardinality::One => {
                        if let Some(name) = &dep.name {
                            named_ids
                                .get(&NamedTypeKey {
                                    type_id: dep.type_id,
                                    name: name.clone(),
                                })
                                .cloned()
                                .into_iter()
                                .collect()
                        } else {
                            single_ids.get(&dep.type_id).cloned().into_iter().collect()
                        }
                    }
                    // Set dependencies are skipped for eager resolution
                    DependencyCardinality::All => Vec::new(),
                }
            }
        };

        for meta in state.singles.values() {
            let node_id = format!("single::{}", meta.type_name);
            let deps: Vec<String> = meta
                .dependencies
                .iter()
                .flat_map(&resolve_dep_targets)
                .collect();
            node_deps.insert(node_id, deps);
        }

        for (key, meta) in &state.named {
            let node_id = format!("named::{}::{}", meta.type_name, key.name);
            let deps: Vec<String> = meta
                .dependencies
                .iter()
                .flat_map(&resolve_dep_targets)
                .collect();
            node_deps.insert(node_id, deps);
        }

        #[cfg(feature = "dynamic")]
        for (name, meta) in &state.dynamics {
            let node_id = format!("dynamic::{}", name);
            let deps: Vec<String> = meta
                .dependencies
                .iter()
                .flat_map(&resolve_dep_targets)
                .collect();
            node_deps.insert(node_id, deps);
        }

        // 3. Topological sort into parallel waves
        let waves = topological_waves(&node_deps)?;

        type BoxedResolveFuture =
            Pin<Box<dyn std::future::Future<Output = Result<(), Error>> + Send>>;

        // 4. Execute each wave concurrently
        for (wave_index, wave) in waves.iter().enumerate() {
            let mut futures: Vec<BoxedResolveFuture> = Vec::new();

            for node_id in wave {
                if let Some(resolver) = resolvers.get(node_id) {
                    let resolver = resolver.clone();
                    let injector = self.clone();
                    futures.push(Box::pin(async move { (resolver)(injector).await }));
                }
            }

            if futures.is_empty() {
                continue;
            }

            let results: Vec<Result<(), Error>> = futures::future::join_all(futures).await;
            let errors: Vec<&Error> = results.iter().filter_map(|r| r.as_ref().err()).collect();

            if !errors.is_empty() {
                let message = format!(
                    "Eager resolution wave {} failed: {}",
                    wave_index,
                    errors
                        .iter()
                        .map(|e| e.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                );
                return Err(Error::new(ErrorKind::EagerResolutionFailed, message));
            }
        }

        Ok(())
    }

    fn collect_eager_resolvers(&self) -> HashMap<String, EagerResolverFn> {
        let mut resolvers = HashMap::new();

        if let Some(parent) = &self.inner.parent {
            let parent_injector = Injector {
                inner: parent.clone(),
            };
            resolvers = parent_injector.collect_eager_resolvers();
        }

        #[cfg(not(feature = "lock-free"))]
        {
            for (key, resolver) in self.inner.eager_resolvers.read().unwrap().iter() {
                resolvers.insert(key.clone(), resolver.clone());
            }
        }

        #[cfg(feature = "lock-free")]
        {
            for entry in self.inner.eager_resolvers.iter() {
                resolvers.insert(entry.key().clone(), entry.value().clone());
            }
        }

        resolvers
    }
}

/// Groups nodes into topological waves using Kahn's algorithm.
///
/// Each wave contains nodes whose dependencies are fully satisfied by
/// previous waves. Nodes within a wave can be resolved in parallel.
fn topological_waves(node_deps: &HashMap<String, Vec<String>>) -> Result<Vec<Vec<String>>, Error> {
    let mut dep_count: HashMap<String, usize> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    for (node, deps) in node_deps {
        // Only count dependencies that are known (registered) nodes
        let known_deps = deps
            .iter()
            .filter(|d| node_deps.contains_key(d.as_str()))
            .count();
        dep_count.insert(node.clone(), known_deps);
        for dep in deps {
            if node_deps.contains_key(dep) {
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(node.clone());
            }
        }
    }

    let mut waves = Vec::new();
    let mut resolved: HashSet<String> = HashSet::new();

    loop {
        let mut current_wave: Vec<String> = dep_count
            .iter()
            .filter(|(node, count)| **count == 0 && !resolved.contains(node.as_str()))
            .map(|(node, _)| node.clone())
            .collect();

        if current_wave.is_empty() {
            break;
        }

        current_wave.sort();

        for node in &current_wave {
            resolved.insert(node.clone());
            if let Some(deps) = dependents.get(node) {
                for dep in deps {
                    if let Some(count) = dep_count.get_mut(dep) {
                        *count = count.saturating_sub(1);
                    }
                }
            }
        }

        waves.push(current_wave);
    }

    if resolved.len() < node_deps.len() {
        return Err(Error::graph_validation_failed(
            "circular dependency detected during eager resolution",
        ));
    }

    Ok(waves)
}
