#![cfg(feature = "opentelemetry")]

use fluxdi::opentelemetry_layer;
use opentelemetry::trace::noop::NoopTracer;

#[test]
fn builds_opentelemetry_layer_from_noop_tracer() {
    let _layer = opentelemetry_layer(NoopTracer::new());
}
