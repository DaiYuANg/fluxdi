#![cfg(feature = "actix")]

use actix_web::{App, HttpResponse, http::StatusCode, test, web};
use fluxdi::{
    Injector, Provider, Shared,
    actix::{Resolved, injector_data},
};

#[derive(Debug)]
struct Greeter {
    message: String,
}

async fn greet(Resolved(greeter): Resolved<Greeter>) -> HttpResponse {
    HttpResponse::Ok().body(greeter.message.clone())
}

#[actix_web::test]
async fn resolved_extractor_resolves_dependency_from_app_data() {
    let injector = Shared::new(Injector::root());
    injector.provide::<Greeter>(Provider::root(|_| {
        Shared::new(Greeter {
            message: "hello".to_string(),
        })
    }));

    let app = test::init_service(
        App::new()
            .app_data(injector_data(injector.clone()))
            .route("/", web::get().to(greet)),
    )
    .await;

    let request = test::TestRequest::get().uri("/").to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = test::read_body(response).await;
    assert_eq!(body.as_ref(), b"hello");
}

#[actix_web::test]
async fn resolved_extractor_returns_internal_error_when_state_missing() {
    let app = test::init_service(App::new().route("/", web::get().to(greet))).await;

    let request = test::TestRequest::get().uri("/").to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
