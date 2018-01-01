extern crate gerust;
extern crate mime;

extern crate http;
extern crate hyper;
extern crate futures;

use futures::sync::oneshot;
use gerust::resource::{Resource, ProvidedPair, Handles};
use gerust::Body;
use gerust::flow::Flow;

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
    fn to_html(&mut self, response: &mut gerust::flow::DelayedResponse) -> () {}
}


#[test]
fn default() {
    let resource = DefaultResource::default();

    let mut flow = gerust::flow::HttpFlow::new();

    let req = http::request::Builder::new()
        .method(http::method::Method::GET)
        .body("".into())
        .unwrap();

    let (sx, rx): (_, _) = oneshot::channel::<http::Response<Body>>();

    flow.execute(resource, req, sx);
}
