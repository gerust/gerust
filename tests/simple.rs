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
struct DefaultResource;

impl Resource for DefaultResource {
    fn content_types_accepted(&self) -> &'static [(mime::Mime, fn (&mut Self, request: &mut http::Request<hyper::Body>, response: &mut gerust::flow::DelayedResponse) -> ())] {
        &[]
    }

    fn content_types_provided(&self) -> &'static [ProvidedPair<Self>] {
        &[ProvidedPair(mime::TEXT_HTML, Self::to_html)]
    }
}

impl DefaultResource {
    fn to_html(&mut self, response: &mut gerust::flow::DelayedResponse) -> () {
        response.response_body().start_send(Ok("Hello, World!".into()));
    }
}

#[test]
fn test_without_accept_header() {
    let resource = DefaultResource::default();

    let req = http::request::Builder::new()
        .method(http::method::Method::GET)
        .body("".into())
        .unwrap();

    let response = helper::execute(resource, req);

    assert_eq!(response.status(), http::StatusCode::OK);
}

#[test]
fn test_with_accept_header() {
    let resource = DefaultResource::default();

    let req = http::request::Builder::new()
        .method(http::method::Method::GET)
        .header("Accept", "text/html")
        .body("".into())
        .unwrap();

    let response = helper::execute(resource, req);

    assert_eq!(response.status(), http::StatusCode::OK);
}
