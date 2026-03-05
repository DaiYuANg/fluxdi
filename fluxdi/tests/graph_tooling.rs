use fluxdi::{ErrorKind, GraphValidationIssueKind, Injector, Provider, Shared};

#[derive(Debug)]
struct ServiceA;
#[derive(Debug)]
struct ServiceB;

#[test]
fn dependency_graph_exports_dot_and_mermaid() {
    let injector = Injector::root();
    injector.provide::<ServiceA>(
        Provider::singleton(|_| Shared::new(ServiceA)).with_dependency::<ServiceB>(),
    );
    injector.provide::<ServiceB>(Provider::singleton(|_| Shared::new(ServiceB)));

    let graph = injector.dependency_graph();
    assert!(graph.nodes.len() >= 2);
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.from.contains("ServiceA") && edge.to.contains("ServiceB"))
    );

    let dot = graph.to_dot();
    assert!(dot.contains("digraph fluxdi"));
    assert!(dot.contains("ServiceA"));
    assert!(dot.contains("ServiceB"));

    let mermaid = graph.to_mermaid();
    assert!(mermaid.contains("graph TD"));
}

#[test]
fn validate_graph_reports_missing_dependency() {
    let injector = Injector::root();
    injector.provide::<ServiceA>(
        Provider::singleton(|_| Shared::new(ServiceA)).with_dependency::<ServiceB>(),
    );

    let report = injector.validate_graph();
    assert!(!report.is_valid());
    assert!(report.issues.iter().any(|issue| {
        issue.kind == GraphValidationIssueKind::MissingDependency
            && issue.message.contains("ServiceB")
    }));
}

#[test]
fn validate_graph_reports_cycles() {
    let injector = Injector::root();
    injector.provide::<ServiceA>(
        Provider::singleton(|_| Shared::new(ServiceA)).with_dependency::<ServiceB>(),
    );
    injector.provide::<ServiceB>(
        Provider::singleton(|_| Shared::new(ServiceB)).with_dependency::<ServiceA>(),
    );

    let report = injector.validate_graph();
    assert!(!report.is_valid());
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == GraphValidationIssueKind::CircularDependency)
    );
}

#[test]
fn try_validate_graph_returns_error_when_invalid() {
    let injector = Injector::root();
    injector.provide::<ServiceA>(
        Provider::singleton(|_| Shared::new(ServiceA)).with_dependency::<ServiceB>(),
    );

    let err = injector.try_validate_graph().unwrap_err();
    assert_eq!(err.kind, ErrorKind::GraphValidationFailed);
    assert!(err.message.contains("ServiceB"));
}

#[test]
fn validate_graph_handles_set_dependencies() {
    let injector = Injector::root();
    injector.provide_into_set::<ServiceB>(Provider::singleton(|_| Shared::new(ServiceB)));
    injector.provide::<ServiceA>(
        Provider::singleton(|_| Shared::new(ServiceA)).with_set_dependency::<ServiceB>(),
    );

    let report = injector.validate_graph();
    assert!(report.is_valid());

    let graph = injector.dependency_graph();
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.label.as_deref() == Some("all"))
    );
}
