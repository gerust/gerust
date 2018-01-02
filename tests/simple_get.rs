extern crate gerust;
extern crate mime;
extern crate tokio_core;
extern crate http;
extern crate hyper;
extern crate futures;
extern crate futures_cpupool;

use futures::Sink;

use gerust::resource::{Resource, ProvidedPair};

mod helper;

#[derive(Default, Debug)]
struct GetResource;

impl Resource for GetResource {
    fn allowed_methods(&self) -> &'static [http::Method] {
        use http::method::Method;

        &[Method::GET, Method::HEAD]
    }

    fn content_types_provided(&self) -> &'static [ProvidedPair<Self>] {
        &[ProvidedPair(mime::TEXT_HTML, Self::to_html)]
    }
}

impl GetResource {
    fn to_html(&mut self, response: &mut gerust::flow::DelayedResponse) -> () {
        response.response_body().start_send(Ok("Hello, World!".into()));
    }
}

#[test]
fn test_without_accept_header() {
    let resource = GetResource::default();

    let req = http::request::Builder::new()
        .method(http::method::Method::GET)
        .body("".into())
        .unwrap();

    let response = helper::execute(resource, req);

    assert_eq!(response.status(), http::StatusCode::OK);
}

#[test]
fn test_with_accept_header() {
    let resource = GetResource::default();

    let req = http::request::Builder::new()
        .method(http::method::Method::GET)
        .header("Accept", "text/html")
        .body("".into())
        .unwrap();

    let response = helper::execute(resource, req);

    assert_eq!(response.status(), http::StatusCode::OK);
}

#[test]
fn test_head() {
    let resource = GetResource::default();

    let req = http::request::Builder::new()
        .method(http::method::Method::HEAD)
        .header("Content-Type", "text/plain")
        .body("".into())
        .unwrap();

    let response = helper::execute(resource, req);

    assert_eq!(response.status(), http::StatusCode::OK);
}

#[test]
fn test_not_acceptable_post() {
    let resource = GetResource::default();

    let req = http::request::Builder::new()
        .method(http::method::Method::POST)
        .header("Content-Type", "text/plain")
        .body("".into())
        .unwrap();

    let response = helper::execute(resource, req);

    assert_eq!(response.status(), http::StatusCode::METHOD_NOT_ALLOWED);
}

