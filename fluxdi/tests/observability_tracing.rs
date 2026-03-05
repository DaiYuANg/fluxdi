#![cfg(feature = "tracing")]

use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex},
};

use fluxdi::{
    EVENT_CIRCULAR_DEPENDENCY, Injector, Provider, SPAN_FACTORY_EXECUTE, SPAN_PROVIDE,
    SPAN_RESOLVE, Shared,
};
use tracing::{Event, Subscriber, field::Visit, span::Attributes};
use tracing_subscriber::{
    Registry,
    layer::{Context, Layer},
    prelude::*,
    registry::LookupSpan,
};

#[derive(Clone, Default)]
struct CapturedTelemetry {
    spans: Arc<Mutex<Vec<CapturedRecord>>>,
    events: Arc<Mutex<Vec<CapturedRecord>>>,
}

#[derive(Clone, Debug)]
struct CapturedRecord {
    name: String,
    fields: BTreeMap<String, String>,
}

#[derive(Default)]
struct FieldCollector {
    fields: BTreeMap<String, String>,
}

impl Visit for FieldCollector {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

struct CaptureLayer {
    sink: CapturedTelemetry,
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &tracing::span::Id, _ctx: Context<'_, S>) {
        let mut fields = FieldCollector::default();
        attrs.record(&mut fields);

        self.sink.spans.lock().unwrap().push(CapturedRecord {
            name: attrs.metadata().name().to_string(),
            fields: fields.fields,
        });
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = FieldCollector::default();
        event.record(&mut fields);

        self.sink.events.lock().unwrap().push(CapturedRecord {
            name: event.metadata().name().to_string(),
            fields: fields.fields,
        });
    }
}

#[test]
fn emits_core_observability_spans() {
    let sink = CapturedTelemetry::default();
    let subscriber = Registry::default().with(CaptureLayer { sink: sink.clone() });

    tracing::subscriber::with_default(subscriber, || {
        let injector = Injector::root();

        injector.provide::<String>(Provider::transient(|_| Shared::new("value".to_string())));

        let resolved = injector.try_resolve::<String>().unwrap();
        assert_eq!(resolved.as_str(), "value");
    });

    let spans = sink.spans.lock().unwrap();

    let provide = spans
        .iter()
        .find(|s| s.name == SPAN_PROVIDE)
        .expect("expected fluxdi.provide span");
    assert!(provide.fields.contains_key("type_name"));
    assert!(provide.fields.contains_key("scope"));

    assert!(spans.iter().any(|s| s.name == SPAN_RESOLVE));
    assert!(spans.iter().any(|s| s.name == SPAN_FACTORY_EXECUTE));
}

#[derive(Debug)]
struct CyclicA;

#[derive(Debug)]
struct CyclicB;

#[test]
fn emits_circular_dependency_event_field() {
    let sink = CapturedTelemetry::default();
    let subscriber = Registry::default().with(CaptureLayer { sink: sink.clone() });

    tracing::subscriber::with_default(subscriber, || {
        let injector = Injector::root();

        injector.provide::<CyclicA>(Provider::transient(|inj| {
            let _ = inj.try_resolve::<CyclicB>();
            Shared::new(CyclicA)
        }));

        injector.provide::<CyclicB>(Provider::transient(|inj| {
            let _ = inj.try_resolve::<CyclicA>();
            Shared::new(CyclicB)
        }));

        let _ = injector.try_resolve::<CyclicA>();
    });

    let events = sink.events.lock().unwrap();
    assert!(events.iter().any(|event| {
        event
            .fields
            .get("event")
            .map(|value| value == EVENT_CIRCULAR_DEPENDENCY)
            .unwrap_or(false)
    }));
}
